use crate::assets::{CSS_STR, JS_STR};
use crate::console::ConsoleHandle;
use crate::errors::{IOError, RenderError};
use crate::templates::TemplateName;
use crate::types::*;
use crate::utils::{
    copy_file, get_entries_in_dir, get_files_in_dir_recursive, md_to_html, read_template,
    remove_path,
};
use chrono::prelude::*;
use color_eyre::eyre::Context;
use color_eyre::{eyre::eyre, Result};
use futures::future::{try_join3, try_join_all};
use futures::stream::FuturesUnordered;
use futures::FutureExt;
use handlebars::Handlebars;
use inflector::Inflector;
use inquire::Confirm;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use serde_json::json;
use std::cmp::Reverse;
use std::{path::Path, sync::mpsc, time::Duration};
use strum::IntoEnumIterator;
use tokio::fs::{create_dir, metadata, read_to_string, write};
use tower_http::services::ServeDir;
use tower_livereload::LiveReloadLayer;

pub async fn run_new(console: &mut ConsoleHandle, root_dir: &Path) -> Result<()> {
    let start = Utc::now();
    let assets_dir = root_dir.join("assets");
    let posts_dir = root_dir.join("posts");
    let templates_dir = root_dir.join("templates");
    let name = root_dir
        .file_stem()
        .expect("Could not derive name from path")
        .to_string_lossy()
        .to_string();
    let date = Utc::now().format("%Y-%m-%d");

    create_dir(root_dir).await?;
    try_join3(
        create_dir(&assets_dir),
        create_dir(&posts_dir),
        create_dir(&templates_dir),
    )
    .await?;

    try_join_all(
        [
            write(
                root_dir.join("index.md"),
                format!("# {}\n", name.to_title_case()),
            ),
            write(root_dir.join("about.md"), "# About\n".to_owned()),
            write(
                assets_dir.join("style.css"),
                CSS_STR.to_string().trim_start().to_owned(),
            ),
            write(
                assets_dir.join("script.js"),
                JS_STR.to_string().trim_start().to_owned(),
            ),
            write(
                posts_dir.join(format!("{date}-hello-world.md")),
                format!(
                    r"<!--metadata
date = {date}
-->

# Hello, World!
"
                )
                .to_owned(),
            ),
        ]
        .into_iter()
        .chain(TemplateName::iter().map(|n| {
            write(
                templates_dir.join(format!("{n}.hbs")),
                n.template_str().trim_start().to_owned(),
            )
        })),
    )
    .await?;

    console.log_elapsed((Utc::now() - start).num_milliseconds())?;

    Ok(())
}

async fn build_page<'a>(
    h: &Handlebars<'a>,
    name: String,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<()> {
    let path = input_dir.join(&name);
    let md_str = read_to_string(&path)
        .await
        .wrap_err(IOError::Read { path: path.into() })?;
    let (_, title, contents) = md_to_html(&md_str);
    let out_name = name.replace(".md", ".html");
    let name_no_ext = name.strip_suffix(".md").unwrap_or(&name);
    let template_name = if h.has_template(name_no_ext) {
        name_no_ext
    } else {
        "page"
    };
    let out = h
        .render(
            template_name,
            &json!(EntityArgs {
                path: &[Breadcrumb {
                    name: &title,
                    link: &out_name,
                }],
                title: &title,
                contents: &contents
            }),
        )
        .wrap_err(RenderError {
            path: name.clone().into(),
            template_name: template_name.to_string(),
        })?;

    write(output_dir.join(out_name), out)
        .await
        .wrap_err(IOError::Create {
            path: name.clone().into(),
        })?;
    Ok(())
}

async fn build_entity<'a>(
    h: &Handlebars<'a>,
    name: &str,
    collection_name: &str,
    breadcrumbs: &'a [Breadcrumb<'a>],
    input_dir: &Path,
    output_dir: &Path,
) -> Result<Entity> {
    let path = input_dir.join(name);
    let md_str = read_to_string(&path)
        .await
        .wrap_err(IOError::Read { path: path.clone() })?;
    let (metadata, title, contents) = md_to_html(&md_str);
    let date_str = metadata
        .as_ref()
        .map(|m| m.date)
        .flatten()
        .map(|dt| dt.date)
        .flatten()
        .map(|d| d.to_string())
        .unwrap_or(Utc::now().date_naive().format("%Y-%m-%d").to_string());
    let created_at = NaiveDate::parse_from_str(date_str.as_str(), "%Y-%m-%d")
        .unwrap_or(Utc::now().date_naive())
        .format("%b %d, %Y")
        .to_string();
    let shortname = metadata
        .as_ref()
        .map(|m| m.shortname.clone())
        .flatten()
        .unwrap_or(created_at.clone());
    let slug = metadata
        .as_ref()
        .map(|m| m.slug.clone())
        .flatten()
        .map(|slug| format!("{}.html", slug.trim().replace(" ", "_")))
        .unwrap_or(name.replace(".md", ".html"));
    let link = format!("{collection_name}/{slug}");
    let template_name = collection_name
        .strip_suffix("s")
        .unwrap_or(&collection_name);
    let out = h
        .render(
            template_name,
            &json!(EntityArgs {
                path: &[
                    breadcrumbs,
                    &[Breadcrumb {
                        name: &shortname,
                        link: &link,
                    }]
                ]
                .concat(),
                title: &title,
                contents: &contents,
            }),
        )
        .wrap_err(RenderError {
            path: path.clone(),
            template_name: template_name.to_string(),
        })?;

    write(output_dir.join(&slug), out)
        .await
        .wrap_err(IOError::Create { path })?;

    Ok(Entity {
        filename: slug,
        created_at_iso: date_str,
        created_at,
        title,
    })
}

pub async fn build_entities<'a>(
    h: &Handlebars<'a>,
    name: String,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<()> {
    let title_case = name.to_title_case();
    let breadcrumbs = &[Breadcrumb {
        name: &title_case,
        link: &name,
    }];

    let entities_input_dir = input_dir.join(&name);
    let entities_output_dir = output_dir.join(&name);
    let entries = get_entries_in_dir(&entities_input_dir)
        .await
        .wrap_err(IOError::Read {
            path: name.clone().into(),
        })?;
    let mut entities = try_join_all(
        entries
            .iter()
            .filter(|(filename, metadata, ..)| metadata.is_file() && filename.ends_with(".md"))
            .map(|(filename, ..)| {
                build_entity(
                    &h,
                    filename,
                    &name,
                    breadcrumbs,
                    &entities_input_dir,
                    &entities_output_dir,
                )
            }),
    )
    .await
    .wrap_err(eyre!("Failed to build entity in collection \"{}\"", name))?;

    entities.sort_by_key(|e| Reverse(e.created_at_iso.clone()));

    let entity_index_path = entities_output_dir.join("index.html");
    let out = h
        .render(
            &name,
            &json!(EntitiesArgs {
                path: breadcrumbs,
                title: &title_case,
                entities,
            }),
        )
        .wrap_err(RenderError {
            path: entity_index_path.clone(),
            template_name: name,
        })?;
    write(&entity_index_path, out)
        .await
        .wrap_err(IOError::Create {
            path: entity_index_path,
        })?;
    Ok(())
}

pub async fn run_build(
    console: &mut ConsoleHandle,
    input_dir: &Path,
    output_dir: &Path,
    should_confirm: bool,
) -> Result<()> {
    let mut start = Utc::now();

    // check that input dir exists
    metadata(&input_dir)
        .await
        .wrap_err(eyre!("\"{}\" does not exist", input_dir.display()))?;

    // confirm output dir overwrite if it exists
    if let Ok(metadata) = metadata(&output_dir).await {
        if metadata.is_file() {
            return Err(eyre!("{} is already a file", output_dir.display()));
        }

        if should_confirm {
            let ans = Confirm::new(
                format!("{} already exists. Continue?", output_dir.display()).as_ref(),
            )
            .with_default(false)
            .with_help_message("All contents will be overwritten except .git/")
            .prompt()?;
            if !ans {
                return Ok(());
            };

            start = Utc::now();
        }

        let output_entries = get_entries_in_dir(&output_dir)
            .await
            .wrap_err(IOError::Read {
                path: output_dir.into(),
            })?;
        let reserved_output_filenames = vec![".git", "CNAME"];
        try_join_all(
            output_entries
                .into_iter()
                .filter_map(|(name, metadata, path)| {
                    if reserved_output_filenames.contains(&name.as_str()) {
                        None
                    } else {
                        Some(remove_path(metadata, path))
                    }
                }),
        )
        .await
        .wrap_err(eyre!(
            "Failed to remove contents of \"{}\"",
            output_dir.display()
        ))?;
    } else {
        create_dir(&output_dir).await.wrap_err(IOError::Create {
            path: output_dir.into(),
        })?;
    }

    console.log("Building...")?;

    // get pages and collections
    let reserved_filenames = vec!["README.md", "readme.md"];
    let reserved_dirnames = vec![".git", "assets", "templates"];
    let input_entries = get_entries_in_dir(input_dir)
        .await
        .wrap_err(IOError::Read {
            path: input_dir.into(),
        })?;
    let page_names = input_entries.iter().filter_map(|(name, metadata, _)| {
        if metadata.is_file()
            && name.ends_with(".md")
            && !reserved_filenames.contains(&name.as_str())
        {
            Some(name)
        } else {
            None
        }
    });
    let collection_names = input_entries.iter().filter_map(|(name, metadata, _)| {
        if metadata.is_dir() && !reserved_dirnames.contains(&name.as_str()) {
            Some(name)
        } else {
            None
        }
    });

    // create root level dirs assets and collections
    let assets_output_dir = output_dir.join("assets");
    try_join_all(
        [create_dir(assets_output_dir.clone())].into_iter().chain(
            collection_names
                .clone()
                .map(|n| create_dir(output_dir.join(n))),
        ),
    )
    .await
    .wrap_err(eyre!("Failed to create output directories"))?;

    // get asset file paths
    let assets_input_dir = input_dir.join("assets");
    let assets_file_paths = get_files_in_dir_recursive(&assets_input_dir);

    // read and register templates
    let templates_input_dir = input_dir.join("templates");
    let template_entries =
        get_entries_in_dir(&templates_input_dir)
            .await
            .wrap_err(IOError::Read {
                path: templates_input_dir.clone().into(),
            })?;
    let templates = try_join_all(
        template_entries
            .iter()
            .map(|(n, ..)| read_template(n.to_string(), &templates_input_dir)),
    )
    .await
    .wrap_err("Failed to read files in \"templates/\"")?;
    let mut h = Handlebars::new();
    for (name, template) in templates {
        h.register_template_string(&name, template)?;
    }

    // build
    let build_actions = FuturesUnordered::new();
    // build assets
    for file_path in assets_file_paths {
        build_actions.push(
            copy_file(
                assets_input_dir.join(&file_path),
                assets_output_dir.join(file_path),
            )
            .boxed_local(),
        );
    }
    // build collections
    for name in collection_names {
        build_actions
            .push(build_entities(&h, name.to_string(), input_dir, output_dir).boxed_local());
    }
    // build pages
    for name in page_names {
        build_actions.push(build_page(&h, name.to_string(), input_dir, output_dir).boxed_local())
    }
    try_join_all(build_actions).await?;

    console.log_elapsed((Utc::now() - start).num_milliseconds())?;

    Ok(())
}

pub async fn run_watch(
    console: &mut ConsoleHandle,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<()> {
    run_build(console, input_dir, output_dir, false).await?;

    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();
    let output_dir_copy = output_dir.to_path_buf();
    let address = "0.0.0.0:3030";
    tokio::spawn(async move {
        let app = axum::Router::new()
            .nest_service("/", ServeDir::new(output_dir_copy))
            .layer(livereload);
        let _ = axum::Server::bind(&address.parse().unwrap())
            .serve(app.into_make_service())
            .await;
        panic!("Unexpected server exit");
    });
    console.set_address(address)?;

    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(250), None, tx)?;
    debouncer
        .watcher()
        .watch(input_dir, RecursiveMode::Recursive)?;

    while let Ok(res) = rx.recv() {
        match res {
            Ok(_) => {
                if let Err(report) = run_build(console, input_dir, output_dir, false).await {
                    console.log_report(report)?;
                } else {
                    reloader.reload();
                }
            }
            Err(e) => panic!("{:?}", e),
        }
    }

    Ok(())
}

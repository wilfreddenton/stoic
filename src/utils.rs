use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use crate::types::*;
use crate::utils::{
    copy_file, get_entries_in_dir, get_files_in_dir_recursive, md_to_html, read_template,
    remove_path,
};
use chrono::prelude::*;
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
use std::error::Error;
use std::{path::Path, sync::mpsc, time::Duration};
use strum::IntoEnumIterator;
use tokio::fs::{create_dir, metadata, read_dir, read_to_string, write};

pub async fn run_new(root_dir: &Path) -> Result<(), Box<dyn Error>> {
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

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

async fn build_index<'a>(
    h: &Handlebars<'a>,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join("index.md")).await?;
    let (_, title, contents) = md_to_html(&md_str);
    let test = &json!(IndexArgs {
        title: &title,
        contents: &contents,
    });
    let out = h.render("index", test)?;
    write(output_dir.join("index.html"), out).await?;
    Ok(())
}

async fn build_page<'a>(
    h: &Handlebars<'a>,
    name: String,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join(&name)).await?;
    let (_, title, contents) = md_to_html(&md_str);
    let out_name = name.replace(".md", ".html");
    let out = h.render(
        "page",
        &json!(EntityArgs {
            path: &[Breadcrumb {
                name: &title,
                link: &out_name,
            }],
            title: &title,
            contents: &contents
        }),
    )?;

    write(output_dir.join(out_name), out).await?;
    Ok(())
}

async fn build_entity<'a>(
    h: &Handlebars<'a>,
    name: &str,
    collection_name: &str,
    breadcrumbs: &'a [Breadcrumb<'a>],
    input_dir: &Path,
    output_dir: &Path,
) -> Result<Entity, Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join(name)).await?;
    let (metadata, title, contents) = md_to_html(&md_str);
    let date_str = metadata
        .map(|m| m.date)
        .flatten()
        .map(|dt| dt.date)
        .flatten()
        .map(|d| d.to_string())
        .unwrap_or(Utc::now().date_naive().format("%Y-%m-%d").to_string());
    let created_at = NaiveDate::parse_from_str(date_str.as_str(), "%Y-%m-%d")
        .unwrap()
        .format("%b %d, %Y")
        .to_string();
    let out_name = name.replace(".md", ".html");
    let link = format!("{collection_name}/{out_name}");
    let out = h.render(
        collection_name
            .strip_suffix("s")
            .unwrap_or(&collection_name),
        &json!(EntityArgs {
            path: &[
                breadcrumbs,
                &[Breadcrumb {
                    name: &created_at,
                    link: &link,
                }]
            ]
            .concat(),
            title: &title,
            contents: &contents,
        }),
    )?;

    write(output_dir.join(&out_name), out).await?;

    Ok(Entity {
        filename: out_name,
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
) -> Result<(), Box<dyn Error>> {
    let title_case = name.to_title_case();
    let breadcrumbs = &[Breadcrumb {
        name: &title_case,
        link: &name,
    }];

    let entities_input_dir = input_dir.join(&name);
    let entities_output_dir = output_dir.join(&name);
    let r_dir = read_dir(&entities_input_dir).await?;
    let entries = get_entries_in_dir(r_dir).await?;
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
            })
            .collect::<Vec<_>>(),
    )
    .await?;

    entities.sort_by_key(|e| Reverse(e.created_at_iso.clone()));

    let out = h.render(
        &name,
        &json!(EntitiesArgs {
            path: breadcrumbs,
            title: &title_case,
            entities,
        }),
    )?;
    write(entities_output_dir.join("index.html"), out).await?;
    Ok(())
}

pub async fn run_build(
    input_dir: &Path,
    output_dir: &Path,
    should_confirm: bool,
) -> Result<(), Box<dyn Error>> {
    let mut start = Utc::now();

    // check that input dir exists
    metadata(&input_dir).await?;

    // confirm output dir overwrite if it exists
    if let Ok(metadata) = metadata(&output_dir).await {
        if metadata.is_file() {
            return Err(format!("{} is a file", output_dir.display()).into());
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

        let r_dir = read_dir(&output_dir).await?;
        let output_entries = get_entries_in_dir(r_dir).await?;
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
        .await?;
    } else {
        create_dir(&output_dir).await?;
    }

    // get pages and collections
    let reserved_filenames = vec!["README.md", "readme.md", "index.md"];
    let reserved_dirnames = vec![".git", "assets", "templates"];
    let input_r_dir = read_dir(input_dir).await?;
    let input_entries = get_entries_in_dir(input_r_dir).await?;
    let page_names = input_entries
        .iter()
        .filter_map(|(name, metadata, _)| {
            if metadata.is_file()
                && name.ends_with(".md")
                && !reserved_filenames.contains(&name.as_str())
            {
                Some(name)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let collection_names = input_entries
        .iter()
        .filter_map(|(name, metadata, _)| {
            if metadata.is_dir() && !reserved_dirnames.contains(&name.as_str()) {
                Some(name)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // create root level dirs assets and collections
    let assets_output_dir = output_dir.join("assets");
    try_join_all(
        [create_dir(assets_output_dir.clone())].into_iter().chain(
            collection_names
                .iter()
                .map(|n| create_dir(output_dir.join(n))),
        ),
    )
    .await?;

    // get asset file paths
    let assets_input_dir = input_dir.join("assets");
    let assets_file_paths = get_files_in_dir_recursive(&assets_input_dir);

    // read and register templates
    let templates_input_dir = input_dir.join("templates");
    let templates_input_r_dir = read_dir(&templates_input_dir).await?;
    let template_entries = get_entries_in_dir(templates_input_r_dir).await?;
    let templates = try_join_all(
        template_entries
            .iter()
            .map(|(n, ..)| read_template(n.to_string(), &templates_input_dir)),
    )
    .await?;
    let mut h = Handlebars::new();
    for (name, template) in templates {
        h.register_template_string(&name, template)?;
    }

    // build
    let build_actions = FuturesUnordered::new();
    // build index
    build_actions.push(build_index(&h, input_dir, output_dir).boxed_local());
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

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

pub async fn run_watch(input_dir: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, tx)?;
    debouncer
        .watcher()
        .watch(input_dir, RecursiveMode::Recursive)?;

    println!("watching: {}", input_dir.display());
    println!("building: {}", output_dir.display());

    run_build(input_dir, output_dir, false).await?;
    while let Ok(res) = rx.recv() {
        match res {
            Ok(_) => {
                println!("change detected; building...");
                if let Err(e) = run_build(input_dir, output_dir, false).await {
                    println!("{}", e);
                }
            }
            Err(e) => panic!("{:?}", e),
        }
    }

    Ok(())
}

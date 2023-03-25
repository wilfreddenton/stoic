use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use crate::types::*;
use crate::utils::{get_dir_paths, get_entries_in_dir, md_to_html, read_template, remove_path};
use chrono::prelude::*;
use futures::future::{try_join3, try_join_all};
use futures::FutureExt;
use handlebars::Handlebars;
use inflector::Inflector;
use inquire::Confirm;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use serde_json::json;
use std::error::Error;
use std::{path::Path, sync::mpsc, time::Duration};
use strum::IntoEnumIterator;
use tokio::fs::{copy, create_dir, metadata, read_dir, read_to_string, write};

pub async fn run_new(root_dir: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    let mut build_template_actions = TemplateName::iter()
        .map(|n| {
            write(
                templates_dir.join(format!("{n}.hbs")).to_path_buf(),
                n.template_str().trim_start().to_owned(),
            )
        })
        .collect::<Vec<_>>();

    build_template_actions.extend([
        write(
            root_dir.join("index.md").to_path_buf(),
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
            format!(r"<!--metadata
date = {date}
-->

# Hello, World!
").to_owned(),
        ),
    ]);

    try_join_all(build_template_actions).await?;

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

async fn build_index<'a>(
    h: &Handlebars<'a>,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let md_str = read_to_string(input_dir.join("index.md")).await?;
    let (_, title, contents) = md_to_html(md_str.to_owned());
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
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let md_str = read_to_string(input_dir.join(&name)).await?;
    let (_, title, contents) = md_to_html(md_str.to_owned());
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
) -> Result<Entity, Box<dyn Error + Send + Sync>> {
    let md_str = read_to_string(input_dir.join(name)).await?;
    let (metadata, title, contents) = md_to_html(md_str.to_owned());
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
    let out_name = &name.replace(".md", ".html");
    let link = format!("{collection_name}/{out_name}");
    let mut breadcrumbs_vec = breadcrumbs.to_vec();
    breadcrumbs_vec.push(Breadcrumb {
        name: &created_at,
        link: &link,
    });
    let out = h.render(
        collection_name
            .strip_suffix("s")
            .unwrap_or(&collection_name),
        &json!(EntityArgs {
            path: &breadcrumbs_vec[..],
            title: &title,
            contents: &contents,
        }),
    )?;

    write(output_dir.join(&out_name), out).await?;

    Ok(Entity {
        filename: out_name.to_owned(),
        created_at_iso: date_str,
        created_at,
        title,
    })
}

pub async fn build_entities<'a>(
    h: &Handlebars<'a>,
    name: &str,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let title_case = name.to_title_case();
    let breadcrumbs = &[Breadcrumb {
        name: &title_case,
        link: &name,
    }];
    let mut entities_args = EntitiesArgs {
        path: breadcrumbs,
        title: &title_case,
        entities: Vec::new(),
    };
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
                    name,
                    breadcrumbs,
                    &entities_input_dir,
                    &entities_output_dir,
                )
            })
            .collect::<Vec<_>>(),
    )
    .await?;

    entities.sort_by_key(|e| std::cmp::Reverse(e.created_at_iso.to_owned()));

    entities_args.entities = entities;

    let out = h.render(&name, &json!(entities_args))?;
    write(entities_output_dir.join("index.html"), out).await?;
    Ok(())
}

pub async fn run_build(
    input_dir: &Path,
    output_dir: &Path,
    should_confirm: bool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut start = Utc::now();
    metadata(&input_dir).await?;
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
            }

            start = Utc::now();
        }

        let r_dir = read_dir(&output_dir).await?;
        let output_entries = get_entries_in_dir(r_dir).await?;
        for (name, metadata, path) in output_entries {
            match name.as_str() {
                ".git" => continue,
                "CNAME" => continue,
                _ => (),
            }

            remove_path(metadata, path).await?;
        }
    } else {
        create_dir(&output_dir).await?;
    }

    let reserved_filenames = vec!["README.md", "readme.md", "index.md"];
    let reserved_dirnames = vec![".git", "assets", "templates"];
    let input_r_dir = read_dir(input_dir).await?;
    let input_entries = get_entries_in_dir(input_r_dir).await?;
    let mut page_names: Vec<String> = vec![];
    let mut collection_names: Vec<String> = vec![];
    for (name, metadata, _) in input_entries {
        if metadata.is_file() {
            if name.ends_with(".md") && !reserved_filenames.contains(&name.as_str()) {
                page_names.push(name);
            }
        } else {
            if !reserved_dirnames.contains(&name.as_str()) {
                collection_names.push(name);
            }
        }
    }

    let assets_output_dir = output_dir.join("assets");
    let mut create_dir_actions = collection_names
        .iter()
        .map(|n| create_dir(output_dir.join(n)))
        .collect::<Vec<_>>();
    create_dir_actions.extend([create_dir(assets_output_dir.to_owned())]);
    try_join_all(create_dir_actions).await?;

    let assets_input_dir = input_dir.join("assets");
    let (dir_paths, file_paths) = get_dir_paths(&assets_input_dir)?;
    try_join_all(
        dir_paths
            .into_iter()
            .map(|p| create_dir(assets_output_dir.join(p))),
    )
    .await?;
    try_join_all(file_paths.into_iter().map(|p| {
        copy(
            assets_input_dir.join(p.to_owned()),
            assets_output_dir.join(p.to_owned()),
        )
    }))
    .await?;

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

    let mut build_actions = vec![build_index(&h, input_dir, output_dir).boxed()];
    build_actions.extend(
        page_names
            .iter()
            .map(|name| build_page(&h, name.to_owned(), input_dir, output_dir).boxed())
            .collect::<Vec<_>>(),
    );
    build_actions.extend(
        collection_names
            .iter()
            .map(|name| build_entities(&h, name, input_dir, output_dir).boxed())
            .collect::<Vec<_>>(),
    );
    try_join_all(build_actions).await?;

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

pub async fn run_watch(
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, tx)?;
    debouncer
        .watcher()
        .watch(input_dir, RecursiveMode::Recursive)?;

    println!("watching: {}", input_dir.display());
    println!("building: {}", output_dir.display());
    while let Ok(res) = rx.recv() {
        match res {
            Ok(_) => {
                println!("change detected; building...");
                run_build(input_dir, output_dir, false).await?
            }
            Err(e) => panic!("{:?}", e),
        }
    }

    Ok(())
}

use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use crate::utils::{get_dir_paths, get_files_in_dir, md_to_html, read_template, remove_path};
use crate::types::*;
use chrono::prelude::*;
use futures::future::{try_join, try_join4, try_join_all};
use futures::FutureExt;
use handlebars::Handlebars;
use inquire::Confirm;
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use regex::Regex;
use serde_json::json;
use std::error::Error;
use std::{path::Path, time::Duration, sync::mpsc};
use strum::IntoEnumIterator;
use tokio::fs::{copy, create_dir, metadata, read_dir, read_to_string, write};

pub async fn run_new(blog_dir: &Path) -> Result<(), Box<dyn Error>> {
    let start = Utc::now();
    let assets_dir = blog_dir.join("assets");
    let pages_dir = blog_dir.join("pages");
    let posts_dir = blog_dir.join("posts");
    let templates_dir = blog_dir.join("template");
    let name = blog_dir
        .file_stem()
        .expect("Could not derive name from path")
        .to_string_lossy()
        .to_string();
    let date = Utc::now().format("%Y-%m-%d");

    create_dir(blog_dir).await?;
    try_join4(
        create_dir(&assets_dir),
        create_dir(&pages_dir),
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
            blog_dir.join("index.md").to_path_buf(),
            format!("# {name}\n"),
        ),
        write(
            assets_dir.join("style.css"),
            CSS_STR.to_string().trim_start().to_owned(),
        ),
        write(
            assets_dir.join("script.js"),
            JS_STR.to_string().trim_start().to_owned(),
        ),
        write(pages_dir.join("about.md"), "# About\n".to_owned()),
        write(
            posts_dir.join(format!("{date}-hello-world.md")),
            "# Hello, World!\n".to_owned(),
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
) -> Result<(), Box<dyn Error>> {
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
    name: &str,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join(name)).await?;
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

async fn build_pages<'a>(
    h: &Handlebars<'a>,
    pages_input_dir: &Path,
    pages_output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"^[A-Za-z0-9\-]+\.md$")?;
    let r_dir = read_dir(&pages_input_dir).await?;
    let page_entries = get_files_in_dir(r_dir).await?;
    for (name, metadata, ..) in page_entries {
        if !(metadata.is_file() && re.is_match(&name)) {
            continue;
        }

        build_page(h, &name, &pages_input_dir, &pages_output_dir).await?;
    }
    Ok(())
}

async fn build_entity<'a>(
    h: &Handlebars<'a>,
    name: &str,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<Entity, Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join(name)).await?;
    let re = Regex::new(r"^(?P<date>\d{4}-\d{2}-\d{2})-(?P<title>[A-Za-z0-9\-]+)\.md$")?;
    let caps = re
        .captures(&name)
        .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "captures"))?;
    let date_str = caps["date"].to_string();
    let dt = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
    let (_, title, contents) = md_to_html(md_str.to_owned());
    let out_name = name.replace(".md", ".html");
    let created_at = dt.format("%b %d, %Y").to_string();
    let out = h.render(
        "post",
        &json!(EntityArgs {
            path: &[
                Breadcrumb {
                    name: "Posts",
                    link: "posts/",
                },
                Breadcrumb {
                    name: &created_at,
                    link: &format!("posts/{out_name}"),
                }
            ],
            title: &title,
            contents: &contents,
        }),
    )?;

    write(output_dir.join(&out_name), out).await?;

    Ok(Entity {
        filename: out_name,
        created_at,
        title,
    })
}

pub async fn build_entities<'a>(
    h: &Handlebars<'a>,
    posts_input_dir: &Path,
    posts_output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut entities_args = EntitiesArgs {
        path: &[Breadcrumb {
            name: "Posts",
            link: "posts",
        }],
        title: "Posts",
        entities: Vec::new(),
    };
    let r_dir = read_dir(&posts_input_dir).await?;
    let mut post_entries = get_files_in_dir(r_dir).await?;
    post_entries.sort_by_key(|(.., path)| path.to_owned());
    for (name, metadata, ..) in post_entries {
        if !metadata.is_file() {
            continue;
        }

        let post = build_entity(&h, &name, &posts_input_dir, &posts_output_dir).await?;
        entities_args.entities.insert(0, post)
    }

    let out = h.render("posts", &json!(entities_args))?;
    write(posts_output_dir.join("index.html"), out).await?;
    Ok(())
}

pub async fn run_build(
    input_dir: &Path,
    output_dir: &Path,
    should_confirm: bool,
) -> Result<(), Box<dyn Error>> {
    let mut start = Utc::now();
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
        let output_entries = get_files_in_dir(r_dir).await?;
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

    let assets_input_dir = input_dir.join("assets");
    let pages_input_dir = input_dir.join("pages");
    let posts_input_dir = input_dir.join("posts");
    let template_input_dir = input_dir.join("templates");
    let assets_output_dir = output_dir.join("assets");
    let posts_output_dir = output_dir.join("posts");
    try_join(
        create_dir(&posts_output_dir),
        create_dir(&assets_output_dir),
    )
    .await?;

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

    let mut h = Handlebars::new();
    let templates =
        try_join_all(TemplateName::iter().map(|n| read_template(n, &template_input_dir))).await?;
    for (name, template) in templates {
        h.register_template_string(&name, template)?;
    }

    try_join_all([
        build_index(&h, input_dir, output_dir).boxed(),
        build_pages(&h, &pages_input_dir, output_dir).boxed(),
        build_entities(&h, &posts_input_dir, &posts_output_dir).boxed(),
    ])
    .await?;

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

pub async fn run_watch(input_dir: &Path, output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_secs(1), None, tx)?;
    debouncer.watcher().watch(input_dir, RecursiveMode::Recursive)?;

    println!("watching: {}", input_dir.display());
    println!("building: {}", output_dir.display());
    while let Ok(res) = rx.recv() {
        match res {
            Ok(_) => {
                println!("change detected; building...");
                run_build(input_dir, output_dir, false).await?
            },
            Err(e) => panic!("{:?}", e),
        }
    }

    Ok(())
}

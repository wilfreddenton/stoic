use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use crate::utils::{get_dir_paths, md_to_html};
use chrono::prelude::*;
use futures::future::{try_join, try_join4, try_join_all};
use handlebars::Handlebars;
use inquire::Confirm;
use pulldown_cmark::Options;
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use tokio::fs::{
    copy, create_dir, metadata, read_dir, read_to_string, remove_dir_all, remove_file, write,
    ReadDir,
};

#[derive(Serialize)]
struct Post {
    filename: String,
    title: String,
    created_at: String,
}

#[derive(Serialize)]
struct Breadcrumb<'a> {
    name: &'a str,
    link: &'a str,
}

#[derive(Serialize)]
struct IndexArgs<'a> {
    title: &'a str,
    contents: &'a str,
}

#[derive(Serialize)]
struct PageArgs<'a> {
    path: &'a [Breadcrumb<'a>],
    title: &'a str,
    contents: &'a str,
}

#[derive(Serialize)]
struct PostsArgs<'a> {
    path: &'a [Breadcrumb<'a>],
    title: &'a str,
    posts: Vec<Post>,
}

#[derive(Serialize)]
struct PostArgs<'a> {
    path: &'a [Breadcrumb<'a>],
    title: &'a str,
    contents: &'a str,
}

fn new_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    return options;
}

async fn get_files_in_dir(
    mut dir: ReadDir,
) -> Result<Vec<(String, Metadata, PathBuf)>, Box<dyn Error>> {
    let mut entries = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push((name, metadata, entry.path()));
    }
    Ok(entries)
}

async fn build_post<'a>(
    h: &Handlebars<'a>,
    name: &str,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<Post, Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join(name)).await?;
    let re = Regex::new(r"^(?P<date>\d{4}-\d{2}-\d{2})-(?P<title>[A-Za-z0-9\-]+)\.md$")?;
    let caps = re
        .captures(&name)
        .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "captures"))?;
    let date_str = caps["date"].to_string();
    let dt = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;
    let (title, contents) = md_to_html(md_str.to_owned(), new_options());
    let out_name = name.replace(".md", ".html");
    let created_at = dt.format("%b %d, %Y").to_string();
    let out = h.render(
        "post",
        &json!(PostArgs {
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

    Ok(Post {
        filename: out_name,
        created_at,
        title,
    })
}

async fn build_page<'a>(
    h: &Handlebars<'a>,
    name: &str,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join(name)).await?;
    let (title, contents) = md_to_html(md_str.to_owned(), new_options());
    let out_name = name.replace(".md", ".html");
    let out = h.render(
        "page",
        &json!(PageArgs {
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

async fn remove_path(metadata: Metadata, path: PathBuf) -> Result<(), std::io::Error> {
    if metadata.is_file() {
        remove_file(path).await
    } else {
        remove_dir_all(path).await
    }
}

async fn read_template(name: &TemplateName, dir: &Path) -> Result<String, Box<dyn Error>> {
    Ok(read_to_string(dir.join(format!("{name}.hbs")))
        .await?
        .split("\n")
        .map(|l| l.trim())
        .collect::<Vec<&str>>()
        .join("\n"))
}

async fn build_index<'a>(
    h: &Handlebars<'a>,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    let md_str = read_to_string(input_dir.join("index.md")).await?;
    let (title, contents) = md_to_html(md_str.to_owned(), new_options());
    let test = &json!(IndexArgs {
        title: &title,
        contents: &contents,
    });
    let out = h.render("index", test)?;
    write(output_dir.join("index.html"), out).await?;
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
    let assets_output_dir = output_dir.join("assets");
    let pages_input_dir = input_dir.join("pages");
    let posts_input_dir = input_dir.join("posts");
    let posts_output_dir = output_dir.join("posts");
    let template_input_dir = input_dir.join("templates");
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
    for name in TemplateName::iter() {
        let template = read_template(&name, &template_input_dir).await?;
        h.register_template_string(&name.to_string(), template)?;
    }

    build_index(&h, input_dir, output_dir).await?;

    let re = Regex::new(r"^[A-Za-z0-9\-]+\.md$")?;
    let mut r_dir = read_dir(&pages_input_dir).await?;
    let page_entries = get_files_in_dir(r_dir).await?;
    for (name, metadata, ..) in page_entries {
        if !(metadata.is_file() && re.is_match(&name)) {
            continue;
        }

        build_page(&h, &name, &pages_input_dir, output_dir).await?;
    }

    let mut posts_args = PostsArgs {
        path: &[Breadcrumb {
            name: "Posts",
            link: "posts",
        }],
        title: "Posts",
        posts: Vec::new(),
    };
    r_dir = read_dir(&posts_input_dir).await?;
    let mut post_entries = get_files_in_dir(r_dir).await?;
    post_entries.sort_by_key(|(.., path)| path.to_owned());
    for (name, metadata, ..) in post_entries {
        if !(metadata.is_file() && re.is_match(&name)) {
            continue;
        }

        let post = build_post(&h, &name, &posts_input_dir, &posts_output_dir).await?;
        posts_args.posts.insert(0, post)
    }

    let out = h.render("posts", &json!(posts_args))?;
    write(posts_output_dir.join("index.html"), out).await?;

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

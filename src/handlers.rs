use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use crate::utils::{create_file, get_dir_paths, md_to_html};
use chrono::prelude::*;
use futures::future::{try_join, try_join4, try_join_all};
use handlebars::Handlebars;
use inquire::Confirm;
use pulldown_cmark::Options;
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fs::{self, DirEntry, Metadata, ReadDir};
use std::iter::FilterMap;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use tokio::fs as afs;

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

fn get_files_in_dir(
    dir: ReadDir,
) -> FilterMap<ReadDir, fn(Result<DirEntry, std::io::Error>) -> Option<(String, Metadata, PathBuf)>>
{
    dir.filter_map(|e| {
        let entry = e.ok()?;
        let metadata = entry.metadata().ok()?;
        let name = entry.file_name().into_string().ok()?;
        Some((name, metadata, entry.path()))
    })
}

fn build_post(
    h: &Handlebars,
    name: &str,
    input_dir: &str,
    output_dir: &str,
) -> Result<Post, Box<dyn Error>> {
    let md_str = fs::read_to_string(format!("{input_dir}{name}"))?;
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

    create_file(format!("{output_dir}{out_name}"), out)?;

    Ok(Post {
        filename: out_name,
        created_at,
        title,
    })
}

fn build_page(
    h: &Handlebars,
    name: &str,
    input_dir: &str,
    output_dir: &str,
) -> Result<(), Box<dyn Error>> {
    let md_str = fs::read_to_string(format!("{input_dir}{name}"))?;
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

    create_file(format!("{output_dir}/{out_name}"), out)
}

pub async fn run_new(path: String) -> Result<(), Box<dyn Error>> {
    let start = Utc::now();
    let blog_dir = Path::new(&path);
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

    afs::create_dir(blog_dir).await?;
    try_join4(
        afs::create_dir(&assets_dir),
        afs::create_dir(&pages_dir),
        afs::create_dir(&posts_dir),
        afs::create_dir(&templates_dir),
    )
    .await?;

    let mut build_template_actions = TemplateName::iter()
        .map(|n| {
            afs::write(
                templates_dir.join(format!("{n}.hbs")).to_path_buf(),
                n.template_str().trim_start().to_owned(),
            )
        })
        .collect::<Vec<_>>();

    build_template_actions.extend([
        afs::write(blog_dir.join("index.md").to_path_buf(), format!("# {name}\n")),
        afs::write(
            assets_dir.join("style.css"),
            CSS_STR.to_string().trim_start().to_owned(),
        ),
        afs::write(
            assets_dir.join("script.js"),
            JS_STR.to_string().trim_start().to_owned(),
        ),
        afs::write(pages_dir.join("about.md"), "# About\n".to_owned()),
        afs::write(
            posts_dir.join(format!("{date}-hello-world.md")),
            "# Hello, World!\n".to_owned(),
        ),
    ]);

    try_join_all(build_template_actions).await?;

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

fn remove_path(metadata: Metadata, path: PathBuf) -> Result<(), std::io::Error> {
    if metadata.is_file() {
        fs::remove_file(path)
    } else {
        fs::remove_dir_all(path)
    }
}

fn read_template(name: &TemplateName, input_dir: &str) -> Result<String, Box<dyn Error>> {
    Ok(
        fs::read_to_string(format!("{input_dir}/templates/{name}.hbs"))?
            .split("\n")
            .map(|l| l.trim())
            .collect::<Vec<&str>>()
            .join("\n"),
    )
}

fn build_index(h: &Handlebars, input_dir: &str, output_dir: &str) -> Result<(), Box<dyn Error>> {
    let md_str = fs::read_to_string(format!("{input_dir}/index.md"))?;
    let (title, contents) = md_to_html(md_str.to_owned(), new_options());
    let test = &json!(IndexArgs {
        title: &title,
        contents: &contents,
    });
    let out = h.render("index", test)?;
    create_file(format!("{output_dir}/index.html"), out)
}

pub async fn run_build(
    input_dir: &str,
    output_dir: &str,
    should_confirm: bool,
) -> Result<(), Box<dyn Error>> {
    let mut start = Utc::now();
    if let Ok(metadata) = fs::metadata(&output_dir) {
        if metadata.is_file() {
            return Err(format!("{output_dir} is already a file").into());
        }

        if should_confirm {
            let ans = Confirm::new(&format!("{output_dir} already exists. Continue?"))
                .with_default(false)
                .with_help_message("All contents will be overwritten except .git/")
                .prompt()?;
            if !ans {
                return Ok(());
            }

            start = Utc::now();
        }

        let r_dir = fs::read_dir(&output_dir)?;
        let output_entries = get_files_in_dir(r_dir);
        for (name, metadata, path) in output_entries {
            match name.as_str() {
                ".git" => continue,
                "CNAME" => continue,
                _ => (),
            }

            remove_path(metadata, path)?;
        }
    } else {
        fs::create_dir(&output_dir)?;
    }

    let posts_output_dir = format!("{output_dir}/posts/");
    try_join(
        afs::create_dir(&posts_output_dir),
        afs::create_dir(format!("{output_dir}/assets/")),
    )
    .await?;

    let input_assets_path = format!("{input_dir}/assets/");
    let output_assets_path = format!("{output_dir}/assets/");
    let (dir_paths, file_paths) = get_dir_paths(input_assets_path.to_owned())?;
    try_join_all(dir_paths.into_iter().map(|p| {
        let d = p.display();
        afs::create_dir(format!("{output_assets_path}{d}"))
    }))
    .await?;
    try_join_all(file_paths.into_iter().map(|p| {
        let d = p.display();
        afs::copy(
            format!("{input_assets_path}{d}"),
            format!("{output_assets_path}{d}"),
        )
    }))
    .await?;

    let mut h = Handlebars::new();
    for name in TemplateName::iter() {
        let template = read_template(&name, &input_dir)?;
        h.register_template_string(&name.to_string(), template)?;
    }

    build_index(&h, &input_dir, &output_dir)?;

    let re = Regex::new(r"^[A-Za-z0-9\-]+\.md$")?;
    let pages_input_dir = format!("{input_dir}/pages/");
    let mut r_dir = fs::read_dir(&pages_input_dir)?;
    let page_entries = get_files_in_dir(r_dir);
    for (name, metadata, ..) in page_entries {
        if !(metadata.is_file() && re.is_match(&name)) {
            continue;
        }

        build_page(&h, &name, &pages_input_dir, &output_dir)?;
    }

    let mut posts_args = PostsArgs {
        path: &[Breadcrumb {
            name: "Posts",
            link: "posts",
        }],
        title: "Posts",
        posts: Vec::new(),
    };
    let posts_input_dir = format!("{input_dir}/posts/");
    r_dir = fs::read_dir(&posts_input_dir)?;
    let mut post_entries = get_files_in_dir(r_dir).collect::<Vec<_>>();
    post_entries.sort_by_key(|(.., path)| path.to_owned());
    for (name, metadata, ..) in post_entries {
        if !(metadata.is_file() && re.is_match(&name)) {
            continue;
        }

        let post = build_post(&h, &name, &posts_input_dir, &posts_output_dir)?;
        posts_args.posts.insert(0, post)
    }

    let out = h.render("posts", &json!(posts_args))?;
    create_file(format!("{output_dir}/posts/index.html"), out)?;

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use crate::utils::{copy_dir_all, create_file, md_to_html};
use chrono::prelude::*;
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

pub fn get_files_in_dir(
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

pub fn run_new(path: String) -> Result<(), Box<dyn Error>> {
    let name = Path::new(&path)
        .file_stem()
        .expect("Invalid input")
        .to_string_lossy();
    fs::create_dir(&path)?;
    create_file(format!("{path}/index.md"), format!("# {name}\n"))?;

    fs::create_dir(format!("{path}/assets"))?;
    create_file(
        format!("{path}/assets/style.css"),
        format!("{}", CSS_STR.to_string().trim_start()),
    )?;
    create_file(
        format!("{path}/assets/script.js"),
        format!("{}", JS_STR.to_string().trim_start()),
    )?;

    fs::create_dir(format!("{path}/posts"))?;
    let date = Utc::now().format("%Y-%m-%d");
    create_file(
        format!("{path}/posts/{date}-hello-world.md"),
        "# Hello, World!\n".to_string(),
    )?;

    fs::create_dir(format!("{path}/pages"))?;
    create_file(format!("{path}/pages/about.md"), "# About\n".to_string())?;

    fs::create_dir(format!("{path}/templates"))?;
    for template_name in TemplateName::iter() {
        create_file(
            format!("{path}/templates/{template_name}.hbs"),
            format!("{}", template_name.template_str().trim_start()),
        )?;
    }

    Ok(())
}

pub fn run_build(
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
        get_files_in_dir(r_dir)
            .map(|(name, metadata, path)| -> Result<(), Box<dyn Error>> {
                if metadata.is_file() {
                    if name != "CNAME" {
                        fs::remove_file(path)?;
                    }
                } else {
                    if name != ".git" {
                        fs::remove_dir_all(path)?;
                    }
                }
                Ok(())
            })
            .for_each(drop);
    } else {
        fs::create_dir(&output_dir)?;
    }

    fs::create_dir(format!("{output_dir}/assets/"))?;
    copy_dir_all(
        format!("{input_dir}/assets"),
        format!("{output_dir}/assets"),
    )?;

    let mut h = Handlebars::new();
    for name in TemplateName::iter() {
        h.register_template_string(
            &name.to_string(),
            fs::read_to_string(format!("{input_dir}/templates/{name}.hbs"))?
                .split("\n")
                .map(|l| l.trim())
                .collect::<Vec<&str>>()
                .join("\n"),
        )?;
    }

    let md_str = fs::read_to_string(format!("{input_dir}/index.md"))?;
    let (title, contents) = md_to_html(md_str.to_owned(), new_options());
    let test = &json!(IndexArgs {
        title: &title,
        contents: &contents,
    });
    let out = h.render("index", test)?;
    create_file(format!("{output_dir}/index.html"), out)?;

    let re = Regex::new(r"^[A-Za-z0-9\-]+\.md$")?;
    let mut dir = format!("{input_dir}/pages/");
    let mut r_dir = fs::read_dir(&dir)?;
    get_files_in_dir(r_dir)
        .filter(|(name, ..)| re.is_match(name))
        .map(|(name, ..)| -> Result<(), Box<dyn Error>> {
            let md_str = fs::read_to_string(format!("{dir}{name}"))?;
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
            create_file(format!("{output_dir}/{out_name}"), out)?;
            Ok(())
        })
        .for_each(drop);

    let mut posts_args = PostsArgs {
        path: &[Breadcrumb {
            name: "Posts",
            link: "posts",
        }],
        title: "Posts",
        posts: Vec::new(),
    };
    dir = format!("{input_dir}/posts/");
    let re = Regex::new(r"^(?P<date>\d{4}-\d{2}-\d{2})-(?P<title>[A-Za-z0-9\-]+)\.md$")?;
    fs::create_dir(format!("{output_dir}/posts/"))?;
    r_dir = fs::read_dir(&dir)?;
    get_files_in_dir(r_dir)
        .filter(|(name, metadata, ..)| metadata.is_file() && re.is_match(name))
        .map(|(name, ..)| -> Result<(), Box<dyn Error>> {
            let md_str = fs::read_to_string(format!("{dir}{name}"))?;
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

            create_file(format!("{output_dir}/posts/{out_name}"), out)?;

            posts_args.posts.insert(
                0,
                Post {
                    filename: out_name,
                    created_at,
                    title,
                },
            );

            Ok(())
        })
        .for_each(drop);

    let out = h.render("posts", &json!(posts_args))?;
    create_file(format!("{output_dir}/posts/index.html"), out)?;

    println!("built in {} ms", (Utc::now() - start).num_milliseconds());

    Ok(())
}

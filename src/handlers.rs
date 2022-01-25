use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use crate::utils::{copy_dir_all, create_file, for_each_dir_entry, md_to_html};
use chrono::prelude::*;
use handlebars::Handlebars;
use inquire::Confirm;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use pulldown_cmark::Options;
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
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
    posts: Vec<Post>,
}

#[derive(Serialize)]
struct PostArgs<'a> {
    path: &'a [Breadcrumb<'a>],
    title: &'a str,
    contents: &'a str,
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
        }

        let entries = fs::read_dir(&output_dir)?;
        for entry in entries {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                fs::remove_file(entry.path())?;
            } else {
                if let Ok(name) = entry.file_name().into_string() {
                    if name == ".git" {
                        continue;
                    }
                }
                fs::remove_dir_all(entry.path())?;
            }
        }
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
            fs::read_to_string(format!("{input_dir}/templates/{name}.hbs"))?,
        )?;
    }

    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    let (_, contents) = md_to_html(format!("{input_dir}/index.md"), options)?;
    let test = &json!(IndexArgs {
        contents: &contents,
    });
    let out = h.render("index", test)?;
    create_file(format!("{output_dir}/index.html"), out)?;

    let mut dir = format!("{input_dir}/pages/");
    for_each_dir_entry(
        &dir,
        &Regex::new(r"^[A-Za-z0-9\-]+\.md$")?,
        |name: &str| -> Result<(), Box<dyn Error>> {
            let (title, contents) = md_to_html(format!("{dir}{name}"), options)?;
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
        },
    )?;

    let mut posts_args = PostsArgs {
        path: &[Breadcrumb {
            name: "Posts",
            link: "posts",
        }],
        posts: Vec::new(),
    };
    dir = format!("{input_dir}/posts/");
    let re = Regex::new(
        r"^(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})-(?P<title>[A-Za-z0-9\-]+)\.md$",
    )?;
    fs::create_dir(format!("{output_dir}/posts/"))?;
    for_each_dir_entry(&dir, &re, |name: &str| -> Result<(), Box<dyn Error>> {
        let caps = re.captures(name).expect("match already performed");
        let dt = Utc.ymd(
            caps["year"].parse()?,
            caps["month"].parse()?,
            caps["day"].parse()?,
        );
        let (title, contents) = md_to_html(format!("{dir}{name}"), options)?;
        let filename = name.replace(".md", ".html");
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
                        link: &format!("posts/{filename}"),
                    }
                ],
                title: &title,
                contents: &contents,
            }),
        )?;
        let out_name = name.replace(".md", ".html");
        create_file(format!("{output_dir}/posts/{out_name}"), out)?;

        posts_args.posts.insert(
            0,
            Post {
                filename: out_name,
                created_at: created_at,
                title: title,
            },
        );

        Ok(())
    })?;

    let out = h.render("posts", &json!(posts_args))?;
    create_file(format!("{output_dir}/posts/index.html"), out)?;

    Ok(())
}

pub fn run_watch(input_dir: String, output_dir: String) -> Result<(), Box<dyn Error>> {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
    watcher.watch(&input_dir, RecursiveMode::Recursive).unwrap();

    run_build(&input_dir, &output_dir, true)?;
    println!("built differently");
    loop {
        let event = rx.recv()?;
        match event {
            DebouncedEvent::NoticeWrite(_) | DebouncedEvent::NoticeRemove(_) => continue,
            _ => {
                run_build(&input_dir, &output_dir, false)?;
                println!("built differently");
            }
        }
    }
}

use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use chrono::prelude::*;
use handlebars::Handlebars;
use inquire::Confirm;
use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use regex::Regex;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use strum::IntoEnumIterator;

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum PageType {
    Index,
    Page,
    Posts,
    Post,
}

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

fn create_file(path: String, contents: String) -> Result<(), Box<dyn Error>> {
    let mut f = File::create(format!("{path}"))?;
    write!(f, "{contents}")?;
    Ok(())
}

pub fn run_new(name: String) -> Result<(), Box<dyn Error>> {
    fs::create_dir(&name)?;
    create_file(format!("{name}/index.md"), format!("# {name}\n"))?;

    fs::create_dir(format!("{name}/assets"))?;
    create_file(
        format!("{name}/assets/style.css"),
        format!("{}", CSS_STR.to_string().trim_start()),
    )?;
    create_file(
        format!("{name}/assets/script.js"),
        format!("{}", JS_STR.to_string().trim_start()),
    )?;

    fs::create_dir(format!("{name}/posts"))?;
    let date = Utc::now().format("%Y-%m-%d");
    create_file(
        format!("{name}/posts/{date}-hello-world.md"),
        "# Hello, World!\n".to_string(),
    )?;

    fs::create_dir(format!("{name}/pages"))?;
    create_file(format!("{name}/pages/about.md"), "# About\n".to_string())?;

    fs::create_dir(format!("{name}/templates"))?;
    for template_name in TemplateName::iter() {
        create_file(
            format!("{name}/templates/{template_name}.hbs"),
            format!("{}", template_name.template_str().trim_start()),
        )?;
    }

    Ok(())
}

fn md_to_html(path: String, options: Options) -> Result<(String, String), Box<dyn Error>> {
    let md_str = fs::read_to_string(path)?;
    let mut parser = Parser::new_ext(&md_str, options);
    let mut inside_header = false;
    let mut title = String::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading(HeadingLevel::H1, _, _)) => inside_header = true,
            Event::Text(text) => {
                if inside_header {
                    title = text.to_string();
                    break;
                }
            }
            _ => (),
        };
    }

    parser = Parser::new_ext(&md_str, options);
    let mut html_str = String::new();
    html::push_html(&mut html_str, parser);
    Ok((title, html_str))
}

fn for_each_dir_entry<F>(dir: &str, re: &Regex, mut f: F) -> Result<(), Box<dyn Error>>
where
    F: FnMut(&str) -> Result<(), Box<dyn Error>>,
{
    let mut entries = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let metadata = entry.metadata()?;
        if let Ok(name) = &entry.file_name().into_string() {
            if metadata.is_file() && re.is_match(name) {
                if let Err(e) = f(name) {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

pub fn run_build(input_dir: String, output_dir: String) -> Result<(), Box<dyn Error>> {
    if let Ok(metadata) = fs::metadata(&output_dir) {
        if metadata.is_file() {
            return Err(format!("{output_dir} is already a file").into());
        }

        let ans = Confirm::new(&format!("{output_dir} already exists. Continue?"))
            .with_default(false)
            .with_help_message("All contents will be overwritten except .git/")
            .prompt()?;
        if !ans {
            return Ok(());
        }

        let entries = fs::read_dir(output_dir)?;
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
        fs::create_dir(output_dir)?;
    }

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
    //println!("{}", out);

    let mut dir = format!("{input_dir}/pages/");
    for_each_dir_entry(
        &dir,
        &Regex::new(r"^[A-Za-z0-9\-]+\.md$")?,
        |name: &str| -> Result<(), Box<dyn Error>> {
            let (title, contents) = md_to_html(format!("{dir}{name}"), options)?;
            let out = h.render(
                "page",
                &json!(PageArgs {
                    path: &[Breadcrumb {
                        name: &title,
                        link: ""
                    }],
                    title: &title,
                    contents: &contents
                }),
            )?;
            //println!("{}", out);
            Ok(())
        },
    )?;

    let mut posts_args = PostsArgs {
        path: &[Breadcrumb {
            name: "Posts",
            link: "/posts",
        }],
        posts: Vec::new(),
    };
    dir = format!("{input_dir}/posts/");
    let re = Regex::new(
        r"^(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})-(?P<title>[A-Za-z0-9\-]+)\.md$",
    )?;
    for_each_dir_entry(&dir, &re, |name: &str| -> Result<(), Box<dyn Error>> {
        let caps = re.captures(name).expect("match already performed");
        let dt = Utc.ymd(
            caps["year"].parse()?,
            caps["month"].parse()?,
            caps["month"].parse()?,
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
                        link: "/posts/",
                    },
                    Breadcrumb {
                        name: &created_at,
                        link: &format!("/posts/{filename}"),
                    }
                ],
                title: &title,
                contents: &contents,
            }),
        )?;
        //println!("{}", out);
        posts_args.posts.insert(
            0,
            Post {
                filename: name.to_string(),
                created_at: created_at,
                title: title,
            },
        );

        Ok(())
    })?;

    let out = h.render("posts", &json!(posts_args))?;
    //println!("{}", out);

    Ok(())
}

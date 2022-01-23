use crate::assets::{CSS_STR, JS_STR};
use crate::templates::TemplateName;
use chrono::prelude::*;
use handlebars::Handlebars;
use pulldown_cmark::{html, Options, Parser};
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
    contents: String,
}

#[derive(Serialize)]
struct Breadcrumb {
    name: String,
    link: String,
}

#[derive(Serialize)]
struct TemplateArgs<'a> {
    title: String,
    page_type: PageType,
    path: &'a [Breadcrumb],
    posts: &'a [Post],
    contents: String,
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

fn md_to_html(path: String, options: Options) -> Result<String, Box<dyn Error>> {
    let md_str = fs::read_to_string(path)?;
    let parser = Parser::new_ext(&md_str, options);
    let mut html_str = String::new();
    html::push_html(&mut html_str, parser);
    Ok(html_str)
}

fn for_each_dir_entry<F>(dir: &str, re: Regex, f: F) -> Result<(), Box<dyn Error>>
where
    F: Fn(&str) -> Result<(), Box<dyn Error>>,
{
    let mut entries = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let ft = entry.file_type()?;
        if let Ok(name) = &entry.file_name().into_string() {
            if ft.is_file() && re.is_match(name) {
                if let Err(e) = f(name) {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

pub fn run_build(input_dir: String, output_dir: String) -> Result<(), Box<dyn Error>> {
    let mut h = Handlebars::new();
    let mut args = TemplateArgs {
        title: "test".to_string(),
        page_type: PageType::Index,
        path: &[Breadcrumb {
            name: "Posts".to_string(),
            link: "/posts".to_string(),
        }],
        posts: &[Post {
            filename: "test.html".to_string(),
            title: "Test".to_string(),
            created_at: "Feb 02, 2022".to_string(),
            contents: "foobar".to_string(),
        }],
        contents: String::new(),
    };

    for name in TemplateName::iter() {
        let template_str = fs::read_to_string(format!("{input_dir}/templates/{name}.hbs"))?;
        h.register_template_string(&name.to_string(), template_str)?;
    }

    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    let index_html = md_to_html(format!("{input_dir}/index.md"), options)?;

    for_each_dir_entry(
        &format!("{input_dir}/pages/"),
        Regex::new(r"^[A-Za-z0-9\-]+\.md$")?,
        |name: &str| -> Result<(), Box<dyn Error>> {
            let page_html = md_to_html(format!("{input_dir}/pages/{name}"), options)?;
            println!("{}", page_html);
            Ok(())
        },
    )?;

    let re = Regex::new(
        r"^(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})-(?P<title>[A-Za-z0-9\-]+)\.md$",
    )?;
    for_each_dir_entry(
        &format!("{input_dir}/posts/"),
        re,
        |name: &str| -> Result<(), Box<dyn Error>> {
            let post_html = md_to_html(format!("{input_dir}/posts/{name}"), options)?;
            println!("{}", name);
            Ok(())
        },
    )?;

    args.contents = index_html;
    let out = h.render("base", &json!(args))?;
    println!("{}", out);
    Ok(())
}

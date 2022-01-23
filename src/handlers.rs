use crate::templates::TemplateName;
use chrono::prelude::*;
use handlebars::Handlebars;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use strum::IntoEnumIterator;

const CSS_STR: &str = r#"
*,
*::before,
*::after {
  box-sizing: border-box;
}

#container {
  position: relative;
  max-width: 500px;
  margin: 0 auto;
}

#posts-list {
  list-style-type: none;
}

#posts-list .posts-list-item {
  display: flex;
}

#posts-list .posts-list-item .posts-list-item-title {
  flex-grow: 8;
}

#posts-list .posts-list-item .posts-list-item-time {
  flex-grow: 4;
  text-align: right;
}

@media (max-width: 576px) {
  #posts-list .posts-list-item {
    flex-direction: column;
  }

  #posts-list .posts-list-item .posts-list-item-time {
    text-align: left;
  }
}
"#;

const JS_STR: &str = r#"
window.addEventListener('DOMContentLoaded', () => {});
"#;

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
    content: String,
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

pub fn run_build(input_dir: String, output_dir: String) -> Result<(), Box<dyn Error>> {
    let mut h = Handlebars::new();
    let args = &json!(TemplateArgs {
        title: "test".to_string(),
        page_type: PageType::Posts,
        path: &[Breadcrumb {
            name: "Posts".to_string(),
            link: "/posts".to_string()
        }],
        posts: &[Post {
            filename: "test.html".to_string(),
            title: "Test".to_string(),
            created_at: "Feb 02, 2022".to_string(),
            content: "foobar".to_string(),
        }],
    });
    for name in TemplateName::iter() {
        let template_str = fs::read_to_string(format!("{input_dir}/templates/{name}.hbs"))?;
        h.register_template_string(&name.to_string(), template_str)?;
    }
    let out = h.render("base", args)?;
    println!("{}", out);
    Ok(())
}

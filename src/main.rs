use chrono::prelude::*;
use clap::{Parser, Subcommand};
use handlebars::Handlebars;
use serde::Serialize;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;

#[derive(Parser)]
#[clap(version, about)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    New {
        /// directory name
        name: String,
    },
    Build,
}

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

const BASE_TEMPLATE: &str = r#"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width,initial-scale=1">
    <title>{{title}}</title>
    <link rel="stylesheet" href="/assets/style.css">
    <script type="text/javascript" src="/assets/script.js" async defer></script>
  </head>
  <body>
    <div id="container">
      {{> (lookup this "page_type") }}
      <footer>
      </footer>
    </div>
  </body>
</html>
"#;

const INDEX_TEMPLATE: &str = r#"
<section>
 <h1>{{title}}</h1>
</section
"#;

const PAGE_TEMPLATE: &str = r#"
{{> nav}}
<section>
  {{{contents}}}
</section>
"#;

const POSTS_TEMPLATE: &str = r#"
{{> nav}}
<section>
  <ul id="posts-list">
    {{#each posts}}
    <li class="posts-list-item">
      <div class="posts-list-item-title">{{this.title}}</div>
      <div class="posts-list-item-time">{{this.created_at}}</div>
    </li>
    {{/each}}
  </ul>
</section>
"#;

const POST_TEMPLATE: &str = r#"
{{> nav}}
<section>
  {{{contents}}}
</section>
"#;

const NAV_TEMPLATE: &str = r#"
<nav>
  <a href="/">Home</a>
  {{#each path}}
    <span class="breadcrumb">></span>
    <a href="{{link}}">{{name}}</a>
  {{/each}}
</nav>
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

fn run_new(name: String) -> Result<(), Box<dyn Error>> {
    fs::create_dir(&name)?;
    create_file(format!("{name}/index.md"), format!("# {name}\n"))?;

    fs::create_dir(format!("{name}/assets"))?;
    create_file(
        format!("{name}/assets/style.css"),
        format!("{}", CSS_STR.to_string().trim()),
    )?;
    create_file(
        format!("{name}/assets/script.js"),
        format!("{}", JS_STR.to_string().trim()),
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
    create_file(
        format!("{name}/templates/base.hbs"),
        format!("{}", BASE_TEMPLATE.trim_start()),
    )?;
    create_file(
        format!("{name}/templates/index.hbs"),
        format!("{}", INDEX_TEMPLATE.trim_start()),
    )?;
    create_file(
        format!("{name}/templates/page.hbs"),
        format!("{}", PAGE_TEMPLATE.trim_start()),
    )?;
    create_file(
        format!("{name}/templates/posts.hbs"),
        format!("{}", POSTS_TEMPLATE.trim_start()),
    )?;
    create_file(
        format!("{name}/templates/post.hbs"),
        format!("{}", POST_TEMPLATE.trim_start()),
    )?;
    create_file(
        format!("{name}/templates/nav.hbs"),
        format!("{}", NAV_TEMPLATE.trim_start()),
    )?;

    Ok(())
}

fn run_build() -> Result<(), Box<dyn Error>> {
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
    h.register_template_string("base", BASE_TEMPLATE.to_string().trim())?;
    h.register_template_string("index", INDEX_TEMPLATE.to_string().trim_start())?;
    h.register_template_string("page", PAGE_TEMPLATE.to_string().trim_start())?;
    h.register_template_string("posts", POSTS_TEMPLATE.to_string().trim_start())?;
    h.register_template_string("post", POST_TEMPLATE.to_string().trim_start())?;
    h.register_template_string("nav", NAV_TEMPLATE.to_string().trim_start())?;
    let out = h.render("base", args)?;
    println!("{}", out);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    match args.command {
        Command::New { name } => run_new(name),
        Command::Build => run_build(),
    }
}

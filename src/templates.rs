use strum_macros::{Display, EnumIter};

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
      {{> filling}}
      <footer>
      </footer>
    </div>
  </body>
</html>
"#;

const INDEX_TEMPLATE: &str = r#"
{{#*inline "filling"}}
<section>
  {{{contents}}}
  <ul>
    <li><a href="/about">About</a></li>
    <li><a href="/posts">Blog</a></li>
  </ul>
</section>
{{/inline}}
{{> base}}
"#;

const PAGE_TEMPLATE: &str = r#"
{{#*inline "filling"}}
{{> nav}}
<section>
  {{{contents}}}
</section>
{{/inline}}
{{> base}}
"#;

const POSTS_TEMPLATE: &str = r#"
{{#*inline "filling"}}
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
{{/inline}}
{{> base}}
"#;

const POST_TEMPLATE: &str = r#"
{{#*inline "filling"}}
{{> nav}}
<section>
  {{{contents}}}
</section>
{{/inline}}
{{> base}}
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

#[derive(EnumIter, Display)]
#[strum(serialize_all = "lowercase")]
pub enum TemplateName {
    Base,
    Index,
    Page,
    Posts,
    Post,
    Nav,
}

impl TemplateName {
    pub fn template_str(&self) -> &str {
        match self {
            TemplateName::Base => BASE_TEMPLATE,
            TemplateName::Index => INDEX_TEMPLATE,
            TemplateName::Page => PAGE_TEMPLATE,
            TemplateName::Posts => POSTS_TEMPLATE,
            TemplateName::Post => POST_TEMPLATE,
            TemplateName::Nav => NAV_TEMPLATE,
        }
    }
}

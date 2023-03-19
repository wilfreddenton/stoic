use serde::Serialize;

#[derive(Serialize)]
pub struct Post {
    pub filename: String,
    pub title: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct Breadcrumb<'a> {
    pub name: &'a str,
    pub link: &'a str,
}

#[derive(Serialize)]
pub struct IndexArgs<'a> {
    pub title: &'a str,
    pub contents: &'a str,
}

#[derive(Serialize)]
pub struct PageArgs<'a> {
    pub path: &'a [Breadcrumb<'a>],
    pub title: &'a str,
    pub contents: &'a str,
}

#[derive(Serialize)]
pub struct PostsArgs<'a> {
    pub path: &'a [Breadcrumb<'a>],
    pub title: &'a str,
    pub posts: Vec<Post>,
}

#[derive(Serialize)]
pub struct PostArgs<'a> {
    pub path: &'a [Breadcrumb<'a>],
    pub title: &'a str,
    pub contents: &'a str,
}

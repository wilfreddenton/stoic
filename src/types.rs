use serde::{Serialize, Deserialize};
use toml_datetime::Datetime;

#[derive(Debug, Deserialize)]
pub struct EntityMetadata {
    pub title: Option<String>,
    pub date: Option<Datetime>,
}

#[derive(Serialize)]
pub struct Entity {
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
pub struct EntitiesArgs<'a> {
    pub path: &'a [Breadcrumb<'a>],
    pub title: &'a str,
    pub entities: Vec<Entity>,
}

#[derive(Serialize)]
pub struct EntityArgs<'a> {
    pub path: &'a [Breadcrumb<'a>],
    pub title: &'a str,
    pub contents: &'a str,
}

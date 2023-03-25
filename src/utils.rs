use crate::types::EntityMetadata;
use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use std::error::Error;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use tokio::fs::{read_to_string, remove_dir_all, remove_file, ReadDir};
use walkdir::WalkDir;

// IO Actions
pub fn get_dir_paths(path: &PathBuf) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error + Send + Sync>> {
    let mut dir_paths = Vec::new();
    let mut file_paths = Vec::new();
    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>()
    {
        let metadata = entry.metadata()?;
        let path = entry.path().strip_prefix(&path).unwrap().to_path_buf();
        if path.to_str().unwrap() == "" {
            continue;
        }
        if metadata.is_dir() {
            dir_paths.push(path);
        } else {
            file_paths.push(path);
        }
    }

    Ok((dir_paths, file_paths))
}

pub async fn get_files_in_dir(
    mut dir: ReadDir,
) -> Result<Vec<(String, Metadata, PathBuf)>, Box<dyn Error + Send + Sync>> {
    let mut entries = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push((name, metadata, entry.path()));
    }
    Ok(entries)
}

pub async fn remove_path(metadata: Metadata, path: PathBuf) -> Result<(), std::io::Error> {
    if metadata.is_file() {
        remove_file(path).await
    } else {
        remove_dir_all(path).await
    }
}

pub async fn read_template(name: String, dir: &Path) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    Ok((name.strip_suffix(".hbs").unwrap_or(&name).to_string(), read_to_string(dir.join(name))
        .await?
        .split("\n")
        .map(|l| l.trim())
        .collect::<Vec<&str>>()
        .join("\n")))
}

// Pure Actions
pub fn md_to_html(md_str: String) -> (Option<EntityMetadata>, String, String) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let mut parser = Parser::new_ext(&md_str, options);
    let mut inside_header = false;
    let mut title = String::new();
    let mut inside_metadata = false;
    let mut metadata_str = String::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading(HeadingLevel::H1, _, _)) => inside_header = true,
            Event::Text(text) => {
                if inside_header {
                    title = text.to_string();
                    break;
                }
            }
            Event::Html(html_text) => {
                if !inside_metadata {
                    if html_text.to_string().trim() == "<!--metadata" {
                        inside_metadata = true;
                    }

                    continue
                }
                if html_text.to_string().trim() == "-->" {
                    inside_metadata = false;
                    continue;
                }
                metadata_str.push_str(html_text.to_string().as_ref());
            }
            _ => (),
        };
    }

    let metadata: Option<EntityMetadata> = toml::from_str(metadata_str.as_ref()).ok();
    parser = Parser::new_ext(&md_str, options);
    let mut html_str = String::new();
    html::push_html(&mut html_str, parser);
    (metadata, title, html_str)
}

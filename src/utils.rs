use crate::types::EntityMetadata;
use color_eyre::eyre::Result;
use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use std::fs::Metadata;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs::{
    copy, create_dir_all, read_dir, read_to_string, remove_dir_all, remove_file, File,
};
use walkdir::WalkDir;

// IO Actions
pub fn get_files_in_dir_recursive(path: &Path) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| {
            let entry = e.ok()?;
            let metadata = entry.metadata().ok()?;
            if metadata.is_dir() {
                return None;
            }
            let path = entry.path().strip_prefix(path).ok()?;
            if path == Path::new("") {
                return None;
            }
            Some(path.to_path_buf())
        })
        .collect::<Vec<_>>()
}

pub async fn get_entries_in_dir(
    path: &Path,
) -> Result<Vec<(String, Metadata, PathBuf)>, io::Error> {
    let mut dir = read_dir(path).await?;
    let mut entries = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        let name = entry.file_name().to_string_lossy().to_string();
        entries.push((name, metadata, entry.path()));
    }
    Ok(entries)
}

pub async fn remove_path(metadata: Metadata, path: PathBuf) -> Result<(), io::Error> {
    if metadata.is_file() {
        remove_file(path).await
    } else {
        remove_dir_all(path).await
    }
}

pub async fn read_template(name: String, dir: &Path) -> Result<(String, String), io::Error> {
    Ok((
        name.strip_suffix(".hbs").unwrap_or(&name).to_string(),
        read_to_string(dir.join(name))
            .await?
            .split("\n")
            .map(|l| l.trim())
            .collect::<Vec<&str>>()
            .join("\n"),
    ))
}

pub async fn copy_file(input_path: PathBuf, output_path: PathBuf) -> Result<()> {
    create_dir_all(output_path.parent().unwrap()).await?;
    // cannot rely on copy-on-write due to this issue: https://github.com/notify-rs/notify/issues/465
    File::create(&output_path).await?;
    copy(input_path, output_path).await?;
    Ok(())
}

// Pure Actions
pub fn md_to_html(md_str: &str) -> (Option<EntityMetadata>, String, String) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let mut parser = Parser::new_ext(md_str, options);
    let mut inside_header = false;
    let mut title = String::new();
    let mut inside_metadata = false;
    let mut metadata_str = String::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => inside_header = true,
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

                    continue;
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
    parser = Parser::new_ext(md_str, options);
    let mut html_str = String::new();
    html::push_html(&mut html_str, parser);
    (metadata, title, html_str)
}

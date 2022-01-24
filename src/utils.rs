use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use regex::Regex;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// https://stackoverflow.com/a/65192210
pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn create_file(path: String, contents: String) -> Result<(), Box<dyn Error>> {
    let mut f = File::create(format!("{path}"))?;
    write!(f, "{contents}")?;
    Ok(())
}

pub fn md_to_html(path: String, options: Options) -> Result<(String, String), Box<dyn Error>> {
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

pub fn for_each_dir_entry<F>(dir: &str, re: &Regex, mut f: F) -> Result<(), Box<dyn Error>>
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

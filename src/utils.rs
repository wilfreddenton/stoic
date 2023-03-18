use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag};
use std::error::Error;
use std::path::PathBuf;
use walkdir::WalkDir;

pub fn get_dir_paths(path: &PathBuf) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error>> {
    let mut dir_paths = Vec::new();
    let mut file_paths = Vec::new();
    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()).collect::<Vec<_>>() {
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

// Pure Actions
pub fn md_to_html(md_str: String, options: Options) -> (String, String) {
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
    (title, html_str)
}

use stoic::types::EntityMetadata;
use stoic::utils::md_to_html;
use toml_datetime::{Date, Datetime};

const TEST_MD: &str = r#"
<!--metadata
date = 2023-03-24
title = "Title"
-->
# Title
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_to_html_works() {
        let (metadata, title, contents) = md_to_html(TEST_MD.to_string());
        assert!(matches!(
            metadata,
            Some(EntityMetadata {
                title: Some(title),
                date: Some(Datetime {
                    date: Some(Date {
                        year: 2023,
                        month: 3,
                        day: 24
                    }),
                    time: None,
                    offset: None
                }),
            }) if title == "Title"
        ));
        assert_eq!(title, "Title");
        assert_eq!(
            contents,
            "<!--metadata\ndate = 2023-03-24\ntitle = \"Title\"\n-->\n<h1>Title</h1>\n"
        );
    }
}

use stoic::types::EntityMetadata;
use stoic::utils::md_to_html;
use toml_datetime::{Date, Datetime};

const TEST_MD: &str = r#"
<!--metadata
date = 2023-03-24
shortname = "title"
slug = " hey there "
-->
# Title
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_to_html_works() {
        let (metadata, title, contents) = md_to_html(TEST_MD);
        assert!(matches!(
            metadata,
            Some(EntityMetadata {
                shortname: Some(shortname),
                slug: Some(slug),
                date: Some(Datetime {
                    date: Some(Date {
                        year: 2023,
                        month: 3,
                        day: 24
                    }),
                    time: None,
                    offset: None
                }),
            }) if shortname == "title" && slug == " hey there "
        ));
        assert_eq!(title, "Title");
        assert_eq!(
            contents,
            r#"<!--metadata
date = 2023-03-24
shortname = "title"
slug = " hey there "
-->
<h1>Title</h1>
"#
        );
    }
}

//! Titles from the library catalog the Kindle app keeps in plain text.
//!
//! The app writes `KindleSyncMetadataCache.xml` with the ASIN and title of every book in the
//! account, so the title of a book is known before it is unlocked. Read-only.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The catalog file, given the app's `Content` folder (`...\Classic\Content`).
pub fn catalog_path(content_dir: &Path) -> Option<PathBuf> {
    let classic = content_dir.parent()?;
    Some(classic.join("Data").join("Cache").join("KindleSyncMetadataCache.xml"))
}

/// ASIN → title of the books in the catalog. Empty if the file is missing or unreadable.
pub fn read_titles(path: &Path) -> HashMap<String, String> {
    let Ok(bytes) = std::fs::read(path) else { return HashMap::new() };
    parse_titles(&String::from_utf8_lossy(&bytes))
}

pub fn parse_titles(xml: &str) -> HashMap<String, String> {
    let xml = xml.trim_start_matches('\u{feff}');
    let Ok(doc) = roxmltree::Document::parse(xml) else { return HashMap::new() };
    doc.descendants()
        .filter(|n| n.has_tag_name("meta_data"))
        .filter_map(|book| {
            let child_text = |name: &str| {
                let text: String = book.children().find(|c| c.has_tag_name(name))?.descendants().filter(|n| n.is_text()).filter_map(|n| n.text()).collect();
                let text = text.trim();
                (!text.is_empty()).then(|| text.to_string())
            };
            Some((child_text("ASIN")?, child_text("title")?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        "\u{feff}<response><sync_time>2026-11-15T01:41:09+0000</sync_time>",
        "<cache_metadata><version>1</version></cache_metadata><add_update_list>",
        "<meta_data><ASIN>B0AAA</ASIN><title pronunciation=\"\">Harry Potter y la cámara secreta</title>",
        "<authors><author pronunciation=\"\">J.K. Rowling</author></authors></meta_data>",
        "<meta_data><ASIN>B0BBB</ASIN><title pronunciation=\"\">Pride &amp; Prejudice</title></meta_data>",
        "<meta_data><ASIN>B0CCC</ASIN><title pronunciation=\"\">   </title></meta_data>",
        "<meta_data><title>No ASIN</title></meta_data>",
        "</add_update_list></response>"
    );

    #[test]
    fn reads_titles_by_asin() {
        let titles = parse_titles(SAMPLE);
        assert_eq!(titles.get("B0AAA").map(String::as_str), Some("Harry Potter y la cámara secreta"));
        assert_eq!(titles.get("B0BBB").map(String::as_str), Some("Pride & Prejudice"));
        assert_eq!(titles.len(), 2, "blank titles and entries without ASIN are skipped");
    }

    #[test]
    fn broken_or_missing_file_is_empty() {
        assert!(parse_titles("<response><meta_data>").is_empty());
        assert!(parse_titles("").is_empty());
        assert!(read_titles(Path::new("does/not/exist.xml")).is_empty());
    }

    #[test]
    fn catalog_sits_next_to_the_content_folder() {
        let path = catalog_path(Path::new("L/Classic/Content")).unwrap();
        assert_eq!(path, Path::new("L/Classic/Data/Cache/KindleSyncMetadataCache.xml"));
    }
}

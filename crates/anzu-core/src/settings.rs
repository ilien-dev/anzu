//! App data paths and persistent settings.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

const FILE_NAME: &str = "settings.json";

/// The app's data folder: `%LOCALAPPDATA%/anzu`.
pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("anzu")
}

/// Name of the data folder used by earlier versions, from when the app had a different name.
const LEGACY_DIR_NAME: &str = "seshat";

/// Adopts the old folder if it exists and the new one does not. Returns whether it did.
pub fn adopt_legacy_dir(old: &Path, new: &Path) -> bool {
    old.is_dir() && !new.exists() && std::fs::rename(old, new).is_ok()
}

/// The data folder, first adopting the one from an earlier version if there is one.
pub fn prepare_data_dir() -> PathBuf {
    let dir = data_dir();
    if let Some(parent) = dir.parent() {
        adopt_legacy_dir(&parent.join(LEGACY_DIR_NAME), &dir);
    }
    dir
}

/// Stores the language chosen in the installer, keeping everything else in the settings.
pub fn save_language(data_dir: &Path, language: Language) -> Result<(), Error> {
    let mut settings = Settings::load(data_dir);
    settings.language = language;
    settings.save(data_dir)
}

/// Interface theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    /// The system's.
    #[default]
    System,
    Light,
    Dark,
}

/// Interface language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// English, the default.
    #[default]
    En,
    Es,
}

/// Reads a language, taking any value that is not a known one as English, so that a
/// language this version does not know costs the user nothing else in the file.
fn language_or_english<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Language, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Lenient {
        Known(Language),
        Unknown(serde::de::IgnoredAny),
    }
    Ok(match Lenient::deserialize(deserializer)? {
        Lenient::Known(language) => language,
        Lenient::Unknown(_) => Language::En,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output_dir: PathBuf,
    /// ASIN -> path of the generated EPUB.
    pub converted: BTreeMap<String, PathBuf>,
    /// ASIN -> EPUBs made before the one in `converted`, oldest first. A book converted again
    /// gets a numbered copy; if the user deletes that copy, the book is still converted.
    pub earlier_epubs: BTreeMap<String, Vec<PathBuf>>,
    /// ASIN -> book title, read from its unlocked copy.
    pub titles: BTreeMap<String, String>,
    pub theme: ThemeChoice,
    /// An unknown value in the file is read as English.
    #[serde(deserialize_with = "language_or_english")]
    pub language: Language,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        Settings {
            output_dir: home.join("Documents").join("EPUB"),
            converted: BTreeMap::new(),
            earlier_epubs: BTreeMap::new(),
            titles: BTreeMap::new(),
            theme: ThemeChoice::System,
            language: Language::En,
        }
    }
}

impl Settings {
    /// Reads the settings; if they are missing or corrupt, returns the defaults.
    pub fn load(dir: &Path) -> Settings {
        std::fs::read_to_string(dir.join(FILE_NAME))
            .ok()
            .and_then(|s| serde_json::from_str(s.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    /// Records a new EPUB of a book, keeping the ones made before it.
    pub fn record_epub(&mut self, asin: &str, epub: &Path) {
        let Some(previous) = self.converted.insert(asin.to_string(), epub.to_path_buf()) else { return };
        if previous == epub {
            return;
        }
        let earlier = self.earlier_epubs.entry(asin.to_string()).or_default();
        earlier.retain(|p| p != &previous && p != epub);
        earlier.push(previous);
    }

    /// The newest EPUB of a book that is still on disk.
    pub fn epub_of(&self, asin: &str) -> Option<PathBuf> {
        let earlier = self.earlier_epubs.get(asin).into_iter().flatten().rev();
        self.converted.get(asin).into_iter().chain(earlier).find(|p| p.is_file()).cloned()
    }

    /// Forgets the EPUBs that are no longer on disk. A book whose newest EPUB was deleted
    /// takes the newest one left; a book with none left is no longer converted. Returns
    /// whether anything changed.
    pub fn prune_missing_epubs(&mut self) -> bool {
        let asins: Vec<String> = self.converted.keys().chain(self.earlier_epubs.keys()).cloned().collect();
        let mut changed = false;
        for asin in asins {
            let mut epubs = self.earlier_epubs.remove(&asin).unwrap_or_default();
            epubs.extend(self.converted.remove(&asin));
            let before = epubs.len();
            epubs.retain(|p| p.is_file());
            changed |= epubs.len() != before;
            if let Some(newest) = epubs.pop() {
                self.converted.insert(asin.clone(), newest);
            }
            if !epubs.is_empty() {
                self.earlier_epubs.insert(asin, epubs);
            }
        }
        changed
    }

    pub fn save(&self, dir: &Path) -> Result<(), Error> {
        std::fs::create_dir_all(dir)?;
        let json = serde_json::to_string_pretty(self).map_err(|e| Error::Other(e.to_string()))?;
        let tmp = dir.join(format!("{FILE_NAME}.tmp"));
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, dir.join(FILE_NAME))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Settings { output_dir: PathBuf::from(r"D:\Books"), ..Settings::default() };
        s.converted.insert("B0AAA".into(), PathBuf::from(r"D:\Books\a.epub"));
        s.save(dir.path()).unwrap();
        assert_eq!(Settings::load(dir.path()), s);
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let s = Settings::load(dir.path());
        assert!(s.converted.is_empty());
        assert!(s.output_dir.ends_with("EPUB"));
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), b"{not json").unwrap();
        assert_eq!(Settings::load(dir.path()), Settings::default());
    }

    #[test]
    fn theme_and_titles_round_trip_and_default_to_system() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Settings::load(dir.path()).theme, ThemeChoice::System);
        let mut s = Settings { theme: ThemeChoice::Light, ..Settings::default() };
        s.titles.insert("B0AAA".into(), "My book".into());
        s.save(dir.path()).unwrap();
        assert_eq!(Settings::load(dir.path()), s);
    }

    #[test]
    fn settings_written_by_an_older_version_still_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), br#"{"output_dir":"D:/Books","converted":{}}"#).unwrap();
        let s = Settings::load(dir.path());
        assert_eq!(s.output_dir, PathBuf::from("D:/Books"));
        assert_eq!(s.theme, ThemeChoice::System);
    }

    #[test]
    fn language_defaults_to_english() {
        assert_eq!(Language::default(), Language::En);
        assert_eq!(Settings::default().language, Language::En);
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Settings::load(dir.path()).language, Language::En);
    }

    #[test]
    fn settings_without_a_language_load_as_english() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), br#"{"output_dir":"D:/Books","theme":"dark"}"#).unwrap();
        let s = Settings::load(dir.path());
        assert_eq!(s.language, Language::En);
        assert_eq!(s.output_dir, PathBuf::from("D:/Books"));
        assert_eq!(s.theme, ThemeChoice::Dark);
    }

    #[test]
    fn language_round_trips_as_a_lowercase_string() {
        for (language, text) in [(Language::En, r#""language": "en""#), (Language::Es, r#""language": "es""#)] {
            let dir = tempfile::tempdir().unwrap();
            let s = Settings { language, ..Settings::default() };
            s.save(dir.path()).unwrap();
            let json = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
            assert!(json.contains(text), "{text} not found in {json}");
            assert_eq!(Settings::load(dir.path()), s);
        }
    }

    #[test]
    fn an_unknown_language_falls_back_to_english_and_keeps_the_rest() {
        for language in [r#""fr""#, r#""ES""#, r#""""#, "null", "7", "true", r#"["es"]"#, r#"{"code":"es"}"#] {
            let dir = tempfile::tempdir().unwrap();
            let json = format!(
                r#"{{"output_dir":"D:/Books","converted":{{"B0AAA":"D:/Books/a.epub"}},"titles":{{"B0AAA":"My book"}},"theme":"dark","language":{language}}}"#
            );
            std::fs::write(dir.path().join("settings.json"), json).unwrap();

            let s = Settings::load(dir.path());

            assert_eq!(s.language, Language::En, "language: {language}");
            assert_eq!(s.output_dir, PathBuf::from("D:/Books"), "language: {language}");
            assert_eq!(s.converted.get("B0AAA"), Some(&PathBuf::from("D:/Books/a.epub")), "language: {language}");
            assert_eq!(s.titles.get("B0AAA").map(String::as_str), Some("My book"), "language: {language}");
            assert_eq!(s.theme, ThemeChoice::Dark, "language: {language}");
        }
    }

    #[test]
    fn a_known_language_is_read_from_the_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), br#"{"output_dir":"D:/Books","language":"es"}"#).unwrap();
        let s = Settings::load(dir.path());
        assert_eq!(s.language, Language::Es);
        assert_eq!(s.output_dir, PathBuf::from("D:/Books"));
    }

    #[test]
    fn legacy_data_folder_is_adopted_once() {
        let tmp = tempfile::tempdir().unwrap();
        let (old, new) = (tmp.path().join("old"), tmp.path().join("new"));
        std::fs::create_dir_all(old.join("keys")).unwrap();
        std::fs::write(old.join("keys").join("k"), b"key").unwrap();

        assert!(adopt_legacy_dir(&old, &new));
        assert_eq!(std::fs::read(new.join("keys").join("k")).unwrap(), b"key");
        assert!(!old.exists());

        std::fs::create_dir_all(&old).unwrap();
        assert!(!adopt_legacy_dir(&old, &new), "must not overwrite a folder that already exists");
        assert!(!adopt_legacy_dir(&tmp.path().join("nothing"), &tmp.path().join("other")));
    }

    #[test]
    fn a_byte_order_mark_does_not_discard_the_settings() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(br#"{"output_dir":"D:/Books","theme":"dark"}"#);
        std::fs::write(dir.path().join("settings.json"), bytes).unwrap();
        let s = Settings::load(dir.path());
        assert_eq!(s.output_dir, PathBuf::from("D:/Books"));
        assert_eq!(s.theme, ThemeChoice::Dark);
    }

    #[test]
    fn the_installer_language_is_saved_and_the_rest_is_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        save_language(&data, Language::Es).unwrap();
        assert_eq!(Settings::load(&data).language, Language::Es);

        let mut s = Settings::load(&data);
        s.theme = ThemeChoice::Dark;
        s.titles.insert("B0AAA".into(), "My book".into());
        s.save(&data).unwrap();
        save_language(&data, Language::En).unwrap();
        let s = Settings::load(&data);
        assert_eq!(s.language, Language::En);
        assert_eq!(s.theme, ThemeChoice::Dark);
        assert_eq!(s.titles.get("B0AAA").map(String::as_str), Some("My book"));
    }

    #[test]
    fn a_book_converted_again_keeps_its_earlier_epubs() {
        let mut s = Settings::default();
        s.record_epub("B0AAA", Path::new("a.epub"));
        s.record_epub("B0AAA", Path::new("a (2).epub"));
        s.record_epub("B0AAA", Path::new("a (2).epub"));
        s.record_epub("B0AAA", Path::new("a (3).epub"));
        assert_eq!(s.converted.get("B0AAA"), Some(&PathBuf::from("a (3).epub")));
        assert_eq!(s.earlier_epubs.get("B0AAA"), Some(&vec![PathBuf::from("a.epub"), PathBuf::from("a (2).epub")]));
    }

    #[test]
    fn the_epub_of_a_book_is_the_newest_one_still_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let [a, b, c] = ["a.epub", "a (2).epub", "a (3).epub"].map(|n| dir.path().join(n));
        let mut s = Settings::default();
        for p in [&a, &b, &c] {
            std::fs::write(p, b"e").unwrap();
            s.record_epub("B0AAA", p);
        }
        assert_eq!(s.epub_of("B0AAA"), Some(c.clone()));
        std::fs::remove_file(&c).unwrap();
        assert_eq!(s.epub_of("B0AAA"), Some(b.clone()));
        std::fs::remove_file(&b).unwrap();
        assert_eq!(s.epub_of("B0AAA"), Some(a.clone()));
        std::fs::remove_file(&a).unwrap();
        assert_eq!(s.epub_of("B0AAA"), None);
        assert_eq!(s.epub_of("B0ZZZ"), None);
    }

    #[test]
    fn pruning_forgets_the_epubs_that_were_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = |n: &str| dir.path().join(n);
        let mut s = Settings::default();
        for n in ["a.epub", "a (2).epub", "a (3).epub"] {
            s.record_epub("B0AAA", &path(n));
        }
        for n in ["b.epub", "b (2).epub"] {
            s.record_epub("B0BBB", &path(n));
        }
        s.record_epub("B0CCC", &path("c.epub"));
        // Still on disk: the first EPUB of A, the newest of B and the only one of C.
        for n in ["a.epub", "b (2).epub", "c.epub"] {
            std::fs::write(path(n), b"e").unwrap();
        }
        s.record_epub("B0DDD", &path("d.epub"));

        assert!(s.prune_missing_epubs());

        // A's newest was deleted: its earlier one takes its place.
        assert_eq!(s.converted.get("B0AAA"), Some(&path("a.epub")));
        assert_eq!(s.converted.get("B0BBB"), Some(&path("b (2).epub")));
        assert_eq!(s.converted.get("B0CCC"), Some(&path("c.epub")));
        // Nothing of D is left: the book goes back to the ones ready for EPUB.
        assert_eq!(s.converted.get("B0DDD"), None);
        assert!(s.earlier_epubs.is_empty(), "{:?}", s.earlier_epubs);

        assert!(!s.prune_missing_epubs(), "a second pass has nothing to forget");
    }

    #[test]
    fn settings_without_earlier_epubs_still_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            br#"{"output_dir":"D:/Books","converted":{"B0AAA":"D:/Books/a.epub"},"titles":{},"theme":"dark","language":"es"}"#,
        )
        .unwrap();
        let s = Settings::load(dir.path());
        assert_eq!(s.converted.get("B0AAA"), Some(&PathBuf::from("D:/Books/a.epub")));
        assert!(s.earlier_epubs.is_empty());
        assert_eq!(s.language, Language::Es);
    }

    #[test]
    fn save_creates_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        Settings::default().save(&nested).unwrap();
        assert!(nested.join("settings.json").is_file());
    }
}

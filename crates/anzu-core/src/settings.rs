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
    fn save_creates_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        Settings::default().save(&nested).unwrap();
        assert!(nested.join("settings.json").is_file());
    }
}

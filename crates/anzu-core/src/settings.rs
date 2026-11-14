//! Rutas de datos de la app y ajustes persistentes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;

const FILE_NAME: &str = "settings.json";

/// Carpeta de datos de la app: `%LOCALAPPDATA%/anzu`.
pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("anzu")
}

/// Nombre de la carpeta de datos de versiones anteriores, de cuando la app tenía otro nombre.
const LEGACY_DIR_NAME: &str = "seshat";

/// Si existe la carpeta antigua y no la nueva, la adopta. Devuelve si lo hizo.
pub fn adopt_legacy_dir(old: &Path, new: &Path) -> bool {
    old.is_dir() && !new.exists() && std::fs::rename(old, new).is_ok()
}

/// La carpeta de datos, adoptando antes la de una versión anterior si la hay.
pub fn prepare_data_dir() -> PathBuf {
    let dir = data_dir();
    if let Some(parent) = dir.parent() {
        adopt_legacy_dir(&parent.join(LEGACY_DIR_NAME), &dir);
    }
    dir
}

/// Tema de la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    /// El del sistema.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub output_dir: PathBuf,
    /// ASIN -> ruta del EPUB generado.
    pub converted: BTreeMap<String, PathBuf>,
    /// ASIN -> título del libro, leído de su copia desbloqueada.
    pub titles: BTreeMap<String, String>,
    pub theme: ThemeChoice,
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        Settings { output_dir: home.join("Documents").join("EPUB"), converted: BTreeMap::new(), titles: BTreeMap::new(), theme: ThemeChoice::System }
    }
}

impl Settings {
    /// Lee los ajustes; si faltan o están corruptos, devuelve los valores por defecto.
    pub fn load(dir: &Path) -> Settings {
        std::fs::read_to_string(dir.join(FILE_NAME))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
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
        let mut s = Settings { output_dir: PathBuf::from(r"D:\Libros"), ..Settings::default() };
        s.converted.insert("B0AAA".into(), PathBuf::from(r"D:\Libros\a.epub"));
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
        s.titles.insert("B0AAA".into(), "Mi libro".into());
        s.save(dir.path()).unwrap();
        assert_eq!(Settings::load(dir.path()), s);
    }

    #[test]
    fn settings_written_by_an_older_version_still_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), br#"{"output_dir":"D:/Libros","converted":{}}"#).unwrap();
        let s = Settings::load(dir.path());
        assert_eq!(s.output_dir, PathBuf::from("D:/Libros"));
        assert_eq!(s.theme, ThemeChoice::System);
    }

    #[test]
    fn legacy_data_folder_is_adopted_once() {
        let tmp = tempfile::tempdir().unwrap();
        let (old, new) = (tmp.path().join("viejo"), tmp.path().join("nuevo"));
        std::fs::create_dir_all(old.join("keys")).unwrap();
        std::fs::write(old.join("keys").join("k"), b"clave").unwrap();

        assert!(adopt_legacy_dir(&old, &new));
        assert_eq!(std::fs::read(new.join("keys").join("k")).unwrap(), b"clave");
        assert!(!old.exists());

        std::fs::create_dir_all(&old).unwrap();
        assert!(!adopt_legacy_dir(&old, &new), "no pisa una carpeta que ya existe");
        assert!(!adopt_legacy_dir(&tmp.path().join("nada"), &tmp.path().join("otro")));
    }

    #[test]
    fn save_creates_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        Settings::default().save(&nested).unwrap();
        assert!(nested.join("settings.json").is_file());
    }
}

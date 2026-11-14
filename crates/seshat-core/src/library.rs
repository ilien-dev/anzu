//! Libros descargados por la app Kindle y su estado en Seshat.
//!
//! Solo lectura: nada de este módulo escribe en la carpeta de Kindle.

use std::path::{Path, PathBuf};

use crate::settings::Settings;

const FOLDER_SUFFIX: &str = "_EBOK";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookState {
    Locked,
    Unlocked,
    Converted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Book {
    pub asin: String,
    pub title: Option<String>,
    /// El `.azw` original dentro de la carpeta de Kindle.
    pub source: PathBuf,
    pub size: u64,
    pub state: BookState,
    pub unlocked: Option<PathBuf>,
    pub epub: Option<PathBuf>,
}

/// Dónde guarda Seshat la copia desbloqueada de un libro.
pub fn unlocked_path(data_dir: &Path, asin: &str) -> PathBuf {
    data_dir.join("unlocked").join(format!("{asin}.kfx-zip"))
}

pub fn scan(content_dir: &Path, data_dir: &Path, settings: &Settings) -> Vec<Book> {
    let Ok(entries) = std::fs::read_dir(content_dir) else { return Vec::new() };
    let mut books: Vec<Book> = entries
        .flatten()
        .filter_map(|entry| {
            let folder = entry.file_name().into_string().ok()?;
            let asin = folder.strip_suffix(FOLDER_SUFFIX).filter(|a| !a.is_empty())?.to_string();
            let source = entry.path().join(format!("{folder}.azw"));
            let size = std::fs::metadata(&source).ok().filter(|m| m.is_file())?.len();

            let unlocked = Some(unlocked_path(data_dir, &asin)).filter(|p| p.is_file());
            let epub = settings.converted.get(&asin).filter(|p| p.is_file()).cloned();
            let state = match (&epub, &unlocked) {
                (Some(_), _) => BookState::Converted,
                (None, Some(_)) => BookState::Unlocked,
                (None, None) => BookState::Locked,
            };
            Some(Book { asin, title: None, source, size, state, unlocked, epub })
        })
        .collect();
    books.sort_by(|a, b| a.asin.cmp(&b.asin));
    books
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Fixture {
        _tmp: tempfile::TempDir,
        content: PathBuf,
        data: PathBuf,
        out: PathBuf,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let content = tmp.path().join("Content");
        let data = tmp.path().join("data");
        let out = tmp.path().join("out");
        for d in [&content, &data, &out] {
            fs::create_dir_all(d).unwrap();
        }
        Fixture { _tmp: tmp, content, data, out }
    }

    fn add_book(content: &Path, asin: &str, bytes: usize) {
        let dir = content.join(format!("{asin}_EBOK"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{asin}_EBOK.azw")), vec![0u8; bytes]).unwrap();
        fs::write(dir.join("extra.voucher"), b"v").unwrap();
    }

    #[test]
    fn missing_content_dir_is_empty() {
        let f = fixture();
        let books = scan(&f.content.join("nope"), &f.data, &Settings::default());
        assert!(books.is_empty());
    }

    #[test]
    fn empty_content_dir_is_empty() {
        let f = fixture();
        assert!(scan(&f.content, &f.data, &Settings::default()).is_empty());
    }

    #[test]
    fn reports_each_state() {
        let f = fixture();
        add_book(&f.content, "B0CCC", 30);
        add_book(&f.content, "B0AAA", 10);
        add_book(&f.content, "B0BBB", 20);

        let unlocked = f.data.join("unlocked");
        fs::create_dir_all(&unlocked).unwrap();
        fs::write(unlocked.join("B0BBB.kfx-zip"), b"z").unwrap();
        fs::write(unlocked.join("B0CCC.kfx-zip"), b"z").unwrap();

        let epub = f.out.join("c.epub");
        fs::write(&epub, b"e").unwrap();
        let mut settings = Settings::default();
        settings.converted.insert("B0CCC".into(), epub.clone());

        let books = scan(&f.content, &f.data, &settings);
        let summary: Vec<(&str, BookState, u64)> =
            books.iter().map(|b| (b.asin.as_str(), b.state, b.size)).collect();
        assert_eq!(
            summary,
            vec![
                ("B0AAA", BookState::Locked, 10),
                ("B0BBB", BookState::Unlocked, 20),
                ("B0CCC", BookState::Converted, 30),
            ]
        );
        assert_eq!(books[0].unlocked, None);
        assert_eq!(books[1].unlocked, Some(unlocked.join("B0BBB.kfx-zip")));
        assert_eq!(books[2].epub, Some(epub));
        assert_eq!(books[0].source, f.content.join("B0AAA_EBOK").join("B0AAA_EBOK.azw"));
    }

    #[test]
    fn deleted_epub_falls_back_to_unlocked() {
        let f = fixture();
        add_book(&f.content, "B0AAA", 10);
        let unlocked = f.data.join("unlocked");
        fs::create_dir_all(&unlocked).unwrap();
        fs::write(unlocked.join("B0AAA.kfx-zip"), b"z").unwrap();
        let mut settings = Settings::default();
        settings.converted.insert("B0AAA".into(), f.out.join("gone.epub"));

        let books = scan(&f.content, &f.data, &settings);
        assert_eq!(books[0].state, BookState::Unlocked);
        assert_eq!(books[0].epub, None);
    }

    #[test]
    fn deleted_epub_without_unlocked_copy_is_locked() {
        let f = fixture();
        add_book(&f.content, "B0AAA", 10);
        let mut settings = Settings::default();
        settings.converted.insert("B0AAA".into(), f.out.join("gone.epub"));
        assert_eq!(scan(&f.content, &f.data, &settings)[0].state, BookState::Locked);
    }

    #[test]
    fn ignores_folders_that_are_not_books() {
        let f = fixture();
        add_book(&f.content, "B0AAA", 10);
        fs::create_dir_all(f.content.join("B0ZZZ_EBOK")).unwrap(); // sin .azw
        fs::create_dir_all(f.content.join("other")).unwrap();
        fs::write(f.content.join("book_asset.db"), b"db").unwrap();
        fs::write(f.content.join("B0YYY_EBOK"), b"un archivo, no una carpeta").unwrap();

        let books = scan(&f.content, &f.data, &Settings::default());
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].asin, "B0AAA");
    }
}

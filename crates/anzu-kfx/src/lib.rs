//! Reader for DRM-free KFX and writer of EPUB 3.

pub mod book;
pub mod container;
pub mod epub;
pub mod ion;
pub mod symbols;
pub mod yj;
#[cfg(test)]
mod testkit;

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid Ion: {0}")]
    Ion(String),
    #[error("the file is not KFX")]
    NotKfx,
    #[error("the book is still encrypted (DRMION); it has to be unlocked first")]
    StillEncrypted,
    #[error("invalid KFX container: {0}")]
    Container(String),
    #[error("the book does not declare a reading order")]
    NoReadingOrder,
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

/// Result of a conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    pub path: std::path::PathBuf,
    pub title: String,
    pub warnings: Vec<String>,
}

/// The few words the converter writes into a book. English by default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    /// Title of the table of contents. Default: "Contents".
    pub contents: String,
    /// Word used for a chapter with no heading, followed by its number. Default: "Chapter".
    pub chapter: String,
    /// Title of a book whose metadata has none. Default: "Untitled".
    pub untitled: String,
}

impl Default for Labels {
    fn default() -> Self {
        Labels { contents: "Contents".to_string(), chapter: "Chapter".to_string(), untitled: "Untitled".to_string() }
    }
}

/// Converts a DRM-free KFX (`.kfx-zip` or a single container) into an EPUB inside `out_dir`.
///
/// The file is named after the book's title. It never overwrites: if the name is taken, it
/// appends ` (2)`, ` (3)`...
pub fn convert(input: &Path, out_dir: &Path) -> Result<Converted, Error> {
    convert_with(input, out_dir, &Labels::default())
}

/// Like [`convert`], with the words written into the book taken from `labels`.
pub fn convert_with(input: &Path, out_dir: &Path, labels: &Labels) -> Result<Converted, Error> {
    let fragments = container::open(input)?;
    let book = book::build_with(&fragments, &labels.untitled)?;
    std::fs::create_dir_all(out_dir)?;

    let base = epub::safe_file_name(&book.meta.title);
    let mut path = out_dir.join(format!("{base}.epub"));
    let mut n = 2;
    while path.exists() {
        path = out_dir.join(format!("{base} ({n}).epub"));
        n += 1;
    }
    epub::write_epub_with(&book, &path, labels)?;
    Ok(Converted { path, title: book.meta.title, warnings: book.warnings })
}

/// The title of a DRM-free KFX, or `None` if it cannot be read or the book declares none.
pub fn read_title(input: &Path) -> Option<String> {
    book::title(&container::open(input).ok()?)
}

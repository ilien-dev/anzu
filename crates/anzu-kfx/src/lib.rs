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
    /// The name was taken, so the file got a number: `Title (2).epub`.
    pub renamed: bool,
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
    let (path, renamed) = claim_name(out_dir, &base)?;
    if let Err(error) = epub::write_epub_with(&book, &path, labels) {
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(Converted { path, title: book.meta.title, warnings: book.warnings, renamed })
}

/// Creates an empty `<base>.epub` in `dir`, or `<base> (2).epub`, `<base> (3).epub`... when
/// the name is taken, and returns its path and whether it got a number. Creating the file
/// claims the name, so two books with the same title converted at the same time never write
/// to the same file.
fn claim_name(dir: &Path, base: &str) -> Result<(std::path::PathBuf, bool), Error> {
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{base}.epub") } else { format!("{base} ({n}).epub") };
        let path = dir.join(name);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Ok((path, n > 1)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(error) => return Err(error.into()),
        }
    }
}

/// The title of a DRM-free KFX, or `None` if it cannot be read or the book declares none.
pub fn read_title(input: &Path) -> Option<String> {
    book::title(&container::open(input).ok()?)
}

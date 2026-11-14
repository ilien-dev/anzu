//! Lector de KFX sin DRM y escritor de EPUB 3.

pub mod book;
pub mod container;
pub mod epub;
pub mod ion;
pub mod symbols;
pub mod yj;
#[cfg(test)]
mod testkit;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Ion no válido: {0}")]
    Ion(String),
    #[error("el archivo no es KFX")]
    NotKfx,
    #[error("el libro sigue cifrado (DRMION); hay que desbloquearlo antes")]
    StillEncrypted,
    #[error("contenedor KFX no válido: {0}")]
    Container(String),
    #[error("el libro no declara un orden de lectura")]
    NoReadingOrder,
    #[error("E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

/// Resultado de una conversión.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    pub path: std::path::PathBuf,
    pub title: String,
    pub warnings: Vec<String>,
}

/// Convierte un KFX sin DRM (`.kfx-zip` o contenedor suelto) en un EPUB dentro de `out_dir`.
///
/// El archivo se llama como el título del libro. Nunca sobrescribe: si ya existe, añade
/// ` (2)`, ` (3)`...
pub fn convert(input: &std::path::Path, out_dir: &std::path::Path) -> Result<Converted, Error> {
    let fragments = container::open(input)?;
    let book = book::build(&fragments)?;
    std::fs::create_dir_all(out_dir)?;

    let base = epub::safe_file_name(&book.meta.title);
    let mut path = out_dir.join(format!("{base}.epub"));
    let mut n = 2;
    while path.exists() {
        path = out_dir.join(format!("{base} ({n}).epub"));
        n += 1;
    }
    epub::write_epub(&book, &path)?;
    Ok(Converted { path, title: book.meta.title, warnings: book.warnings })
}

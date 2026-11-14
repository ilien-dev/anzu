//! Lector de KFX sin DRM y escritor de EPUB 3.

pub mod ion;
pub mod symbols;

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

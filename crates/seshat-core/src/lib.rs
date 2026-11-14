//! Detección de Kindle, publicaciones de la herramienta, compatibilidad y estado de la biblioteca.

pub mod compat;
pub mod releases;
pub mod version;

pub use version::KindleVersion;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("versión no válida: {0}")]
    BadVersion(String),
    #[error("sin conexión o error de red: {0}")]
    Network(String),
    #[error("GitHub ha limitado las peticiones; inténtalo más tarde")]
    RateLimited,
    #[error("no hay ninguna publicación de la herramienta")]
    NoRelease,
    #[error("respuesta de GitHub no válida: {0}")]
    BadResponse(String),
    #[error("la publicación no trae huella SHA-256; no se ejecutará")]
    NoDigest,
    #[error("la huella SHA-256 de la descarga no coincide")]
    ChecksumMismatch,
    #[error("la app Kindle está abierta")]
    KindleRunning,
    #[error("el desbloqueo falló (código {code}):\n{tail}")]
    UnlockFailed { code: i32, tail: String },
    #[error("E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

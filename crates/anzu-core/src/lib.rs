//! Kindle detection, tool releases, compatibility and library state.

pub mod compat;
pub mod kindle;
pub mod library;
pub mod releases;
pub mod settings;
pub mod tool;
pub mod unlock;
pub mod version;

pub use version::KindleVersion;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid version: {0}")]
    BadVersion(String),
    #[error("no connection or network error: {0}")]
    Network(String),
    #[error("GitHub has rate-limited the requests; try again later")]
    RateLimited,
    #[error("no release of the tool is available")]
    NoRelease,
    #[error("invalid response from GitHub: {0}")]
    BadResponse(String),
    #[error("the release has no SHA-256 digest; it will not be run")]
    NoDigest,
    #[error("the SHA-256 digest of the download does not match")]
    ChecksumMismatch,
    #[error("the Kindle app is open")]
    KindleRunning,
    #[error("unlocking failed (code {code}):\n{tail}")]
    UnlockFailed { code: i32, tail: String },
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

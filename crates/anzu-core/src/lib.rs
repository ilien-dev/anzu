//! Kindle detection, tool releases, compatibility and library state.

pub mod catalog;
pub mod compat;
pub mod install;
pub mod kindle;
pub mod library;
pub mod releases;
pub mod settings;
pub mod tool;
pub mod unlock;
pub mod update;
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
    #[error("taskkill did not accept the request: {0}")]
    CloseRefused(String),
    #[error("it was still running {0} s after it was asked to close")]
    CloseTimedOut(u64),
    #[error("the installer is larger than expected")]
    InstallerTooLarge,
    #[error("the installed copy of Anzu is in use; close it and try again")]
    InstallInUse,
    #[error("the shortcut could not be created: {0}")]
    Shortcut(String),
    #[error("the app could not be registered in Windows: {0}")]
    Registry(String),
    #[error("unlocking failed (code {code}):\n{tail}")]
    UnlockFailed { code: i32, tail: String },
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

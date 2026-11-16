//! Anzu: Kindle to EPUB.
//!
//! The library holds the whole app, so the same interface runs as the Windows program
//! (`main.rs`) and as the web demo (`crates/anzu-web`).

// The web demo leaves out the native window, the installer, the icon and the update
// watch, so what only they use is unused there.
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

pub mod app;
pub mod cli;
pub mod demo;
pub mod i18n;
pub mod pipeline;
#[cfg(not(target_arch = "wasm32"))]
pub mod setup;
pub mod ui;

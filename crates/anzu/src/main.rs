//! Anzu: Kindle to EPUB.

// The release build is a windowed program: it opens no console.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod cli;
mod i18n;
mod pipeline;
mod ui;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anzu_core::settings::{self, Settings};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return match ui::launch() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Error: {}: {error}", i18n::EN.window_error);
                ExitCode::FAILURE
            }
        };
    }

    attach_parent_console();
    // The command line speaks the language chosen in the app.
    let t = i18n::strings(Settings::load(&settings::data_dir()).language);
    match cli::parse(&args) {
        Ok(cli::Command::Gui) => ExitCode::SUCCESS,
        Ok(cli::Command::Help) => {
            println!("{}", t.usage);
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Version) => {
            println!("anzu {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Convert { input, out_dir }) => convert(t, &input, out_dir),
        Err(error) => {
            eprintln!("Error: {}\n\n{}", t.cli_error(&error), t.usage);
            ExitCode::from(2)
        }
    }
}

/// A windowed program has no console of its own. When used from a terminal (with
/// arguments) it attaches to the one that launched it, so it can write there.
#[cfg(windows)]
fn attach_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    // SAFETY: the call takes no pointers; with no console to attach to (or one already
    // attached) it returns 0 and changes nothing.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}

/// Converts one file: the EPUB path goes to stdout; warnings and errors, to stderr.
fn convert(t: &i18n::Strings, input: &Path, out_dir: Option<PathBuf>) -> ExitCode {
    let out_dir = out_dir.unwrap_or_else(|| PathBuf::from("."));
    match anzu_kfx::convert_with(input, &out_dir, &t.labels()) {
        Ok(converted) => {
            for warning in &converted.warnings {
                eprintln!("{}", t.warning(warning));
            }
            println!("{}", converted.path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Error: {}", t.kfx_error(&error));
            ExitCode::FAILURE
        }
    }
}

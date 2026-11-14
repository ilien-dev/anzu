//! Seshat: de Kindle a EPUB.

// En la versión de publicación la app es un programa de ventana: no abre consola.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod cli;
mod pipeline;
mod ui;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        attach_parent_console();
    }
    match cli::parse(&args) {
        Ok(cli::Command::Gui) => match ui::launch() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Error: no se pudo abrir la interfaz: {error}");
                ExitCode::FAILURE
            }
        },
        Ok(cli::Command::Help) => {
            println!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Version) => {
            println!("seshat {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(cli::Command::Convert { input, out_dir }) => convert(&input, out_dir),
        Err(message) => {
            eprintln!("Error: {message}\n\n{}", cli::USAGE);
            ExitCode::from(2)
        }
    }
}

/// Un programa de ventana no tiene consola propia. Cuando se usa desde una terminal
/// (con argumentos), se engancha a la de quien lo lanzó para poder escribir en ella.
#[cfg(windows)]
fn attach_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    // SAFETY: la llamada no recibe punteros; si no hay consola a la que engancharse
    // (o ya hay una) devuelve 0 y no cambia nada.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}

/// Convierte un archivo: la ruta del EPUB va a stdout; avisos y errores, a stderr.
fn convert(input: &Path, out_dir: Option<PathBuf>) -> ExitCode {
    let out_dir = out_dir.unwrap_or_else(|| PathBuf::from("."));
    match seshat_kfx::convert(input, &out_dir) {
        Ok(converted) => {
            for warning in &converted.warnings {
                eprintln!("aviso: {warning}");
            }
            println!("{}", converted.path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Error: {error}");
            ExitCode::FAILURE
        }
    }
}

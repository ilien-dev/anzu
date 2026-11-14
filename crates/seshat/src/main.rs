//! Seshat: de Kindle a EPUB.

mod cli;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        Ok(cli::Command::Gui) => {
            eprintln!("La interfaz todavía no está disponible. Usa `seshat convert <archivo>`.");
            ExitCode::from(2)
        }
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

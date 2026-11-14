//! Ejecución del archiver de Satsuoni como proceso externo.
//!
//! Interfaz confirmada en `Other_Tools/KRFKeyExtractor/MSIXKFXArchiver.cpp`:
//!
//! ```text
//! exe [kindle-documents] [output-folder] [output-k4i] [dll-folder] [extra ...] [--data-dir <ruta absoluta>]
//! ```
//!
//! - Escribe `<output-folder>/<ASIN>_EBOK.kfx-zip` por libro y omite los que ya existen.
//! - Deja `books.keyfile` en el directorio desde el que se lanza.
//! - Sin `--data-dir` usa `<unidad>:/Data` como zona temporal; aquí siempre se le pasa
//!   una carpeta propia para no escribir fuera de los datos de Seshat.
//!
//! Este módulo no descifra nada: solo lanza el proceso y recoge sus archivos.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;

use crate::{kindle, library, Error};

const OUTPUT_SUFFIX: &str = "_EBOK.kfx-zip";
const TAIL_LINES: usize = 20;

pub struct UnlockRequest<'a> {
    pub exe: &'a Path,
    pub content_dir: &'a Path,
    /// Carpeta de salida del archiver; debe estar vacía para que no omita libros.
    pub out_dir: &'a Path,
    pub keyfile: &'a Path,
    /// Directorio de trabajo del proceso y raíz de su zona temporal.
    pub work_dir: &'a Path,
}

pub fn build_command(req: &UnlockRequest) -> Command {
    let mut cmd = kindle::quiet_command(&req.exe.to_string_lossy());
    cmd.arg(req.content_dir)
        .arg(req.out_dir)
        .arg(req.keyfile)
        .arg("--data-dir")
        .arg(req.work_dir.join("runtime"))
        .current_dir(req.work_dir);
    cmd
}

/// Los `<ASIN>_EBOK.kfx-zip` de una carpeta, como `(ASIN, ruta)`, ordenados por ASIN.
pub fn collect_outputs(out_dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(out_dir) else { return Vec::new() };
    let mut found: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let asin = name.strip_suffix(OUTPUT_SUFFIX).filter(|a| !a.is_empty())?.to_string();
            Some((asin, e.path()))
        })
        .collect();
    found.sort();
    found
}

/// Mueve cada salida a `data_dir/unlocked/<ASIN>.kfx-zip`, sustituyendo copias anteriores.
pub fn store_outputs(outputs: Vec<(String, PathBuf)>, data_dir: &Path) -> Result<Vec<(String, PathBuf)>, Error> {
    let mut stored = Vec::with_capacity(outputs.len());
    for (asin, from) in outputs {
        let to = library::unlocked_path(data_dir, &asin);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if to.exists() {
            std::fs::remove_file(&to)?;
        }
        if std::fs::rename(&from, &to).is_err() {
            std::fs::copy(&from, &to)?;
            std::fs::remove_file(&from)?;
        }
        stored.push((asin, to));
    }
    Ok(stored)
}

/// Lanza el proceso, entrega cada línea de stdout y stderr a `on_line` y devuelve su código de salida.
pub fn run_streaming(mut cmd: Command, on_line: &mut dyn FnMut(&str)) -> Result<i32, Error> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let (tx, rx) = mpsc::channel::<String>();
    let mut readers = Vec::new();
    let pipes: [Option<Box<dyn Read + Send>>; 2] = [
        child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
        child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
    ];
    for pipe in pipes.into_iter().flatten() {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || {
            let mut reader = BufReader::new(pipe);
            let mut buf = Vec::new();
            while reader.read_until(b'\n', &mut buf).is_ok_and(|n| n > 0) {
                let line = String::from_utf8_lossy(&buf);
                if tx.send(line.trim_end_matches(['\r', '\n']).to_string()).is_err() {
                    break;
                }
                buf.clear();
            }
        }));
    }
    drop(tx);
    for line in rx {
        on_line(&line);
    }
    for r in readers {
        let _ = r.join();
    }
    Ok(child.wait()?.code().unwrap_or(-1))
}

/// Decide el resultado de una ejecución a partir de su código de salida y de lo que produjo.
pub fn outcome(code: i32, log: &[String], outputs: Vec<(String, PathBuf)>) -> Result<Vec<(String, PathBuf)>, Error> {
    if code != 0 && outputs.is_empty() {
        let start = log.len().saturating_sub(TAIL_LINES);
        return Err(Error::UnlockFailed { code, tail: log[start..].join("\n") });
    }
    Ok(outputs)
}

pub fn unlock_with(
    req: &UnlockRequest,
    kindle_running: bool,
    data_dir: &Path,
    on_line: &mut dyn FnMut(&str),
) -> Result<Vec<(String, PathBuf)>, Error> {
    if kindle_running {
        return Err(Error::KindleRunning);
    }
    if req.out_dir.exists() {
        std::fs::remove_dir_all(req.out_dir)?;
    }
    std::fs::create_dir_all(req.out_dir)?;
    std::fs::create_dir_all(req.work_dir.join("runtime"))?;
    if let Some(parent) = req.keyfile.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut log = Vec::new();
    let code = run_streaming(build_command(req), &mut |line| {
        log.push(line.to_string());
        on_line(line);
    })?;
    let produced = outcome(code, &log, collect_outputs(req.out_dir))?;
    store_outputs(produced, data_dir)
}

/// Desbloquea los libros de `content_dir`. Devuelve `(ASIN, ruta del .kfx-zip)` por libro.
pub fn unlock(req: &UnlockRequest, data_dir: &Path, on_line: &mut dyn FnMut(&str)) -> Result<Vec<(String, PathBuf)>, Error> {
    unlock_with(req, kindle::is_running(), data_dir, on_line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::fs;

    fn request<'a>(exe: &'a Path, content: &'a Path, out: &'a Path, key: &'a Path, work: &'a Path) -> UnlockRequest<'a> {
        UnlockRequest { exe, content_dir: content, out_dir: out, keyfile: key, work_dir: work }
    }

    #[test]
    fn command_matches_the_archiver_interface() {
        let (exe, content, out, key, work) =
            (Path::new("tool.exe"), Path::new("Content"), Path::new("out"), Path::new("k.k4i"), Path::new("work"));
        let cmd = build_command(&request(exe, content, out, key, work));

        assert_eq!(cmd.get_program(), OsStr::new("tool.exe"));
        let args: Vec<&OsStr> = cmd.get_args().collect();
        let runtime = work.join("runtime");
        assert_eq!(
            args,
            vec![
                OsStr::new("Content"),
                OsStr::new("out"),
                OsStr::new("k.k4i"),
                OsStr::new("--data-dir"),
                runtime.as_os_str(),
            ]
        );
        assert_eq!(cmd.get_current_dir(), Some(work));
    }

    #[test]
    fn collects_only_book_archives() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("B0AAA_EBOK.kfx-zip"), b"z").unwrap();
        fs::write(tmp.path().join("B0BBB_EBOK.kfx-zip"), b"z").unwrap();
        fs::write(tmp.path().join("notes.txt"), b"n").unwrap();
        fs::write(tmp.path().join("_EBOK.kfx-zip"), b"z").unwrap();
        fs::write(tmp.path().join("loose.kfx-zip"), b"z").unwrap();
        fs::create_dir(tmp.path().join("B0DIR_EBOK.kfx-zip")).unwrap();

        let found = collect_outputs(tmp.path());
        assert_eq!(
            found,
            vec![
                ("B0AAA".to_string(), tmp.path().join("B0AAA_EBOK.kfx-zip")),
                ("B0BBB".to_string(), tmp.path().join("B0BBB_EBOK.kfx-zip")),
            ]
        );
        assert!(collect_outputs(&tmp.path().join("missing")).is_empty());
    }

    #[test]
    fn stores_outputs_under_the_data_dir_replacing_old_copies() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("out");
        let data = tmp.path().join("data");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("B0AAA_EBOK.kfx-zip"), b"new").unwrap();
        let old = crate::library::unlocked_path(&data, "B0AAA");
        fs::create_dir_all(old.parent().unwrap()).unwrap();
        fs::write(&old, b"old").unwrap();

        let stored = store_outputs(collect_outputs(&out), &data).unwrap();

        assert_eq!(stored, vec![("B0AAA".to_string(), old.clone())]);
        assert_eq!(fs::read(&old).unwrap(), b"new");
        assert!(!out.join("B0AAA_EBOK.kfx-zip").exists());
    }

    #[test]
    fn refuses_to_start_while_kindle_is_running() {
        let tmp = tempfile::tempdir().unwrap();
        let missing_exe = tmp.path().join("no-such-tool.exe");
        let out = tmp.path().join("out");
        let work = tmp.path().join("work");
        let req = request(&missing_exe, tmp.path(), &out, Path::new("k.k4i"), &work);
        let mut lines = Vec::new();

        let result = unlock_with(&req, true, tmp.path(), &mut |l| lines.push(l.to_string()));

        assert!(matches!(result, Err(Error::KindleRunning)));
        assert!(lines.is_empty());
        assert!(!work.exists(), "no debe prepararse nada si Kindle está abierta");
    }

    #[test]
    fn streams_lines_from_both_pipes_and_returns_the_exit_code() {
        let mut cmd = crate::kindle::quiet_command("cmd");
        cmd.args(["/C", "echo uno& echo dos 1>&2& exit 3"]);
        let mut lines = Vec::new();

        let code = run_streaming(cmd, &mut |l| lines.push(l.trim().to_string())).unwrap();

        lines.sort();
        assert_eq!(lines, vec!["dos".to_string(), "uno".to_string()]);
        assert_eq!(code, 3);
    }

    #[test]
    fn missing_executable_is_an_io_error() {
        let cmd = crate::kindle::quiet_command("seshat-no-such-program-xyz");
        assert!(matches!(run_streaming(cmd, &mut |_| {}), Err(Error::Io(_))));
    }

    #[test]
    fn failure_without_outputs_reports_code_and_log_tail() {
        let log: Vec<String> = (1..=30).map(|i| format!("línea {i}")).collect();
        match outcome(-1, &log, Vec::new()) {
            Err(Error::UnlockFailed { code, tail }) => {
                assert_eq!(code, -1);
                let kept: Vec<&str> = tail.lines().collect();
                assert_eq!(kept.len(), 20);
                assert_eq!(kept[0], "línea 11");
                assert_eq!(kept[19], "línea 30");
            }
            other => panic!("se esperaba UnlockFailed, llegó {other:?}"),
        }
    }

    #[test]
    fn partial_success_keeps_what_was_unlocked() {
        let got = vec![("B0AAA".to_string(), PathBuf::from("a"))];
        assert_eq!(outcome(1, &[], got.clone()).unwrap(), got);
    }

    #[test]
    fn clean_exit_without_outputs_is_an_empty_result() {
        assert!(outcome(0, &[], Vec::new()).unwrap().is_empty());
    }
}

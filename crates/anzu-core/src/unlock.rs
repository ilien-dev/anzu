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
//!   una carpeta propia para no escribir fuera de los datos de Anzu.
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
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.len() > 0))
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

/// Tiempo máximo sin salida antes de dar la herramienta por colgada.
pub const IDLE_LIMIT: std::time::Duration = std::time::Duration::from_secs(600);

/// Lanza el proceso, entrega cada línea de stdout y stderr a `on_line` y devuelve su código de salida.
pub fn run_streaming(cmd: Command, on_line: &mut dyn FnMut(&str)) -> Result<i32, Error> {
    run_streaming_with(cmd, IDLE_LIMIT, on_line)
}

/// Como [`run_streaming`], deteniendo el proceso si pasa `idle` sin escribir nada.
pub fn run_streaming_with(
    mut cmd: Command,
    idle: std::time::Duration,
    on_line: &mut dyn FnMut(&str),
) -> Result<i32, Error> {
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
    loop {
        match rx.recv_timeout(idle) {
            Ok(line) => on_line(&line),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Los hilos lectores no se esperan: un proceso nieto puede retener la tubería.
                let _ = child.kill();
                return Err(Error::Other(
                    "la herramienta de desbloqueo dejó de responder y se detuvo".to_string(),
                ));
            }
        }
    }
    for r in readers {
        let _ = r.join();
    }
    Ok(child.wait()?.code().unwrap_or(-1))
}

const HIDDEN: &str = "[oculto]";
/// Lo que se muestra en lugar de una línea que el filtro no reconoce.
pub const WITHHELD: &str = "[línea de la herramienta omitida por seguridad]";
const MIN_BLOB: usize = 24;
/// Comienzos de las líneas de progreso que se sabe que no llevan secretos.
const SAFE_PREFIXES: [&str; 10] = [
    "Read file of",
    "Decrypting DRMION",
    "DRMION decrypted",
    "Current keyid",
    "Could not open",
    "Skipping",
    "Found keyless book",
    "File ",
    "WARNING",
    "Error",
];

/// Filtra una línea de la herramienta antes de mostrarla o guardarla.
///
/// La herramienta imprime el número de serie del dispositivo, secretos de la cuenta y
/// claves de libros en muchos formatos distintos. Por eso el filtro es una lista blanca:
/// solo pasan las líneas de progreso conocidas, y aun en ellas se oculta cualquier
/// cadena larga que parezca una clave. Todo lo demás se sustituye por [`WITHHELD`].
pub fn redact(line: &str) -> String {
    if line.trim().is_empty() {
        return line.to_string();
    }
    if !SAFE_PREFIXES.iter().any(|p| line.starts_with(p)) {
        return WITHHELD.to_string();
    }
    let is_blob_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '+' | '=' | '/' | '-' | '_');
    let mut out = String::with_capacity(line.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if run.len() >= MIN_BLOB {
            out.push_str(HIDDEN);
        } else {
            out.push_str(run);
        }
        run.clear();
    };
    for c in line.chars() {
        if is_blob_char(c) {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Como [`run_streaming`], pero cada línea pasa por [`redact`] antes de salir de aquí.
/// Varias líneas omitidas seguidas se muestran una sola vez.
pub fn run_redacted(cmd: Command, on_line: &mut dyn FnMut(&str)) -> Result<(i32, Vec<String>), Error> {
    let mut log: Vec<String> = Vec::new();
    let code = run_streaming(cmd, &mut |line| {
        let safe = redact(line);
        if safe == WITHHELD && log.last().is_some_and(|l| l == WITHHELD) {
            return;
        }
        on_line(&safe);
        log.push(safe);
    })?;
    Ok((code, log))
}

/// Deja en `out_dir` un marcador vacío por cada libro que ya está desbloqueado, para que
/// la herramienta lo omita en lugar de rehacerlo. Devuelve cuántos marcó.
pub fn seed_placeholders(out_dir: &Path, data_dir: &Path) -> Result<usize, Error> {
    let Ok(entries) = std::fs::read_dir(data_dir.join("unlocked")) else { return Ok(0) };
    let mut seeded = 0;
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else { continue };
        let Some(asin) = name.strip_suffix(".kfx-zip").filter(|a| !a.is_empty()) else { continue };
        if entry.metadata().is_ok_and(|m| m.is_file() && m.len() > 0) {
            std::fs::write(out_dir.join(format!("{asin}{OUTPUT_SUFFIX}")), b"")?;
            seeded += 1;
        }
    }
    Ok(seeded)
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
    seed_placeholders(req.out_dir, data_dir)?;
    std::fs::create_dir_all(req.work_dir.join("runtime"))?;
    if let Some(parent) = req.keyfile.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let (code, log) = run_redacted(build_command(req), on_line)?;
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
        let cmd = crate::kindle::quiet_command("anzu-no-such-program-xyz");
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

    #[test]
    fn lines_the_filter_does_not_recognise_are_withheld() {
        for line in [
            "Working secret: QUJD",
            "device_serial_number:0123456789ABCDEF0123456789ABCDEF",
            "Found key in map, 00112233445566778899aabbccddeeff ",
            "DSN: G000ABCDEF",
            "Md5: 0011 Value: hunter2",
            "Got key: 0011",
            "some_setting : short-value",
            "key 00112233445566778899aabbccddeeff secret: x",
            "Writing DSN and secrets into somewhere",
        ] {
            assert_eq!(redact(line), WITHHELD, "{line}");
        }
    }

    #[test]
    fn known_progress_lines_pass() {
        for line in [
            "Read file of 871790 bytes",
            "Decrypting DRMION... añö",
            "DRMION decrypted and saved.",
            "Skipping",
            "Found keyless book, packing it for completion",
            "",
        ] {
            assert_eq!(redact(line), line);
        }
    }

    #[test]
    fn long_tokens_are_masked_even_in_known_lines() {
        assert_eq!(
            redact("Current keyid amzn1.drm-key.v1.4d3c7674-dc00-4b3b-95b2-07d47e91cede"),
            "Current keyid amzn1.drm-key.v1.[oculto]"
        );
        assert_eq!(redact("Could not open QUJDREVGR0hJSktM/TU5PUFFSU1RVVldYWVo="), "Could not open [oculto]");
        assert_eq!(redact("Read file of 00112233445566778899aabb bytes"), "Read file of [oculto] bytes");
    }

    #[test]
    fn process_output_is_filtered_and_withheld_runs_collapse() {
        let mut cmd = crate::kindle::quiet_command("cmd");
        cmd.args(["/C", "echo Working secret: hunter2& echo DSN: abc& echo Skipping"]);
        let mut seen = Vec::new();

        let (code, log) = run_redacted(cmd, &mut |l| seen.push(l.to_string())).unwrap();

        assert_eq!(code, 0);
        assert_eq!(seen, vec![WITHHELD.to_string(), "Skipping".to_string()]);
        assert_eq!(seen, log);
    }

    #[test]
    fn a_tool_that_goes_silent_is_stopped() {
        let mut cmd = crate::kindle::quiet_command("cmd");
        cmd.args(["/C", "echo Skipping& ping -n 9 127.0.0.1 >nul"]);
        let started = std::time::Instant::now();

        let result = run_streaming_with(cmd, std::time::Duration::from_secs(1), &mut |_| {});

        assert!(matches!(result, Err(Error::Other(_))), "{result:?}");
        assert!(started.elapsed() < std::time::Duration::from_secs(6));
    }

    #[test]
    fn books_already_unlocked_are_not_redone() {
        let tmp = tempfile::tempdir().unwrap();
        let (out, data) = (tmp.path().join("out"), tmp.path().join("data"));
        fs::create_dir_all(&out).unwrap();
        let kept = crate::library::unlocked_path(&data, "B0AAA");
        fs::create_dir_all(kept.parent().unwrap()).unwrap();
        fs::write(&kept, b"ya desbloqueado").unwrap();

        assert_eq!(seed_placeholders(&out, &data).unwrap(), 1);

        assert_eq!(fs::metadata(out.join("B0AAA_EBOK.kfx-zip")).unwrap().len(), 0);
        assert!(collect_outputs(&out).is_empty(), "un marcador vacío no es un libro");
        fs::write(out.join("B0BBB_EBOK.kfx-zip"), b"nuevo").unwrap();
        let stored = store_outputs(collect_outputs(&out), &data).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, "B0BBB");
        assert_eq!(fs::read(&kept).unwrap(), b"ya desbloqueado");
    }
}

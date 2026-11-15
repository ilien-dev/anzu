//! Running Satsuoni's archiver as an external process.
//!
//! Interface confirmed in `Other_Tools/KRFKeyExtractor/MSIXKFXArchiver.cpp`:
//!
//! ```text
//! exe [kindle-documents] [output-folder] [output-k4i] [dll-folder] [extra ...] [--data-dir <absolute path>]
//! ```
//!
//! - Writes `<output-folder>/<ASIN>_EBOK.kfx-zip` per book and skips the ones that already exist.
//! - Leaves `books.keyfile` in the directory it is launched from.
//! - Without `--data-dir` it uses `<drive>:/Data` as its scratch area; here it is always given
//!   a folder of its own so that it does not write outside Anzu's data.
//!
//! This module decrypts nothing: it only launches the process and collects its files.

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
    /// The archiver's output folder; it must be empty so that it skips no books.
    pub out_dir: &'a Path,
    pub keyfile: &'a Path,
    /// The process's working directory and the root of its scratch area.
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

/// The `<ASIN>_EBOK.kfx-zip` files of a folder, as `(ASIN, path)`, sorted by ASIN.
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

/// Moves each output to `data_dir/unlocked/<ASIN>.kfx-zip`, replacing earlier copies.
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

/// Longest time without output before the tool is taken to have hung.
pub const IDLE_LIMIT: std::time::Duration = std::time::Duration::from_secs(600);

/// Launches the process, hands each line of stdout and stderr to `on_line` and returns its exit code.
pub fn run_streaming(cmd: Command, on_line: &mut dyn FnMut(&str)) -> Result<i32, Error> {
    run_streaming_with(cmd, IDLE_LIMIT, on_line)
}

/// Like [`run_streaming`], stopping the process if `idle` passes without it writing anything.
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
                // The reader threads are not joined: a grandchild process can keep the pipe open.
                let _ = child.kill();
                return Err(Error::Other(
                    "the unlock tool stopped responding and was stopped".to_string(),
                ));
            }
        }
    }
    for r in readers {
        let _ = r.join();
    }
    Ok(child.wait()?.code().unwrap_or(-1))
}

const HIDDEN: &str = "[hidden]";
/// What is shown in place of a line the filter does not recognise.
pub const WITHHELD: &str = "[tool output line withheld for safety]";
const MIN_BLOB: usize = 24;
/// Beginnings of the progress lines that are known to carry no secrets.
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

/// Filters a line from the tool before it is shown or stored.
///
/// The tool prints the device serial number, account secrets and book keys in many
/// different formats. That is why the filter is an allowlist: only the known progress
/// lines pass, and even in those any long string that looks like a key is hidden.
/// Everything else is replaced by [`WITHHELD`].
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

/// Like [`run_streaming`], but every line goes through [`redact`] before it leaves here.
/// Several withheld lines in a row are shown only once.
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

/// Leaves an empty placeholder in `out_dir` for each book that is already unlocked, so that
/// the tool skips it instead of redoing it. Returns how many it placed.
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

/// Decides the result of a run from its exit code and from what it produced.
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

/// Unlocks the books in `content_dir`. Returns `(ASIN, path of the .kfx-zip)` per book.
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
        assert!(!work.exists(), "nothing must be prepared while Kindle is open");
    }

    #[test]
    fn streams_lines_from_both_pipes_and_returns_the_exit_code() {
        let mut cmd = crate::kindle::quiet_command("cmd");
        cmd.args(["/C", "echo one& echo two 1>&2& exit 3"]);
        let mut lines = Vec::new();

        let code = run_streaming(cmd, &mut |l| lines.push(l.trim().to_string())).unwrap();

        lines.sort();
        assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(code, 3);
    }

    #[test]
    fn missing_executable_is_an_io_error() {
        let cmd = crate::kindle::quiet_command("anzu-no-such-program-xyz");
        assert!(matches!(run_streaming(cmd, &mut |_| {}), Err(Error::Io(_))));
    }

    #[test]
    fn failure_without_outputs_reports_code_and_log_tail() {
        let log: Vec<String> = (1..=30).map(|i| format!("line {i}")).collect();
        match outcome(-1, &log, Vec::new()) {
            Err(Error::UnlockFailed { code, tail }) => {
                assert_eq!(code, -1);
                let kept: Vec<&str> = tail.lines().collect();
                assert_eq!(kept.len(), 20);
                assert_eq!(kept[0], "line 11");
                assert_eq!(kept[19], "line 30");
            }
            other => panic!("expected UnlockFailed, got {other:?}"),
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
            "Current keyid amzn1.drm-key.v1.[hidden]"
        );
        assert_eq!(redact("Could not open QUJDREVGR0hJSktM/TU5PUFFSU1RVVldYWVo="), "Could not open [hidden]");
        assert_eq!(redact("Read file of 00112233445566778899aabb bytes"), "Read file of [hidden] bytes");
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
        fs::write(&kept, b"already unlocked").unwrap();

        assert_eq!(seed_placeholders(&out, &data).unwrap(), 1);

        assert_eq!(fs::metadata(out.join("B0AAA_EBOK.kfx-zip")).unwrap().len(), 0);
        assert!(collect_outputs(&out).is_empty(), "an empty placeholder is not a book");
        fs::write(out.join("B0BBB_EBOK.kfx-zip"), b"new").unwrap();
        let stored = store_outputs(collect_outputs(&out), &data).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].0, "B0BBB");
        assert_eq!(fs::read(&kept).unwrap(), b"already unlocked");
    }
}

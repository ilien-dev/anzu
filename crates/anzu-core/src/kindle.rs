//! Detection of the Microsoft Store Kindle app and of its process.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::{Error, KindleVersion};

pub const PACKAGE_NAME: &str = "AMZNKindle.AmazonKindleReadingApp";
pub const PACKAGE_FAMILY: &str = "AMZNKindle.AmazonKindleReadingApp_m1sc522ngdk36";
const PROCESS_IMAGE: &str = "Kindle.exe";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindleInstall {
    pub version: KindleVersion,
    pub content_dir: PathBuf,
}

/// Parses the output of `(Get-AppxPackage ...).Version`. Empty means "not installed".
pub fn parse_appx_output(stdout: &str) -> Option<KindleVersion> {
    stdout.lines().map(str::trim).find(|l| !l.is_empty())?.parse().ok()
}

/// The folder where the app keeps the books, given the path of `%LOCALAPPDATA%`.
pub fn content_dir_in(local_app_data: &Path) -> PathBuf {
    local_app_data
        .join("Packages")
        .join(PACKAGE_FAMILY)
        .join("LocalState")
        .join("Classic")
        .join("Content")
}

pub fn tasklist_shows_kindle(stdout: &str) -> bool {
    let image = PROCESS_IMAGE.to_ascii_lowercase();
    stdout.lines().any(|l| l.trim_start().to_ascii_lowercase().starts_with(&image))
}

/// A child process that opens no console window.
pub(crate) fn quiet_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn stdout_of(cmd: &mut Command) -> Option<String> {
    let out = cmd.output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Looks for the installed Kindle app. `None` if it is absent or its version cannot be determined.
pub fn detect() -> Option<KindleInstall> {
    let script = format!("(Get-AppxPackage -Name {PACKAGE_NAME}).Version");
    let out = stdout_of(quiet_command("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]))?;
    let version = parse_appx_output(&out)?;
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    Some(KindleInstall { version, content_dir: content_dir_in(&local) })
}

pub fn is_running() -> bool {
    let filter = format!("IMAGENAME eq {PROCESS_IMAGE}");
    stdout_of(quiet_command("tasklist").args(["/FI", &filter, "/NH"])).is_some_and(|o| tasklist_shows_kindle(&o))
}

/// How long the app is given to quit after it is asked to. Measured: it hides its window at
/// once, then its process stays alive for 10.6 to 12 seconds while it saves its state.
pub const CLOSE_TIMEOUT: Duration = Duration::from_secs(60);
const CLOSE_POLL: Duration = Duration::from_millis(250);

/// What `taskkill` answered: its exit code and what it printed.
#[derive(Debug)]
pub struct Asked {
    pub code: i32,
    pub text: String,
}

fn ask_to_close() -> Result<Asked, Error> {
    let out = quiet_command("taskkill").args(["/IM", PROCESS_IMAGE]).output()?;
    let mut text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.trim().is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(stderr.trim());
    }
    Ok(Asked { code: out.status.code().unwrap_or(-1), text })
}

/// Asks the app to close, without forcing it, and waits until its process has gone.
///
/// Forcing it is avoided on purpose: the app saves the reading position as it quits.
pub fn close_with(
    mut running: impl FnMut() -> bool,
    ask: impl FnOnce() -> Result<Asked, Error>,
    timeout: Duration,
    poll: Duration,
) -> Result<(), Error> {
    if !running() {
        return Ok(());
    }
    let asked = ask()?;
    if asked.code != 0 && running() {
        return Err(Error::CloseRefused(asked.text));
    }
    let deadline = Instant::now() + timeout;
    loop {
        if !running() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Error::CloseTimedOut(timeout.as_secs()));
        }
        std::thread::sleep(poll);
    }
}

/// Asks the app to close and waits up to [`CLOSE_TIMEOUT`].
pub fn close() -> Result<(), Error> {
    close_with(is_running, ask_to_close, CLOSE_TIMEOUT, CLOSE_POLL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_appx_version_line() {
        let v = parse_appx_output("1.1.5634.0\r\n").unwrap();
        assert_eq!(v.to_string(), "1.1.5634.0");
    }

    #[test]
    fn tolerates_surrounding_blank_lines() {
        assert!(parse_appx_output("\r\n  1.0.22920.0  \r\n\r\n").is_some());
    }

    #[test]
    fn empty_output_means_not_installed() {
        assert_eq!(parse_appx_output(""), None);
        assert_eq!(parse_appx_output("\r\n"), None);
    }

    #[test]
    fn error_text_is_not_a_version() {
        assert_eq!(parse_appx_output("Get-AppxPackage : Access is denied."), None);
    }

    #[test]
    fn content_dir_is_under_the_package_family() {
        let dir = content_dir_in(Path::new("L"));
        let expected: PathBuf =
            ["L", "Packages", PACKAGE_FAMILY, "LocalState", "Classic", "Content"].iter().collect();
        assert_eq!(dir, expected);
    }

    #[test]
    fn tasklist_output_with_the_process_means_running() {
        let out = "\r\nImage Name                     PID Session Name\r\n\
                   ========================= ======== ================\r\n\
                   Kindle.exe                  131024 Console\r\n";
        assert!(tasklist_shows_kindle(out));
    }

    fn asked(code: i32, text: &str) -> Result<Asked, Error> {
        Ok(Asked { code, text: text.to_string() })
    }

    #[test]
    fn closing_waits_for_an_app_that_takes_longer_than_ten_seconds() {
        // Measured: Kindle hides its window at once and its process lingers for 10.6 to 12 s.
        assert!(CLOSE_TIMEOUT >= Duration::from_secs(30), "{CLOSE_TIMEOUT:?}");
        let mut polls = 0;
        let result = close_with(
            || {
                polls += 1;
                polls < 6
            },
            || asked(0, "SUCCESS: Sent termination signal"),
            Duration::from_secs(5),
            Duration::from_millis(1),
        );
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(polls, 6);
    }

    #[test]
    fn an_app_that_is_not_running_is_not_asked() {
        let result = close_with(|| false, || panic!("taskkill must not run"), CLOSE_TIMEOUT, Duration::ZERO);
        assert!(result.is_ok());
    }

    #[test]
    fn a_refusal_carries_what_taskkill_said() {
        let said = "ERROR: The process \"Kindle.exe\" with PID 4 could not be terminated.";
        let result = close_with(|| true, || asked(1, said), CLOSE_TIMEOUT, Duration::ZERO);
        match result {
            Err(Error::CloseRefused(text)) => assert_eq!(text, said),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failed_taskkill_for_an_app_that_already_left_is_fine() {
        let mut first = true;
        let result = close_with(
            || std::mem::replace(&mut first, false),
            || asked(128, "ERROR: The process \"Kindle.exe\" not found."),
            CLOSE_TIMEOUT,
            Duration::ZERO,
        );
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn an_app_that_never_leaves_times_out_with_the_wait_in_the_error() {
        let result = close_with(|| true, || asked(0, "SUCCESS"), Duration::from_millis(20), Duration::from_millis(2));
        assert!(matches!(result, Err(Error::CloseTimedOut(_))), "{result:?}");
    }

    #[test]
    fn tasklist_output_without_the_process_means_not_running() {
        assert!(!tasklist_shows_kindle("INFO: No tasks are running which match the specified criteria.\r\n"));
        assert!(!tasklist_shows_kindle(""));
    }
}

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

/// Asks the app to close (without forcing it) and waits up to 10 seconds.
pub fn close() -> Result<(), Error> {
    if !is_running() {
        return Ok(());
    }
    quiet_command("taskkill").args(["/IM", PROCESS_IMAGE]).output()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if !is_running() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(Error::KindleRunning)
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

    #[test]
    fn tasklist_output_without_the_process_means_not_running() {
        assert!(!tasklist_shows_kindle("INFO: No tasks are running which match the specified criteria.\r\n"));
        assert!(!tasklist_shows_kindle(""));
    }
}

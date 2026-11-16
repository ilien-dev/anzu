//! Installing Anzu on Windows, for the current user only.
//!
//! The installer is `anzu.exe` itself, published as `anzu-<version>-windows-x64-setup.exe`:
//! it copies itself into the chosen folder, creates the shortcuts and registers itself in
//! the apps of Windows, all under the user's own profile, so nothing needs administrator
//! rights. The same binary updates an installation (`--update`) and removes it
//! (`--uninstall`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::Error;

/// The executable in the installation folder.
pub const EXE_NAME: &str = "anzu.exe";
/// The name of the shortcuts, on the desktop and in the Start menu.
#[cfg_attr(not(windows), allow(dead_code))]
const SHORTCUT_NAME: &str = "Anzu.lnk";
/// Where Windows lists the apps of the current user that can be uninstalled.
#[cfg(windows)]
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Anzu";

/// The folder proposed for a new installation: `%LOCALAPPDATA%\Programs\Anzu`.
pub fn default_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Programs")
        .join("Anzu")
}

/// Whether an executable with this name is the installer: the release publishes it as
/// `anzu-<version>-windows-x64-setup.exe`, and a browser may add ` (1)` to the name.
pub fn is_setup_name(exe: &Path) -> bool {
    exe.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.to_ascii_lowercase().contains("setup"))
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Copies `source` to `dir\anzu.exe`, replacing the copy there. While that copy is in use
/// (the app is still closing) it tries again until `wait` has passed.
pub fn copy_exe(source: &Path, dir: &Path, wait: Duration) -> Result<PathBuf, Error> {
    let target = dir.join(EXE_NAME);
    if same_file(source, &target) {
        return Ok(target);
    }
    std::fs::create_dir_all(dir)?;
    let staged = dir.join(format!("{EXE_NAME}.new"));
    std::fs::copy(source, &staged)?;
    let start = Instant::now();
    loop {
        match std::fs::rename(&staged, &target) {
            Ok(()) => return Ok(target),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && start.elapsed() < wait => {
                std::thread::sleep(Duration::from_millis(250));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&staged);
                return Err(if e.kind() == std::io::ErrorKind::PermissionDenied { Error::InstallInUse } else { e.into() });
            }
        }
    }
}

/// The shortcuts that point to the app: on the desktop and in the Start menu.
fn shortcut_paths() -> [Option<PathBuf>; 2] {
    #[cfg(windows)]
    {
        [
            dirs::desktop_dir().map(|d| d.join(SHORTCUT_NAME)),
            dirs::data_dir().map(|d| d.join(r"Microsoft\Windows\Start Menu\Programs").join(SHORTCUT_NAME)),
        ]
    }
    #[cfg(not(windows))]
    {
        [None, None]
    }
}

/// Creates the shortcuts asked for, pointing to `exe`.
pub fn create_shortcuts(exe: &Path, desktop: bool, start_menu: bool) -> Result<(), Error> {
    let [desktop_path, start_path] = shortcut_paths();
    for (wanted, path) in [(desktop, desktop_path), (start_menu, start_path)] {
        let Some(path) = path.filter(|_| wanted) else { continue };
        write_shortcut(exe, &path)?;
    }
    Ok(())
}

#[cfg(windows)]
fn write_shortcut(exe: &Path, path: &Path) -> Result<(), Error> {
    let failed = |e: &dyn std::fmt::Display| Error::Shortcut(e.to_string());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut link = mslnk::ShellLink::new(exe).map_err(|e| failed(&e))?;
    link.set_working_dir(exe.parent().map(|d| d.display().to_string()));
    link.set_icon_location(Some(exe.display().to_string()));
    link.create_lnk(path).map_err(|e| failed(&e))
}

#[cfg(not(windows))]
fn write_shortcut(_exe: &Path, _path: &Path) -> Result<(), Error> {
    Ok(())
}

/// Tells Explorer that the icon of `exe` may have changed. Windows caches icons by path, so
/// after an update replaces `anzu.exe` the taskbar, a pinned shortcut and the Start menu
/// would keep the old icon until the cache is rebuilt.
#[cfg(windows)]
pub fn refresh_icons(exe: &Path) {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHChangeNotify(event: i32, flags: u32, item1: *const c_void, item2: *const c_void);
    }
    const SHCNE_UPDATEITEM: i32 = 0x0000_2000;
    const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
    const SHCNF_IDLIST: u32 = 0x0000;
    const SHCNF_PATHW: u32 = 0x0005;

    let pinned = dirs::data_dir()
        .map(|d| d.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar").join(SHORTCUT_NAME));
    let paths = std::iter::once(Some(exe.to_path_buf())).chain(shortcut_paths()).chain([pinned]).flatten();
    for path in paths.filter(|p| p.exists()) {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 path that outlives the call.
        unsafe { SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW, wide.as_ptr().cast(), std::ptr::null()) };
    }
    // SAFETY: this event takes no items.
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, std::ptr::null(), std::ptr::null()) };
}

#[cfg(not(windows))]
pub fn refresh_icons(_exe: &Path) {}

/// Removes the shortcuts, wherever they were created.
pub fn remove_shortcuts() {
    for path in shortcut_paths().into_iter().flatten() {
        let _ = std::fs::remove_file(path);
    }
}

/// Lists the installation in the apps of Windows, with its version and how to remove it.
#[cfg(windows)]
pub fn register(dir: &Path, version: &str) -> Result<(), Error> {
    fn failed(e: impl std::fmt::Display) -> Error {
        Error::Registry(e.to_string())
    }
    let exe = dir.join(EXE_NAME);
    let size_kb = std::fs::metadata(&exe).map(|m| (m.len() / 1024) as u32).unwrap_or(0);
    let key = windows_registry::CURRENT_USER.create(UNINSTALL_KEY).map_err(failed)?;
    let exe_text = exe.display().to_string();
    for (name, value) in [
        ("DisplayName", "Anzu".to_string()),
        ("DisplayVersion", version.to_string()),
        ("Publisher", "ilien-dev".to_string()),
        ("DisplayIcon", exe_text.clone()),
        ("InstallLocation", dir.display().to_string()),
        ("UninstallString", format!("\"{exe_text}\" --uninstall")),
        ("URLInfoAbout", "https://github.com/ilien-dev/anzu".to_string()),
    ] {
        key.set_string(name, value).map_err(failed)?;
    }
    for (name, value) in [("NoModify", 1), ("NoRepair", 1), ("EstimatedSize", size_kb)] {
        key.set_u32(name, value).map_err(failed)?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn register(_dir: &Path, _version: &str) -> Result<(), Error> {
    Ok(())
}

/// The folder of the registered installation, if there is one and it still has the app.
pub fn registered_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let key = windows_registry::CURRENT_USER.open(UNINSTALL_KEY).ok()?;
        let dir = PathBuf::from(key.get_string("InstallLocation").ok()?);
        dir.join(EXE_NAME).is_file().then_some(dir)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Removes the entry from the apps of Windows.
pub fn unregister() {
    #[cfg(windows)]
    {
        let _ = windows_registry::CURRENT_USER.remove_tree(UNINSTALL_KEY);
    }
}

/// Deletes the installed executable and its folder a moment after this process ends: a
/// running exe cannot delete itself. Only `anzu.exe` is deleted, and the folder only if
/// nothing else is left in it.
pub fn schedule_removal(dir: &Path) -> Result<(), Error> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let exe = dir.join(EXE_NAME);
        let script = format!(
            "/C ping -n 3 127.0.0.1 >nul & del /f /q \"{}\" & rmdir \"{}\"",
            exe.display(),
            dir.display()
        );
        std::process::Command::new("cmd").raw_arg(script).creation_flags(CREATE_NO_WINDOW).spawn()?;
    }
    #[cfg(not(windows))]
    let _ = dir;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setup_is_recognised_by_its_name() {
        assert!(is_setup_name(Path::new(r"C:\Downloads\anzu-0.3.0-windows-x64-setup.exe")));
        assert!(is_setup_name(Path::new(r"C:\Downloads\anzu-0.3.0-windows-x64-setup (1).exe")));
        assert!(is_setup_name(Path::new("ANZU-SETUP.EXE")));
        assert!(!is_setup_name(Path::new(r"C:\Programs\Anzu\anzu.exe")));
    }

    #[test]
    fn the_default_folder_is_under_programs() {
        assert!(default_dir().ends_with(Path::new("Programs").join("Anzu")));
    }

    #[test]
    fn copy_replaces_the_installed_executable() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("setup.exe");
        std::fs::write(&source, b"new").unwrap();
        let dir = tmp.path().join("Anzu");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(EXE_NAME), b"old").unwrap();

        let target = copy_exe(&source, &dir, Duration::ZERO).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(!dir.join("anzu.exe.new").exists());
    }

    /// The update waits for the app it replaces to close: Windows refuses to replace a
    /// running executable, and the copy must wait instead of failing at once.
    #[cfg(windows)]
    #[test]
    fn a_running_copy_is_waited_for_and_then_replaced() {
        let tmp = tempfile::tempdir().unwrap();
        let ping = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join(r"System32\PING.EXE");
        let installed = tmp.path().join(EXE_NAME);
        std::fs::copy(&ping, &installed).unwrap();
        let mut running = std::process::Command::new(&installed)
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let source = tmp.path().join("setup.exe");
        std::fs::write(&source, b"new").unwrap();

        assert!(matches!(copy_exe(&source, tmp.path(), Duration::ZERO), Err(Error::InstallInUse)));

        let closer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(800));
            let _ = running.kill();
            let _ = running.wait();
        });
        let target = copy_exe(&source, tmp.path(), Duration::from_secs(10)).unwrap();
        closer.join().unwrap();
        assert_eq!(std::fs::read(target).unwrap(), b"new");
    }

    #[test]
    fn copying_onto_itself_changes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join(EXE_NAME);
        std::fs::write(&exe, b"same").unwrap();
        assert_eq!(copy_exe(&exe, tmp.path(), Duration::ZERO).unwrap(), exe);
        assert_eq!(std::fs::read(&exe).unwrap(), b"same");
    }
}

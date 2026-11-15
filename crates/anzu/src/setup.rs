//! Updating and removing an installation, without the main window. The installer window
//! itself lives in `ui::installer`.

use std::path::Path;
use std::time::Duration;

use anzu_core::install;
use anzu_core::settings::{self, Settings};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

use crate::i18n::{self, Strings};

/// How long to wait for the app being updated to close.
const CLOSE_WAIT: Duration = Duration::from_secs(30);

fn strings() -> &'static Strings {
    i18n::strings(Settings::load(&settings::data_dir()).language)
}

fn tell(level: MessageLevel, title: &str, text: &str) {
    MessageDialog::new().set_level(level).set_title(title).set_description(text).set_buttons(MessageButtons::Ok).show();
}

fn ask(title: &str, text: &str) -> bool {
    let answer = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title(title)
        .set_description(text)
        .set_buttons(MessageButtons::YesNo)
        .show();
    answer == MessageDialogResult::Yes
}

/// Puts this executable in place of the one in `dir`, then opens it. If that fails, says
/// why and opens the version that was there, so the user is never left without the app.
pub fn update(dir: &Path) -> Result<(), String> {
    let result = std::env::current_exe().map_err(anzu_core::Error::from).and_then(|me| install::copy_exe(&me, dir, CLOSE_WAIT));
    match result {
        Ok(exe) => {
            let _ = install::register(dir, env!("CARGO_PKG_VERSION"));
            std::process::Command::new(exe).spawn().map(|_| ()).map_err(|e| e.to_string())
        }
        Err(error) => {
            let t = strings();
            let text = t.core_error(&error);
            tell(MessageLevel::Error, t.update_failed_title, &text);
            let _ = std::process::Command::new(dir.join(install::EXE_NAME)).spawn();
            Err(text)
        }
    }
}

/// Asks, then removes the shortcuts, the entry in the apps of Windows and the installed
/// files; the data folder only if the user also says so.
pub fn uninstall() {
    let t = strings();
    if !ask(t.uninstall_title, t.uninstall_confirm) {
        return;
    }
    let delete_data = ask(t.uninstall_title, t.uninstall_data);
    install::remove_shortcuts();
    let here = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf));
    // Files are only deleted from the folder Windows has registered, never from wherever
    // a copy of the program happens to run.
    let registered = install::registered_dir();
    install::unregister();
    if delete_data {
        let _ = std::fs::remove_dir_all(settings::data_dir());
    }
    if let (Some(here), Some(registered)) = (here, registered)
        && here.canonicalize().ok() == registered.canonicalize().ok()
    {
        let _ = install::schedule_removal(&registered);
    }
    tell(MessageLevel::Info, t.uninstall_title, t.uninstall_done);
}

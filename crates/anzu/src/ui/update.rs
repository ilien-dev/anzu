//! Updates of Anzu itself: the background watch, where things stand, the card that
//! offers a new version and the part of the settings that talks about it.
//!
//! The watch is one sleeping thread: it looks 30 seconds after start-up and then every
//! hour, one small request to GitHub each time, and costs nothing in between. A look
//! nobody asked for that fails says nothing. Nothing downloads or restarts unless the
//! user asks.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use anzu_core::update::{self, AppVersion, Offer};
use eframe::egui::{self, Align2, RichText, Stroke};

use super::{icons, theme};
use crate::i18n::Strings;

/// The first look waits, so it does not compete with the start-up check of the tool.
const FIRST_LOOK: Duration = Duration::from_secs(30);
/// How often to look while the app is open.
const EVERY: Duration = Duration::from_secs(60 * 60);
/// How many bytes arrive between two progress reports.
const PROGRESS_STEP: u64 = 256 * 1024;
const CARD_WIDTH: f32 = 360.0;

pub enum Event {
    Checked { asked: bool, result: Result<Option<Offer>, anzu_core::Error> },
    Progress(u64),
    Downloaded(Result<PathBuf, anzu_core::Error>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Check,
    Download,
    Install,
}

pub enum State {
    /// Nothing known yet.
    Unknown,
    /// A look the user asked for, under way.
    Checking,
    Current,
    Available(Offer),
    Downloading { offer: Offer, done: u64 },
    Ready { offer: Offer, installer: PathBuf },
    /// `offer` is set when a download or an install failed; without it, a look did.
    Failed { offer: Option<Offer>, stage: Stage, error: anzu_core::Error },
}

pub struct Updates {
    pub state: State,
    /// The version the user put away with "Later": its card stays hidden.
    dismissed: Option<AppVersion>,
    data_dir: PathBuf,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl Updates {
    /// Starts the watch. Installers left by an earlier update are removed first.
    pub fn start(data_dir: &Path, ctx: &egui::Context) -> Updates {
        let _ = std::fs::remove_dir_all(update::updates_dir(data_dir));
        let (tx, rx) = mpsc::channel();
        let (watch_tx, ctx) = (tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            std::thread::sleep(FIRST_LOOK);
            loop {
                let result = update::check();
                if watch_tx.send(Event::Checked { asked: false, result }).is_err() {
                    return;
                }
                ctx.request_repaint();
                std::thread::sleep(EVERY);
            }
        });
        Updates { state: State::Unknown, dismissed: None, data_dir: data_dir.to_path_buf(), tx, rx }
    }

    /// Takes in what the background threads found.
    pub fn poll(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            self.handle(event);
        }
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::Checked { asked, result } => {
                if !self.settled() {
                    return;
                }
                match result {
                    Ok(Some(offer)) => {
                        let same = self.offered().is_some_and(|o| o.version == offer.version);
                        if !same || !matches!(self.state, State::Available(_)) {
                            self.state = State::Available(offer);
                        }
                    }
                    Ok(None) => self.state = State::Current,
                    Err(error) if asked => self.state = State::Failed { offer: None, stage: Stage::Check, error },
                    Err(_) => {}
                }
            }
            Event::Progress(bytes) => {
                if let State::Downloading { done, .. } = &mut self.state {
                    *done = bytes;
                }
            }
            Event::Downloaded(result) => {
                let State::Downloading { offer, .. } = &self.state else { return };
                let offer = offer.clone();
                self.state = match result {
                    Ok(installer) => State::Ready { offer, installer },
                    Err(error) => State::Failed { offer: Some(offer), stage: Stage::Download, error },
                };
            }
        }
    }

    /// A download, or an installer waiting for the restart, is not a look's to replace.
    fn settled(&self) -> bool {
        !matches!(self.state, State::Downloading { .. } | State::Ready { .. })
    }

    /// The newer version on offer, whatever its state.
    pub fn offered(&self) -> Option<&Offer> {
        match &self.state {
            State::Available(offer) | State::Downloading { offer, .. } | State::Ready { offer, .. } => Some(offer),
            State::Failed { offer, .. } => offer.as_ref(),
            State::Unknown | State::Checking | State::Current => None,
        }
    }

    /// Whether there is news the user has not put away: the settings button shows a dot.
    pub fn has_news(&self) -> bool {
        self.offered().is_some()
    }

    fn card_visible(&self) -> bool {
        self.offered().is_some_and(|o| Some(o.version) != self.dismissed)
    }

    /// Looks for a newer version now, because the user asked.
    pub fn check(&mut self, ctx: &egui::Context) {
        if !self.settled() || matches!(self.state, State::Checking) {
            return;
        }
        self.state = State::Checking;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let result = update::check();
            let _ = tx.send(Event::Checked { asked: true, result });
            ctx.request_repaint();
        });
    }

    fn download(&mut self, ctx: &egui::Context) {
        let Some(offer) = self.offered().cloned() else { return };
        if !self.settled() {
            return;
        }
        self.state = State::Downloading { offer: offer.clone(), done: 0 };
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let dir = update::updates_dir(&self.data_dir);
        std::thread::spawn(move || {
            let mut reported = 0u64;
            let result = update::download(&offer, &dir, &mut |done| {
                if done - reported >= PROGRESS_STEP {
                    reported = done;
                    let _ = tx.send(Event::Progress(done));
                    ctx.request_repaint();
                }
            });
            let _ = tx.send(Event::Downloaded(result));
            ctx.request_repaint();
        });
    }

    /// Opens the verified installer and closes the app: the installer replaces it and
    /// opens the new version when it ends.
    fn restart(&mut self, ctx: &egui::Context) {
        let State::Ready { offer, installer } = &self.state else { return };
        // The installer replaces this very executable, wherever it is installed.
        let dir = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf));
        let spawned = match dir {
            Some(dir) => std::process::Command::new(installer).args(update::installer_args(&dir)).spawn(),
            None => Err(std::io::Error::other("the folder of the app is unknown")),
        };
        match spawned {
            Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(error) => {
                self.state = State::Failed { offer: Some(offer.clone()), stage: Stage::Install, error: error.into() };
            }
        }
    }

    fn later(&mut self) {
        self.dismissed = self.offered().map(|o| o.version);
    }
}

/// What failed, in the user's language.
fn failure(t: &Strings, stage: Stage, error: &anzu_core::Error) -> String {
    let what = match stage {
        Stage::Check => t.update_check_failed,
        Stage::Download => t.update_download_failed,
        Stage::Install => t.update_install_failed,
    };
    format!("{what}: {}", t.core_error(error))
}

/// The card in the bottom right corner that offers a new version. `busy`: a run is under
/// way, so the app must not close.
pub fn card(updates: &mut Updates, ctx: &egui::Context, t: &Strings, busy: bool) {
    if !updates.card_visible() {
        return;
    }
    let p = theme::Palette::of(ctx);
    let current = AppVersion::current().to_string();
    egui::Area::new(egui::Id::unique("update-card"))
        .anchor(Align2::RIGHT_BOTTOM, egui::vec2(-24.0, -84.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let edge = if busy && matches!(updates.state, State::Ready { .. }) { p.line } else { p.accent };
            theme::panel(p).stroke(Stroke::new(1.0, edge)).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                ui.set_width(CARD_WIDTH);
                ui.spacing_mut().item_spacing.y = 8.0;
                match &updates.state {
                    State::Available(offer) => {
                        let (version, page) = (offer.version.to_string(), offer.page.clone());
                        ui.horizontal(|ui| {
                            let (mark, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                            icons::update(ui.painter(), mark, p.accent);
                            ui.label(theme::bold(t.update_new).size(14.0));
                        });
                        ui.label(t.update_available(&version, &current));
                        ui.label(theme::muted(p, t.update_body).size(12.0));
                        ui.horizontal(|ui| {
                            if theme::button(ui, theme::Surface::Page, true, theme::primary_button(p, t.update_download)).clicked() {
                                updates.download(ui.ctx());
                            }
                            if theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.update_later).min_size(egui::vec2(0.0, 38.0))).clicked() {
                                updates.later();
                            }
                            if !page.is_empty() {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.hyperlink_to(RichText::new(t.update_notes).color(p.accent).size(12.0), page);
                                });
                            }
                        });
                    }
                    State::Downloading { offer, done } => {
                        let fraction = if offer.size > 0 { (*done as f32 / offer.size as f32).min(1.0) } else { 0.0 };
                        ui.label(theme::bold(t.update_downloading(&offer.version.to_string())).size(14.0));
                        ui.add(egui::ProgressBar::new(fraction).desired_height(6.0).fill(p.accent).corner_radius(1));
                        ui.label(theme::muted(p, t.update_progress(*done, offer.size)).size(12.0));
                    }
                    State::Ready { offer, .. } => {
                        let version = offer.version.to_string();
                        ui.horizontal(|ui| {
                            let (mark, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                            icons::check(ui.painter(), mark, p.ok);
                            ui.label(theme::bold(t.update_ready(&version)).size(14.0));
                        });
                        let body = if busy { t.update_wait_run } else { t.update_ready_body };
                        ui.label(theme::muted(p, body).size(12.0));
                        ui.horizontal(|ui| {
                            if theme::button(ui, theme::Surface::Page, !busy, theme::primary_button(p, t.update_restart)).clicked() {
                                updates.restart(ui.ctx());
                            }
                            if theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.update_later).min_size(egui::vec2(0.0, 38.0))).clicked() {
                                updates.later();
                            }
                        });
                    }
                    State::Failed { stage, error, .. } => {
                        let message = failure(t, *stage, error);
                        ui.label(theme::bold(t.update_new).size(14.0));
                        ui.label(RichText::new(message).color(p.warn).size(12.0));
                        ui.horizontal(|ui| {
                            if theme::button(ui, theme::Surface::Page, true, theme::primary_button(p, t.update_retry)).clicked() {
                                updates.download(ui.ctx());
                            }
                            if theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.update_later).min_size(egui::vec2(0.0, 38.0))).clicked() {
                                updates.later();
                            }
                        });
                    }
                    State::Unknown | State::Checking | State::Current => {}
                }
            });
        });
}

/// The updates part of the settings: the version, where things stand and the button.
pub fn settings_section(updates: &mut Updates, ui: &mut egui::Ui, t: &Strings, busy: bool) {
    let p = theme::Palette::of(ui.ctx());
    let current = AppVersion::current().to_string();
    ui.horizontal(|ui| {
        ui.label(theme::muted(p, t.version).size(theme::SMALL));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(&current).size(12.0));
        });
    });
    match &updates.state {
        State::Checking => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new(t.update_checking).size(12.0));
            });
        }
        State::Current => {
            ui.horizontal(|ui| {
                let (mark, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                icons::check(ui.painter(), mark, p.ok);
                ui.label(RichText::new(t.update_current).color(p.ok).size(12.0));
            });
        }
        State::Failed { offer: None, stage, error } => {
            ui.label(RichText::new(failure(t, *stage, error)).color(p.warn).size(12.0));
        }
        State::Downloading { offer, done } => {
            let text = format!("{} · {}", t.update_downloading(&offer.version.to_string()), t.update_progress(*done, offer.size));
            ui.label(RichText::new(text).color(p.accent).size(12.0));
        }
        State::Ready { offer, .. } => {
            ui.label(RichText::new(t.update_ready(&offer.version.to_string())).color(p.accent).size(12.0));
        }
        State::Unknown => {}
        State::Available(_) | State::Failed { .. } => {
            if let Some(offer) = updates.offered() {
                ui.label(RichText::new(t.update_available(&offer.version.to_string(), &current)).color(p.accent).size(12.0));
            }
        }
    }
    ui.label(theme::muted(p, t.update_schedule).size(theme::SMALL));

    enum Action {
        Check,
        Download,
        Restart,
    }
    let (label, action, enabled) = match &updates.state {
        State::Available(_) | State::Failed { offer: Some(_), .. } => (t.update_download, Action::Download, true),
        State::Downloading { .. } => (t.update_download, Action::Download, false),
        State::Ready { .. } => (t.update_restart, Action::Restart, !busy),
        State::Checking => (t.check_updates, Action::Check, false),
        State::Unknown | State::Current | State::Failed { offer: None, .. } => (t.check_updates, Action::Check, true),
    };
    let button = egui::Button::new(RichText::new(label).size(12.0)).min_size(egui::vec2(ui.available_width(), 32.0));
    if theme::button(ui, theme::Surface::Page, enabled, button).clicked() {
        // Bring the card back if it was put away: it shows the progress.
        updates.dismissed = None;
        match action {
            Action::Check => updates.check(ui.ctx()),
            Action::Download => updates.download(ui.ctx()),
            Action::Restart => updates.restart(ui.ctx()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(version: &str) -> Offer {
        Offer {
            version: version.parse().unwrap(),
            name: format!("anzu-{version}-windows-x64-setup.exe"),
            url: String::new(),
            sha256: String::new(),
            size: 10,
            page: String::new(),
        }
    }

    fn updates() -> Updates {
        let (tx, rx) = mpsc::channel();
        Updates { state: State::Unknown, dismissed: None, data_dir: PathBuf::new(), tx, rx }
    }

    #[test]
    fn a_background_look_that_fails_says_nothing() {
        let mut u = updates();
        u.handle(Event::Checked { asked: false, result: Err(anzu_core::Error::RateLimited) });
        assert!(matches!(u.state, State::Unknown));
        u.handle(Event::Checked { asked: true, result: Err(anzu_core::Error::RateLimited) });
        assert!(matches!(u.state, State::Failed { offer: None, stage: Stage::Check, .. }));
    }

    #[test]
    fn a_look_does_not_interrupt_a_download_or_a_ready_installer() {
        let mut u = updates();
        u.state = State::Downloading { offer: offer("0.9.0"), done: 4 };
        u.handle(Event::Checked { asked: false, result: Ok(None) });
        assert!(matches!(u.state, State::Downloading { done: 4, .. }));
        u.handle(Event::Downloaded(Ok(PathBuf::from("setup.exe"))));
        assert!(matches!(u.state, State::Ready { .. }));
        u.handle(Event::Checked { asked: false, result: Ok(Some(offer("0.9.1"))) });
        assert!(matches!(&u.state, State::Ready { offer, .. } if offer.version.to_string() == "0.9.0"));
    }

    #[test]
    fn later_hides_the_card_until_a_newer_version_comes() {
        let mut u = updates();
        u.handle(Event::Checked { asked: false, result: Ok(Some(offer("0.9.0"))) });
        assert!(u.card_visible());
        u.later();
        assert!(!u.card_visible());
        u.handle(Event::Checked { asked: false, result: Ok(Some(offer("0.9.0"))) });
        assert!(!u.card_visible(), "the same version stays put away");
        u.handle(Event::Checked { asked: false, result: Ok(Some(offer("0.9.1"))) });
        assert!(u.card_visible());
    }

    #[test]
    fn a_failed_download_keeps_the_offer_for_a_retry() {
        let mut u = updates();
        u.state = State::Downloading { offer: offer("0.9.0"), done: 0 };
        u.handle(Event::Downloaded(Err(anzu_core::Error::ChecksumMismatch)));
        assert!(matches!(u.state, State::Failed { offer: Some(_), stage: Stage::Download, .. }));
        assert!(u.has_news());
    }
}

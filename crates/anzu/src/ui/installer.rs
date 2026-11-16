//! The installer window: language, folder and shortcuts, the install itself, and done.
//!
//! It is shown when `anzu.exe` runs under its published name, `…-setup.exe`: the program
//! copies itself into the chosen folder (see `anzu_core::install`).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anzu_core::install;
use anzu_core::settings::{self, Language, Settings, ThemeChoice};
use eframe::egui::{self, Align, Layout, RichText, Sense, Stroke};

use super::icons::{self, Icon};
use super::theme::{self, Kind};
use super::{logo, settings_menu};
use crate::i18n::{self, Strings};

const WIDTH: f32 = 720.0;
const HEIGHT: f32 = 480.0;
/// How long to wait for a copy of Anzu that is still closing.
const COPY_WAIT: Duration = Duration::from_secs(5);
/// How long a stage of the header track takes to fill, in seconds.
const TRACK_TIME: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Language,
    Folder,
    Installing,
    Done,
    Failed,
}

enum Event {
    /// How many of the three tasks are finished.
    Progress(usize),
    Finished(Result<PathBuf, anzu_core::Error>),
}

struct Installer {
    language: Language,
    data_dir: PathBuf,
    page: Page,
    dir: String,
    desktop: bool,
    start_menu: bool,
    tasks_done: usize,
    rx: Option<Receiver<Event>>,
    installed: Option<PathBuf>,
    error: Option<anzu_core::Error>,
}

/// The language of Windows, when it is one Anzu speaks; English otherwise.
fn system_language() -> Language {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }
        // SAFETY: the call takes no arguments and only returns a language id.
        let id = unsafe { GetUserDefaultUILanguage() };
        const LANG_SPANISH: u16 = 0x0A;
        if id & 0x3FF == LANG_SPANISH {
            return Language::Es;
        }
    }
    Language::En
}

/// Opens the installer. Does not return until it is closed.
pub fn launch(icon: egui::IconData) -> Result<(), String> {
    let data_dir = settings::prepare_data_dir();
    // An earlier installation already knows the language; a first one asks Windows.
    let language = if data_dir.join("settings.json").is_file() { Settings::load(&data_dir).language } else { system_language() };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(i18n::strings(language).setup_window_title)
            .with_inner_size([WIDTH, HEIGHT])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(icon),
        ..Default::default()
    };
    let dir = install::registered_dir().unwrap_or_else(install::default_dir);
    eframe::run_native(
        "Anzu setup",
        options,
        Box::new(move |cc| {
            theme::install(&cc.egui_ctx);
            theme::set_choice(&cc.egui_ctx, ThemeChoice::System);
            Ok(Box::new(Installer {
                language,
                data_dir,
                page: Page::Language,
                dir: dir.display().to_string(),
                desktop: true,
                start_menu: true,
                tasks_done: 0,
                rx: None,
                installed: None,
                error: None,
            }))
        }),
    )
    .map_err(|e| e.to_string())
}

impl Installer {
    fn t(&self) -> &'static Strings {
        i18n::strings(self.language)
    }

    fn start(&mut self, ctx: &egui::Context) {
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.page = Page::Installing;
        self.tasks_done = 0;
        self.error = None;
        let dir = PathBuf::from(self.dir.trim());
        let (desktop, start_menu, language) = (self.desktop, self.start_menu, self.language);
        let (data_dir, ctx) = (self.data_dir.clone(), ctx.clone());
        std::thread::spawn(move || {
            let send = |event| {
                let _ = tx.send(event);
                ctx.request_repaint();
            };
            let result = (|| {
                let source = std::env::current_exe()?;
                let exe = install::copy_exe(&source, &dir, COPY_WAIT)?;
                send(Event::Progress(1));
                install::create_shortcuts(&exe, desktop, start_menu)?;
                send(Event::Progress(2));
                install::register(&dir, env!("CARGO_PKG_VERSION"))?;
                install::refresh_icons(&exe);
                settings::save_language(&data_dir, language)?;
                send(Event::Progress(3));
                Ok(exe)
            })();
            send(Event::Finished(result));
        });
    }

    fn poll(&mut self) {
        let Some(rx) = &self.rx else { return };
        while let Ok(event) = rx.try_recv() {
            match event {
                Event::Progress(n) => self.tasks_done = n,
                Event::Finished(Ok(exe)) => {
                    self.installed = Some(exe);
                    self.page = Page::Done;
                }
                Event::Finished(Err(error)) => {
                    self.error = Some(error);
                    self.page = Page::Failed;
                }
            }
        }
    }

    fn pick_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title(self.t().setup_folder_dialog);
        if let Some(parent) = Path::new(self.dir.trim()).parent().filter(|p| p.is_dir()) {
            dialog = dialog.set_directory(parent);
        }
        if let Some(mut picked) = dialog.pick_folder() {
            // A folder of its own, unless the one picked already is.
            if !picked.file_name().is_some_and(|n| n.eq_ignore_ascii_case("anzu")) {
                picked.push("Anzu");
            }
            self.dir = picked.display().to_string();
        }
    }
}

impl eframe::App for Installer {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let p = theme::Palette::of(ui.ctx());
        let t = self.t();
        egui::Panel::top("setup-header").frame(theme::band(p.shell, 16)).show_separator_line(false).show(ui, |ui| {
            header(self, ui, p, t);
        });
        egui::Panel::bottom("setup-footer").frame(theme::band(p.bg, 14)).show_separator_line(false).show(ui, |ui| {
            let top = ui.max_rect().top() - 14.0;
            let line = egui::Rect::from_min_max(egui::pos2(ui.ctx().content_rect().left(), top), egui::pos2(ui.ctx().content_rect().right(), top + 1.0));
            ui.painter().rect_filled(line, 0.0, p.line);
            let page = self.page as u64;
            theme::entrance(ui, egui::Id::unique(("setup-page", false)), page, theme::by_order, false, |ui| footer(self, ui, p, t));
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::symmetric(40, 30))).show(
            ui,
            |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                // Each page comes in from the side it lies on: the next one from the right,
                // "Back" from the left.
                let page = self.page as u64;
                theme::entrance(ui, egui::Id::unique(("setup-page", true)), page, theme::by_order, true, |ui| match self.page {
                    Page::Language => language_page(self, ui, p, t),
                    Page::Folder => folder_page(self, ui, p, t),
                    Page::Installing => installing_page(self, ui, p, t),
                    Page::Done => done_page(self, ui, p, t),
                    Page::Failed => failed_page(self, ui, p, t),
                });
            },
        );
    }
}

fn header(app: &Installer, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.horizontal(|ui| {
        logo::badge(ui, 30.0, p.shell_chip);
        ui.label(theme::display("Anzu").color(p.on_shell).size(19.0));
        ui.label(RichText::new(format!("{} · {}", t.setup_badge, env!("CARGO_PKG_VERSION"))).color(p.shell_muted).size(theme::SMALL));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (filled, label, color) = match app.page {
                Page::Language => (1, format!("1/3 {}", t.setup_step_language), p.shell_accent),
                Page::Folder => (2, format!("2/3 {}", t.setup_step_folder), p.shell_accent),
                Page::Installing | Page::Failed => (3, format!("3/3 {}", t.setup_step_install), p.shell_accent),
                Page::Done => (3, t.setup_step_done.to_string(), p.ok),
            };
            ui.label(RichText::new(label).color(p.on_shell).size(theme::SMALL));
            // The same three-stage track as the books: one stage per step.
            let (rect, response) = ui.allocate_exact_size(egui::vec2(3.0 * 18.0 + 2.0 * 4.0, 6.0), Sense::hover());
            // The stage of a new step fills from the left, as the page comes in.
            let filled = ui.ctx().animate_value_with_time(response.id.with("filled"), filled as f32, TRACK_TIME);
            for i in 0..3 {
                let x = rect.left() + i as f32 * 22.0;
                let stage = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(18.0, 6.0));
                let part = (filled - i as f32).clamp(0.0, 1.0);
                if part < 1.0 {
                    ui.painter().rect_stroke(stage, 0.0, Stroke::new(1.0, p.shell_line), egui::StrokeKind::Inside);
                }
                if part > 0.0 {
                    let fill = egui::Rect::from_min_size(stage.min, egui::vec2(stage.width() * part, stage.height()));
                    ui.painter().rect_filled(fill, 0.0, color);
                }
            }
        });
    });
}

fn heading(ui: &mut egui::Ui, p: theme::Palette, icon: Icon, fill: egui::Color32, title: &str, intro: &str) {
    ui.horizontal(|ui| {
        icons::tile(ui, icon, 34.0, fill, theme::INK);
        ui.label(theme::display(title).size(22.0));
    });
    ui.label(theme::muted(p, intro));
    ui.add_space(6.0);
}

fn language_page(app: &mut Installer, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    heading(ui, p, Icon::Globe, theme::INFO, t.setup_title, t.setup_intro);
    theme::panel(p).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::muted(p, t.setup_language).size(theme::SMALL));
        let languages = [Language::En, Language::Es];
        let current = languages.iter().position(|l| *l == app.language).unwrap_or(0);
        ui.scope(|ui| {
            ui.set_max_width(320.0);
            let options = [(Icon::Globe, t.language_en), (Icon::Globe, t.language_es)];
            if let Some(i) = settings_menu::segmented(ui, p, "setup-language", &options, current) {
                app.language = languages[i];
                let title = app.t().setup_window_title.to_string();
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title));
            }
        });
        ui.label(theme::muted(p, t.setup_language_note).size(theme::SMALL));
    });
}

fn folder_page(app: &mut Installer, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    heading(ui, p, Icon::Folder, theme::UNLOCKED, t.setup_folder_title, t.setup_folder_intro);
    theme::panel(p).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::muted(p, t.setup_folder_label).size(theme::SMALL));
        ui.horizontal(|ui| {
            let button_width = 170.0;
            let field = egui::TextEdit::singleline(&mut app.dir).desired_width(ui.available_width() - button_width - 8.0);
            ui.add_sized([ui.available_width() - button_width - 8.0, 34.0], field);
            if theme::icon_button_wide(ui, Kind::Secondary, true, Icon::Folder, t.setup_change_folder, button_width).clicked() {
                app.pick_folder();
            }
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        icons::show(ui, Icon::Monitor, 16.0, p.muted);
        ui.checkbox(&mut app.desktop, t.setup_desktop).on_hover_cursor(egui::CursorIcon::PointingHand);
    });
    ui.horizontal(|ui| {
        icons::show(ui, Icon::Book, 16.0, p.muted);
        ui.checkbox(&mut app.start_menu, t.setup_start_menu).on_hover_cursor(egui::CursorIcon::PointingHand);
    });
}

fn installing_page(app: &Installer, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    heading(ui, p, Icon::Download, theme::BUTTER, t.setup_installing_title, t.setup_installing_intro);
    theme::panel(p).inner_margin(egui::Margin::symmetric(16, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        for (i, label) in [t.setup_task_copy, t.setup_task_shortcuts, t.setup_task_register].into_iter().enumerate() {
            ui.horizontal(|ui| {
                ui.set_min_height(36.0);
                let (mark, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                let (state, color) = if i < app.tasks_done {
                    icons::check(ui.painter(), mark, p.ok);
                    (t.setup_state_done, p.ok)
                } else if i == app.tasks_done {
                    ui.put(mark, egui::Spinner::new().size(14.0).color(p.fg));
                    (t.setup_state_running, p.fg)
                } else {
                    ui.painter().rect_stroke(mark, 2.0, Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
                    (t.setup_state_pending, p.muted)
                };
                ui.label(if i <= app.tasks_done { RichText::new(label) } else { theme::muted(p, label) });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(state).color(color).size(theme::SMALL));
                });
            });
        }
    });
    ui.add(egui::ProgressBar::new(app.tasks_done as f32 / 3.0).desired_height(6.0).fill(p.progress).corner_radius(3));
}

fn done_page(app: &Installer, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        icons::tile(ui, Icon::Check, 34.0, theme::EPUB, theme::INK);
        ui.label(theme::display(t.setup_done_title).size(22.0));
    });
    ui.label(theme::muted(p, t.setup_done_body));
    ui.add_space(6.0);
    theme::panel(p).inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Folder, 16.0, p.muted);
            ui.label(theme::muted(p, t.setup_installed_in).size(12.0));
            let dir = app.installed.as_deref().and_then(Path::parent).map(|d| d.display().to_string()).unwrap_or_default();
            ui.label(RichText::new(dir).size(12.0));
        });
    });
}

fn failed_page(app: &Installer, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.horizontal(|ui| {
        icons::tile(ui, Icon::Alert, 34.0, theme::DRM, theme::INK);
        ui.label(theme::display(t.setup_failed_title).size(22.0));
    });
    if let Some(error) = &app.error {
        theme::warn_panel(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(t.core_error(error));
        });
    }
}

fn footer(app: &mut Installer, ui: &mut egui::Ui, _p: theme::Palette, t: &Strings) {
    ui.horizontal(|ui| {
        let ctx = ui.ctx().clone();
        let left = match app.page {
            Page::Language | Page::Done | Page::Failed => Some((Icon::Close, t.setup_close)),
            Page::Folder => Some((Icon::ArrowLeft, t.setup_back)),
            Page::Installing => None,
        };
        if let Some((icon, label)) = left
            && theme::icon_button(ui, Kind::Secondary, true, icon, label).clicked()
        {
            match app.page {
                Page::Folder => app.page = Page::Language,
                _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| match app.page {
            Page::Language => {
                if theme::icon_button(ui, Kind::Primary, true, Icon::ArrowRight, t.setup_continue).clicked() {
                    app.page = Page::Folder;
                }
            }
            Page::Folder => {
                let ready = !app.dir.trim().is_empty();
                if theme::icon_button(ui, Kind::Primary, ready, Icon::Download, t.setup_install).clicked() {
                    app.start(&ctx);
                }
            }
            Page::Installing => {
                theme::icon_button(ui, Kind::Primary, false, Icon::Clock, t.setup_installing_button);
            }
            Page::Done => {
                if theme::icon_button(ui, Kind::Primary, true, Icon::External, t.setup_open).clicked() {
                    if let Some(exe) = &app.installed {
                        let _ = std::process::Command::new(exe).spawn();
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Page::Failed => {
                if theme::icon_button(ui, Kind::Primary, true, Icon::Refresh, t.setup_retry).clicked() {
                    app.start(&ctx);
                }
            }
        });
    });
}

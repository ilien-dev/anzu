//! The interface: window state, worker threads and the split between screens.

mod blocked;
mod icons;
mod installer;
mod library;
mod logo;
mod mark;
mod progress;
mod settings_menu;
mod theme;
mod update;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use anzu_core::library::{Book, BookState};
use anzu_core::settings::{self, Language, Settings, ThemeChoice};
use anzu_core::{kindle, library as core_library};
use eframe::egui;

use crate::app::{self, Health, RunPlan};
use crate::i18n::{self, Strings};
use crate::pipeline::{self, CoreBackend, Msg, Stage, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Library,
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Locked,
    Unlocked,
    Converted,
}

/// The column the book table is sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Title,
    Asin,
    Size,
    State,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sort {
    column: Column,
    ascending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepState {
    Running,
    Done,
    Skipped,
}

/// What is known about the current run, or the last one.
#[derive(Default)]
struct Run {
    asins: Vec<String>,
    /// Whether the run goes through the preparation (check, download, close Kindle).
    prepares: bool,
    steps: Vec<(Step, StepState)>,
    /// Why the preparation stopped, if it did.
    preparation_failed: Option<String>,
    states: HashMap<String, BookState>,
    /// The stage each book is in right now.
    active: HashMap<String, Stage>,
    /// The books that failed, with the stage and the reason.
    errors: HashMap<String, (Stage, String)>,
    /// The EPUB written for each book, in the order they were saved.
    saved: Vec<(String, PathBuf)>,
    log: Vec<String>,
    show_log: bool,
    result: Option<(usize, Vec<(String, String)>)>,
}

pub struct AnzuApp {
    data_dir: PathBuf,
    content_dir: PathBuf,
    settings: Settings,
    health: Health,
    books: Vec<Book>,
    selected: Vec<String>,
    filter: Filter,
    sort: Sort,
    screen: Screen,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    /// Channel to answer "close Kindle?" during a run.
    close_tx: Option<Sender<bool>>,
    asking_to_close: bool,
    run: Run,
    /// New versions of Anzu itself.
    updates: update::Updates,
}

impl AnzuApp {
    fn new(ctx: &egui::Context) -> Self {
        let data_dir = settings::prepare_data_dir();
        let settings = Settings::load(&data_dir);
        theme::set_choice(ctx, settings.theme);
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
        let (tx, rx) = mpsc::channel();
        let updates = update::Updates::start(&data_dir, ctx);
        let mut app = AnzuApp {
            content_dir: kindle::content_dir_in(&local),
            data_dir,
            settings,
            health: Health::Checking,
            books: Vec::new(),
            selected: Vec::new(),
            filter: Filter::All,
            sort: Sort { column: Column::Title, ascending: true },
            screen: Screen::Library,
            tx,
            rx,
            close_tx: None,
            asking_to_close: false,
            run: Run::default(),
            updates,
        };
        app.refresh_books();
        app.start_check(ctx);
        app
    }

    /// The text of the language in use.
    fn t(&self) -> &'static Strings {
        i18n::strings(self.settings.language)
    }

    fn refresh_books(&mut self) {
        // What the user chose in this window wins over what is on disk.
        let (theme, language) = (self.settings.theme, self.settings.language);
        self.settings = Settings::load(&self.data_dir);
        self.settings.theme = theme;
        self.settings.language = language;
        self.books = core_library::scan(&self.content_dir, &self.data_dir, &self.settings);
        self.learn_titles();
        let ready = self.ready();
        self.selected.retain(|asin| self.books.iter().any(|b| &b.asin == asin && app::selectable(b.state, ready)));
    }

    /// Reads the title of unlocked books that do not have one yet, and remembers it.
    fn learn_titles(&mut self) {
        let mut learned = false;
        for book in &mut self.books {
            if book.title.is_some() {
                continue;
            }
            let Some(title) = book.unlocked.as_deref().and_then(anzu_kfx::read_title) else { continue };
            self.settings.titles.insert(book.asin.clone(), title.clone());
            book.title = Some(title);
            learned = true;
        }
        if learned {
            let _ = self.settings.save(&self.data_dir);
        }
    }

    fn ready(&self) -> bool {
        matches!(self.health, Health::Ready { .. })
    }

    fn busy(&self) -> bool {
        self.screen == Screen::Progress && self.run.result.is_none()
    }

    fn plan(&self) -> RunPlan {
        app::plan_run(&self.books, &self.selected)
    }

    fn set_theme(&mut self, ctx: &egui::Context, choice: ThemeChoice) {
        self.settings.theme = choice;
        let _ = self.settings.save(&self.data_dir);
        theme::set_choice(ctx, choice);
    }

    fn set_language(&mut self, ctx: &egui::Context, language: Language) {
        self.settings.language = language;
        let _ = self.settings.save(&self.data_dir);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.t().window_title.to_string()));
    }

    fn start_check(&mut self, ctx: &egui::Context) {
        self.health = Health::Checking;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let backend = CoreBackend { data_dir: self.data_dir.clone() };
        std::thread::spawn(move || {
            let (health, _) = pipeline::check(&backend);
            let _ = tx.send(Msg::Health(health));
            ctx.request_repaint();
        });
    }

    fn start_run(&mut self, ctx: &egui::Context) {
        let plan = self.plan();
        if plan.asins.is_empty() || self.busy() {
            return;
        }
        self.run = Run { asins: plan.asins.clone(), prepares: pipeline::prepares(&plan, &self.books), ..Run::default() };
        for book in self.books.iter().filter(|b| plan.asins.contains(&b.asin)) {
            self.run.states.insert(book.asin.clone(), progress::starting_state(book.state));
        }
        self.screen = Screen::Progress;

        let (close_tx, close_rx) = mpsc::channel::<bool>();
        self.close_tx = Some(close_tx);
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let backend = CoreBackend { data_dir: self.data_dir.clone() };
        let books = self.books.clone();
        let out_dir = self.settings.output_dir.clone();
        let language = self.settings.language;
        std::thread::spawn(move || {
            let mut pacer = pipeline::Pacer::new(pipeline::MIN_STEP);
            pipeline::run(
                &backend,
                &plan,
                &books,
                &out_dir,
                language,
                &mut |message| {
                    pacer.wait_before(&message);
                    let _ = tx.send(message);
                    ctx.request_repaint();
                },
                &mut || close_rx.recv().unwrap_or(false),
            );
        });
    }

    fn answer_close(&mut self, close: bool) {
        self.asking_to_close = false;
        if let Some(tx) = &self.close_tx {
            let _ = tx.send(close);
        }
    }

    fn handle(&mut self, message: Msg) {
        match message {
            Msg::Health(health) => {
                self.health = health;
                self.refresh_books();
            }
            Msg::StepStarted(step) => self.run.steps.push((step, StepState::Running)),
            Msg::StepDone(step) => self.end_step(step, StepState::Done),
            Msg::StepSkipped(step) => self.end_step(step, StepState::Skipped),
            Msg::PreparationFailed(reason) => self.run.preparation_failed = Some(reason),
            Msg::BookStarted { asin, stage } => {
                self.run.active.insert(asin, stage);
            }
            Msg::BookProgress { asin, state } => {
                self.run.active.remove(&asin);
                self.run.states.insert(asin, state);
            }
            Msg::BookFailed { asin, stage, reason } => {
                self.run.active.remove(&asin);
                self.run.errors.insert(asin, (stage, reason));
            }
            Msg::Saved { asin, path } => self.run.saved.push((asin, path)),
            Msg::Log(line) => self.run.log.push(line),
            Msg::NeedCloseKindle => self.asking_to_close = true,
            Msg::Finished { converted, failed } => {
                self.run.result = Some((converted, failed));
                self.close_tx = None;
                self.asking_to_close = false;
                self.refresh_books();
            }
        }
    }

    fn end_step(&mut self, step: Step, state: StepState) {
        if let Some(entry) = self.run.steps.iter_mut().find(|(s, _)| *s == step) {
            entry.1 = state;
        }
    }

    /// Leaves a finished run and shows the books again.
    fn back_to_library(&mut self) {
        self.selected.clear();
        self.refresh_books();
        self.screen = Screen::Library;
    }

    fn pick_output_dir(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title(self.t().folder_dialog);
        if self.settings.output_dir.is_dir() {
            dialog = dialog.set_directory(&self.settings.output_dir);
        }
        if let Some(dir) = dialog.pick_folder() {
            self.settings.output_dir = dir;
            let _ = self.settings.save(&self.data_dir);
        }
    }

    /// The title to show: the real one when known, or the ASIN while the book has DRM.
    fn display_title(book: &Book, t: &Strings) -> String {
        book.title.clone().unwrap_or_else(|| t.book_fallback(&book.asin))
    }

    fn close_dialog(&mut self, ctx: &egui::Context) {
        let p = theme::Palette::of(ctx);
        let t = self.t();
        let mut answer = None;
        let response = egui::Modal::new(egui::Id::unique("close-kindle")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.label(theme::bold(t.close_title).size(16.0));
            ui.add_space(2.0);
            ui.label(t.close_body);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if theme::button(ui, theme::Surface::Page, true, theme::primary_button(p, t.close_confirm)).clicked() {
                    answer = Some(true);
                }
                if theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.cancel)).clicked() {
                    answer = Some(false);
                }
            });
        });
        if response.should_close() {
            answer = answer.or(Some(false));
        }
        if let Some(close) = answer {
            self.answer_close(close);
        }
    }

    /// The logo and the name, for the header band.
    fn brand(ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
        logo::badge(ui, 28.0, p.shell_accent, p.on_shell_accent);
        ui.label(theme::bold("anzu").color(p.on_shell).size(17.0));
        ui.label(egui::RichText::new(t.tagline).color(p.shell_muted).size(theme::SMALL));
    }
}

/// Opens Explorer on the folder of `path`, with the file selected.
fn show_in_folder(path: &Path) {
    let mut command = std::process::Command::new("explorer");
    #[cfg(windows)]
    {
        // Explorer reads `/select,"path"` itself: the default quoting of the whole argument breaks it.
        use std::os::windows::process::CommandExt;
        command.raw_arg(format!("/select,\"{}\"", path.display()));
    }
    #[cfg(not(windows))]
    command.arg(path.parent().unwrap_or(path));
    let _ = command.spawn();
}

/// Opens a file with the app Windows has for it.
fn open_file(path: &Path) {
    let mut command = std::process::Command::new("explorer");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.raw_arg(format!("\"{}\"", path.display()));
    }
    #[cfg(not(windows))]
    command.arg(path);
    let _ = command.spawn();
}

impl eframe::App for AnzuApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.rx.try_recv() {
            self.handle(message);
        }
        self.updates.poll();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::Library => library::show(self, ui),
            Screen::Progress => progress::show(self, ui),
        }
        let ctx = ui.ctx().clone();
        if self.asking_to_close {
            self.close_dialog(&ctx);
        } else {
            let (t, busy) = (self.t(), self.busy());
            update::card(&mut self.updates, &ctx, t, busy);
        }
    }
}

/// The logo for the title bar and the taskbar.
fn window_icon() -> egui::IconData {
    const ICON_SIZE: u32 = 128;
    egui::IconData {
        rgba: logo::icon_rgba(ICON_SIZE, theme::ICON_BACKGROUND, theme::ICON_MARK),
        width: ICON_SIZE,
        height: ICON_SIZE,
    }
}

/// Opens the installer window. Does not return until it is closed.
pub fn launch_installer() -> Result<(), String> {
    installer::launch(window_icon())
}

/// Opens the app window. Does not return until it is closed.
pub fn launch() -> Result<(), String> {
    let language = Settings::load(&settings::prepare_data_dir()).language;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(i18n::strings(language).window_title)
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([820.0, 500.0])
            .with_icon(window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Anzu",
        options,
        Box::new(|cc| {
            theme::install(&cc.egui_ctx);
            Ok(Box::new(AnzuApp::new(&cc.egui_ctx)))
        }),
    )
    .map_err(|e| e.to_string())
}

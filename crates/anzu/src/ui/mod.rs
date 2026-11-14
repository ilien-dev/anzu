//! La interfaz: estado de la ventana, hilos de trabajo y reparto entre pantallas.

mod blocked;
mod icons;
mod library;
mod logo;
mod progress;
mod theme;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use anzu_core::library::{Book, BookState};
use anzu_core::settings::{self, Settings, ThemeChoice};
use anzu_core::{kindle, library as core_library};
use eframe::egui;

use crate::app::{self, Health, RunPlan};
use crate::pipeline::{self, CoreBackend, Msg, Step};

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

/// Columna por la que se ordena la tabla de libros.
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
}

/// Lo que se sabe de la ejecución en curso o de la última.
#[derive(Default)]
struct Run {
    asins: Vec<String>,
    steps: Vec<(Step, StepState)>,
    states: HashMap<String, BookState>,
    log: Vec<String>,
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
    /// Canal para responder a "¿cerrar Kindle?" durante una ejecución.
    close_tx: Option<Sender<bool>>,
    asking_to_close: bool,
    run: Run,
}

impl AnzuApp {
    fn new(ctx: &egui::Context) -> Self {
        let data_dir = settings::prepare_data_dir();
        let settings = Settings::load(&data_dir);
        theme::set_choice(ctx, settings.theme);
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
        let (tx, rx) = mpsc::channel();
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
        };
        app.refresh_books();
        app.start_check(ctx);
        app
    }

    fn refresh_books(&mut self) {
        self.settings = Settings::load(&self.data_dir);
        self.books = core_library::scan(&self.content_dir, &self.data_dir, &self.settings);
        self.learn_titles();
        let ready = self.ready();
        self.selected.retain(|asin| self.books.iter().any(|b| &b.asin == asin && app::selectable(b.state, ready)));
    }

    /// Lee el título de los libros desbloqueados que aún no lo tienen y lo recuerda.
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
        self.run = Run { asins: plan.asins.clone(), ..Run::default() };
        for book in self.books.iter().filter(|b| plan.asins.contains(&b.asin)) {
            self.run.states.insert(book.asin.clone(), book.state);
        }
        self.screen = Screen::Progress;

        let (close_tx, close_rx) = mpsc::channel::<bool>();
        self.close_tx = Some(close_tx);
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let backend = CoreBackend { data_dir: self.data_dir.clone() };
        let books = self.books.clone();
        let out_dir = self.settings.output_dir.clone();
        std::thread::spawn(move || {
            pipeline::run(
                &backend,
                &plan,
                &books,
                &out_dir,
                &mut |message| {
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
            Msg::StepDone(step) => {
                if let Some(entry) = self.run.steps.iter_mut().find(|(s, _)| *s == step) {
                    entry.1 = StepState::Done;
                }
            }
            Msg::BookProgress { asin, state } => {
                self.run.states.insert(asin, state);
            }
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

    fn pick_output_dir(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Carpeta para los EPUB");
        if self.settings.output_dir.is_dir() {
            dialog = dialog.set_directory(&self.settings.output_dir);
        }
        if let Some(dir) = dialog.pick_folder() {
            self.settings.output_dir = dir;
            let _ = self.settings.save(&self.data_dir);
        }
    }

    /// Título para mostrar: el real si se conoce, o el ASIN mientras el libro tenga DRM.
    fn display_title(book: &Book) -> String {
        book.title.clone().unwrap_or_else(|| format!("Libro {}", book.asin))
    }

    fn close_dialog(&mut self, ctx: &egui::Context) {
        let p = theme::Palette::of(ctx);
        let mut answer = None;
        let response = egui::Modal::new(egui::Id::unique("cerrar-kindle")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.label(theme::bold("La app Kindle está abierta").size(16.0));
            ui.add_space(2.0);
            ui.label("Para desbloquear hay que cerrarla. Tu posición de lectura se guarda al cerrar.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add(theme::primary_button(p, "Cerrar Kindle y continuar")).clicked() {
                    answer = Some(true);
                }
                if ui.button("Cancelar").clicked() {
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

    /// El logotipo y el nombre, para la banda de cabecera.
    fn brand(ui: &mut egui::Ui, p: theme::Palette) {
        logo::badge(ui, 28.0, p.shell_accent, p.on_shell_accent);
        ui.label(theme::bold("anzu").color(p.on_shell).size(17.0));
        ui.label(egui::RichText::new("de Kindle a EPUB").color(p.shell_muted).size(theme::SMALL));
    }
}

impl eframe::App for AnzuApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.rx.try_recv() {
            self.handle(message);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::Library => library::show(self, ui),
            Screen::Progress => progress::show(self, ui),
        }
        if self.asking_to_close {
            let ctx = ui.ctx().clone();
            self.close_dialog(&ctx);
        }
    }
}

/// Abre la ventana de la app. No vuelve hasta que se cierra.
pub fn launch() -> Result<(), String> {
    const ICON_SIZE: u32 = 128;
    let icon = egui::IconData {
        rgba: logo::icon_rgba(ICON_SIZE, theme::ICON_BACKGROUND, theme::ICON_MARK),
        width: ICON_SIZE,
        height: ICON_SIZE,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Anzu: de Kindle a EPUB")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([760.0, 500.0])
            .with_icon(icon),
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

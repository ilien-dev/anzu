//! La interfaz: estado de la ventana, hilos de trabajo y reparto entre pantallas.

mod blocked;
mod library;
mod progress;
mod theme;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use seshat_core::library::{Book, BookState};
use seshat_core::settings::{self, Settings};
use seshat_core::{kindle, library as core_library};

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

pub struct SeshatApp {
    data_dir: PathBuf,
    content_dir: PathBuf,
    settings: Settings,
    health: Health,
    books: Vec<Book>,
    selected: Vec<String>,
    filter: Filter,
    screen: Screen,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    /// Canal para responder a "¿cerrar Kindle?" durante una ejecución.
    close_tx: Option<Sender<bool>>,
    asking_to_close: bool,
    run: Run,
}

impl SeshatApp {
    fn new(ctx: &egui::Context) -> Self {
        let data_dir = settings::data_dir();
        let settings = Settings::load(&data_dir);
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
        let (tx, rx) = mpsc::channel();
        let mut app = SeshatApp {
            content_dir: kindle::content_dir_in(&local),
            data_dir,
            settings,
            health: Health::Checking,
            books: Vec::new(),
            selected: Vec::new(),
            filter: Filter::All,
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
        let ready = self.ready();
        self.selected.retain(|asin| self.books.iter().any(|b| &b.asin == asin && app::selectable(b.state, ready)));
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

    /// Título para mostrar: el del EPUB si ya se convirtió, o el ASIN.
    fn display_title(book: &Book) -> String {
        book.title
            .clone()
            .or_else(|| book.epub.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| format!("Libro {}", book.asin))
    }

    fn close_dialog(&mut self, ctx: &egui::Context) {
        let mut answer = None;
        let response = egui::Modal::new(egui::Id::unique("cerrar-kindle")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.heading("La app Kindle está abierta");
            ui.add_space(4.0);
            ui.label("Para desbloquear hay que cerrarla. ¿Cerrarla para continuar?");
            ui.label(theme::muted("Si estás leyendo, tu posición se guarda al cerrar."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add(theme::primary_button("Cerrar Kindle")).clicked() {
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
}

impl eframe::App for SeshatApp {
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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Seshat: de Kindle a EPUB")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Seshat",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(SeshatApp::new(&cc.egui_ctx)))
        }),
    )
    .map_err(|e| e.to_string())
}

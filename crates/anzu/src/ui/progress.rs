//! Pantalla de progreso: pasos, avance por libro y registro.

use anzu_core::library::BookState;
use eframe::egui::{self, Align, Layout, RichText, Sense};

use super::icons::{self, StepMark};
use super::{AnzuApp, Screen, StepState, theme};
use crate::pipeline::Step;

const STEPS: [(Step, &str, &str); 5] = [
    (Step::Check, "Comprobar versiones", "Kindle instalada y última herramienta publicada"),
    (Step::Download, "Descargar la herramienta", "Con su huella SHA-256 verificada"),
    (Step::CloseKindle, "Cerrar la app Kindle", "Solo si la confirmas"),
    (Step::Unlock, "Desbloquear", "Puede tardar un minuto por libro"),
    (Step::Convert, "Convertir a EPUB", "Con el conversor de Anzu"),
];

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui) {
    let p = theme::Palette::of(ui.ctx());
    egui::Panel::top("cabecera-progreso").frame(theme::band(p.shell, 10)).show_separator_line(false).show(ui, |ui| {
        theme::centered(ui, |ui| header(app, ui, p));
    });
    egui::Panel::bottom("pie-progreso").frame(theme::band(p.bg, 12)).show_separator_line(false).show(ui, |ui| {
        theme::centered(ui, |ui| footer(app, ui, p));
    });
    egui::CentralPanel::default().frame(theme::band(p.bg, 12)).show(ui, |ui| {
        theme::centered(ui, |ui| {
            ui.columns(2, |columns| {
                steps(app, &mut columns[0], p);
                columns[1].vertical(|ui| {
                    books(app, ui, p);
                    ui.add_space(4.0);
                    log(app, ui, p);
                });
            });
        });
    });
}

fn header(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    ui.horizontal(|ui| {
        AnzuApp::brand(ui, p);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let total = app.run.asins.len();
            let libros = if total == 1 { "1 libro".to_string() } else { format!("{total} libros") };
            match &app.run.result {
                None => {
                    ui.label(
                        RichText::new(format!("Procesando {libros}. No abras la app Kindle hasta que termine."))
                            .color(p.on_shell),
                    );
                    ui.spinner();
                }
                Some((converted, failed)) if failed.is_empty() => {
                    ui.label(theme::bold(format!("Terminado: {converted} convertidos")).color(p.shell_accent));
                }
                Some((converted, failed)) => {
                    ui.label(
                        theme::bold(format!("Terminado: {converted} convertidos, {} con problemas", failed.len()))
                            .color(p.shell_accent),
                    );
                }
            }
        });
    });
}

fn steps(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    theme::panel(p).inner_margin(egui::Margin::symmetric(16, 14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::bold("Pasos").size(15.0));
        ui.add_space(6.0);
        let finished = app.run.result.is_some();
        for (step, title, detail) in STEPS {
            let state = app.run.steps.iter().find(|(s, _)| *s == step).map(|(_, state)| *state);
            let (mark, note) = match (state, finished) {
                (Some(StepState::Done), _) => (StepMark::Done, detail),
                (Some(StepState::Running), false) => (StepMark::Running, detail),
                (Some(StepState::Running), true) => (StepMark::Failed, "No se completó"),
                (None, false) => (StepMark::Pending, "Pendiente"),
                (None, true) => (StepMark::Pending, "No hizo falta"),
            };
            ui.horizontal_top(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 20.0), Sense::hover());
                icons::step(ui.painter(), rect.center(), mark, p);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let running = state == Some(StepState::Running) && !finished;
                    let color = if state.is_some() { p.fg } else { p.muted };
                    ui.label(if running { theme::bold(title).color(color) } else { RichText::new(title).color(color) });
                    ui.label(theme::muted(p, note).size(theme::SMALL));
                });
            });
            ui.add_space(6.0);
        }
    });
}

fn books(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    let unlocking = app.run.steps.iter().any(|(s, state)| *s == Step::Unlock && *state == StepState::Running);
    let failures = app.run.result.as_ref().map(|(_, failed)| failed.as_slice()).unwrap_or(&[]);
    theme::panel(p).inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::vertical().id_salt("libros-progreso").max_height(230.0).show(ui, |ui| {
            for asin in &app.run.asins {
                let title = app
                    .books
                    .iter()
                    .find(|b| &b.asin == asin)
                    .map_or_else(|| format!("Libro {asin}"), AnzuApp::display_title);
                let state = app.run.states.get(asin).copied().unwrap_or(BookState::Locked);
                let failure = failures.iter().find(|(a, _)| a == asin).map(|(_, reason)| reason.as_str());
                let (status, color) = match (failure, state) {
                    (Some(_), BookState::Locked) => ("No se pudo desbloquear", p.warn),
                    (Some(_), _) => ("No se pudo convertir", p.warn),
                    (None, BookState::Converted) => ("EPUB listo", p.ok),
                    (None, BookState::Unlocked) => ("Convirtiendo…", p.accent),
                    (None, BookState::Locked) if unlocking => ("Desbloqueando…", p.accent),
                    (None, BookState::Locked) => ("En cola", p.muted),
                };
                ui.add(egui::Label::new(theme::bold(title)).truncate());
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(icons::TRACK_WIDTH, 16.0), Sense::hover());
                    icons::track(ui.painter(), rect.left_center(), state, failure.is_none(), p);
                    ui.label(RichText::new(status).color(color).size(12.0));
                });
                if let Some(reason) = failure {
                    ui.label(theme::muted(p, reason.lines().next().unwrap_or(reason)).size(theme::SMALL));
                }
                ui.add_space(8.0);
            }
        });
    });
}

fn log(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    theme::panel(p).fill(p.log_bg).inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::bold("Registro").color(p.muted).size(theme::SMALL));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add_enabled(!app.run.log.is_empty(), egui::Button::new("Copiar")).clicked() {
                    ui.ctx().copy_text(app.run.log.join("\n"));
                }
            });
        });
        egui::ScrollArea::vertical()
            .id_salt("registro")
            .max_height(250.0)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                for line in &app.run.log {
                    ui.label(RichText::new(line.as_str()).size(12.0));
                }
            });
    });
}

fn footer(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    theme::panel(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::muted(p, "Los EPUB se guardan en"));
            ui.label(app.settings.output_dir.display().to_string());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let finished = app.run.result.is_some();
                let back = ui.add_enabled(finished, theme::primary_button(p, "Volver a los libros"));
                if back.on_disabled_hover_text("Espera a que termine").clicked() {
                    app.selected.clear();
                    app.refresh_books();
                    app.screen = Screen::Library;
                }
                if finished && ui.button("Abrir la carpeta").clicked() {
                    let _ = std::process::Command::new("explorer").arg(&app.settings.output_dir).spawn();
                }
            });
        });
    });
}

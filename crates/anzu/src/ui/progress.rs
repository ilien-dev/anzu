//! Pantalla de progreso: pasos, avance por libro y registro.

use eframe::egui::{self, RichText};
use anzu_core::library::BookState;

use super::{Screen, AnzuApp, StepState, theme};
use crate::pipeline::Step;

const STEPS: [(Step, &str, &str); 5] = [
    (Step::Check, "Comprobar versiones", "Kindle instalada y última herramienta publicada"),
    (Step::Download, "Descargar la herramienta", "Con huella SHA-256 verificada"),
    (Step::CloseKindle, "Cerrar la app Kindle", "Solo con tu confirmación"),
    (Step::Unlock, "Desbloquear", "Puede tardar un minuto por libro"),
    (Step::Convert, "Convertir a EPUB", "Con el conversor propio de Anzu"),
];

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui) {
    header(app, ui);
    footer(app, ui);
    let frame = egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(24, 16));
    egui::CentralPanel::default().frame(frame).show(ui, |ui| {
        ui.columns(2, |columns| {
            steps(app, &mut columns[0]);
            columns[1].vertical(|ui| {
                books(app, ui);
                ui.add_space(6.0);
                log(app, ui);
            });
        });
    });
}

fn header(app: &AnzuApp, ui: &mut egui::Ui) {
    let frame = egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(24, 12));
    egui::Panel::top("cabecera-progreso").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Anzu").size(18.0).strong());
            let total = app.run.asins.len();
            let libros = if total == 1 { "1 libro".to_string() } else { format!("{total} libros") };
            match &app.run.result {
                None => {
                    ui.spinner();
                    ui.label(theme::muted(format!("Procesando {libros} · no abras la app Kindle hasta que termine")));
                }
                Some((converted, failed)) if failed.is_empty() => {
                    ui.label(RichText::new(format!("✔ Terminado: {converted} convertidos")).color(theme::OK));
                }
                Some((converted, failed)) => {
                    ui.label(
                        RichText::new(format!("⚠ Terminado: {converted} convertidos, {} con problemas", failed.len()))
                            .color(theme::WARN),
                    );
                }
            }
        });
    });
}

fn steps(app: &AnzuApp, ui: &mut egui::Ui) {
    theme::card(theme::PANEL, theme::LINE).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Pasos").size(16.0).strong());
        ui.add_space(4.0);
        let finished = app.run.result.is_some();
        for (step, title, detail) in STEPS {
            let state = app.run.steps.iter().find(|(s, _)| *s == step).map(|(_, state)| *state);
            let (mark, color, note) = match (state, finished) {
                (Some(StepState::Done), _) => ("✔", theme::OK, detail),
                (Some(StepState::Running), false) => ("●", theme::ACCENT, detail),
                (Some(StepState::Running), true) => ("⚠", theme::WARN, "No se completó"),
                (None, false) => ("○", theme::MUTED, "Pendiente"),
                (None, true) => ("–", theme::MUTED, "No fue necesario"),
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(mark).color(color).size(15.0));
                ui.vertical(|ui| {
                    let running = state == Some(StepState::Running) && !finished;
                    let text = RichText::new(title).color(if state.is_some() { theme::TEXT } else { theme::MUTED });
                    ui.label(if running { text.strong() } else { text });
                    ui.label(theme::muted(note));
                });
            });
            ui.add_space(2.0);
        }
    });
}

fn books(app: &AnzuApp, ui: &mut egui::Ui) {
    let unlocking = app.run.steps.iter().any(|(s, state)| *s == Step::Unlock && *state == StepState::Running);
    let failures = app.run.result.as_ref().map(|(_, failed)| failed.as_slice()).unwrap_or(&[]);
    theme::card(theme::PANEL, theme::LINE).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::vertical().id_salt("libros-progreso").max_height(220.0).show(ui, |ui| {
            for asin in &app.run.asins {
                let title = app
                    .books
                    .iter()
                    .find(|b| &b.asin == asin)
                    .map_or_else(|| format!("Libro {asin}"), AnzuApp::display_title);
                let state = app.run.states.get(asin).copied().unwrap_or(BookState::Locked);
                let failure = failures.iter().find(|(a, _)| a == asin).map(|(_, reason)| reason.as_str());
                let (status, fraction) = match (failure, state) {
                    (Some(_), BookState::Unlocked) => (RichText::new("No se pudo convertir").color(theme::WARN), 0.5),
                    (Some(_), _) => (RichText::new("No se pudo desbloquear").color(theme::WARN), 0.0),
                    (None, BookState::Converted) => (RichText::new("✔ Convertido").color(theme::OK), 1.0),
                    (None, BookState::Unlocked) => (RichText::new("Convirtiendo…").color(theme::DONE), 0.5),
                    (None, BookState::Locked) if unlocking => (RichText::new("Desbloqueando…").color(theme::DONE), 0.1),
                    (None, BookState::Locked) => (RichText::new("En cola").color(theme::MUTED), 0.0),
                };
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(title).truncate());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| ui.label(status));
                });
                ui.add(egui::ProgressBar::new(fraction).desired_height(6.0).fill(theme::ACCENT));
                if let Some(reason) = failure {
                    ui.label(theme::muted(reason.lines().next().unwrap_or(reason)));
                }
                ui.add_space(4.0);
            }
        });
    });
}

fn log(app: &AnzuApp, ui: &mut egui::Ui) {
    theme::card(theme::LOG_BG, theme::LINE).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::muted("Registro"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled(!app.run.log.is_empty(), egui::Button::new("Copiar registro")).clicked() {
                    ui.ctx().copy_text(app.run.log.join("\n"));
                }
            });
        });
        egui::ScrollArea::vertical()
            .id_salt("registro")
            .max_height(260.0)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &app.run.log {
                    ui.label(theme::mono(line.as_str()).size(12.0));
                }
            });
    });
}

fn footer(app: &mut AnzuApp, ui: &mut egui::Ui) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(24, 12));
    egui::Panel::bottom("pie-progreso").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(theme::muted("Los EPUB se guardan en"));
            ui.label(theme::mono(app.settings.output_dir.display().to_string()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let finished = app.run.result.is_some();
                if ui.add_enabled(finished, theme::primary_button("Volver a la biblioteca")).clicked() {
                    app.selected.clear();
                    app.refresh_books();
                    app.screen = Screen::Library;
                }
                if finished && ui.button("Abrir carpeta").clicked() {
                    let _ = std::process::Command::new("explorer").arg(&app.settings.output_dir).spawn();
                }
            });
        });
    });
}

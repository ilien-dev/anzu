//! Pantalla principal: estado del sistema, libros detectados y acción.

use eframe::egui::{self, Color32, RichText};
use seshat_core::library::BookState;

use super::{Filter, SeshatApp, blocked, theme};
use crate::app::{self, BlockReason, Health};

pub fn show(app: &mut SeshatApp, ui: &mut egui::Ui) {
    header(app, ui);
    footer(app, ui);
    let frame = egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(24, 16));
    egui::CentralPanel::default().frame(frame).show(ui, |ui| {
        status_cards(app, ui);
        ui.add_space(6.0);
        if let Health::Blocked(reason) = app.health.clone() {
            blocked::show(app, ui, &reason);
            ui.add_space(6.0);
        }
        filters(app, ui);
        ui.add_space(2.0);
        table(app, ui);
    });
}

fn header(app: &mut SeshatApp, ui: &mut egui::Ui) {
    let frame = egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin::symmetric(24, 12));
    egui::Panel::top("cabecera").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Seshat").size(18.0).strong());
            ui.label(theme::muted("de Kindle a EPUB"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let checking = app.health == Health::Checking;
                let label = if checking { "Comprobando…" } else { "Volver a comprobar" };
                if ui.add_enabled(!checking, egui::Button::new(label)).clicked() {
                    let ctx = ui.ctx().clone();
                    app.start_check(&ctx);
                }
                if checking {
                    ui.spinner();
                }
            });
        });
    });
}

fn status_card(ui: &mut egui::Ui, fill: Color32, line: Color32, label: &str, value: RichText, detail: &str) {
    theme::card(fill, line).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::muted(label));
        ui.label(value.size(15.0));
        ui.label(theme::muted(detail));
    });
}

fn status_cards(app: &SeshatApp, ui: &mut egui::Ui) {
    let (kindle, kindle_detail) = match &app.health {
        Health::Checking => ("Comprobando…".to_string(), "Buscando la app de Microsoft Store".to_string()),
        Health::Ready { kindle, .. } => (kindle.to_string(), "Instalada desde Microsoft Store".to_string()),
        Health::Blocked(BlockReason::KindleMissing) => ("No instalada".to_string(), "No se encontró en este equipo".to_string()),
        Health::Blocked(BlockReason::TooNew { kindle, .. }) => (kindle.clone(), "Más nueva que la herramienta".to_string()),
        Health::Blocked(_) => ("Instalada".to_string(), "Detectada en este equipo".to_string()),
    };
    let (tool, tool_detail) = match &app.health {
        Health::Checking => ("Comprobando…".to_string(), "Consultando la última publicación".to_string()),
        Health::Ready { tool_tag, .. } => (tool_tag.clone(), "Última versión · huella verificada".to_string()),
        Health::Blocked(BlockReason::TooNew { max_supported, .. }) => {
            ("Última versión".to_string(), format!("Admite Kindle hasta {max_supported}"))
        }
        Health::Blocked(BlockReason::Offline | BlockReason::RateLimited) => {
            ("Sin comprobar".to_string(), "No se pudo consultar la última versión".to_string())
        }
        Health::Blocked(BlockReason::NoDigest | BlockReason::BadDownload) => {
            ("No verificada".to_string(), "No se ejecutará".to_string())
        }
        Health::Blocked(_) => ("—".to_string(), "Pendiente".to_string()),
    };

    ui.columns(3, |columns| {
        status_card(&mut columns[0], theme::PANEL, theme::LINE, "App Kindle instalada", theme::mono(kindle), &kindle_detail);
        status_card(&mut columns[1], theme::PANEL, theme::LINE, "Herramienta de desbloqueo", theme::mono(tool), &tool_detail);
        match &app.health {
            Health::Checking => status_card(
                &mut columns[2],
                theme::PANEL,
                theme::LINE,
                "Compatibilidad",
                RichText::new("Comprobando…"),
                "Un momento",
            ),
            Health::Ready { exe_name, .. } => status_card(
                &mut columns[2],
                theme::OK_BG,
                theme::OK_LINE,
                "Compatibilidad",
                RichText::new("✔ Compatibles").color(theme::OK).strong(),
                &format!("Se usará {exe_name}"),
            ),
            Health::Blocked(reason) => status_card(
                &mut columns[2],
                theme::WARN_BG,
                theme::WARN_LINE,
                "Compatibilidad",
                RichText::new("⚠ Desbloqueo bloqueado").color(theme::WARN).strong(),
                &reason.message().0,
            ),
        }
    });
}

fn filters(app: &mut SeshatApp, ui: &mut egui::Ui) {
    let count = |state: BookState| app.books.iter().filter(|b| b.state == state).count();
    let options = [
        (Filter::All, format!("Todos · {}", app.books.len())),
        (Filter::Locked, format!("Con DRM · {}", count(BookState::Locked))),
        (Filter::Unlocked, format!("Desbloqueados · {}", count(BookState::Unlocked))),
        (Filter::Converted, format!("Convertidos · {}", count(BookState::Converted))),
    ];
    ui.horizontal_wrapped(|ui| {
        for (filter, label) in options {
            if ui.selectable_label(app.filter == filter, label).clicked() {
                app.filter = filter;
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(theme::muted(format!("Libros de {}", app.content_dir.display())));
        });
    });
}

fn state_text(state: BookState, ready: bool) -> RichText {
    match state {
        BookState::Locked if ready => RichText::new("🔒 Con DRM").color(theme::WARN),
        BookState::Locked => RichText::new("🔒 Con DRM · en espera").color(theme::MUTED),
        BookState::Unlocked => RichText::new("🔓 Desbloqueado").color(theme::TEXT),
        BookState::Converted => RichText::new("✔ Convertido a EPUB").color(theme::DONE),
    }
}

fn table(app: &mut SeshatApp, ui: &mut egui::Ui) {
    if app.books.is_empty() {
        theme::card(theme::PANEL, theme::LINE).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("No hay libros descargados").strong());
            ui.label(theme::muted(
                "Abre la app Kindle, descarga los libros que quieras convertir y pulsa «Volver a comprobar».",
            ));
        });
        return;
    }

    let ready = app.ready();
    let busy = app.busy();
    let rows: Vec<(String, String, u64, BookState)> = app
        .books
        .iter()
        .filter(|b| match app.filter {
            Filter::All => true,
            Filter::Locked => b.state == BookState::Locked,
            Filter::Unlocked => b.state == BookState::Unlocked,
            Filter::Converted => b.state == BookState::Converted,
        })
        .map(|b| (b.asin.clone(), SeshatApp::display_title(b), b.size, b.state))
        .collect();

    theme::card(theme::PANEL, theme::LINE).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::vertical().id_salt("libros").auto_shrink([false, true]).show(ui, |ui| {
            egui::Grid::new("tabla-libros").num_columns(5).spacing([18.0, 10.0]).striped(true).show(ui, |ui| {
                ui.label("");
                for heading in ["Título", "ASIN", "Tamaño", "Estado"] {
                    ui.label(theme::muted(heading));
                }
                ui.end_row();

                for (asin, title, size, state) in rows {
                    let can_select = !busy && app::selectable(state, ready);
                    let mut checked = app.selected.contains(&asin);
                    if ui.add_enabled(can_select, egui::Checkbox::without_text(&mut checked)).changed() {
                        if checked {
                            app.selected.push(asin.clone());
                        } else {
                            app.selected.retain(|a| a != &asin);
                        }
                    }
                    let title_color = if can_select { theme::TEXT } else { theme::MUTED };
                    ui.add(egui::Label::new(RichText::new(title).color(title_color)).truncate());
                    ui.label(theme::mono(asin).color(theme::MUTED));
                    ui.label(theme::muted(app::human_size(size)));
                    ui.label(state_text(state, ready));
                    ui.end_row();
                }
            });
        });
    });
}

fn footer(app: &mut SeshatApp, ui: &mut egui::Ui) {
    let frame = egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(24, 12));
    egui::Panel::bottom("pie").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(theme::muted("Guardar los EPUB en"));
            ui.label(theme::mono(app.settings.output_dir.display().to_string()));
            if ui.button("Cambiar carpeta").clicked() {
                app.pick_output_dir();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let plan = app.plan();
                let enabled = !plan.asins.is_empty() && !app.busy();
                if ui.add_enabled(enabled, theme::primary_button(app::action_label(&plan))).clicked() {
                    let ctx = ui.ctx().clone();
                    app.start_run(&ctx);
                }
                let selected = match plan.asins.len() {
                    0 => "Ningún libro seleccionado".to_string(),
                    1 => "1 libro seleccionado".to_string(),
                    n => format!("{n} libros seleccionados"),
                };
                ui.label(theme::muted(selected));
            });
        });
    });
}

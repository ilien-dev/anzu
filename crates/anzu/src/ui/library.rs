//! Pantalla principal: estado del sistema, tabla de libros y acción.

use anzu_core::library::BookState;
use anzu_core::settings::ThemeChoice;
use eframe::egui::{self, Align, Align2, Layout, Pos2, Rect, RichText, Sense, Stroke};

use super::{AnzuApp, Column, Filter, Sort, blocked, icons, theme};
use crate::app::{self, Health, RunPlan};

const HEAD_HEIGHT: f32 = 34.0;
const ROW_HEIGHT: f32 = 40.0;
const CHECK_WIDTH: f32 = 44.0;
const ASIN_WIDTH: f32 = 132.0;
const SIZE_WIDTH: f32 = 92.0;
const STATE_WIDTH: f32 = 236.0;

#[derive(Debug, Clone, PartialEq)]
struct Row {
    asin: String,
    title: String,
    size: u64,
    state: BookState,
}

fn state_rank(state: BookState) -> u8 {
    match state {
        BookState::Locked => 0,
        BookState::Unlocked => 1,
        BookState::Converted => 2,
    }
}

fn sort_rows(rows: &mut [Row], sort: Sort) {
    rows.sort_by(|a, b| {
        let order = match sort.column {
            Column::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            Column::Asin => a.asin.cmp(&b.asin),
            Column::Size => a.size.cmp(&b.size),
            Column::State => state_rank(a.state).cmp(&state_rank(b.state)),
        }
        .then_with(|| a.asin.cmp(&b.asin));
        if sort.ascending { order } else { order.reverse() }
    });
}

/// El texto del botón principal: dice qué va a pasar y con cuántos libros.
fn run_label(plan: &RunPlan) -> String {
    match plan.asins.len() {
        0 => app::action_label(plan).to_string(),
        1 => format!("{} 1 libro", app::action_label(plan)),
        n => format!("{} {n} libros", app::action_label(plan)),
    }
}

struct Cells {
    check: Rect,
    title: Rect,
    asin: Rect,
    size: Rect,
    state: Rect,
}

fn cells(row: Rect) -> Cells {
    let state_left = row.right() - STATE_WIDTH;
    let size_left = state_left - SIZE_WIDTH;
    let asin_left = size_left - ASIN_WIDTH;
    let title_left = row.left() + CHECK_WIDTH;
    let span = |from: f32, to: f32| Rect::from_min_max(Pos2::new(from, row.top()), Pos2::new(to.max(from), row.bottom()));
    Cells {
        check: span(row.left(), title_left),
        title: span(title_left, asin_left - 12.0),
        asin: span(asin_left, size_left),
        size: span(size_left, state_left - 20.0),
        state: span(state_left, row.right()),
    }
}

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui) {
    let p = theme::Palette::of(ui.ctx());
    egui::Panel::top("cabecera").frame(theme::band(p.shell, 10)).show_separator_line(false).show(ui, |ui| {
        theme::centered(ui, |ui| header(app, ui, p));
    });
    egui::Panel::bottom("pie").frame(theme::band(p.bg, 12)).show_separator_line(false).show(ui, |ui| {
        theme::centered(ui, |ui| footer(app, ui, p));
    });
    egui::CentralPanel::default().frame(theme::band(p.bg, 12)).show(ui, |ui| {
        theme::centered(ui, |ui| {
            status(app, ui, p);
            ui.add_space(6.0);
            toolbar(app, ui, p);
            table(app, ui, p);
        });
    });
}

fn header(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    ui.horizontal(|ui| {
        AnzuApp::brand(ui, p);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let checking = app.health == Health::Checking;
            let label = if checking { "Comprobando…" } else { "Volver a comprobar" };
            if ui.add_enabled(!checking, theme::shell_button(p, label, false)).clicked() {
                let ctx = ui.ctx().clone();
                app.start_check(&ctx);
            }
            ui.add_space(10.0);
            ui.spacing_mut().item_spacing.x = 0.0;
            // De derecha a izquierda: el primero que se añade queda más a la derecha.
            for (choice, label) in
                [(ThemeChoice::Dark, "Oscuro"), (ThemeChoice::Light, "Claro"), (ThemeChoice::System, "Sistema")]
            {
                if ui.add(theme::shell_button(p, label, app.settings.theme == choice)).clicked() {
                    let ctx = ui.ctx().clone();
                    app.set_theme(&ctx, choice);
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Tema").color(p.shell_muted).size(theme::SMALL));
        });
    });
}

fn status(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    match app.health.clone() {
        Health::Blocked(reason) => blocked::show(app, ui, &reason),
        Health::Checking => {
            theme::panel(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Comprobando la app Kindle y la última versión de la herramienta…");
                });
            });
        }
        Health::Ready { kindle, tool_tag, .. } => {
            theme::panel(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    let (mark, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                    icons::check(ui.painter(), mark, p.ok);
                    ui.label(theme::bold("Lista para desbloquear").color(p.ok));
                    ui.add_space(14.0);
                    fact(ui, p, "Kindle", &kindle.to_string());
                    fact(ui, p, "Herramienta", &tool_tag);
                    fact(ui, p, "Huella", "verificada");
                });
            });
        }
    }
}

fn fact(ui: &mut egui::Ui, p: theme::Palette, label: &str, value: &str) {
    ui.label(theme::muted(p, label));
    ui.label(value);
    ui.add_space(14.0);
}

fn toolbar(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    let count = |state: BookState| app.books.iter().filter(|b| b.state == state).count();
    let options = [
        (Filter::All, format!("Todos {}", app.books.len())),
        (Filter::Locked, format!("Con DRM {}", count(BookState::Locked))),
        (Filter::Unlocked, format!("Desbloqueados {}", count(BookState::Unlocked))),
        (Filter::Converted, format!("Convertidos {}", count(BookState::Converted))),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (filter, label) in options {
            if ui.selectable_label(app.filter == filter, label).clicked() {
                app.filter = filter;
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let selected = app.selected.len();
            let text = match selected {
                0 => "Ninguno seleccionado".to_string(),
                1 => "1 seleccionado".to_string(),
                n => format!("{n} seleccionados"),
            };
            ui.label(theme::muted(p, text).size(theme::SMALL));
        });
    });
}

fn empty_state(ui: &mut egui::Ui, p: theme::Palette) {
    theme::panel(p).inner_margin(egui::Margin::symmetric(20, 18)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::bold("Aún no hay libros que convertir").size(15.0));
        ui.add_space(4.0);
        for step in [
            "1. Abre la app Kindle de Microsoft Store e inicia sesión.",
            "2. Descarga en ella los libros que quieras convertir.",
            "3. Vuelve aquí y pulsa «Volver a comprobar».",
        ] {
            ui.label(step);
        }
        ui.add_space(4.0);
        ui.label(theme::muted(p, "Anzu solo lee esa carpeta: nunca cambia ni borra tus libros de Kindle.").size(theme::SMALL));
    });
}

fn table(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    if app.books.is_empty() {
        empty_state(ui, p);
        return;
    }

    let mut rows: Vec<Row> = app
        .books
        .iter()
        .filter(|b| match app.filter {
            Filter::All => true,
            Filter::Locked => b.state == BookState::Locked,
            Filter::Unlocked => b.state == BookState::Unlocked,
            Filter::Converted => b.state == BookState::Converted,
        })
        .map(|b| Row { asin: b.asin.clone(), title: AnzuApp::display_title(b), size: b.size, state: b.state })
        .collect();
    sort_rows(&mut rows, app.sort);

    let ready = app.ready();
    let busy = app.busy();
    theme::panel(p).inner_margin(egui::Margin::ZERO).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 0.0;
        table_head(app, ui, p, &rows, ready, busy);
        if rows.is_empty() {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(theme::muted(p, "No hay libros en este grupo."));
            });
            ui.add_space(14.0);
            return;
        }
        egui::ScrollArea::vertical().id_salt("libros").auto_shrink([false, true]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in &rows {
                table_row(app, ui, p, row, ready, busy);
            }
        });
    });
}

fn table_head(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, rows: &[Row], ready: bool, busy: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), HEAD_HEIGHT), Sense::hover());
    let c = cells(rect);
    let painter = ui.painter().clone();
    painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line));

    // Casilla de "seleccionar todos" los que se pueden elegir.
    let selectable: Vec<&String> =
        rows.iter().filter(|r| app::selectable(r.state, ready)).map(|r| &r.asin).collect();
    let all_selected = !selectable.is_empty() && selectable.iter().all(|a| app.selected.contains(a));
    let enabled = !busy && !selectable.is_empty();
    let box_rect = Rect::from_center_size(c.check.center(), egui::vec2(16.0, 16.0));
    icons::checkbox(&painter, box_rect, all_selected, enabled, p);
    if enabled {
        let response = ui
            .interact(c.check, ui.scope_id().with("todos"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(if all_selected { "Quitar la selección" } else { "Seleccionar todos" });
        if response.clicked() {
            if all_selected {
                app.selected.retain(|a| !selectable.contains(&a));
            } else {
                for asin in selectable {
                    if !app.selected.contains(asin) {
                        app.selected.push(asin.clone());
                    }
                }
            }
        }
    }

    let columns = [
        (Column::Title, "TÍTULO", c.title, false),
        (Column::Asin, "ASIN", c.asin, false),
        (Column::Size, "TAMAÑO", c.size, true),
        (Column::State, "ESTADO", c.state, false),
    ];
    for (column, label, cell, right) in columns {
        let active = app.sort.column == column;
        let color = if active { p.fg } else { p.muted };
        let (anchor, align) =
            if right { (cell.right_center(), Align2::RIGHT_CENTER) } else { (cell.left_center(), Align2::LEFT_CENTER) };
        let text = painter.text(anchor, align, label, theme::bold_font(theme::SMALL), color);
        if active {
            let x = if right { text.left() - 9.0 } else { text.right() + 9.0 };
            icons::sort_arrow(&painter, Pos2::new(x, cell.center().y), app.sort.ascending, p.accent);
        }
        let response = ui
            .interact(cell, ui.scope_id().with(("columna", label)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Ordenar por esta columna");
        if response.clicked() {
            app.sort = Sort { column, ascending: if active { !app.sort.ascending } else { true } };
        }
    }
}

fn state_label(state: BookState, ready: bool) -> &'static str {
    match state {
        BookState::Locked if ready => "Con DRM",
        BookState::Locked => "Con DRM, en espera",
        BookState::Unlocked => "Desbloqueado",
        BookState::Converted => "EPUB listo",
    }
}

fn table_row(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, row: &Row, ready: bool, busy: bool) {
    let can_select = !busy && app::selectable(row.state, ready);
    let selected = app.selected.contains(&row.asin);
    let sense = if can_select { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_HEIGHT), sense);
    let c = cells(rect);
    let painter = ui.painter().clone();

    if selected {
        painter.rect_filled(rect, 0, p.select_bg);
    } else if can_select && response.hovered() {
        painter.rect_filled(rect, 0, p.hover_bg);
    }
    painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line.gamma_multiply(0.55)));

    icons::checkbox(&painter, Rect::from_center_size(c.check.center(), egui::vec2(16.0, 16.0)), selected, can_select, p);

    let text_color = if can_select { p.fg } else { p.muted };
    let mut title_ui = ui.new_child(egui::UiBuilder::new().max_rect(c.title).layout(Layout::left_to_right(Align::Center)));
    title_ui.add(egui::Label::new(RichText::new(&row.title).color(text_color)).truncate().selectable(false));

    let small = theme::regular_font(12.0);
    painter.text(c.asin.left_center(), Align2::LEFT_CENTER, &row.asin, small.clone(), p.muted);
    painter.text(c.size.right_center(), Align2::RIGHT_CENTER, app::human_size(row.size), small.clone(), p.muted);

    let end = icons::track(&painter, c.state.left_center(), row.state, can_select || row.state != BookState::Locked, p);
    let (_, state_color) = icons::track_progress(row.state, p);
    let label_color = if row.state == BookState::Locked && !ready { p.muted } else { state_color };
    painter.text(
        Pos2::new(end + 10.0, c.state.center().y),
        Align2::LEFT_CENTER,
        state_label(row.state, ready),
        small,
        label_color,
    );

    let response = response.on_hover_cursor(if can_select { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Default });
    if response.clicked() {
        if selected {
            app.selected.retain(|a| a != &row.asin);
        } else {
            app.selected.push(row.asin.clone());
        }
    }
}

fn footer(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette) {
    theme::panel(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::muted(p, "Guardar en"));
            ui.label(app.settings.output_dir.display().to_string());
            if ui.button("Cambiar carpeta").clicked() {
                app.pick_output_dir();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let plan = app.plan();
                let enabled = !plan.asins.is_empty() && !app.busy();
                if ui.add_enabled(enabled, theme::primary_button(p, &run_label(&plan))).clicked() {
                    let ctx = ui.ctx().clone();
                    app.start_run(&ctx);
                }
                if plan.asins.is_empty() {
                    ui.label(theme::muted(p, "Marca los libros que quieras convertir").size(theme::SMALL));
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(asin: &str, title: &str, size: u64, state: BookState) -> Row {
        Row { asin: asin.into(), title: title.into(), size, state }
    }

    fn sample() -> Vec<Row> {
        vec![
            row("B3", "zeta", 30, BookState::Converted),
            row("B1", "Alfa", 20, BookState::Locked),
            row("B2", "beta", 10, BookState::Unlocked),
        ]
    }

    fn order(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.asin.as_str()).collect()
    }

    #[test]
    fn titles_sort_without_regard_to_case() {
        let mut rows = sample();
        sort_rows(&mut rows, Sort { column: Column::Title, ascending: true });
        assert_eq!(order(&rows), vec!["B1", "B2", "B3"]);
        sort_rows(&mut rows, Sort { column: Column::Title, ascending: false });
        assert_eq!(order(&rows), vec!["B3", "B2", "B1"]);
    }

    #[test]
    fn size_and_state_sort_by_their_own_order() {
        let mut rows = sample();
        sort_rows(&mut rows, Sort { column: Column::Size, ascending: true });
        assert_eq!(order(&rows), vec!["B2", "B1", "B3"]);
        sort_rows(&mut rows, Sort { column: Column::State, ascending: true });
        assert_eq!(order(&rows), vec!["B1", "B2", "B3"], "con DRM, desbloqueado, convertido");
    }

    #[test]
    fn equal_keys_keep_a_stable_order_by_asin() {
        let mut rows = vec![row("B2", "Igual", 1, BookState::Locked), row("B1", "igual", 1, BookState::Locked)];
        sort_rows(&mut rows, Sort { column: Column::Title, ascending: true });
        assert_eq!(order(&rows), vec!["B1", "B2"]);
    }

    #[test]
    fn the_button_says_what_will_happen_and_to_how_many() {
        let plan = |needs_unlock, n: usize| RunPlan { needs_unlock, asins: (0..n).map(|i| i.to_string()).collect() };
        assert_eq!(run_label(&plan(false, 0)), "Convertir");
        assert_eq!(run_label(&plan(false, 1)), "Convertir 1 libro");
        assert_eq!(run_label(&plan(true, 3)), "Desbloquear y convertir 3 libros");
    }

    #[test]
    fn columns_never_overlap_even_in_a_narrow_window() {
        for width in [300.0, 700.0, 1160.0] {
            let c = cells(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, ROW_HEIGHT)));
            assert!(c.title.width() >= 0.0, "ancho {width}");
            assert!(c.check.right() <= c.title.left() + 0.01);
            assert!(c.asin.right() <= c.size.left() + 0.01);
            assert!(c.size.right() <= c.state.left() + 0.01);
        }
    }
}

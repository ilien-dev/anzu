//! Main screen: system status, the book table and the action.

use anzu_core::library::BookState;
use anzu_core::settings::{Language, ThemeChoice};
use eframe::egui::{self, Align, Align2, Layout, Pos2, Rect, RichText, Sense, Stroke};

use super::{AnzuApp, Column, Filter, Sort, blocked, icons, theme};
use crate::app::{self, Health};
use crate::i18n::Strings;

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
    let t = app.t();
    egui::Panel::top("header").frame(theme::band(p.shell, 10)).show_separator_line(false).show(ui, |ui| {
        theme::centered(ui, |ui| header(app, ui, p, t));
    });
    egui::Panel::bottom("footer").frame(theme::band(p.bg, 12)).show_separator_line(false).show(ui, |ui| {
        theme::centered(ui, |ui| footer(app, ui, p, t));
    });
    egui::CentralPanel::default().frame(theme::band(p.bg, 12)).show(ui, |ui| {
        theme::centered(ui, |ui| {
            status(app, ui, p, t);
            ui.add_space(6.0);
            toolbar(app, ui, p, t);
            table(app, ui, p, t);
        });
    });
}

fn header(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.horizontal(|ui| {
        AnzuApp::brand(ui, p, t);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let checking = app.health == Health::Checking;
            let label = if checking { t.checking } else { t.recheck };
            if ui.add_enabled(!checking, theme::shell_button(p, label, false)).clicked() {
                let ctx = ui.ctx().clone();
                app.start_check(&ctx);
            }
            ui.add_space(10.0);
            ui.spacing_mut().item_spacing.x = 0.0;
            // Right to left: the first one added ends up furthest right.
            for (choice, label) in [
                (ThemeChoice::Dark, t.theme_dark),
                (ThemeChoice::Light, t.theme_light),
                (ThemeChoice::System, t.theme_system),
            ] {
                if ui.add(theme::shell_button(p, label, app.settings.theme == choice)).clicked() {
                    let ctx = ui.ctx().clone();
                    app.set_theme(&ctx, choice);
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new(t.theme).color(p.shell_muted).size(theme::SMALL));
            ui.add_space(14.0);
            for (language, label) in [(Language::Es, "ES"), (Language::En, "EN")] {
                if ui.add(theme::shell_button(p, label, app.settings.language == language)).clicked() {
                    let ctx = ui.ctx().clone();
                    app.set_language(&ctx, language);
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new(t.language).color(p.shell_muted).size(theme::SMALL));
        });
    });
}

fn status(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    match app.health.clone() {
        Health::Blocked(reason) => blocked::show(app, ui, &reason),
        Health::Checking => {
            theme::panel(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(t.checking_long);
                });
            });
        }
        Health::Ready { kindle, tool_tag, .. } => {
            theme::panel(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    let (mark, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                    icons::check(ui.painter(), mark, p.ok);
                    ui.label(theme::bold(t.ready).color(p.ok));
                    ui.add_space(14.0);
                    fact(ui, p, t.fact_kindle, &kindle.to_string());
                    fact(ui, p, t.fact_tool, &tool_tag);
                    fact(ui, p, t.fact_digest, t.verified);
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

fn toolbar(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let count = |state: BookState| app.books.iter().filter(|b| b.state == state).count();
    let options = [
        (Filter::All, format!("{} {}", t.filter_all, app.books.len())),
        (Filter::Locked, format!("{} {}", t.filter_locked, count(BookState::Locked))),
        (Filter::Unlocked, format!("{} {}", t.filter_unlocked, count(BookState::Unlocked))),
        (Filter::Converted, format!("{} {}", t.filter_converted, count(BookState::Converted))),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for (filter, label) in options {
            if ui.selectable_label(app.filter == filter, label).clicked() {
                app.filter = filter;
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(theme::muted(p, t.selected(app.selected.len())).size(theme::SMALL));
        });
    });
}

fn empty_state(ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    theme::panel(p).inner_margin(egui::Margin::symmetric(20, 18)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::bold(t.empty_title).size(15.0));
        ui.add_space(4.0);
        for step in t.empty_steps {
            ui.label(step);
        }
        ui.add_space(4.0);
        ui.label(theme::muted(p, t.empty_note).size(theme::SMALL));
    });
}

fn table(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    if app.books.is_empty() {
        empty_state(ui, p, t);
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
        .map(|b| Row { asin: b.asin.clone(), title: AnzuApp::display_title(b, t), size: b.size, state: b.state })
        .collect();
    sort_rows(&mut rows, app.sort);

    let ready = app.ready();
    let busy = app.busy();
    theme::panel(p).inner_margin(egui::Margin::ZERO).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.spacing_mut().item_spacing.y = 0.0;
        table_head(app, ui, p, t, &rows, ready, busy);
        if rows.is_empty() {
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(theme::muted(p, t.empty_group));
            });
            ui.add_space(14.0);
            return;
        }
        egui::ScrollArea::vertical().id_salt("books").auto_shrink([false, true]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for row in &rows {
                table_row(app, ui, p, t, row, ready, busy);
            }
        });
    });
}

fn table_head(
    app: &mut AnzuApp,
    ui: &mut egui::Ui,
    p: theme::Palette,
    t: &Strings,
    rows: &[Row],
    ready: bool,
    busy: bool,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), HEAD_HEIGHT), Sense::hover());
    let c = cells(rect);
    let painter = ui.painter().clone();
    painter.hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, p.line));

    // The box that selects every book that can be picked.
    let selectable: Vec<&String> =
        rows.iter().filter(|r| app::selectable(r.state, ready)).map(|r| &r.asin).collect();
    let all_selected = !selectable.is_empty() && selectable.iter().all(|a| app.selected.contains(a));
    let enabled = !busy && !selectable.is_empty();
    let box_rect = Rect::from_center_size(c.check.center(), egui::vec2(16.0, 16.0));
    icons::checkbox(&painter, box_rect, all_selected, enabled, p);
    if enabled {
        let response = ui
            .interact(c.check, ui.scope_id().with("all"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(if all_selected { t.clear_selection } else { t.select_all });
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
        (Column::Title, t.col_title, c.title, false),
        (Column::Asin, t.col_asin, c.asin, false),
        (Column::Size, t.col_size, c.size, true),
        (Column::State, t.col_state, c.state, false),
    ];
    for (index, (column, label, cell, right)) in columns.into_iter().enumerate() {
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
            .interact(cell, ui.scope_id().with(("column", index)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(t.sort_hint);
        if response.clicked() {
            app.sort = Sort { column, ascending: if active { !app.sort.ascending } else { true } };
        }
    }
}

fn table_row(
    app: &mut AnzuApp,
    ui: &mut egui::Ui,
    p: theme::Palette,
    t: &Strings,
    row: &Row,
    ready: bool,
    busy: bool,
) {
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
    painter.text(c.size.right_center(), Align2::RIGHT_CENTER, t.human_size(row.size), small.clone(), p.muted);

    let end = icons::track(&painter, c.state.left_center(), row.state, can_select || row.state != BookState::Locked, p);
    let (_, state_color) = icons::track_progress(row.state, p);
    let label_color = if row.state == BookState::Locked && !ready { p.muted } else { state_color };
    painter.text(
        Pos2::new(end + 10.0, c.state.center().y),
        Align2::LEFT_CENTER,
        t.state(row.state, ready),
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

fn footer(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    theme::panel(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::muted(p, t.save_in));
            ui.label(app.settings.output_dir.display().to_string());
            if ui.button(t.change_folder).clicked() {
                app.pick_output_dir();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let plan = app.plan();
                let enabled = !plan.asins.is_empty() && !app.busy();
                let label = t.run_label(plan.needs_unlock, plan.asins.len());
                if ui.add_enabled(enabled, theme::primary_button(p, &label)).clicked() {
                    let ctx = ui.ctx().clone();
                    app.start_run(&ctx);
                }
                if plan.asins.is_empty() {
                    ui.label(theme::muted(p, t.pick_hint).size(theme::SMALL));
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
            row("B1", "Alpha", 20, BookState::Locked),
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
        assert_eq!(order(&rows), vec!["B1", "B2", "B3"], "with DRM, unlocked, converted");
    }

    #[test]
    fn equal_keys_keep_a_stable_order_by_asin() {
        let mut rows = vec![row("B2", "Same", 1, BookState::Locked), row("B1", "same", 1, BookState::Locked)];
        sort_rows(&mut rows, Sort { column: Column::Title, ascending: true });
        assert_eq!(order(&rows), vec!["B1", "B2"]);
    }

    #[test]
    fn columns_never_overlap_even_in_a_narrow_window() {
        for width in [300.0, 700.0, 1160.0] {
            let c = cells(Rect::from_min_size(Pos2::ZERO, egui::vec2(width, ROW_HEIGHT)));
            assert!(c.title.width() >= 0.0, "width {width}");
            assert!(c.check.right() <= c.title.left() + 0.01);
            assert!(c.asin.right() <= c.size.left() + 0.01);
            assert!(c.size.right() <= c.state.left() + 0.01);
        }
    }
}

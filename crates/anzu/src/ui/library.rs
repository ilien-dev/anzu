//! Main screen: the system status, the books on shelves by what they still need, and a side
//! panel that says what is going to happen before the button does it.

use anzu_core::library::{Book, BookState};
use eframe::egui::{self, Align, Align2, Layout, Pos2, Rect, Sense, Stroke};

use super::icons::{self, Icon};
use super::theme::{self, Kind};
use super::{AnzuApp, Column, Screen, Sort, blocked, demo, settings_menu};
use crate::app::{self, Health};
use crate::i18n::Strings;

const CARD_HEIGHT: f32 = 74.0;
const CARD_MIN_WIDTH: f32 = 250.0;
const CARD_GAP: f32 = 10.0;
/// How long a card takes to show the pointer over it, and to become chosen, in seconds.
const CARD_HOVER_TIME: f32 = 0.12;
const CARD_SELECT_TIME: f32 = 0.16;
const ASIDE_WIDTH: f32 = 320.0;
const GAP: f32 = 20.0;
/// The chosen books listed by name in the side panel before "and n more".
const ASIDE_LIST: usize = 6;

/// The shelves, in the order a book moves along them.
const SHELVES: [BookState; 3] = [BookState::Locked, BookState::Unlocked, BookState::Converted];

#[derive(Debug, Clone, PartialEq)]
struct Row {
    asin: String,
    title: String,
    size: u64,
    state: BookState,
}

fn sort_rows(rows: &mut [Row], sort: Sort) {
    rows.sort_by(|a, b| {
        let order = match sort.column {
            Column::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            Column::Asin => a.asin.cmp(&b.asin),
            Column::Size => a.size.cmp(&b.size),
        }
        .then_with(|| a.asin.cmp(&b.asin));
        if sort.ascending { order } else { order.reverse() }
    });
}

/// How many cards fit side by side in `width`.
fn columns(width: f32) -> usize {
    (((width + CARD_GAP) / (CARD_MIN_WIDTH + CARD_GAP)).floor() as usize).clamp(1, 3)
}

/// Where each shelf sits in [`AnzuApp::shelf_open`].
pub(super) fn shelf_index(state: BookState) -> usize {
    match state {
        BookState::Locked => 0,
        BookState::Unlocked => 1,
        BookState::Converted => 2,
    }
}

/// Up to two initials of a title, for its cover: the first and last words, or the first two
/// letters of a single word.
fn initials(title: &str) -> String {
    let words: Vec<&str> = title
        .split_whitespace()
        .map(|w| w.trim_start_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();
    let upper = |c: char| c.to_uppercase().next().unwrap_or(c);
    match words.as_slice() {
        [] => String::new(),
        [one] => one.chars().take(2).enumerate().map(|(i, c)| if i == 0 { upper(c) } else { c }).collect(),
        [first, .., last] => [first, last].iter().filter_map(|w| w.chars().next()).map(upper).collect(),
    }
}

/// Lays `text` out on one line, cut with an ellipsis at `width`.
fn elided(painter: &egui::Painter, text: &str, font: egui::FontId, color: egui::Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::single_section(text.to_string(), egui::TextFormat::simple(font, color));
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(10.0));
    painter.layout_job(job)
}

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui) {
    let p = theme::Palette::of(ui.ctx());
    let t = app.t();
    egui::Panel::top("header").frame(theme::band(p.shell, 10)).show_separator_line(false).show(ui, |ui| {
        super::enter(ui, Screen::Library, false, |ui| theme::centered(ui, |ui| header(app, ui, p, t)));
    });
    egui::CentralPanel::default().frame(theme::band(p.bg, 18)).show(ui, |ui| {
        super::enter(ui, Screen::Library, true, |ui| theme::centered(ui, |ui| {
            let area = ui.available_rect_before_wrap();
            let aside_width = ASIDE_WIDTH.min((area.width() * 0.37).max(260.0));
            let main = Rect::from_min_max(area.min, Pos2::new(area.right() - aside_width - GAP, area.bottom()));
            let aside = Rect::from_min_max(Pos2::new(area.right() - aside_width, area.top()), area.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(main), |ui| {
                egui::ScrollArea::vertical().id_salt("shelves").auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    status(app, ui, p, t);
                    ui.add_space(10.0);
                    toolbar(app, ui, p, t);
                    ui.add_space(6.0);
                    shelves(app, ui, p, t);
                    ui.add_space(8.0);
                });
            });
            ui.scope_builder(egui::UiBuilder::new().max_rect(aside), |ui| side_panel(app, ui, p, t));
            ui.allocate_rect(area, Sense::hover());
        }));
    });
}

fn header(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &'static Strings) {
    ui.horizontal(|ui| {
        AnzuApp::brand(ui, p, t);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Right to left: the first one added ends up furthest right.
            settings_menu::show(app, ui, p, t);
        });
    });
}

/// "Check again", at the end of the status line: it checks again what the line shows,
/// the Kindle app and the unlock tool.
fn recheck_button(app: &mut AnzuApp, ui: &mut egui::Ui, t: &Strings) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let checking = app.health == Health::Checking;
        let label = if checking { t.checking } else { t.recheck };
        if theme::icon_button(ui, Kind::Secondary, !checking, Icon::Refresh, label).clicked() {
            let ctx = ui.ctx().clone();
            app.start_check(&ctx);
        }
    });
}

fn status(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    match app.health.clone() {
        Health::Blocked(reason) => blocked::show(app, ui, &reason),
        Health::Checking => {
            theme::panel(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (mark, _) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::hover());
                    ui.put(mark, egui::Spinner::new().size(18.0).color(p.fg));
                    ui.label(t.checking_long);
                    recheck_button(app, ui, t);
                });
            });
        }
        Health::Ready { kindle, tool_tag, .. } => {
            theme::panel(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ready_line(app, ui, p, t, &kindle.to_string(), &tool_tag);
            });
        }
    }
}

/// The line that says the system is ready: the facts that were checked, as chips that wrap,
/// and "Check again" at its end.
fn ready_line(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings, kindle: &str, tool_tag: &str) {
    // The facts wrap in what the button leaves free.
    let label = ui.painter().layout_no_wrap(t.recheck.to_string(), theme::bold_font(13.0), p.fg);
    let button_width = 14.0 + 16.0 + 8.0 + label.size().x + 14.0;
    let facts_width = (ui.available_width() - button_width - 12.0).max(120.0);
    // The chips are lower than the button: their first row is moved down to the button's
    // middle, so that the line sits in the middle of the panel.
    const ROW: f32 = 28.0;
    const BUTTON: f32 = 34.0;
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(egui::vec2(facts_width, 0.0), Layout::top_down(Align::Min), |ui| {
            ui.add_space((BUTTON - ROW) / 2.0);
            let wrap = Layout::left_to_right(Align::Center).with_main_wrap(true);
            ui.allocate_ui_with_layout(egui::vec2(facts_width, 0.0), wrap, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                // A wrapping row centres each item on the row height known when it is placed;
                // set it to the chips' height so the text lines up with them.
                ui.set_row_height(ROW);
                icons::tile(ui, Icon::Shield, 26.0, theme::EPUB, theme::INK);
                ui.label(theme::bold(t.ready));
                ui.add_space(6.0);
                theme::chip(ui, Icon::Device, p.quiet, p.fg, &format!("{} {kindle}", t.fact_kindle));
                theme::chip(ui, Icon::Package, p.quiet, p.fg, &format!("{} {tool_tag}", t.fact_tool));
                theme::chip(ui, Icon::Hash, p.quiet, p.fg, &format!("{} {}", t.fact_digest, t.verified));
            });
        });
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            if theme::icon_button(ui, Kind::Secondary, true, Icon::Refresh, t.recheck).clicked() {
                let ctx = ui.ctx().clone();
                app.start_check(&ctx);
            }
        });
    });
}

fn toolbar(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.horizontal(|ui| {
        ui.label(theme::display(t.shelf).size(24.0));
        ui.label(theme::muted(p, format!("· {}", t.books(app.books.len()))).size(16.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            refresh_button(app, ui, t);
            if let Some(found) = app.refreshed {
                ui.label(theme::muted(p, t.new_books(found)).size(theme::SMALL));
            }
        });
    });
    // What the shelves are sorted by, as pills to pick one, and apart from them the
    // direction, said in words, on a button that reverses it.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(theme::muted(p, t.sort_by).size(theme::SMALL));
        ui.add_space(2.0);
        for (column, label) in [(Column::Title, t.sort_title), (Column::Asin, t.sort_asin), (Column::Size, t.sort_size)] {
            if sort_pill(ui, p, app.sort.column == column, label).clicked() {
                app.sort = Sort { column, ascending: true };
            }
        }
        ui.add_space(4.0);
        let (divider, _) = ui.allocate_exact_size(egui::vec2(1.0, 22.0), Sense::hover());
        ui.painter().rect_filled(divider, 0.0, p.line_strong);
        ui.add_space(4.0);
        let ascending = app.sort.ascending;
        let arrow = if ascending { Icon::ArrowUp } else { Icon::ArrowDown };
        let word = match (app.sort.column, ascending) {
            (Column::Size, true) => t.sort_smallest,
            (Column::Size, false) => t.sort_largest,
            (_, true) => t.sort_a_to_z,
            (_, false) => t.sort_z_to_a,
        };
        if theme::icon_button(ui, Kind::Secondary, true, arrow, word).on_hover_text(t.sort_reverse).clicked() {
            app.sort.ascending = !ascending;
        }
    });
}

/// One of the fields the shelves can be sorted by: an outlined pill, filled with ink and
/// ticked when it is the one in use.
fn sort_pill(ui: &mut egui::Ui, p: theme::Palette, active: bool, label: &str) -> egui::Response {
    const HEIGHT: f32 = 32.0;
    const CHECK: f32 = 14.0;
    let ink = if active { p.bg } else { p.fg };
    let galley = ui.painter().layout_no_wrap(label.to_string(), theme::bold_font(13.0), ink);
    let content = galley.size().x + if active { CHECK + 6.0 } else { 0.0 };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(14.0 + content + 14.0, HEIGHT), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, active, label));
    theme::pointer(&response, true);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same((HEIGHT / 2.0) as u8);
        if active {
            painter.rect_filled(rect, radius, p.fg);
        } else {
            painter.rect_stroke(rect, radius, egui::Stroke::new(1.0, p.line_strong), egui::StrokeKind::Inside);
        }
        let mut x = rect.left() + 14.0;
        if active {
            let check = egui::Rect::from_min_size(egui::pos2(x, rect.center().y - CHECK / 2.0), egui::vec2(CHECK, CHECK));
            icons::paint(painter, Icon::Check, check, ink);
            x += CHECK + 6.0;
        }
        painter.galley(egui::pos2(x, rect.center().y - galley.size().y / 2.0), galley, ink);
        if !active {
            let hover = ui.ctx().animate_bool_with_time(response.id.with("hover"), response.hovered(), 0.12);
            if hover > 0.0 {
                painter.rect_filled(rect, radius, p.fg.gamma_multiply(0.06 * hover));
            }
        }
    }
    response
}

/// "Refresh list": shows the books downloaded in Kindle since the window opened.
fn refresh_button(app: &mut AnzuApp, ui: &mut egui::Ui, t: &Strings) {
    let enabled = !app.busy();
    let response = theme::icon_button(ui, Kind::Secondary, enabled, Icon::Refresh, t.refresh).on_hover_text(t.refresh_hint);
    app.mark(demo::Target::Refresh, response.rect);
    if response.clicked() {
        app.refresh_by_hand();
    }
}

fn empty_state(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    theme::panel(p).inner_margin(egui::Margin::symmetric(24, 22)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            icons::tile(ui, Icon::Book, 40.0, theme::UNLOCKED, theme::INK);
            ui.label(theme::display(t.empty_title).size(20.0));
        });
        ui.add_space(8.0);
        for (step, icon) in t.empty_steps.iter().zip([Icon::Device, Icon::Download, Icon::Refresh]) {
            ui.horizontal(|ui| {
                icons::tile(ui, icon, 26.0, p.quiet, p.fg);
                ui.label(*step);
            });
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Info, 16.0, p.muted);
            ui.label(theme::muted(p, t.empty_note).size(theme::SMALL));
        });
        ui.add_space(10.0);
        refresh_button(app, ui, t);
    });
}

fn shelves(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    if app.books.is_empty() {
        empty_state(app, ui, p, t);
        return;
    }
    let ready = app.ready();
    let busy = app.busy();
    for state in SHELVES {
        let mut rows: Vec<Row> = app
            .books
            .iter()
            .filter(|b| b.state == state)
            .map(|b| Row { asin: b.asin.clone(), title: AnzuApp::display_title(b, t), size: b.size, state: b.state })
            .collect();
        if rows.is_empty() {
            continue;
        }
        sort_rows(&mut rows, app.sort);
        ui.add_space(8.0);
        shelf_head(app, ui, p, t, state, &rows, ready, busy);
        ui.add_space(2.0);
        if app.shelf_open[shelf_index(state)] {
            cards(app, ui, p, t, &rows, ready, busy);
        } else {
            folded(ui, p, t, &rows);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn shelf_head(
    app: &mut AnzuApp,
    ui: &mut egui::Ui,
    p: theme::Palette,
    t: &Strings,
    state: BookState,
    rows: &[Row],
    ready: bool,
    busy: bool,
) {
    let name = match state {
        BookState::Locked => t.shelf_locked,
        BookState::Unlocked => t.shelf_unlocked,
        BookState::Converted => t.shelf_converted,
    };
    let index = shelf_index(state);
    let open = app.shelf_open[index];
    ui.horizontal(|ui| {
        icons::tile(ui, icons::state_icon(state), 26.0, icons::state_fill(state), theme::INK);
        ui.label(theme::display(name).size(17.0));
        ui.label(theme::muted(p, rows.len().to_string()).size(15.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Right to left: the first one added ends up furthest right.
            if state == BookState::Converted {
                let (icon, label) = if open { (Icon::ChevronDown, t.shelf_hide) } else { (Icon::ChevronRight, t.shelf_show) };
                if theme::icon_button(ui, Kind::Quiet, true, icon, label).clicked() {
                    app.shelf_open[index] = !open;
                }
            }
            let selectable: Vec<&String> =
                rows.iter().filter(|r| app::selectable(r.state, ready)).map(|r| &r.asin).collect();
            if !open || selectable.is_empty() {
                return;
            }
            let all = selectable.iter().all(|a| app.selected.contains(a));
            let (icon, label) = if all { (Icon::Close, t.unmark_all.to_string()) } else { (Icon::Check, t.mark_all(selectable.len())) };
            if theme::icon_button(ui, Kind::Quiet, !busy, icon, &label).clicked() {
                if all {
                    app.selected.retain(|a| !selectable.contains(&a));
                } else {
                    for asin in selectable {
                        if !app.selected.contains(asin) {
                            app.selected.push(asin.clone());
                        }
                    }
                }
            }
        });
    });
}

/// A folded shelf: the names of its books on one quiet line.
fn folded(ui: &mut egui::Ui, p: theme::Palette, t: &Strings, rows: &[Row]) {
    let names: Vec<&str> = rows.iter().take(3).map(|r| r.title.as_str()).collect();
    let mut text = names.join(", ");
    if rows.len() > names.len() {
        text = format!("{text} {}", t.and_more(rows.len() - names.len()));
    }
    egui::Frame::new().fill(p.quiet).corner_radius(theme::CARD_RADIUS).inner_margin(egui::Margin::symmetric(14, 10)).show(
        ui,
        |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(theme::muted(p, text).size(13.0)).truncate());
        },
    );
}

fn cards(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings, rows: &[Row], ready: bool, busy: bool) {
    let width = ui.available_width();
    let n = columns(width);
    let card_width = (width - CARD_GAP * (n - 1) as f32) / n as f32;
    for line in rows.chunks(n) {
        let (strip, _) = ui.allocate_exact_size(egui::vec2(width, CARD_HEIGHT), Sense::hover());
        for (i, row) in line.iter().enumerate() {
            let left = strip.left() + i as f32 * (card_width + CARD_GAP);
            let rect = Rect::from_min_size(Pos2::new(left, strip.top()), egui::vec2(card_width, CARD_HEIGHT));
            card(app, ui, p, t, row, rect, ready, busy);
        }
        ui.add_space(CARD_GAP - ui.spacing().item_spacing.y);
    }
}

#[allow(clippy::too_many_arguments)]
fn card(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings, row: &Row, rect: Rect, ready: bool, busy: bool) {
    let can_select = !busy && app::selectable(row.state, ready);
    let selected = app.selected.contains(&row.asin);
    let sense = if can_select { Sense::click() } else { Sense::hover() };
    let response = ui.interact(rect, ui.scope_id().with(("card", &row.asin)), sense);
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, can_select, selected, &row.title));
    theme::pointer(&response, can_select);
    let painter = ui.painter().clone();

    // Hover and selection blend in rather than snap: the card warms up under the pointer and
    // takes the butter of what is chosen in a short fade.
    let ctx = ui.ctx();
    let hover = ctx.animate_bool_with_time(response.id.with("hover"), can_select && response.hovered(), CARD_HOVER_TIME);
    let chosen = ctx.animate_bool_with_time(response.id.with("chosen"), selected, CARD_SELECT_TIME);
    let fill = theme::blend(theme::blend(p.panel, p.hover_bg, hover), p.select_bg, chosen);
    let edge = theme::blend(theme::blend(p.line, p.line_strong, hover), p.select_line, chosen);
    painter.rect_filled(rect, theme::CARD_RADIUS, fill);
    painter.rect_stroke(rect, theme::CARD_RADIUS, Stroke::new(1.0 + chosen, edge), egui::StrokeKind::Inside);
    let dim = |c: egui::Color32| if can_select || row.state != BookState::Locked { c } else { c.gamma_multiply(0.55) };

    // The cover: the pastel of the state, with a lock or the initials of the title.
    let cover = Rect::from_min_size(Pos2::new(rect.left() + 12.0, rect.center().y - 25.0), egui::vec2(38.0, 50.0));
    painter.rect_filled(cover, egui::CornerRadius::same(6), dim(icons::state_fill(row.state)));
    if row.state == BookState::Locked {
        icons::paint(&painter, Icon::Lock, cover.shrink2(egui::vec2(10.0, 16.0)), theme::INK);
    } else {
        painter.text(cover.center(), Align2::CENTER_CENTER, initials(&row.title), theme::display_font(15.0), theme::INK);
    }

    let check = Rect::from_center_size(Pos2::new(rect.right() - 22.0, rect.center().y), egui::vec2(20.0, 20.0));
    icons::checkbox(&painter, check, chosen, can_select, p);

    let text_left = cover.right() + 12.0;
    let text_width = check.left() - 10.0 - text_left;
    let title_color = dim(if row.state == BookState::Locked { p.muted } else { p.fg });
    let title = if row.state == BookState::Locked { row.asin.clone() } else { row.title.clone() };
    let title = elided(&painter, &title, theme::bold_font(14.0), title_color, text_width);
    let detail = match row.state {
        BookState::Locked => format!("{} · {}", t.human_size(row.size), t.title_after_unlock),
        _ => format!("{} · {}", row.asin, t.human_size(row.size)),
    };
    let detail = elided(&painter, &detail, theme::regular_font(12.0), dim(p.muted), text_width);
    let top = rect.center().y - (title.size().y + 3.0 + detail.size().y) / 2.0;
    painter.galley(Pos2::new(text_left, top), title.clone(), title_color);
    painter.galley(Pos2::new(text_left, top + title.size().y + 3.0), detail, dim(p.muted));

    // A chosen book that already has an EPUB will be converted again: say so on its card.
    if selected && row.state == BookState::Converted {
        let text = painter.layout_no_wrap(t.again_badge.to_string(), theme::bold_font(10.0), theme::INK);
        let badge = Rect::from_min_size(
            Pos2::new(check.left() - text.size().x - 22.0, rect.top() + 8.0),
            egui::vec2(text.size().x + 12.0, 18.0),
        );
        painter.rect_filled(badge, 9, theme::BUTTER);
        painter.galley(Pos2::new(badge.left() + 6.0, badge.center().y - text.size().y / 2.0), text, theme::INK);
    }

    if let Some(asin) = demo::BOOKS.iter().find(|a| **a == row.asin) {
        app.mark(demo::Target::Book(asin), check);
    }
    if response.clicked() {
        if selected {
            app.selected.retain(|a| a != &row.asin);
        } else {
            app.selected.push(row.asin.clone());
        }
    }
}

/// The chosen books that already have an EPUB, in library order.
fn chosen_converted(app: &AnzuApp) -> Vec<&Book> {
    app.books.iter().filter(|b| b.state == BookState::Converted && app.selected.contains(&b.asin)).collect()
}

/// The side panel: what is chosen, what will happen to each, where it goes, and the button.
fn side_panel(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, theme::CARD_RADIUS, p.aside);
    ui.painter().rect_stroke(rect, theme::CARD_RADIUS, Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
    let inner = rect.shrink(18.0);
    let plan = app.plan();
    let footer_height = 42.0;
    let body = Rect::from_min_max(inner.min, Pos2::new(inner.right(), inner.bottom() - footer_height - 12.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
        egui::ScrollArea::vertical().id_salt("aside").auto_shrink([false, false]).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 8.0;
            ui.horizontal(|ui| {
                icons::tile(ui, Icon::Check, 26.0, theme::BUTTER, theme::INK);
                ui.label(theme::display(t.selected(app.selected.len())).size(18.0));
            });
            chosen_list(app, ui, p, t);
            ui.add_space(4.0);
            separator(ui, p);
            destination(app, ui, p, t);
            if plan.needs_unlock {
                note(ui, p.drm_tint, Icon::Lock, p.fg, t.kindle_note);
            }
            reconvert_notice(app, ui, p, t);
        });
    });
    let footer = Rect::from_min_max(Pos2::new(inner.left(), inner.bottom() - footer_height), inner.max);
    ui.scope_builder(egui::UiBuilder::new().max_rect(footer), |ui| {
        let enabled = !plan.asins.is_empty() && !app.busy();
        let label = t.run_label(plan.needs_unlock, plan.asins.len());
        let icon = if plan.needs_unlock { Icon::Unlock } else { Icon::ArrowRight };
        let run = theme::icon_button_wide(ui, Kind::Primary, enabled, icon, &label, footer.width());
        app.mark(demo::Target::Run, run.rect);
        if run.clicked() {
            let ctx = ui.ctx().clone();
            app.start_run(&ctx);
        }
    });
}

fn separator(ui: &mut egui::Ui, p: theme::Palette) {
    let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(line, 0.0, p.line);
}

/// A soft note with an icon.
fn note(ui: &mut egui::Ui, fill: egui::Color32, icon: Icon, ink: egui::Color32, text: &str) {
    egui::Frame::new().fill(fill).corner_radius(theme::RADIUS).inner_margin(egui::Margin::symmetric(12, 10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_top(|ui| {
            icons::show(ui, icon, 16.0, ink);
            ui.add(egui::Label::new(egui::RichText::new(text).size(12.5).color(ink)).wrap());
        });
    });
}

/// The chosen books by name, each with what will be done to it.
fn chosen_list(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    if app.selected.is_empty() {
        ui.label(theme::muted(p, t.pick_hint).size(13.0));
        return;
    }
    let width = ui.available_width();
    for asin in app.selected.iter().take(ASIDE_LIST) {
        let Some(book) = app.books.iter().find(|b| &b.asin == asin) else { continue };
        let what = match book.state {
            BookState::Locked => t.plan_unlock,
            BookState::Unlocked => t.plan_convert,
            BookState::Converted => t.plan_again,
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::hover());
        let painter = ui.painter();
        let dot = Rect::from_center_size(Pos2::new(rect.left() + 5.0, rect.center().y), egui::vec2(10.0, 10.0));
        painter.rect_filled(dot, 3, icons::state_fill(book.state));
        let what = painter.layout_no_wrap(what.to_string(), theme::regular_font(12.0), p.muted);
        let what_left = rect.right() - what.size().x;
        let name = if book.state == BookState::Locked { book.asin.clone() } else { AnzuApp::display_title(book, t) };
        let name = elided(painter, &name, theme::regular_font(13.0), p.fg, what_left - 10.0 - (rect.left() + 18.0));
        painter.galley(Pos2::new(rect.left() + 18.0, rect.center().y - name.size().y / 2.0), name, p.fg);
        painter.galley(Pos2::new(what_left, rect.center().y - what.size().y / 2.0), what, p.muted);
    }
    if app.selected.len() > ASIDE_LIST {
        ui.label(theme::muted(p, t.and_more(app.selected.len() - ASIDE_LIST)).size(12.0));
    }
}

fn destination(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.label(theme::muted(p, t.save_in).size(12.0));
    ui.horizontal_top(|ui| {
        icons::show(ui, Icon::Folder, 18.0, p.fg);
        let folder = theme::bold(app.settings.output_dir.display().to_string()).size(13.0);
        theme::copyable(ui, |ui| ui.add(egui::Label::new(folder).wrap()));
    });
    if theme::icon_button(ui, Kind::Secondary, !app.busy(), Icon::Folder, t.change_folder).clicked() {
        app.pick_output_dir();
    }
}

/// Which of the chosen books already have an EPUB, and a way to leave them out.
fn reconvert_notice(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let again = chosen_converted(app);
    if again.is_empty() || app.busy() {
        return;
    }
    let names: Vec<String> = again.iter().map(|b| AnzuApp::display_title(b, t)).collect();
    let asins: Vec<String> = again.iter().map(|b| b.asin.clone()).collect();
    let all = asins.len() == app.selected.len();
    let (title, body) = t.already_epub(&names, app.selected.len());
    let leave_label = match (all, asins.len()) {
        (true, _) => t.clear_selection,
        (false, 1) => t.leave_out_one,
        (false, _) => t.leave_out_many,
    };
    let mut leave = false;
    egui::Frame::new()
        .fill(p.epub_tint)
        .corner_radius(theme::RADIUS)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                icons::show(ui, Icon::File, 16.0, p.fg);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.add(egui::Label::new(theme::bold(title).size(12.5)).wrap());
                    ui.add(egui::Label::new(egui::RichText::new(body).size(12.0).color(p.muted)).wrap());
                });
            });
            if theme::icon_button(ui, Kind::Quiet, true, Icon::Close, leave_label).clicked() {
                leave = true;
            }
        });
    if leave {
        app.selected.retain(|a| !asins.contains(a));
    }
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
    fn size_and_asin_sort_by_their_own_order() {
        let mut rows = sample();
        sort_rows(&mut rows, Sort { column: Column::Size, ascending: true });
        assert_eq!(order(&rows), vec!["B2", "B1", "B3"]);
        sort_rows(&mut rows, Sort { column: Column::Asin, ascending: false });
        assert_eq!(order(&rows), vec!["B3", "B2", "B1"]);
    }

    #[test]
    fn equal_keys_keep_a_stable_order_by_asin() {
        let mut rows = vec![row("B2", "Same", 1, BookState::Locked), row("B1", "same", 1, BookState::Locked)];
        sort_rows(&mut rows, Sort { column: Column::Title, ascending: true });
        assert_eq!(order(&rows), vec!["B1", "B2"]);
    }

    #[test]
    fn the_shelves_fit_as_many_cards_as_the_width_allows() {
        assert_eq!(columns(200.0), 1, "never fewer than one");
        assert_eq!(columns(520.0), 2);
        assert_eq!(columns(900.0), 3);
        assert_eq!(columns(3000.0), 3, "never more than three");
    }

    #[test]
    fn a_cover_shows_up_to_two_initials() {
        assert_eq!(initials("Moby Dick"), "MD");
        assert_eq!(initials("Frankenstein"), "Fr");
        assert_eq!(initials("La isla del tesoro"), "LT");
        assert_eq!(initials("¿Quién?"), "Qu");
        assert_eq!(initials(""), "");
    }
}

//! Progress screen: one column with the steps of the whole run, which says what Anzu is
//! doing now and what comes next. Under the conversion step every book is a tile in the
//! pastel of its state; the last step saves them and says where, with one way to reach them.

use std::path::{Path, PathBuf};

use anzu_core::library::BookState;
use eframe::egui::{self, Align, Layout, Pos2, Rect, RichText, Sense, Stroke};

use super::icons::{self, Icon};
use super::theme::{self, Kind};
use super::{AnzuApp, Screen, StepState, demo, open_file, show_in_folder};
use crate::i18n::Strings;
use crate::pipeline::{self, Stage, Step};

/// The preparation, done once for all the books.
const STEPS: [Step; 3] = [Step::Check, Step::Download, Step::CloseKindle];
/// The widest the column of the run gets.
const COLUMN_WIDTH: f32 = 700.0;
/// The circle of a step, and where its text starts.
const DOT: f32 = 30.0;
const TEXT_LEFT: f32 = DOT + 16.0;
/// How long a step takes to turn from running to done, in seconds.
const STEP_DONE_TIME: f32 = 0.28;
/// How long a tile takes to turn into an EPUB, in seconds.
const TILE_DONE_TIME: f32 = 0.3;

/// How a step is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Look {
    Waiting,
    Running,
    Done,
    Skipped,
    Failed,
}

fn look(app: &AnzuApp, step: Step) -> Look {
    let finished = app.run.result.is_some();
    match (app.run.steps.iter().find(|(s, _)| *s == step).map(|(_, state)| *state), finished) {
        (Some(StepState::Done), _) => Look::Done,
        (Some(StepState::Skipped), _) => Look::Skipped,
        (Some(StepState::Running), false) => Look::Running,
        (Some(StepState::Running), true) => Look::Failed,
        // The run ended before reaching it.
        (None, true) => Look::Skipped,
        (None, false) => Look::Waiting,
    }
}

/// Where one book stands in the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Queued,
    Unlocking,
    /// Unlocked, waiting for its turn to convert.
    Unlocked,
    Converting,
    Converted,
    Failed(Stage),
    /// The run ended before reaching it.
    NotStarted,
}

fn status(state: BookState, active: Option<Stage>, error: Option<Stage>, finished: bool) -> Status {
    match (error, active, state) {
        (Some(stage), _, _) => Status::Failed(stage),
        (None, Some(Stage::Unlock), _) => Status::Unlocking,
        (None, Some(Stage::Convert), _) => Status::Converting,
        (None, None, BookState::Converted) => Status::Converted,
        (None, None, BookState::Unlocked) => Status::Unlocked,
        (None, None, BookState::Locked) if finished => Status::NotStarted,
        (None, None, BookState::Locked) => Status::Queued,
    }
}

/// The state a book starts a run in. A book converted before is converted again from its
/// unlocked file, so it starts as unlocked and not as already done.
pub(super) fn starting_state(state: BookState) -> BookState {
    match state {
        BookState::Converted => BookState::Unlocked,
        other => other,
    }
}

/// "Book n of total": the book being converted, or the next one to finish.
#[cfg(test)]
fn book_number(statuses: &[Status]) -> usize {
    let total = statuses.len();
    let n = match statuses.iter().position(|s| *s == Status::Converting) {
        Some(index) => index + 1,
        None => statuses.iter().filter(|s| matches!(s, Status::Converted | Status::Failed(_))).count() + 1,
    };
    n.clamp(1, total.max(1))
}

fn status_of(app: &AnzuApp, asin: &str) -> Status {
    let state = app.run.states.get(asin).copied().unwrap_or(BookState::Locked);
    let active = app.run.active.get(asin).copied();
    let error = app.run.errors.get(asin).map(|(stage, _)| *stage);
    status(state, active, error, app.run.result.is_some())
}

fn title_of(app: &AnzuApp, t: &Strings, asin: &str) -> String {
    app.books.iter().find(|b| b.asin == asin).map_or_else(|| t.book_fallback(asin), |b| AnzuApp::display_title(b, t))
}

/// What a step holds below its title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Preparation,
    /// The tiles of the books.
    Convert,
    /// Where the EPUB files went, and the ways to reach them.
    Save,
}

/// One line of the list of steps.
struct Line {
    title: String,
    detail: String,
    look: Look,
    part: Part,
    /// "7 / 12", on the right of the title.
    count: Option<String>,
}

/// How far a stage of the books has got: how many are past it, how many failed in it, and
/// whether one is in it now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tally {
    total: usize,
    done: usize,
    failed: usize,
    running: bool,
}

/// The look of a stage of the books from its tally.
fn stage_look(tally: Tally, finished: bool, preparation_failed: bool) -> Look {
    if tally.running {
        Look::Running
    } else if tally.total > 0 && tally.done + tally.failed >= tally.total {
        if tally.done == 0 { Look::Failed } else { Look::Done }
    } else if finished || preparation_failed {
        if tally.done > 0 { Look::Done } else { Look::Skipped }
    } else {
        Look::Waiting
    }
}

/// The look of the last step: it runs from the moment every book is converted until the
/// run says it has finished, and is done when something was saved.
fn save_look(convert: Tally, finished: bool, saved: usize) -> Look {
    if finished {
        if saved > 0 { Look::Done } else { Look::Skipped }
    } else if convert.total > 0 && convert.done + convert.failed >= convert.total && !convert.running {
        Look::Running
    } else {
        Look::Waiting
    }
}

fn unlock_tally(app: &AnzuApp) -> Tally {
    let asins = &app.run.unlock_asins;
    Tally {
        total: asins.len(),
        done: asins.iter().filter(|a| app.run.states.get(*a).is_some_and(|s| *s != BookState::Locked)).count(),
        failed: asins.iter().filter(|a| matches!(app.run.errors.get(*a), Some((Stage::Unlock, _)))).count(),
        running: app.run.active.values().any(|s| *s == Stage::Unlock),
    }
}

fn convert_tally(app: &AnzuApp) -> Tally {
    let asins = &app.run.asins;
    Tally {
        total: asins.len(),
        done: asins.iter().filter(|a| app.run.states.get(*a) == Some(&BookState::Converted)).count(),
        failed: asins.iter().filter(|a| app.run.errors.contains_key(*a)).count(),
        running: app.run.active.values().any(|s| *s == Stage::Convert),
    }
}

/// How many books are converted at the same time. The web demo converts one at a time.
fn workers(app: &AnzuApp) -> usize {
    if app.demo.is_some() { 1 } else { pipeline::convert_workers(app.run.asins.len()) }
}

/// The folder the EPUB files go to.
fn out_dir(app: &AnzuApp) -> PathBuf {
    app.run
        .saved
        .first()
        .and_then(|(_, path)| path.parent())
        .map_or_else(|| app.settings.output_dir.clone(), Path::to_path_buf)
}

/// Every step of the run in order: the preparation, unlocking, converting and saving.
fn lines(app: &AnzuApp, t: &Strings) -> Vec<Line> {
    let mut lines = Vec::new();
    if app.run.prepares {
        for step in STEPS {
            let (title, detail) = t.step(step);
            lines.push(Line {
                title: title.to_string(),
                detail: detail.to_string(),
                look: look(app, step),
                part: Part::Preparation,
                count: None,
            });
        }
    }
    let finished = app.run.result.is_some();
    let stopped = app.run.preparation_failed.is_some();
    let unlock = unlock_tally(app);
    if unlock.total > 0 {
        let look = stage_look(unlock, finished, stopped);
        let detail =
            if look == Look::Waiting { t.stage(Stage::Unlock).1.to_string() } else { t.done_of(unlock.done, unlock.total) };
        lines.push(Line { title: t.unlock_step(unlock.total), detail, look, part: Part::Preparation, count: None });
    }
    let convert = convert_tally(app);
    let look = stage_look(convert, finished, stopped);
    let detail = match look {
        Look::Waiting => t.stage(Stage::Convert).1.to_string(),
        Look::Running => t.at_a_time(workers(app)),
        _ => t.done_of(convert.done, convert.total),
    };
    let count = (look != Look::Waiting).then(|| format!("{} / {}", convert.done + convert.failed, convert.total));
    lines.push(Line { title: t.convert_step(convert.total), detail, look, part: Part::Convert, count });
    let saved = app.run.saved.len();
    let look = save_look(convert, finished, saved);
    let title = if look == Look::Done { t.epubs_ready(saved) } else { t.save_step.to_string() };
    lines.push(Line { title, detail: t.in_folder(&out_dir(app)), look, part: Part::Save, count: None });
    lines
}

/// "Step k of n": the first step not finished yet.
fn step_number(lines: &[Line]) -> usize {
    lines.iter().position(|l| matches!(l.look, Look::Running | Look::Waiting)).map_or(lines.len(), |i| i + 1)
}

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui) {
    let p = theme::Palette::of(ui.ctx());
    let t = app.t();
    egui::Panel::top("progress-header").frame(theme::band(p.shell, 10)).show_separator_line(false).show(ui, |ui| {
        super::enter(ui, Screen::Progress, false, |ui| {
            theme::centered(ui, |ui| {
                ui.horizontal(|ui| {
                    let home = !app.busy();
                    if home_link(ui, p, t, home) {
                        app.back_to_library();
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| band_status(app, ui, p, t));
                });
            });
        });
    });
    egui::CentralPanel::default().frame(theme::band(p.bg, 18)).show(ui, |ui| {
        super::enter(ui, Screen::Progress, true, |ui| {
            theme::centered(ui, |ui| {
                egui::ScrollArea::vertical().id_salt("run").auto_shrink([false, false]).show(ui, |ui| {
                    let width = ui.available_width().min(COLUMN_WIDTH);
                    let margin = ((ui.available_width() - width) / 2.0).max(0.0);
                    ui.horizontal_top(|ui| {
                        ui.add_space(margin);
                        ui.vertical(|ui| {
                            ui.set_width(width);
                            run_card(app, ui, p, t);
                            log(app, ui, p, t);
                        });
                    });
                });
            })
        });
    });
    if app.busy() {
        // The running step turns and the tiles change: keep drawing while the run goes on.
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// What the header says on the right: working, or finished.
fn band_status(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    // Right to left: the text first, then its mark.
    if app.run.result.is_some() {
        ui.label(RichText::new(t.finished_band).color(p.shell_muted).size(13.0));
        icons::show(ui, Icon::Check, 16.0, theme::EPUB);
    } else {
        ui.label(RichText::new(t.working).color(p.shell_muted).size(13.0));
        ui.add(egui::Spinner::new().size(14.0).color(theme::BUTTER));
    }
}

/// The logo and the name, which lead back to the books once the run has finished.
fn home_link(ui: &mut egui::Ui, p: theme::Palette, t: &Strings, enabled: bool) -> bool {
    let inner = ui.horizontal(|ui| {
        if enabled {
            icons::show(ui, Icon::ArrowLeft, 18.0, p.on_shell);
        }
        AnzuApp::brand(ui, p, t)
    });
    let rect = inner.response.rect.expand2(egui::vec2(6.0, 3.0));
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let response = ui.interact(rect, ui.scope_id().with("home"), sense);
    if !enabled {
        theme::pointer(&response, false);
        response.on_hover_text(t.wait_hint);
        return false;
    }
    theme::feedback(ui, theme::Surface::Shell, &response);
    response.on_hover_text(t.back).clicked()
}

/// The card of the run: the heading, every step with what it holds, and the way back.
fn run_card(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    theme::panel(p).inner_margin(egui::Margin::symmetric(28, 24)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let lines = lines(app, t);
        let kicker = match &app.run.result {
            Some((converted, failed)) => t.finished(*converted, failed.len()),
            None => t.step_of(step_number(&lines), lines.len()),
        };
        ui.label(theme::bold(kicker).size(12.5).color(p.muted));
        let title = match app.run.asins.as_slice() {
            [asin] => title_of(app, t, asin),
            asins => t.books(asins.len()),
        };
        ui.add(egui::Label::new(theme::display(title).size(26.0)).wrap());
        ui.add_space(14.0);
        let time = ui.input(|i| i.time) as f32;
        let count = lines.len();
        for (i, line) in lines.iter().enumerate() {
            step(ui, p, t, i + 1, line, i + 1 < count, time, |ui| match line.part {
                Part::Preparation => {}
                Part::Convert => tiles(app, ui, p, t, time),
                Part::Save => saved(app, ui, p, t),
            });
        }
        if let Some(reason) = &app.run.preparation_failed {
            ui.add_space(8.0);
            theme::copyable(ui, |ui| note(ui, p.warn_bg, p.warn_line, Icon::Alert, p.warn, reason));
        }
        if app.run.result.is_none() {
            if app.run.asins.len() > 1 {
                ui.add_space(10.0);
                ui.horizontal_top(|ui| {
                    icons::show(ui, Icon::Info, 16.0, p.muted);
                    ui.add(egui::Label::new(theme::muted(p, t.failure_note).size(12.5)).wrap());
                });
            }
            return;
        }
        ui.add_space(16.0);
        let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(line, 0.0, p.line);
        ui.add_space(12.0);
        if !app.run.errors.is_empty() {
            note(ui, p.warn_bg, p.warn_line, Icon::Alert, p.fg, t.failed_stays);
            ui.add_space(8.0);
        }
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            let kind = if app.run.saved.is_empty() { Kind::Primary } else { Kind::Secondary };
            let more = theme::icon_button(ui, kind, true, Icon::Book, t.convert_more);
            app.mark(demo::Target::ConvertMore, more.rect);
            if more.clicked() {
                app.back_to_library();
            }
        });
    });
}

/// A soft note with an icon, its text wrapped.
fn note(ui: &mut egui::Ui, fill: egui::Color32, line: egui::Color32, icon: Icon, ink: egui::Color32, text: &str) {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, line))
        .corner_radius(theme::RADIUS)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                icons::show(ui, icon, 16.0, ink);
                ui.add(egui::Label::new(RichText::new(text).color(ink).size(12.5)).wrap());
            });
        });
}

/// One step: its circle and title, what it holds below, and the line to the next one.
#[allow(clippy::too_many_arguments)]
fn step(
    ui: &mut egui::Ui,
    p: theme::Palette,
    t: &Strings,
    number: usize,
    line: &Line,
    more: bool,
    time: f32,
    body: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    let center = step_head(ui, p, t, number, line, width, time);
    ui.horizontal_top(|ui| {
        ui.add_space(TEXT_LEFT);
        ui.vertical(|ui| {
            ui.set_width((width - TEXT_LEFT).max(0.0));
            body(ui);
        });
    });
    if more {
        ui.add_space(18.0);
        let bottom = ui.cursor().min.y - 4.0;
        let color = if line.look == Look::Done { p.fg } else { p.line_strong };
        ui.painter().line_segment(
            [Pos2::new(center.x, center.y + DOT / 2.0 + 3.0), Pos2::new(center.x, bottom)],
            Stroke::new(2.0, color),
        );
    }
}

/// The circle, the title, the detail and the count of a step. Returns the circle's centre.
fn step_head(ui: &mut egui::Ui, p: theme::Palette, t: &Strings, number: usize, line: &Line, width: f32, time: f32) -> Pos2 {
    let title_font = match (line.part, line.look) {
        (Part::Save, Look::Done) => theme::display_font(18.0),
        (_, Look::Running) => theme::bold_font(14.5),
        _ => theme::regular_font(14.5),
    };
    let quiet = matches!(line.look, Look::Waiting | Look::Skipped);
    let title_color = if quiet { p.muted } else { p.fg };
    let detail = match line.look {
        Look::Skipped => format!("{} · {}", line.detail, t.step_skipped),
        Look::Failed => format!("{} · {}", line.detail, t.step_not_completed),
        _ => line.detail.clone(),
    };
    let detail_color = if line.look == Look::Failed { p.warn } else { p.muted };
    let count = line.count.as_ref().map(|c| ui.painter().layout_no_wrap(c.clone(), theme::display_font(20.0), p.fg));
    let count_width = count.as_ref().map_or(0.0, |g| g.size().x + 12.0);
    let title = ui.painter().layout(line.title.clone(), title_font, title_color, (width - TEXT_LEFT - count_width).max(40.0));
    let detail = ui.painter().layout(detail, theme::regular_font(12.5), detail_color, (width - TEXT_LEFT).max(40.0));
    let text_height = title.size().y + 2.0 + detail.size().y;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, text_height.max(DOT)), Sense::hover());
    let painter = ui.painter();
    let center = Pos2::new(rect.left() + DOT / 2.0, rect.top() + DOT / 2.0);
    let dot = Rect::from_center_size(center, egui::vec2(DOT, DOT));
    let number_text = |color| {
        painter.text(center, egui::Align2::CENTER_CENTER, number.to_string(), theme::bold_font(13.0), color);
    };
    // A step that finishes turns from butter to mint, and its tick fades in over the spinner.
    let done = ui.ctx().animate_bool_with_time(egui::Id::unique(("run-step", number)), line.look == Look::Done, STEP_DONE_TIME);
    match line.look {
        Look::Done => {
            painter.circle_filled(center, DOT / 2.0, theme::blend(theme::BUTTER, theme::EPUB, done));
            icons::paint(painter, Icon::Check, dot.shrink(8.0), theme::INK.gamma_multiply(done));
        }
        Look::Running => {
            // The step at work turns: a butter disc with an ink arc spinning inside it, where
            // the number or the check sits on the other steps.
            painter.circle_filled(center, DOT / 2.0, theme::BUTTER);
            spinner_arc(painter, center, DOT / 2.0 - 6.5, time, theme::INK);
        }
        Look::Failed => {
            painter.circle_filled(center, DOT / 2.0, theme::DRM);
            icons::paint(painter, Icon::Close, dot.shrink(9.0), theme::INK);
        }
        Look::Skipped => {
            painter.circle_filled(center, DOT / 2.0, p.quiet);
            number_text(p.muted);
        }
        Look::Waiting => {
            painter.circle_stroke(center, DOT / 2.0 - 1.0, Stroke::new(1.5, p.line_strong));
            number_text(p.muted);
        }
    }
    let top = rect.top() + (DOT - title.size().y).max(0.0) / 2.0 - 2.0;
    let title_height = title.size().y;
    painter.galley(Pos2::new(rect.left() + TEXT_LEFT, top), title, title_color);
    painter.galley(Pos2::new(rect.left() + TEXT_LEFT, top + title_height + 2.0), detail, detail_color);
    if let Some(count) = count {
        let at = Pos2::new(rect.right() - count.size().x, rect.top() + (DOT - count.size().y) / 2.0);
        painter.galley(at, count, p.fg);
    }
    center
}

/// A three-quarter arc that turns once a second around `center`.
fn spinner_arc(painter: &egui::Painter, center: Pos2, radius: f32, time: f32, color: egui::Color32) {
    let start = time * std::f32::consts::TAU;
    let points: Vec<Pos2> = (0..=24)
        .map(|i| {
            let a = start + i as f32 / 24.0 * std::f32::consts::TAU * 0.72;
            center + egui::vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    painter.add(egui::Shape::line(points, Stroke::new(2.4, color)));
}

/// The size of the tiles of a run of `books`, and the gap between them: large for a few
/// books, small enough that hundreds still fit in a few rows.
fn tile_size(books: usize) -> (f32, f32, f32) {
    match books {
        0..=2 => (40.0, 54.0, 8.0),
        3..=12 => (30.0, 40.0, 6.0),
        13..=40 => (20.0, 27.0, 5.0),
        _ => (14.0, 19.0, 4.0),
    }
}

/// Every book of the run as a tile, and the books that failed with their reason.
fn tiles(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings, time: f32) {
    let asins = &app.run.asins;
    if asins.is_empty() {
        return;
    }
    let (w, h, gap) = tile_size(asins.len());
    ui.add_space(10.0);
    let width = ui.available_width();
    let columns = (((width + gap) / (w + gap)).floor() as usize).max(1);
    let rows = asins.len().div_ceil(columns);
    let (area, _) = ui.allocate_exact_size(egui::vec2(width, rows as f32 * (h + gap) - gap), Sense::hover());
    for (i, asin) in asins.iter().enumerate() {
        let at = area.min + egui::vec2((i % columns) as f32 * (w + gap), (i / columns) as f32 * (h + gap));
        tile(app, ui, p, t, asin, Rect::from_min_size(at, egui::vec2(w, h)), time);
    }
    // The reason of a stopped preparation is said once, below the steps.
    let shared = app.run.preparation_failed.as_deref();
    let failed: Vec<String> = asins
        .iter()
        .filter_map(|asin| {
            let (_, reason) = app.run.errors.get(asin)?;
            // Compared whole, then shortened: a reason of several lines is still the same one.
            if Some(reason.as_str()) == shared {
                return None;
            }
            let first = reason.lines().next().unwrap_or(reason);
            Some(format!("{} · {first}", title_of(app, t, asin)))
        })
        .collect();
    if !failed.is_empty() {
        ui.add_space(10.0);
        theme::copyable(ui, |ui| note(ui, p.warn_bg, p.warn_line, Icon::Alert, p.fg, &failed.join("\n")));
    }
}

/// One book: a quiet tile while it waits, butter while it is worked on, and a short swell
/// into mint when it turns into an EPUB.
fn tile(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings, asin: &str, rect: Rect, time: f32) {
    let state = app.run.states.get(asin).copied().unwrap_or(BookState::Locked);
    let status = status_of(app, asin);
    let id = egui::Id::unique(("run-tile", asin));
    let done = ui.ctx().animate_bool_with_time(id, status == Status::Converted, TILE_DONE_TIME);
    let swell = 1.0 + 0.14 * (done * std::f32::consts::PI).sin();
    let shown = Rect::from_center_size(rect.center(), rect.size() * swell);
    let radius = egui::CornerRadius::same((rect.width() * 0.18).round() as u8);
    let painter = ui.painter();
    match status {
        Status::Unlocking | Status::Converting => {
            painter.rect_filled(shown, radius, theme::BUTTER);
        }
        Status::Converted => {
            painter.rect_filled(shown, radius, theme::blend(theme::BUTTER, theme::EPUB, done));
        }
        Status::Failed(_) => {
            painter.rect_filled(shown, radius, icons::state_fill(state));
            painter.rect_stroke(shown, radius, Stroke::new(2.0, p.warn), egui::StrokeKind::Inside);
        }
        Status::Queued | Status::Unlocked | Status::NotStarted => {
            painter.rect_filled(shown, radius, p.quiet);
            painter.rect_stroke(shown, radius, Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
        }
    }
    match status {
        Status::Unlocking | Status::Converting if rect.width() >= 20.0 => {
            spinner_arc(painter, shown.center(), rect.width() * 0.22, time, theme::INK);
        }
        Status::Converted if rect.width() >= 30.0 => {
            icons::paint(painter, Icon::Check, shown.shrink(rect.width() * 0.26), theme::INK.gamma_multiply(done));
        }
        _ => {}
    }
    ui.interact(rect, id.with("hover"), Sense::hover()).on_hover_text(title_of(app, t, asin));
}

/// Once the run has finished: what was saved, with one way to the folder, and the books.
fn saved(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    if app.run.result.is_none() || app.run.saved.is_empty() {
        return;
    }
    let saved = app.run.saved.clone();
    ui.add_space(10.0);
    if app.run.renamed > 0 {
        ui.add(egui::Label::new(theme::muted(p, t.copies_note(app.run.renamed)).size(12.5)).wrap());
        ui.add_space(8.0);
    }
    ui.horizontal_wrapped(|ui| {
        if theme::icon_button(ui, Kind::Primary, true, Icon::Folder, t.show_in_folder).clicked() {
            show_in_folder(&saved[0].1);
        }
        if let [(_, path)] = saved.as_slice() {
            if theme::icon_button(ui, Kind::Secondary, true, Icon::External, t.open_epub).clicked() {
                open_file(path);
            }
        } else {
            let label = if app.run.show_books { t.hide_books.to_string() } else { t.see_books(app.run.asins.len()) };
            if theme::icon_button(ui, Kind::Quiet, true, Icon::Book, &label).clicked() {
                app.run.show_books = !app.run.show_books;
            }
        }
    });
    if app.run.show_books && saved.len() > 1 {
        book_list(app, ui, p, t, &saved);
    }
}

/// Every book of a finished run, each with its EPUB or what went wrong.
fn book_list(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings, saved: &[(String, PathBuf)]) {
    ui.add_space(8.0);
    for asin in &app.run.asins {
        let (rule, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(rule, 0.0, p.line);
        ui.horizontal(|ui| {
            let state = app.run.states.get(asin).copied().unwrap_or(BookState::Locked);
            let failed = app.run.errors.contains_key(asin);
            let (cover, _) = ui.allocate_exact_size(egui::vec2(16.0, 22.0), Sense::hover());
            ui.painter().rect_filled(cover, egui::CornerRadius::same(3), icons::state_fill(state));
            // The three-stage track of the book: with DRM, unlocked, EPUB.
            let (track, _) = ui.allocate_exact_size(egui::vec2(icons::TRACK_WIDTH, 16.0), Sense::hover());
            icons::track(ui.painter(), track.left_center(), state, !failed, p);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Right to left: the action first, then the title takes what is left.
                match (saved.iter().find(|(a, _)| a == asin), app.run.errors.get(asin)) {
                    (Some((_, path)), _) => {
                        if theme::icon_button(ui, Kind::Quiet, true, Icon::External, t.open_epub).clicked() {
                            open_file(path);
                        }
                    }
                    (None, Some((stage, _))) => {
                        let text = if *stage == Stage::Unlock { t.failed_unlock } else { t.failed_convert };
                        ui.label(RichText::new(text).color(p.warn).size(12.0));
                    }
                    (None, None) => {
                        ui.label(theme::muted(p, t.not_started).size(12.0));
                    }
                }
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    ui.add(egui::Label::new(RichText::new(title_of(app, t, asin)).size(13.0)).truncate());
                });
            });
        });
    }
}

fn log(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    if app.run.log.is_empty() {
        return;
    }
    ui.add_space(8.0);
    let label = t.log_toggle(app.run.show_log, app.run.log.len());
    if theme::icon_button(ui, Kind::Quiet, true, Icon::Terminal, &label).clicked() {
        app.run.show_log = !app.run.show_log;
    }
    if !app.run.show_log {
        return;
    }
    theme::panel(p).fill(p.log_bg).inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(theme::bold(t.log).color(p.muted).size(theme::SMALL));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, Kind::Secondary, true, Icon::Copy, t.copy).clicked() {
                    ui.ctx().copy_text(app.run.log.join("\n"));
                }
            });
        });
        egui::ScrollArea::vertical().id_salt("log").max_height(220.0).stick_to_bottom(true).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            theme::copyable(ui, |ui| {
                for line in &app.run.log {
                    ui.label(RichText::new(line.as_str()).monospace().size(12.0));
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_book_shows_the_stage_it_is_in() {
        assert_eq!(status(BookState::Locked, None, None, false), Status::Queued);
        assert_eq!(status(BookState::Locked, Some(Stage::Unlock), None, false), Status::Unlocking);
        assert_eq!(status(BookState::Unlocked, None, None, false), Status::Unlocked);
        assert_eq!(status(BookState::Unlocked, Some(Stage::Convert), None, false), Status::Converting);
        assert_eq!(status(BookState::Converted, None, None, true), Status::Converted);
    }

    #[test]
    fn a_failure_names_the_stage_where_it_happened() {
        assert_eq!(status(BookState::Locked, None, Some(Stage::Unlock), true), Status::Failed(Stage::Unlock));
        assert_eq!(status(BookState::Unlocked, None, Some(Stage::Convert), true), Status::Failed(Stage::Convert));
    }

    #[test]
    fn a_book_converted_before_waits_its_turn_again() {
        let state = starting_state(BookState::Converted);
        assert_eq!(status(state, None, None, false), Status::Unlocked, "not shown as done before its turn");
        assert_eq!(starting_state(BookState::Locked), BookState::Locked);
        // Three books converted before: the run starts at the first, not at "3 of 3".
        assert_eq!(book_number(&[Status::Unlocked; 3]), 1);
    }

    #[test]
    fn the_book_number_follows_the_book_being_converted() {
        use Status::*;
        assert_eq!(book_number(&[Converted, Converting, Unlocked]), 2);
        assert_eq!(book_number(&[Converted, Failed(Stage::Convert), Unlocked]), 3);
        assert_eq!(book_number(&[Converted, Converted]), 2, "never past the last one");
        assert_eq!(book_number(&[]), 1);
    }

    #[test]
    fn a_book_the_run_never_reached_is_not_started_rather_than_failed() {
        // The preparation stopped: its reason is shown once, not repeated on every book.
        assert_eq!(status(BookState::Locked, None, None, true), Status::NotStarted);
    }

    #[test]
    fn a_stage_of_the_books_is_done_when_every_book_is_past_it() {
        let tally = |done, failed, running| Tally { total: 2, done, failed, running };
        assert_eq!(stage_look(tally(0, 0, false), false, false), Look::Waiting);
        assert_eq!(stage_look(tally(1, 0, true), false, false), Look::Running);
        assert_eq!(stage_look(tally(1, 1, false), false, false), Look::Done, "one failure does not fail the stage");
        assert_eq!(stage_look(tally(0, 2, false), true, false), Look::Failed);
        assert_eq!(stage_look(tally(0, 0, false), true, true), Look::Skipped, "the preparation stopped first");
    }

    #[test]
    fn saving_runs_between_the_last_book_and_the_end_of_the_run() {
        let tally = |done, running| Tally { total: 3, done, failed: 0, running };
        assert_eq!(save_look(tally(2, true), false, 2), Look::Waiting);
        assert_eq!(save_look(tally(3, false), false, 3), Look::Running, "every book is done, the run not yet");
        assert_eq!(save_look(tally(3, false), true, 3), Look::Done);
        assert_eq!(save_look(Tally { total: 3, done: 0, failed: 3, running: false }, true, 0), Look::Skipped);
    }

    #[test]
    fn the_step_number_is_the_first_step_not_finished() {
        let line = |look| Line { title: String::new(), detail: String::new(), look, part: Part::Preparation, count: None };
        assert_eq!(step_number(&[line(Look::Done), line(Look::Running), line(Look::Waiting)]), 2);
        assert_eq!(step_number(&[line(Look::Done), line(Look::Skipped), line(Look::Waiting)]), 3);
        assert_eq!(step_number(&[line(Look::Done), line(Look::Done)]), 2, "all done: the last one");
    }

    #[test]
    fn hundreds_of_books_get_small_tiles() {
        assert!(tile_size(2).0 > tile_size(12).0);
        assert!(tile_size(200).0 < tile_size(40).0);
    }
}

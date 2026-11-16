//! Progress screen: on the left, the list of steps of the whole run, which says what Anzu is
//! doing now and what comes next; on the right, each book of the run with where it stands,
//! then the saved files and the log behind a link.

use anzu_core::library::BookState;
use eframe::egui::{self, Align, Align2, Layout, Pos2, Rect, RichText, Sense, Stroke};

use super::icons::{self, Icon};
use super::theme::{self, Kind};
use super::{AnzuApp, StepState, demo, open_file, show_in_folder};
use crate::i18n::Strings;
use crate::pipeline::{Stage, Step};

/// The preparation, done once for all the books.
const STEPS: [Step; 3] = [Step::Check, Step::Download, Step::CloseKindle];
const STEPS_WIDTH: f32 = 360.0;
const GAP: f32 = 20.0;

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

/// One line of the list of steps.
struct Line {
    title: String,
    detail: String,
    look: Look,
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

/// Every step of the run in order: the preparation, unlocking and converting.
fn lines(app: &AnzuApp, t: &Strings) -> Vec<Line> {
    let mut lines = Vec::new();
    if app.run.prepares {
        for step in STEPS {
            let (title, detail) = t.step(step);
            lines.push(Line { title: title.to_string(), detail: detail.to_string(), look: look(app, step) });
        }
    }
    let finished = app.run.result.is_some();
    let stopped = app.run.preparation_failed.is_some();
    let unlock = unlock_tally(app);
    if unlock.total > 0 {
        let look = stage_look(unlock, finished, stopped);
        let detail =
            if look == Look::Waiting { t.stage(Stage::Unlock).1.to_string() } else { t.done_of(unlock.done, unlock.total) };
        lines.push(Line { title: t.unlock_step(unlock.total), detail, look });
    }
    let convert = convert_tally(app);
    let look = stage_look(convert, finished, stopped);
    let detail = match look {
        Look::Waiting => t.stage(Stage::Convert).1.to_string(),
        _ => t.done_of(convert.done, convert.total),
    };
    lines.push(Line { title: t.convert_step(convert.total), detail, look });
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
    egui::CentralPanel::default().frame(theme::band(p.bg, 18)).show(ui, |ui| {
        theme::centered(ui, |ui| {
            let area = ui.available_rect_before_wrap();
            let steps_width = STEPS_WIDTH.min(area.width() * 0.42);
            let left = Rect::from_min_max(area.min, Pos2::new(area.left() + steps_width, area.bottom()));
            let right = Rect::from_min_max(Pos2::new(left.right() + GAP, area.top()), area.max);
            ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| steps_panel(app, ui, p, t));
            ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
                egui::ScrollArea::vertical().id_salt("run").auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    books(app, ui, p, t);
                    if app.run.result.is_some() {
                        result(app, ui, p, t);
                    }
                    log(app, ui, p, t);
                });
            });
            ui.allocate_rect(area, Sense::hover());
        });
    });
    if app.busy() {
        // The running step pulses and the bars move: keep drawing while the run goes on.
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
        response.on_hover_text(t.wait_hint);
        return false;
    }
    theme::feedback(ui, theme::Surface::Shell, &response);
    response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(t.back).clicked()
}

/// The list of steps of the whole run, with the heading on top.
fn steps_panel(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, theme::CARD_RADIUS, p.panel);
    ui.painter().rect_stroke(rect, theme::CARD_RADIUS, Stroke::new(1.0, p.line), egui::StrokeKind::Inside);
    let inner = rect.shrink(22.0);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        egui::ScrollArea::vertical().id_salt("steps").auto_shrink([false, false]).show(ui, |ui| {
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
            ui.add(egui::Label::new(theme::display(title).size(24.0)).wrap());
            ui.add_space(12.0);
            let time = ui.input(|i| i.time) as f32;
            for (i, line) in lines.iter().enumerate() {
                step_line(ui, p, t, i + 1, line, i + 1 < lines.len(), time);
            }
            if let Some(reason) = &app.run.preparation_failed {
                ui.add_space(8.0);
                note(ui, p.warn_bg, p.warn_line, Icon::Alert, p.warn, reason);
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

/// One step: its circle, the line to the next one, its title and its detail.
fn step_line(ui: &mut egui::Ui, p: theme::Palette, t: &Strings, number: usize, line: &Line, more: bool, time: f32) {
    const DOT: f32 = 30.0;
    let width = ui.available_width();
    let text_left = DOT + 14.0;
    let title_font = if line.look == Look::Running { theme::bold_font(14.5) } else { theme::regular_font(14.5) };
    let quiet = matches!(line.look, Look::Waiting | Look::Skipped);
    let title_color = if quiet { p.muted } else { p.fg };
    let detail = match line.look {
        Look::Skipped => format!("{} · {}", line.detail, t.step_skipped),
        Look::Failed => format!("{} · {}", line.detail, t.step_not_completed),
        _ => line.detail.clone(),
    };
    let detail_color = if line.look == Look::Failed { p.warn } else { p.muted };
    let title = ui.painter().layout(line.title.clone(), title_font, title_color, width - text_left);
    let detail = ui.painter().layout(detail, theme::regular_font(12.5), detail_color, width - text_left);
    let text_height = title.size().y + 2.0 + detail.size().y;
    let height = text_height.max(DOT) + if more { 22.0 } else { 4.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter();
    let center = Pos2::new(rect.left() + DOT / 2.0, rect.top() + DOT / 2.0);
    if more {
        let color = if line.look == Look::Done { p.fg } else { p.line_strong };
        painter.line_segment(
            [Pos2::new(center.x, center.y + DOT / 2.0 + 3.0), Pos2::new(center.x, rect.bottom() - 2.0)],
            Stroke::new(2.0, color),
        );
    }
    let dot = Rect::from_center_size(center, egui::vec2(DOT, DOT));
    let number_text = |color| {
        painter.text(center, Align2::CENTER_CENTER, number.to_string(), theme::bold_font(13.0), color);
    };
    match line.look {
        Look::Done => {
            painter.circle_filled(center, DOT / 2.0, theme::EPUB);
            icons::paint(painter, Icon::Check, dot.shrink(8.0), theme::INK);
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
    painter.galley(Pos2::new(rect.left() + text_left, top), title, title_color);
    painter.galley(Pos2::new(rect.left() + text_left, top + title_height + 2.0), detail, detail_color);
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

/// Every book of the run with where it stands. The list is not clipped: the column scrolls.
fn books(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.horizontal(|ui| {
        ui.label(theme::display(t.run_books).size(18.0));
        ui.label(theme::muted(p, app.run.asins.len().to_string()).size(15.0));
    });
    ui.add_space(2.0);
    let time = ui.input(|i| i.time) as f32;
    for asin in &app.run.asins {
        book_card(app, ui, p, t, asin, time);
    }
    if app.run.result.is_none() && app.run.asins.len() > 1 {
        ui.horizontal_top(|ui| {
            icons::show(ui, Icon::Info, 16.0, p.muted);
            ui.add(egui::Label::new(theme::muted(p, t.failure_note).size(12.5)).wrap());
        });
    }
}

fn book_card(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings, asin: &str, time: f32) {
    let state = app.run.states.get(asin).copied().unwrap_or(BookState::Locked);
    let status = status_of(app, asin);
    let running = matches!(status, Status::Unlocking | Status::Converting);
    let failed = matches!(status, Status::Failed(_));
    let (text, pill_fill, pill_ink) = match status {
        Status::Queued => (t.queued, p.quiet, p.muted),
        Status::Unlocking => (t.unlocking, p.accent, p.on_accent),
        Status::Unlocked => (t.state_unlocked, theme::UNLOCKED, theme::INK),
        Status::Converting => (t.converting, p.accent, p.on_accent),
        Status::Converted => (t.state_converted, theme::EPUB, theme::INK),
        Status::Failed(Stage::Unlock) => (t.failed_unlock, p.panel, p.warn),
        Status::Failed(Stage::Convert) => (t.failed_convert, p.panel, p.warn),
        Status::NotStarted => (t.not_started, p.quiet, p.muted),
    };
    let (fill, stroke) = if running {
        (p.select_bg, Stroke::new(2.0, p.select_line))
    } else if failed {
        (p.warn_bg, Stroke::new(1.0, p.warn_line))
    } else if status == Status::Converted {
        (p.epub_tint, Stroke::new(1.0, p.line))
    } else {
        (p.panel, Stroke::new(1.0, p.line))
    };
    let detail = match app.run.errors.get(asin) {
        Some((_, reason)) => reason.lines().next().unwrap_or(reason).to_string(),
        None => match app.books.iter().find(|b| b.asin == asin) {
            Some(b) if b.title.is_some() => format!("{} · {}", b.asin, t.human_size(b.size)),
            Some(b) => t.human_size(b.size),
            None => asin.to_string(),
        },
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(theme::CARD_RADIUS)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (cover, _) = ui.allocate_exact_size(egui::vec2(34.0, 46.0), Sense::hover());
                let painter = ui.painter();
                painter.rect_filled(cover, egui::CornerRadius::same(6), icons::state_fill(state));
                icons::paint(painter, icons::state_icon(state), cover.shrink2(egui::vec2(8.0, 14.0)), theme::INK);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Right to left: the pill first, then the title takes what is left.
                    pill(ui, text, pill_fill, pill_ink, running);
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.add(egui::Label::new(theme::bold(title_of(app, t, asin)).size(14.5)).truncate());
                        ui.horizontal(|ui| {
                            // The three-stage track of the book: with DRM, unlocked, EPUB.
                            let (track, _) = ui.allocate_exact_size(egui::vec2(icons::TRACK_WIDTH, 16.0), Sense::hover());
                            icons::track(ui.painter(), track.left_center(), state, !failed, p);
                            let color = if failed { p.warn } else { p.muted };
                            ui.add(egui::Label::new(RichText::new(detail).size(12.0).color(color)).truncate());
                        });
                    });
                });
            });
            if running {
                ui.add_space(4.0);
                moving_bar(ui, p, time);
            }
        });
}

/// A small rounded label with the status of a book.
fn pill(ui: &mut egui::Ui, text: &str, fill: egui::Color32, ink: egui::Color32, running: bool) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), theme::bold_font(12.0), ink);
    let spinner = if running { 18.0 } else { 0.0 };
    let (rect, _) = ui.allocate_exact_size(egui::vec2(galley.size().x + 20.0 + spinner, 26.0), Sense::hover());
    ui.painter().rect_filled(rect, 13, fill);
    if running {
        let at = Rect::from_center_size(Pos2::new(rect.left() + 17.0, rect.center().y), egui::vec2(12.0, 12.0));
        ui.put(at, egui::Spinner::new().size(12.0).color(ink));
    }
    ui.painter().galley(Pos2::new(rect.left() + 10.0 + spinner, rect.center().y - galley.size().y / 2.0), galley, ink);
}

/// A bar that moves while a book is worked on. The tool and the converter give no
/// percentage, so it shows that something is happening, not how much is left.
fn moving_bar(ui: &mut egui::Ui, p: theme::Palette, time: f32) {
    let (bar, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 6.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(bar, 3, p.progress_track);
    let span = bar.width() * 0.3;
    let phase = (time * 0.6).fract();
    let left = bar.left() - span + (bar.width() + span) * phase;
    let piece = Rect::from_min_max(
        Pos2::new(left.max(bar.left()), bar.top()),
        Pos2::new((left + span).min(bar.right()), bar.bottom()),
    );
    if piece.width() > 0.0 {
        painter.rect_filled(piece, 3, p.progress);
    }
}

/// The saved files, each with a way to reach it, and the way back to the books.
fn result(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let saved = app.run.saved.clone();
    ui.add_space(8.0);
    if !saved.is_empty() {
        theme::panel(p).inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                icons::tile(ui, Icon::Check, 28.0, theme::EPUB, theme::INK);
                ui.label(theme::display(t.epubs_ready(saved.len())).size(18.0));
            });
            ui.add_space(6.0);
            let single = saved.len() == 1;
            for (index, (_, path)) in saved.iter().enumerate() {
                if index > 0 {
                    ui.add_space(4.0);
                    let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().rect_filled(line, 0.0, p.line);
                    ui.add_space(4.0);
                }
                ui.horizontal_top(|ui| {
                    icons::tile(ui, Icon::File, 34.0, p.epub_tint, p.fg);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        let name =
                            path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().to_string());
                        ui.add(egui::Label::new(theme::bold(name)).truncate());
                        if let Some(dir) = path.parent() {
                            ui.add(egui::Label::new(theme::muted(p, t.in_folder(dir)).size(theme::SMALL)).truncate());
                        }
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            let kind = if single { Kind::Primary } else { Kind::Secondary };
                            if theme::icon_button(ui, kind, true, Icon::Folder, t.show_in_folder).clicked() {
                                show_in_folder(path);
                            }
                            if theme::icon_button(ui, Kind::Secondary, true, Icon::External, t.open_epub).clicked() {
                                open_file(path);
                            }
                        });
                    });
                });
            }
        });
    }
    if !app.run.errors.is_empty() {
        note(ui, p.warn_bg, p.warn_line, Icon::Alert, p.fg, t.failed_stays);
    }
    ui.add_space(4.0);
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        let kind = if saved.is_empty() { Kind::Primary } else { Kind::Secondary };
        let more = theme::icon_button(ui, kind, true, Icon::Book, t.convert_more);
        app.mark(demo::Target::ConvertMore, more.rect);
        if more.clicked() {
            app.back_to_library();
        }
    });
}

fn log(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    if app.run.log.is_empty() {
        return;
    }
    ui.add_space(4.0);
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
            for line in &app.run.log {
                ui.label(RichText::new(line.as_str()).monospace().size(12.0));
            }
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
    fn the_step_number_is_the_first_step_not_finished() {
        let line = |look| Line { title: String::new(), detail: String::new(), look };
        assert_eq!(step_number(&[line(Look::Done), line(Look::Running), line(Look::Waiting)]), 2);
        assert_eq!(step_number(&[line(Look::Done), line(Look::Skipped), line(Look::Waiting)]), 3);
        assert_eq!(step_number(&[line(Look::Done), line(Look::Done)]), 2, "all done: the last one");
    }
}

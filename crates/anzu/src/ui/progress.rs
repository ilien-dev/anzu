//! Progress screen: one focused column with the preparation, what is happening now, each
//! book with its own track, the result with the saved files, and the log behind a link.

use anzu_core::library::BookState;
use eframe::egui::{self, Align, CornerRadius, Layout, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};

use super::{AnzuApp, StepState, icons, open_file, show_in_folder, theme};
use crate::i18n::Strings;
use crate::pipeline::{Stage, Step};

/// The preparation, done once for all the books.
const STEPS: [Step; 3] = [Step::Check, Step::Download, Step::CloseKindle];
/// Width of the column on wide windows.
const COLUMN: f32 = 680.0;
const BAR_HEIGHT: f32 = 8.0;
const BAR_GAP: f32 = 6.0;

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

/// The step of the preparation running now, or the next one to start.
fn current(app: &AnzuApp) -> usize {
    STEPS
        .iter()
        .position(|s| look(app, *s) == Look::Running)
        .unwrap_or_else(|| STEPS.iter().filter(|s| look(app, **s) != Look::Waiting).count().min(STEPS.len() - 1))
}

/// Whether the run is still in its preparation.
fn preparing(app: &AnzuApp) -> bool {
    app.run.prepares
        && app.run.result.is_none()
        && app.run.preparation_failed.is_none()
        && STEPS.iter().any(|s| !matches!(look(app, *s), Look::Done | Look::Skipped))
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

/// The line above the title while the run goes on.
fn kicker(app: &AnzuApp, t: &Strings) -> String {
    if preparing(app) {
        return t.preparing(current(app) + 1, STEPS.len());
    }
    let unlocking = app.run.active.values().filter(|s| **s == Stage::Unlock).count();
    if unlocking > 0 {
        return t.unlocking_books(unlocking);
    }
    let statuses: Vec<Status> = app.run.asins.iter().map(|a| status_of(app, a)).collect();
    t.book_of(book_number(&statuses), statuses.len())
}

fn title_of(app: &AnzuApp, t: &Strings, asin: &str) -> String {
    app.books.iter().find(|b| b.asin == asin).map_or_else(|| t.book_fallback(asin), |b| AnzuApp::display_title(b, t))
}

/// A slow pulse between 0.45 and 1, for what is running.
fn pulse(ui: &egui::Ui) -> f32 {
    let time = ui.input(|i| i.time);
    0.45 + 0.55 * (0.5 + 0.5 * (time * 4.0).sin() as f32)
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
            });
        });
    });
    egui::CentralPanel::default().frame(theme::band(p.bg, 28)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| column(ui, |ui| body(app, ui, p, t)));
    });
    if app.busy() {
        // The running step pulses and the spinners turn: keep drawing while the run goes on.
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// The logo and the name, which lead back to the books once the run has finished.
fn home_link(ui: &mut egui::Ui, p: theme::Palette, t: &Strings, enabled: bool) -> bool {
    let inner = ui.horizontal(|ui| AnzuApp::brand(ui, p, t));
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

/// Lays the content out in a centred column of at most [`COLUMN`].
fn column(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let full = ui.available_width();
    let width = full.min(COLUMN);
    let pad = ((full - width) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            add(ui);
        });
    });
}

fn body(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let kicker = match &app.run.result {
        Some((converted, failed)) => theme::bold(t.finished(*converted, failed.len())).color(p.accent),
        None => theme::bold(kicker(app, t)).color(p.muted),
    };
    ui.label(kicker.size(theme::SMALL));
    let title = match app.run.asins.as_slice() {
        [asin] => title_of(app, t, asin),
        asins => t.books(asins.len()),
    };
    ui.label(theme::bold(title).size(20.0));
    ui.add_space(10.0);

    if app.run.prepares {
        preparation(app, ui, p, t);
        ui.add_space(6.0);
    }
    if app.run.result.is_none() {
        now(app, ui, p, t);
    }
    books(app, ui, p, t);
    if app.run.result.is_some() {
        result(app, ui, p, t);
    }
    ui.add_space(4.0);
    log(app, ui, p, t);
}

/// The steps done once for all the books, and why they stopped if they did.
fn preparation(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    ui.horizontal(|ui| {
        ui.label(theme::bold(t.preparation).size(theme::SMALL));
        ui.label(theme::muted(p, t.preparation_note).size(theme::SMALL));
    });
    step_bar(app, ui, p, t);
    if let Some(reason) = &app.run.preparation_failed {
        ui.add_space(4.0);
        ui.horizontal_top(|ui| {
            let (mark, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
            icons::alert(ui.painter(), mark.center(), p.warn);
            ui.add(egui::Label::new(RichText::new(reason.as_str()).color(p.warn).size(12.0)).wrap());
        });
    }
}

fn step_bar(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let width = ui.available_width();
    let segment = (width - BAR_GAP * (STEPS.len() - 1) as f32) / STEPS.len() as f32;
    let (bar, _) = ui.allocate_exact_size(egui::vec2(width, BAR_HEIGHT), Sense::hover());
    let pulse = pulse(ui);
    let painter = ui.painter();
    for (i, step) in STEPS.iter().enumerate() {
        let left = bar.left() + i as f32 * (segment + BAR_GAP);
        let rect = Rect::from_min_size(Pos2::new(left, bar.top()), egui::vec2(segment, BAR_HEIGHT));
        let radius = CornerRadius::same(2);
        match look(app, *step) {
            Look::Done => {
                painter.rect_filled(rect, radius, p.ok);
            }
            Look::Running => {
                painter.rect_filled(rect, radius, p.accent.gamma_multiply(pulse));
            }
            Look::Failed => {
                painter.rect_filled(rect, radius, p.warn);
            }
            Look::Skipped => icons::skipped_bar(painter, rect, p),
            Look::Waiting => {
                painter.rect_filled(rect, radius, p.panel);
                painter.rect_stroke(rect, radius, Stroke::new(1.0, p.line), StrokeKind::Inside);
            }
        }
    }
    ui.add_space(2.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = BAR_GAP;
        for step in STEPS {
            let look = look(app, step);
            let (state, color) = match look {
                Look::Done => (t.step_done, p.ok),
                Look::Running => (t.step_running, p.accent),
                Look::Failed => (t.step_not_completed, p.warn),
                Look::Skipped => (t.step_skipped, p.muted),
                Look::Waiting => (t.step_pending, p.muted),
            };
            ui.allocate_ui_with_layout(egui::vec2(segment, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(segment);
                ui.spacing_mut().item_spacing.y = 2.0;
                let title = t.step(step).0;
                let text = if look == Look::Running { theme::bold(title) } else { RichText::new(title) };
                let title_color = if matches!(look, Look::Waiting | Look::Skipped) { p.muted } else { p.fg };
                ui.label(text.size(theme::SMALL).color(title_color));
                ui.label(RichText::new(state).size(10.5).color(color));
            });
        }
    });
}

/// What is happening right now.
fn now(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let unlocking = app.run.active.values().filter(|s| **s == Stage::Unlock).count();
    let converting = app.run.asins.iter().find(|a| app.run.active.get(*a) == Some(&Stage::Convert));
    let (title, detail): (String, &str) = if preparing(app) {
        let (title, detail) = t.step(STEPS[current(app)]);
        (title.to_string(), detail)
    } else if unlocking > 0 {
        (t.unlocking_books(unlocking), t.stage(Stage::Unlock).1)
    } else if let Some(asin) = converting {
        (t.converting_book(&title_of(app, t, asin)), t.stage(Stage::Convert).1)
    } else {
        // Between two stages: nothing to show for a moment.
        ui.label(theme::muted(p, t.processing(app.run.asins.len())).size(theme::SMALL));
        return;
    };
    theme::panel(p).inner_margin(egui::Margin::symmetric(16, 14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let (mark, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
            ui.put(mark, egui::Spinner::new().size(14.0).color(p.accent));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(theme::bold(title));
                ui.label(theme::muted(p, detail).size(theme::SMALL));
            });
        });
    });
    ui.label(theme::muted(p, t.processing(app.run.asins.len())).size(theme::SMALL));
}

/// Every book of the run with its own track and where it stands. The list is not clipped:
/// the page scrolls instead.
fn books(app: &AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    theme::panel(p).inner_margin(egui::Margin::symmetric(16, 12)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        for (index, asin) in app.run.asins.iter().enumerate() {
            if index > 0 {
                ui.add_space(6.0);
            }
            let state = app.run.states.get(asin).copied().unwrap_or(BookState::Locked);
            let status = status_of(app, asin);
            let (text, color) = match status {
                Status::Queued => (t.queued, p.muted),
                Status::Unlocking => (t.unlocking, p.accent),
                Status::Unlocked => (t.state_unlocked, p.muted),
                Status::Converting => (t.converting, p.accent),
                Status::Converted => (t.state_converted, p.ok),
                Status::Failed(Stage::Unlock) => (t.failed_unlock, p.warn),
                Status::Failed(Stage::Convert) => (t.failed_convert, p.warn),
                Status::NotStarted => (t.not_started, p.muted),
            };
            let running = matches!(status, Status::Unlocking | Status::Converting);
            ui.add(egui::Label::new(theme::bold(title_of(app, t, asin))).truncate());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(icons::TRACK_WIDTH, 16.0), Sense::hover());
                let ok = !matches!(status, Status::Failed(_) | Status::NotStarted);
                icons::track(ui.painter(), rect.left_center(), state, ok, p);
                if running {
                    let (mark, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                    ui.put(mark, egui::Spinner::new().size(12.0).color(p.accent));
                }
                let label = RichText::new(text).color(color).size(12.0);
                ui.label(if running { label.strong() } else { label });
            });
            if let Some((_, reason)) = app.run.errors.get(asin) {
                ui.label(theme::muted(p, reason.lines().next().unwrap_or(reason)).size(theme::SMALL));
            }
        }
    });
}

/// The saved files, each with a way to reach it, and the way back to the books.
fn result(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    let saved = app.run.saved.clone();
    if !saved.is_empty() {
        theme::panel(p).stroke(Stroke::new(1.0, p.ok.gamma_multiply(0.6))).inner_margin(egui::Margin::same(18)).show(
            ui,
            |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (mark, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                    icons::check(ui.painter(), mark, p.ok);
                    ui.label(theme::bold(t.epubs_ready(saved.len())).color(p.ok));
                });
                ui.add_space(4.0);
                let single = saved.len() == 1;
                for (index, (_, path)) in saved.iter().enumerate() {
                    if index > 0 {
                        ui.separator();
                    }
                    ui.horizontal_top(|ui| {
                        let (icon, _) = ui.allocate_exact_size(egui::vec2(22.0, 28.0), Sense::hover());
                        icons::file(ui.painter(), icon.shrink2(egui::vec2(2.0, 1.0)), p.muted);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 3.0;
                            let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().to_string());
                            ui.label(theme::bold(name));
                            if let Some(dir) = path.parent() {
                                ui.label(theme::muted(p, t.in_folder(dir)).size(theme::SMALL));
                            }
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let show = if single {
                                    theme::button(ui, theme::Surface::Page, true, theme::primary_button(p, t.show_in_folder))
                                } else {
                                    theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.show_in_folder))
                                };
                                if show.clicked() {
                                    show_in_folder(path);
                                }
                                if theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.open_epub)).clicked() {
                                    open_file(path);
                                }
                            });
                        });
                    });
                }
            },
        );
    }
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        let button = if saved.is_empty() { theme::primary_button(p, t.convert_more) } else { egui::Button::new(t.convert_more) };
        if theme::button(ui, theme::Surface::Page, true, button).clicked() {
            app.back_to_library();
        }
    });
}

fn log(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &Strings) {
    if app.run.log.is_empty() {
        return;
    }
    let label = RichText::new(t.log_toggle(app.run.show_log, app.run.log.len())).size(12.0).color(p.muted);
    let toggle = ui.add(egui::Button::new(label).frame(false));
    if toggle.hovered() {
        ui.painter().hline(toggle.rect.x_range().shrink(4.0), toggle.rect.bottom() - 6.0, Stroke::new(1.0, p.muted));
    }
    if toggle.clicked() {
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
                if theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.copy)).clicked() {
                    ui.ctx().copy_text(app.run.log.join("\n"));
                }
            });
        });
        egui::ScrollArea::vertical().id_salt("log").max_height(220.0).stick_to_bottom(true).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            for line in &app.run.log {
                ui.label(RichText::new(line.as_str()).size(12.0));
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
}

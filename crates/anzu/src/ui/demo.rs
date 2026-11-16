//! The window as a demo: the real [`AnzuApp`], played by a script instead of a person.
//!
//! The script moves a pointer and clicks the real widgets through the same input egui
//! gets from a mouse, so every hover, press and screen is the app's own. The system
//! behind the window is [`crate::demo::DemoBackend`], and runs go through the real
//! [`pipeline::run`]; only the clock that paces their messages is the frame time.
//! Input from the visitor is ignored, so the loop cannot be broken.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anzu_core::library::Book;
use anzu_core::settings::{Language, ThemeChoice};
use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke};

use crate::app::RunPlan;
use crate::demo::{DemoBackend, DemoSystem};
use crate::pipeline::{self, Holds, Msg};

/// How long each step of a run is shown running. Longer than in the app, which also
/// waits on the network and the disk, so a visitor can follow it.
const STEP: Duration = Duration::from_millis(1100);
/// The pause between two messages of a run, in seconds.
const GAP: f64 = 0.18;
/// How long the start-up check takes, in seconds.
const CHECK: f64 = 1.6;
/// How long the pointer takes to reach a widget, in seconds.
const TRAVEL: f64 = 0.75;
/// How long the pointer rests on a widget before pressing it, in seconds.
const REST: f64 = 0.22;

/// Where the demo tells the page what just happened: "download", "saved", "finished", "loop".
pub type OnEvent = Box<dyn Fn(&str)>;

/// What the page around the demo asks for and hears about.
#[derive(Default)]
pub struct Remote {
    pub language: Option<Language>,
    pub theme: Option<ThemeChoice>,
    pub on_event: Option<OnEvent>,
}

pub type RemoteHandle = Rc<RefCell<Remote>>;

/// The books of the demo library the script ticks.
pub const BOOKS: [&str; 2] = ["B0DEMO0001", "B0DEMO0002"];

/// The widgets the script points at. Each one reports where it was drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Refresh,
    Book(&'static str),
    Run,
    CloseConfirm,
    ConvertMore,
}

#[derive(Debug, Clone, PartialEq)]
enum Step {
    Wait(f64),
    /// The Kindle app finishes downloading the books.
    Download,
    Click(Target),
    /// Waits until the widget is on screen.
    WaitFor(Target),
    Event(&'static str),
    Restart,
}

/// Where the pointer is in a click.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Click {
    Travel { from: Pos2, start: f64 },
    Rest { start: f64 },
    Press,
    Release,
}

fn script() -> VecDeque<Step> {
    use Step::*;
    VecDeque::from([
        Wait(CHECK + 0.8),
        Event("download"),
        Wait(1.4),
        Download,
        Click(Target::Refresh),
        Wait(1.4),
        Click(Target::Book(BOOKS[0])),
        Wait(0.35),
        Click(Target::Book(BOOKS[1])),
        Wait(0.9),
        Click(Target::Run),
        Click(Target::CloseConfirm),
        WaitFor(Target::ConvertMore),
        Wait(3.2),
        Click(Target::ConvertMore),
        Wait(3.6),
        Event("loop"),
        Restart,
    ])
}

pub struct Driver {
    pub system: Rc<DemoSystem>,
    pub remote: RemoteHandle,
    script: VecDeque<Step>,
    /// When the current step began.
    step_start: Option<f64>,
    click: Option<Click>,
    pointer: Pos2,
    pressed: bool,
    /// The widgets drawn in the last frame, and those of the frame being drawn.
    targets: Vec<(Target, Rect)>,
    seen: Vec<(Target, Rect)>,
    /// Messages of the start-up check, with the time they arrive.
    later: Vec<(f64, Msg)>,
    /// The recorded run, still to be shown.
    replay: VecDeque<Msg>,
    holds: Holds,
    /// When the last message of the run was shown, and when the next one is due.
    clock: f64,
    due: Option<f64>,
    /// The run is waiting for the answer to "close Kindle?".
    asking: bool,
    pub restart: bool,
}

impl Driver {
    pub fn new(remote: RemoteHandle, pointer: Pos2) -> Driver {
        Driver {
            system: DemoSystem::new(),
            remote,
            script: script(),
            step_start: None,
            click: None,
            pointer,
            pressed: false,
            targets: Vec::new(),
            seen: Vec::new(),
            later: Vec::new(),
            replay: VecDeque::new(),
            holds: Holds::new(STEP),
            clock: 0.0,
            due: None,
            asking: false,
            restart: false,
        }
    }

    pub fn pointer(&self) -> Pos2 {
        self.pointer
    }

    fn backend(&self) -> DemoBackend {
        DemoBackend { system: self.system.clone() }
    }

    fn emit(&self, event: &str) {
        if let Some(on_event) = &self.remote.borrow().on_event {
            on_event(event);
        }
    }

    /// Records where a scripted widget was drawn in this frame.
    pub fn mark(&mut self, target: Target, rect: Rect) {
        self.seen.push((target, rect));
    }

    fn rect_of(&self, target: &Target) -> Option<Rect> {
        self.targets.iter().find(|(t, _)| t == target).map(|(_, r)| *r)
    }

    /// The start-up check, answered after a while as if it had gone online.
    pub fn start_check(&mut self, now: f64) {
        let (health, _) = pipeline::check(&self.backend());
        self.later.push((now + CHECK, Msg::Health(health)));
    }

    /// Runs the plan through the real pipeline and keeps its messages to show them in time.
    /// The run happens on a copy of the library; each change reaches the library the
    /// window reads only when its message is shown, as it would on a real system.
    pub fn start_run(&mut self, now: f64, plan: &RunPlan, books: &[Book], out_dir: &std::path::Path, lang: Language) {
        let backend = DemoBackend { system: self.system.rehearsal() };
        let mut messages = Vec::new();
        pipeline::run(&backend, plan, books, out_dir, lang, &mut |m| messages.push(m), &mut || true);
        self.replay = messages.into();
        // The demo's backend cannot be shared between threads: one book at a time.
        self.holds = Holds::new(STEP).for_books(plan.asins.len(), 1);
        self.clock = now;
        self.due = None;
    }

    /// The dialog was answered: the run goes on.
    pub fn answered(&mut self, now: f64) {
        self.asking = false;
        self.clock = now;
        self.due = None;
    }

    /// Sends the messages that are due to the window, as its workers would.
    pub fn deliver(&mut self, now: f64, tx: &Sender<Msg>) {
        let (due, waiting): (Vec<_>, Vec<_>) = self.later.drain(..).partition(|(at, _)| *at <= now);
        self.later = waiting;
        for (_, message) in due {
            let _ = tx.send(message);
        }
        while !self.asking && !self.replay.is_empty() {
            let at = match self.due {
                Some(at) => at,
                None => {
                    let held = self.holds.hold(&self.replay[0], Duration::from_secs_f64(self.clock)).as_secs_f64();
                    let at = self.clock + GAP + held;
                    self.due = Some(at);
                    at
                }
            };
            if now < at {
                break;
            }
            let Some(message) = self.replay.pop_front() else { break };
            self.clock = at;
            self.due = None;
            match &message {
                Msg::BookProgress { asin, state } => self.system.set_state(asin, *state),
                Msg::NeedCloseKindle => self.asking = true,
                Msg::Saved { .. } => self.emit("saved"),
                Msg::Finished { .. } => self.emit("finished"),
                _ => {}
            }
            let _ = tx.send(message);
        }
    }

    /// Advances the script and writes the pointer events of this frame into `input`,
    /// in place of the visitor's.
    pub fn drive(&mut self, input: &mut egui::RawInput) {
        let now = input.time.unwrap_or(0.0);
        input.events.retain(|e| matches!(e, egui::Event::WindowFocused(_)));
        input.hovered_files.clear();
        input.dropped_files.clear();
        while let Some(step) = self.script.front().cloned() {
            let start = *self.step_start.get_or_insert(now);
            let done = match step {
                Step::Wait(seconds) => now - start >= seconds,
                Step::Download => {
                    self.system.download_all();
                    true
                }
                Step::Event(name) => {
                    self.emit(name);
                    true
                }
                Step::WaitFor(target) => self.rect_of(&target).is_some(),
                Step::Restart => {
                    self.restart = true;
                    false
                }
                Step::Click(target) => self.click(&target, now, input),
            };
            if !done {
                break;
            }
            self.script.pop_front();
            self.step_start = None;
        }
        input.events.push(egui::Event::PointerMoved(self.pointer));
    }

    /// One frame of a click on `target`. Returns whether it is over.
    fn click(&mut self, target: &Target, now: f64, input: &mut egui::RawInput) -> bool {
        let Some(rect) = self.rect_of(target) else { return false };
        let aim = rect.center();
        let state = *self.click.get_or_insert(Click::Travel { from: self.pointer, start: now });
        let button = |pressed| egui::Event::PointerButton {
            pos: aim,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        match state {
            Click::Travel { from, start } => {
                let t = ((now - start) / TRAVEL).clamp(0.0, 1.0) as f32;
                // Ease out: quick at first, settling on the widget.
                let eased = 1.0 - (1.0 - t).powi(3);
                self.pointer = from + (aim - from) * eased;
                if t >= 1.0 {
                    self.click = Some(Click::Rest { start: now });
                }
                false
            }
            Click::Rest { start } => {
                self.pointer = aim;
                if now - start >= REST {
                    self.click = Some(Click::Press);
                }
                false
            }
            Click::Press => {
                input.events.push(egui::Event::PointerMoved(aim));
                input.events.push(button(true));
                self.pressed = true;
                self.click = Some(Click::Release);
                false
            }
            Click::Release => {
                input.events.push(egui::Event::PointerMoved(aim));
                input.events.push(button(false));
                self.pressed = false;
                self.click = None;
                true
            }
        }
    }

    /// Ends the frame: the widgets just drawn are the ones the next frame aims at, and the
    /// pointer is painted over everything.
    pub fn end_frame(&mut self, ctx: &egui::Context) {
        self.targets = std::mem::take(&mut self.seen);
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Debug, egui::Id::unique("demo-pointer")));
        paint_pointer(&painter, self.pointer, if self.pressed { 0.9 } else { 1.0 });
        ctx.request_repaint();
    }
}

/// The arrow pointer, drawn white with a dark edge as on Windows.
fn paint_pointer(painter: &egui::Painter, tip: Pos2, scale: f32) {
    const ARROW: [(f32, f32); 7] = [(0.0, 0.0), (0.0, 17.0), (4.2, 13.2), (7.0, 19.4), (9.6, 18.2), (6.9, 12.2), (12.2, 12.2)];
    // The outline is not convex, so the fill is laid as triangles.
    const TRIANGLES: [u32; 15] = [0, 1, 2, 0, 2, 5, 0, 5, 6, 2, 3, 4, 2, 4, 5];
    let points: Vec<Pos2> = ARROW.iter().map(|(x, y)| tip + egui::vec2(*x, *y) * scale).collect();
    let mut fill = egui::Mesh::default();
    for point in &points {
        fill.colored_vertex(*point, Color32::WHITE);
    }
    fill.indices.extend(TRIANGLES);
    painter.add(Shape::mesh(fill));
    painter.add(Shape::closed_line(points, Stroke::new(1.0, Color32::from_rgb(0x14, 0x17, 0x1A))));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(time: f64) -> egui::RawInput {
        egui::RawInput { time: Some(time), ..Default::default() }
    }

    #[test]
    fn a_click_travels_rests_presses_and_releases_on_the_widget() {
        let mut driver = Driver::new(RemoteHandle::default(), Pos2::ZERO);
        driver.script = VecDeque::from([Step::Click(Target::Run)]);
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(40.0, 20.0));
        driver.targets = vec![(Target::Run, rect)];
        let mut presses = Vec::new();
        for frame in 0..200 {
            let mut raw = input(frame as f64 / 60.0);
            driver.drive(&mut raw);
            presses.extend(raw.events.into_iter().filter_map(|e| match e {
                egui::Event::PointerButton { pos, pressed, .. } => Some((pos, pressed)),
                _ => None,
            }));
        }
        assert_eq!(presses, vec![(rect.center(), true), (rect.center(), false)]);
        assert!(driver.script.is_empty());
    }

    #[test]
    fn the_visitors_input_is_dropped() {
        let mut driver = Driver::new(RemoteHandle::default(), Pos2::ZERO);
        let mut raw = input(0.0);
        raw.events.push(egui::Event::Text("x".into()));
        raw.events.push(egui::Event::PointerMoved(Pos2::new(5.0, 5.0)));
        driver.drive(&mut raw);
        assert_eq!(raw.events, vec![egui::Event::PointerMoved(Pos2::ZERO)]);
    }

    /// Plays whole loops of the demo on a headless window, at 60 frames a second.
    fn play(seconds: f64, language: Language) -> Vec<String> {
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let remote = RemoteHandle::default();
        remote.borrow_mut().on_event = Some(Box::new(move |e: &str| sink.borrow_mut().push(e.to_string())));
        let ctx = egui::Context::default();
        let mut app = super::super::demo_app(&ctx, remote, language, ThemeChoice::Dark);
        let mut frame = eframe::Frame::_new_kittest();
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(super::super::WINDOW_WIDTH, super::super::WINDOW_HEIGHT));
        for n in 0..(seconds * 60.0) as u64 {
            let mut raw = egui::RawInput { time: Some(n as f64 / 60.0), screen_rect: Some(screen), ..Default::default() };
            app.raw_input_hook(&ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| {
                app.logic(ui.ctx(), &mut frame);
                app.ui(ui, &mut frame);
            });
            output.textures_delta.clear();
        }
        events.take()
    }

    #[test]
    fn the_script_runs_the_real_window_from_download_to_epub_and_loops() {
        for language in [Language::En, Language::Es] {
            let events = play(90.0, language);
            let first: Vec<&str> = events.iter().map(String::as_str).take(5).collect();
            assert_eq!(first, ["download", "saved", "saved", "finished", "loop"], "{language:?}: {events:?}");
            assert!(events.iter().filter(|e| *e == "loop").count() >= 2, "{events:?}");
        }
    }

    /// As in the app, a book's real title appears only once the run has finished and the
    /// window reads the library again, never ahead of what has been shown.
    #[test]
    fn titles_appear_when_the_run_is_shown_finished() {
        use eframe::App;
        let ctx = egui::Context::default();
        super::super::theme::install(&ctx);
        let mut app = super::super::AnzuApp::demo(&ctx, RemoteHandle::default(), Language::En, ThemeChoice::Dark, Pos2::ZERO);
        let mut frame = eframe::Frame::_new_kittest();
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(1180.0, 760.0));
        let mut seen_finished = false;
        for n in 0..(22.0 * 60.0) as u64 {
            let mut raw = egui::RawInput { time: Some(n as f64 / 60.0), screen_rect: Some(screen), ..Default::default() };
            app.raw_input_hook(&ctx, &mut raw);
            let mut output = ctx.run_ui(raw, |ui| {
                app.logic(ui.ctx(), &mut frame);
                app.ui(ui, &mut frame);
            });
            output.textures_delta.clear();
            let titled = app.books.iter().filter(|b| b.title.is_some()).count();
            if app.run.result.is_none() {
                assert_eq!(titled, 1, "only Walden has a title before the run ends (frame {n})");
            } else {
                seen_finished = true;
                assert_eq!(titled, 3);
            }
        }
        assert!(seen_finished);
    }

    #[test]
    fn the_script_waits_for_widgets_that_are_not_drawn() {
        let mut driver = Driver::new(RemoteHandle::default(), Pos2::ZERO);
        driver.script = VecDeque::from([Step::Click(Target::CloseConfirm)]);
        for frame in 0..100 {
            driver.drive(&mut input(frame as f64 / 60.0));
        }
        assert_eq!(driver.script.len(), 1);
    }
}

//! The run: check, download, close Kindle, unlock and convert.
//!
//! The real operations (network, processes, disk) sit behind [`Backend`], so the rules of
//! the run can be tested without touching the system:
//!
//! - nothing is unlocked unless compatibility is confirmed at that very moment;
//! - the Kindle app is never closed without the user's consent;
//! - one failed book does not stop the others.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anzu_core::Error;
use anzu_core::kindle::KindleInstall;
use anzu_core::library::{Book, BookState};
use anzu_core::releases::Release;
use anzu_core::settings::Language;
use anzu_core::tool::Tool;
use anzu_kfx::{Converted, Labels};

use crate::app::{self, Health, RunPlan};
use crate::i18n::{self, Strings};

/// How many conversion warnings of one book are shown in the log.
const MAX_WARNINGS_SHOWN: usize = 20;

/// The system operations a run needs.
pub trait Backend {
    fn detect_kindle(&self) -> Option<KindleInstall>;
    fn fetch_release(&self) -> Result<Release, Error>;
    fn ensure_tool(&self, release: &Release) -> Result<Tool, Error>;
    fn kindle_running(&self) -> bool;
    fn close_kindle(&self) -> Result<(), Error>;
    /// Unlocks the books in `content_dir`; returns `(ASIN, path)` for each.
    fn unlock(
        &self,
        exe: &Path,
        content_dir: &Path,
        on_line: &mut dyn FnMut(&str),
    ) -> Result<Vec<(String, PathBuf)>, Error>;
    fn unlocked_path(&self, asin: &str) -> PathBuf;
    fn convert(&self, input: &Path, out_dir: &Path, labels: &Labels) -> Result<Converted, anzu_kfx::Error>;
    fn mark_converted(&self, asin: &str, epub: &Path);
    /// The same backend when it can be shared between threads, so that several books are
    /// converted at once. The web demo's cannot, and converts one book at a time.
    fn shared(&self) -> Option<&(dyn Backend + Sync)> {
        None
    }
}

/// The most books converted at the same time.
pub const MAX_CONVERT_WORKERS: usize = 4;

/// How many books of `books` are converted at the same time: one core is left for the
/// window, and never more than [`MAX_CONVERT_WORKERS`].
pub fn convert_workers(books: usize) -> usize {
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    cores.saturating_sub(1).clamp(1, MAX_CONVERT_WORKERS).min(books.max(1))
}

/// What happens to one book of the conversion stage, by its index in the list.
enum Converting {
    Started(usize),
    Done(usize, Result<Converted, anzu_kfx::Error>),
}

/// Converts every book of `inputs` into `out_dir`, several at a time when the backend can be
/// shared, and reports each start and end on the calling thread.
fn convert_all(
    backend: &dyn Backend,
    inputs: &[PathBuf],
    out_dir: &Path,
    labels: &Labels,
    report: &mut dyn FnMut(Converting),
) {
    let workers = convert_workers(inputs.len());
    let Some(shared) = backend.shared().filter(|_| workers > 1) else {
        for (index, input) in inputs.iter().enumerate() {
            report(Converting::Started(index));
            report(Converting::Done(index, backend.convert(input, out_dir, labels)));
        }
        return;
    };
    let next = std::sync::atomic::AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let (tx, next) = (tx.clone(), &next);
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(input) = inputs.get(index) else { break };
                    if tx.send(Converting::Started(index)).is_err() {
                        break;
                    }
                    let result = shared.convert(input, out_dir, labels);
                    if tx.send(Converting::Done(index, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for event in rx {
            report(event);
        }
    });
}

/// The preparation: what is done once for the whole run, before any book is unlocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Check,
    Download,
    CloseKindle,
}

/// What is done to each book. The tool unlocks every book of a run in one go, so all the
/// locked books are in [`Stage::Unlock`] at the same time; then up to
/// [`MAX_CONVERT_WORKERS`] are converted at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Unlock,
    Convert,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Health(Health),
    StepStarted(Step),
    StepDone(Step),
    /// The step was not needed for this run.
    StepSkipped(Step),
    /// The preparation stopped, so no book was unlocked. Said once, not once per book.
    PreparationFailed(String),
    BookStarted { asin: String, stage: Stage },
    /// A book finished a stage and is now in `state`.
    BookProgress { asin: String, state: BookState },
    BookFailed { asin: String, stage: Stage, reason: String },
    /// Where the EPUB of a book was written, and whether its name got a number because it
    /// was taken.
    Saved { asin: String, path: PathBuf, renamed: bool },
    Log(String),
    /// The Kindle app is open: ask before closing it.
    NeedCloseKindle,
    Finished { converted: usize, failed: Vec<(String, String)> },
}

/// The shortest time a step is shown running, so a step that finishes at once still reads
/// as work done instead of a mark that flips without anything seeming to happen.
pub const MIN_STEP: Duration = Duration::from_millis(450);

/// What the pacer times: a step of the preparation, or a stage of one book.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Timed {
    Step(Step),
    Book(String, Stage),
}

impl Timed {
    fn started_by(message: &Msg) -> Option<Timed> {
        match message {
            Msg::StepStarted(step) => Some(Timed::Step(*step)),
            Msg::BookStarted { asin, stage } => Some(Timed::Book(asin.clone(), *stage)),
            _ => None,
        }
    }

    fn ended_by(message: &Msg) -> Option<Timed> {
        match message {
            Msg::StepDone(step) | Msg::StepSkipped(step) => Some(Timed::Step(*step)),
            Msg::BookProgress { asin, state: BookState::Unlocked } => Some(Timed::Book(asin.clone(), Stage::Unlock)),
            Msg::BookProgress { asin, state: BookState::Converted } => Some(Timed::Book(asin.clone(), Stage::Convert)),
            Msg::BookFailed { asin, stage, .. } => Some(Timed::Book(asin.clone(), *stage)),
            _ => None,
        }
    }
}

/// The shortest time the conversions of a run are shown, whatever the number of books: a
/// fast computer converts a few books in a blink, and a list that turns green at once reads
/// as a fault rather than as work done.
const BATCH_FLOOR: Duration = Duration::from_millis(1300);
/// What each book of the run adds to [`BATCH_FLOOR`].
const BATCH_FLOOR_PER_BOOK: Duration = Duration::from_millis(120);
/// The floor never grows past this, so a library of hundreds of books is not slowed down.
const BATCH_FLOOR_MAX: Duration = Duration::from_millis(2600);
/// The first conversion ends no sooner than this after the batch starts.
const BATCH_LEAD: Duration = Duration::from_millis(150);
/// The shortest and longest time one book is shown converting.
const BOOK_MIN: Duration = Duration::from_millis(40);
const BOOK_MAX: Duration = Duration::from_millis(350);
/// How long the run stays with every book done before it says it has finished.
const FINISH_HOLD: Duration = Duration::from_millis(500);
/// How fast the ends of the conversions slow down towards the last one: 1 spreads them
/// evenly, more makes the first ones quick and the last ones settle.
const BATCH_EASE: f32 = 1.6;

/// The floor of the conversions of a run of `books`.
fn batch_floor(books: usize) -> Duration {
    (BATCH_FLOOR + BATCH_FLOOR_PER_BOOK * books as u32).min(BATCH_FLOOR_MAX)
}

/// The rule of the pacer on any clock: how long a message must wait so that the step or
/// stage it ends has been shown running for a minimum time. `now` is measured from any
/// fixed start; the web demo uses the frame time, the app a real clock.
///
/// It only ever holds a message back: nothing is shown done before it is.
pub struct Holds {
    min: Duration,
    started: Vec<(Timed, Duration)>,
    /// How many books the run converts, 0 when unknown, and how many at the same time.
    books: usize,
    workers: usize,
    /// When the first conversion started, and how many have ended since.
    batch: Option<(Duration, usize)>,
    /// When the last conversion is shown ending.
    last_end: Option<Duration>,
}

impl Holds {
    pub fn new(min: Duration) -> Self {
        Holds { min, started: Vec::new(), books: 0, workers: 1, batch: None, last_end: None }
    }

    /// Spreads the ends of the conversions of a run of `books`, converted `workers` at a
    /// time, over the floor of the batch.
    pub fn for_books(mut self, books: usize, workers: usize) -> Self {
        self.books = books;
        self.workers = workers.max(1);
        self
    }

    /// Records `message` sent at `now` and returns how long it must be held first.
    pub fn hold(&mut self, message: &Msg, now: Duration) -> Duration {
        if let Some(timed) = Timed::started_by(message) {
            if matches!(timed, Timed::Book(_, Stage::Convert)) && self.batch.is_none() {
                self.batch = Some((now, 0));
            }
            self.started.retain(|(t, _)| *t != timed);
            self.started.push((timed, now));
        } else if let Some(timed) = Timed::ended_by(message)
            && let Some(index) = self.started.iter().position(|(t, _)| *t == timed)
        {
            let (timed, start) = self.started.remove(index);
            if !matches!(timed, Timed::Book(_, Stage::Convert)) {
                return (start + self.min).saturating_sub(now);
            }
            let wait = self.conversion_end(start).saturating_sub(now);
            self.last_end = Some(now + wait);
            return wait;
        } else if matches!(message, Msg::Finished { .. })
            && let Some(end) = self.last_end
        {
            return (end + FINISH_HOLD).saturating_sub(now);
        }
        Duration::ZERO
    }

    /// The earliest moment a conversion that started at `start` may be shown ending.
    fn conversion_end(&mut self, start: Duration) -> Duration {
        if self.books == 0 {
            // No batch to spread over: a short hold of its own.
            return start + self.min / 3;
        }
        let floor = batch_floor(self.books);
        // Each lane of workers fits its books in half the floor, so the books of one lane,
        // one after another, never stretch the batch past the curve of the floor.
        let per_book = (floor * self.workers as u32 / (2 * self.books as u32)).clamp(BOOK_MIN, BOOK_MAX);
        let (batch_start, ended) = self.batch.get_or_insert((start, 0));
        *ended += 1;
        let done = (*ended as f32 / self.books as f32).min(1.0);
        let planned = *batch_start + BATCH_LEAD + floor.mul_f32(1.0 - (1.0 - done).powf(BATCH_EASE));
        planned.max(start + per_book)
    }
}

/// Holds back the end of each step, and of each stage of a book, until it has been shown
/// running for a minimum time.
pub struct Pacer {
    holds: Holds,
    origin: Instant,
}

impl Pacer {
    pub fn new(min: Duration) -> Self {
        Pacer { holds: Holds::new(min), origin: Instant::now() }
    }

    /// See [`Holds::for_books`].
    pub fn for_books(mut self, books: usize, workers: usize) -> Self {
        self.holds = self.holds.for_books(books, workers);
        self
    }

    /// Call before sending `message`: it sleeps when a step would end too soon.
    pub fn wait_before(&mut self, message: &Msg) {
        let left = self.holds.hold(message, self.origin.elapsed());
        if !left.is_zero() {
            std::thread::sleep(left);
        }
    }
}

/// The books of `plan` that still have DRM.
pub fn locked_in(plan: &RunPlan, books: &[Book]) -> Vec<String> {
    plan.asins
        .iter()
        .filter(|asin| books.iter().any(|b| &b.asin == *asin && b.state == BookState::Locked))
        .cloned()
        .collect()
}

/// Whether a run of `plan` goes through the preparation (check, download, close Kindle).
pub fn prepares(plan: &RunPlan, books: &[Book]) -> bool {
    plan.needs_unlock && !locked_in(plan, books).is_empty()
}

/// What is needed to launch the unlock once compatibility is confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ready {
    pub exe: PathBuf,
    pub content_dir: PathBuf,
}

fn check_with(backend: &dyn Backend, on_step: &mut dyn FnMut(Step, bool)) -> (Health, Option<Ready>) {
    on_step(Step::Check, true);
    let Some(kindle) = backend.detect_kindle() else {
        return (app::health(None, Err(&Error::NoRelease), &[]), None);
    };
    let release = match backend.fetch_release() {
        Ok(release) => release,
        Err(error) => return (app::health(Some(&kindle), Err(&error), &[]), None),
    };
    // Without a published checksum there is nothing to verify: do not even download.
    if release.tools_zip().is_none_or(|zip| zip.sha256.is_none()) {
        return (app::health(Some(&kindle), Ok(&release), &[]), None);
    }
    on_step(Step::Check, false);

    on_step(Step::Download, true);
    let tool = match backend.ensure_tool(&release) {
        Ok(tool) => tool,
        Err(error) => return (app::health(Some(&kindle), Err(&error), &[]), None),
    };
    on_step(Step::Download, false);

    let health = app::health(Some(&kindle), Ok(&release), &tool.exe_names);
    let ready = match &health {
        Health::Ready { exe_name, .. } => Some(Ready { exe: tool.dir.join(exe_name), content_dir: kindle.content_dir }),
        _ => None,
    };
    (health, ready)
}

/// Checks whether the system is ready to unlock.
pub fn check(backend: &dyn Backend) -> (Health, Option<Ready>) {
    check_with(backend, &mut |_, _| {})
}

/// Checks, downloads and closes Kindle. Returns what the unlock needs, or why it stopped.
fn prepare(
    backend: &dyn Backend,
    t: &Strings,
    send: &mut dyn FnMut(Msg),
    confirm_close: &mut dyn FnMut() -> bool,
) -> Result<Ready, String> {
    let (health, ready) =
        check_with(backend, &mut |step, start| send(if start { Msg::StepStarted(step) } else { Msg::StepDone(step) }));
    let Some(ready) = ready else {
        let reason = match &health {
            Health::Blocked(reason) => t.block_message(reason).0,
            _ => t.compatibility_unconfirmed.to_string(),
        };
        send(Msg::Health(health));
        return Err(reason);
    };
    send(Msg::Health(health));

    send(Msg::StepStarted(Step::CloseKindle));
    if backend.kindle_running() {
        send(Msg::NeedCloseKindle);
        if !confirm_close() {
            return Err(t.kindle_left_open.to_string());
        }
        backend.close_kindle().map_err(|e| t.could_not_close_kindle(&t.core_error(&e)))?;
        send(Msg::StepDone(Step::CloseKindle));
    } else {
        send(Msg::StepSkipped(Step::CloseKindle));
    }
    Ok(ready)
}

/// Prepares and unlocks the `locked` books, reporting each one. Failures go to `failed`.
fn unlock_books(
    backend: &dyn Backend,
    t: &Strings,
    locked: &[String],
    send: &mut dyn FnMut(Msg),
    confirm_close: &mut dyn FnMut() -> bool,
    failed: &mut Vec<(String, String)>,
) {
    let ready = match prepare(backend, t, send, confirm_close) {
        Ok(ready) => ready,
        Err(reason) => {
            send(Msg::Log(t.unlock_not_done(&reason)));
            send(Msg::PreparationFailed(reason.clone()));
            failed.extend(locked.iter().map(|asin| (asin.clone(), reason.clone())));
            return;
        }
    };

    for asin in locked {
        send(Msg::BookStarted { asin: asin.clone(), stage: Stage::Unlock });
    }
    let result = backend.unlock(&ready.exe, &ready.content_dir, &mut |line| {
        // The filter's placeholder is the one line of tool output that is ours to translate.
        let line = if line == anzu_core::unlock::WITHHELD { t.withheld } else { line };
        send(Msg::Log(line.to_string()));
    });
    let unlocked: Vec<String> = match result {
        Ok(unlocked) => unlocked.into_iter().map(|(asin, _)| asin).collect(),
        Err(error) => {
            let reason = t.core_error(&error);
            send(Msg::Log(t.unlock_not_done(&reason)));
            for asin in locked {
                send(Msg::BookFailed { asin: asin.clone(), stage: Stage::Unlock, reason: reason.clone() });
                failed.push((asin.clone(), reason.clone()));
            }
            return;
        }
    };
    for asin in locked {
        if unlocked.contains(asin) {
            send(Msg::BookProgress { asin: asin.clone(), state: BookState::Unlocked });
        } else {
            let reason = t.tool_could_not_unlock.to_string();
            send(Msg::BookFailed { asin: asin.clone(), stage: Stage::Unlock, reason: reason.clone() });
            failed.push((asin.clone(), reason));
        }
    }
}

/// Runs the plan. Emits progress messages and always ends with [`Msg::Finished`].
pub fn run(
    backend: &dyn Backend,
    plan: &RunPlan,
    books: &[Book],
    out_dir: &Path,
    lang: Language,
    send: &mut dyn FnMut(Msg),
    confirm_close: &mut dyn FnMut() -> bool,
) {
    let t = i18n::strings(lang);
    let mut failed: Vec<(String, String)> = Vec::new();
    if prepares(plan, books) {
        unlock_books(backend, t, &locked_in(plan, books), send, confirm_close, &mut failed);
    }

    let mut converted = 0;
    let labels = t.labels();
    let pending: Vec<&String> = plan.asins.iter().filter(|a| !failed.iter().any(|(f, _)| f == *a)).collect();
    let inputs: Vec<PathBuf> = pending.iter().map(|asin| backend.unlocked_path(asin)).collect();
    convert_all(backend, &inputs, out_dir, &labels, &mut |event| {
        let (index, outcome) = match event {
            Converting::Started(index) => {
                send(Msg::BookStarted { asin: pending[index].clone(), stage: Stage::Convert });
                return;
            }
            Converting::Done(index, outcome) => (index, outcome),
        };
        let asin = pending[index];
        // Settings are written here, on one thread, never by two conversions at once.
        match outcome {
            Ok(result) => {
                backend.mark_converted(asin, &result.path);
                converted += 1;
                for warning in result.warnings.iter().take(MAX_WARNINGS_SHOWN) {
                    send(Msg::Log(t.warning(warning)));
                }
                if result.warnings.len() > MAX_WARNINGS_SHOWN {
                    send(Msg::Log(t.more_warnings(result.warnings.len() - MAX_WARNINGS_SHOWN)));
                }
                send(Msg::Log(t.epub_saved(&result.path)));
                send(Msg::Saved { asin: asin.clone(), path: result.path.clone(), renamed: result.renamed });
                send(Msg::BookProgress { asin: asin.clone(), state: BookState::Converted });
            }
            Err(error) => {
                let detail = t.kfx_error(&error);
                send(Msg::Log(t.could_not_convert(asin, &detail)));
                send(Msg::BookFailed { asin: asin.clone(), stage: Stage::Convert, reason: detail.clone() });
                failed.push((asin.clone(), detail));
            }
        }
    });
    // Failures are reported in plan order.
    failed.sort_by_key(|(asin, _)| plan.asins.iter().position(|a| a == asin));
    send(Msg::Finished { converted, failed });
}

/// The real system.
pub struct CoreBackend {
    pub data_dir: PathBuf,
}

impl Backend for CoreBackend {
    fn detect_kindle(&self) -> Option<KindleInstall> {
        anzu_core::kindle::detect()
    }

    fn fetch_release(&self) -> Result<Release, Error> {
        anzu_core::releases::fetch_latest()
    }

    fn ensure_tool(&self, release: &Release) -> Result<Tool, Error> {
        anzu_core::tool::ensure_tool(release, &self.data_dir.join("tool"))
    }

    fn kindle_running(&self) -> bool {
        anzu_core::kindle::is_running()
    }

    fn close_kindle(&self) -> Result<(), Error> {
        anzu_core::kindle::close()
    }

    fn unlock(
        &self,
        exe: &Path,
        content_dir: &Path,
        on_line: &mut dyn FnMut(&str),
    ) -> Result<Vec<(String, PathBuf)>, Error> {
        let work = self.data_dir.join("work");
        let out = work.join("out");
        let keyfile = self.data_dir.join("keys").join("anzu.k4i");
        let request = anzu_core::unlock::UnlockRequest {
            exe,
            content_dir,
            out_dir: &out,
            keyfile: &keyfile,
            work_dir: &work,
        };
        anzu_core::unlock::unlock(&request, &self.data_dir, on_line)
    }

    fn unlocked_path(&self, asin: &str) -> PathBuf {
        anzu_core::library::unlocked_path(&self.data_dir, asin)
    }

    fn convert(&self, input: &Path, out_dir: &Path, labels: &Labels) -> Result<Converted, anzu_kfx::Error> {
        anzu_kfx::convert_with(input, out_dir, labels)
    }

    fn mark_converted(&self, asin: &str, epub: &Path) {
        let mut settings = anzu_core::settings::Settings::load(&self.data_dir);
        settings.converted.insert(asin.to_string(), epub.to_path_buf());
        let _ = settings.save(&self.data_dir);
    }

    fn shared(&self) -> Option<&(dyn Backend + Sync)> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::BlockReason;
    use anzu_core::releases::Asset;
    use std::sync::Mutex;

    /// A simulated system that records every operation it is asked for.
    struct Fake {
        calls: Mutex<Vec<String>>,
        kindle_version: Option<&'static str>,
        release: Result<Option<&'static str>, Error>,
        tool: Result<Vec<&'static str>, Error>,
        running: bool,
        close_ok: bool,
        unlock: Result<Vec<&'static str>, Error>,
        unlock_log: Vec<&'static str>,
        failing_conversions: Vec<&'static str>,
        conversion_warnings: Vec<&'static str>,
        /// Converts several books at once, like the real system.
        parallel: bool,
    }

    impl Fake {
        fn ready() -> Self {
            Fake {
                calls: Mutex::new(Vec::new()),
                kindle_version: Some("1.1.5634.0"),
                release: Ok(Some("abc")),
                tool: Ok(vec!["MSIXKFXArchiver_x64_1_5634.exe"]),
                running: false,
                close_ok: true,
                unlock: Ok(vec!["A"]),
                unlock_log: vec![],
                failing_conversions: vec![],
                conversion_warnings: vec![],
                parallel: false,
            }
        }

        fn note(&self, call: &str) {
            self.calls.lock().unwrap().push(call.to_string());
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    fn clone_error(e: &Error) -> Error {
        match e {
            Error::Network(m) => Error::Network(m.clone()),
            Error::RateLimited => Error::RateLimited,
            Error::NoDigest => Error::NoDigest,
            Error::ChecksumMismatch => Error::ChecksumMismatch,
            Error::KindleRunning => Error::KindleRunning,
            Error::UnlockFailed { code, tail } => Error::UnlockFailed { code: *code, tail: tail.clone() },
            other => Error::Other(other.to_string()),
        }
    }

    impl Backend for Fake {
        fn detect_kindle(&self) -> Option<KindleInstall> {
            self.note("detect");
            self.kindle_version
                .map(|v| KindleInstall { version: v.parse().unwrap(), content_dir: PathBuf::from("Content") })
        }

        fn fetch_release(&self) -> Result<Release, Error> {
            self.note("fetch");
            match &self.release {
                Ok(sha) => Ok(Release {
                    tag: "v10.0.35".into(),
                    published_at: String::new(),
                    notes: String::new(),
                    assets: vec![Asset { name: "DeDRM_tools.zip".into(), url: "u".into(), sha256: sha.map(String::from) }],
                }),
                Err(e) => Err(clone_error(e)),
            }
        }

        fn ensure_tool(&self, release: &Release) -> Result<Tool, Error> {
            self.note("ensure");
            match &self.tool {
                Ok(names) => Ok(Tool {
                    tag: release.tag.clone(),
                    dir: PathBuf::from("tool"),
                    exe_names: names.iter().map(|n| n.to_string()).collect(),
                }),
                Err(e) => Err(clone_error(e)),
            }
        }

        fn kindle_running(&self) -> bool {
            self.note("running?");
            self.running
        }

        fn close_kindle(&self) -> Result<(), Error> {
            self.note("close");
            if self.close_ok { Ok(()) } else { Err(Error::CloseTimedOut(60)) }
        }

        fn unlock(
            &self,
            exe: &Path,
            _content_dir: &Path,
            on_line: &mut dyn FnMut(&str),
        ) -> Result<Vec<(String, PathBuf)>, Error> {
            self.note(&format!("unlock {}", exe.file_name().unwrap().to_string_lossy()));
            for line in &self.unlock_log {
                on_line(line);
            }
            match &self.unlock {
                Ok(asins) => Ok(asins.iter().map(|a| (a.to_string(), self.unlocked_path(a))).collect()),
                Err(e) => Err(clone_error(e)),
            }
        }

        fn unlocked_path(&self, asin: &str) -> PathBuf {
            PathBuf::from(format!("{asin}.kfx-zip"))
        }

        fn convert(&self, input: &Path, out_dir: &Path, labels: &Labels) -> Result<Converted, anzu_kfx::Error> {
            let asin = input.file_stem().unwrap().to_string_lossy().to_string();
            self.note(&format!("convert {asin} [{}]", labels.untitled));
            if self.failing_conversions.contains(&asin.as_str()) {
                return Err(anzu_kfx::Error::NotKfx);
            }
            Ok(Converted {
                path: out_dir.join(format!("{asin}.epub")),
                title: format!("Book {asin}"),
                warnings: self.conversion_warnings.iter().map(|w| w.to_string()).collect(),
                renamed: false,
            })
        }

        fn mark_converted(&self, asin: &str, epub: &Path) {
            self.note(&format!("mark {asin} {}", epub.file_name().unwrap().to_string_lossy()));
        }

        fn shared(&self) -> Option<&(dyn Backend + Sync)> {
            if self.parallel { Some(self) } else { None }
        }
    }

    fn book(asin: &str, state: BookState) -> Book {
        Book { asin: asin.into(), title: None, source: PathBuf::from("x.azw"), size: 1, state, unlocked: None, epub: None }
    }

    fn library() -> Vec<Book> {
        vec![book("A", BookState::Locked), book("B", BookState::Unlocked)]
    }

    fn plan(asins: &[&str], needs_unlock: bool) -> RunPlan {
        RunPlan { needs_unlock, asins: asins.iter().map(|a| a.to_string()).collect() }
    }

    /// Runs the plan in the given language and returns the messages it emitted.
    fn run_in(fake: &Fake, plan: &RunPlan, accept_close: bool, lang: Language) -> Vec<Msg> {
        let mut messages = Vec::new();
        run(fake, plan, &library(), Path::new("out"), lang, &mut |m| messages.push(m), &mut || accept_close);
        messages
    }

    fn run_with(fake: &Fake, plan: &RunPlan, accept_close: bool) -> Vec<Msg> {
        run_in(fake, plan, accept_close, Language::En)
    }

    fn finished(messages: &[Msg]) -> (usize, Vec<String>) {
        match messages.last() {
            Some(Msg::Finished { converted, failed }) => (*converted, failed.iter().map(|(a, _)| a.clone()).collect()),
            other => panic!("the last message must be Finished, got {other:?}"),
        }
    }

    fn called(fake: &Fake, prefix: &str) -> bool {
        fake.calls().iter().any(|c| c.starts_with(prefix))
    }

    #[test]
    fn check_reports_a_ready_system() {
        let fake = Fake::ready();
        let (health, ready) = check(&fake);
        assert!(matches!(health, Health::Ready { ref exe_name, .. } if exe_name == "MSIXKFXArchiver_x64_1_5634.exe"));
        let ready = ready.expect("what is needed to unlock");
        assert_eq!(ready.exe, PathBuf::from("tool").join("MSIXKFXArchiver_x64_1_5634.exe"));
        assert_eq!(ready.content_dir, PathBuf::from("Content"));
        assert_eq!(fake.calls(), vec!["detect", "fetch", "ensure"]);
    }

    #[test]
    fn check_does_not_touch_the_network_without_kindle() {
        let fake = Fake { kindle_version: None, ..Fake::ready() };
        let (health, ready) = check(&fake);
        assert_eq!(health, Health::Blocked(BlockReason::KindleMissing));
        assert!(ready.is_none());
        assert_eq!(fake.calls(), vec!["detect"]);
    }

    #[test]
    fn check_maps_failures_to_block_reasons() {
        let offline = Fake { release: Err(Error::Network("x".into())), ..Fake::ready() };
        assert_eq!(check(&offline).0, Health::Blocked(BlockReason::Offline));
        assert!(!called(&offline, "ensure"), "nothing is downloaded without a release");

        let tampered = Fake { tool: Err(Error::ChecksumMismatch), ..Fake::ready() };
        assert_eq!(check(&tampered).0, Health::Blocked(BlockReason::BadDownload));

        let no_digest = Fake { release: Ok(None), ..Fake::ready() };
        assert_eq!(check(&no_digest).0, Health::Blocked(BlockReason::NoDigest));
        assert!(!called(&no_digest, "ensure"), "nothing is downloaded without a checksum");

        let too_new = Fake { kindle_version: Some("1.1.6000.0"), ..Fake::ready() };
        assert!(matches!(check(&too_new).0, Health::Blocked(BlockReason::TooNew { .. })));
    }

    #[test]
    fn converting_unlocked_books_needs_nothing_else() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&["B"], false), true);

        assert_eq!(fake.calls(), vec!["convert B [Untitled]", "mark B B.epub"]);
        assert_eq!(finished(&messages), (1, vec![]));
        assert!(messages.contains(&Msg::BookProgress { asin: "B".into(), state: BookState::Converted }));
    }

    #[test]
    fn full_run_checks_then_unlocks_then_converts() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        assert_eq!(
            fake.calls(),
            vec![
                "detect",
                "fetch",
                "ensure",
                "running?",
                "unlock MSIXKFXArchiver_x64_1_5634.exe",
                "convert A [Untitled]",
                "mark A A.epub",
                "convert B [Untitled]",
                "mark B B.epub",
            ]
        );
        assert_eq!(finished(&messages), (2, vec![]));
        let unlocked = messages.iter().position(|m| *m == Msg::BookProgress { asin: "A".into(), state: BookState::Unlocked });
        let converted = messages.iter().position(|m| *m == Msg::BookProgress { asin: "A".into(), state: BookState::Converted });
        assert!(unlocked.is_some() && unlocked < converted, "{messages:?}");
    }

    #[test]
    fn steps_are_announced_in_order() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&["A"], true), true);
        let steps: Vec<Step> = messages
            .iter()
            .filter_map(|m| match m {
                Msg::StepStarted(s) => Some(*s),
                _ => None,
            })
            .collect();
        assert_eq!(steps, vec![Step::Check, Step::Download, Step::CloseKindle]);
        for step in steps {
            let ended = messages.contains(&Msg::StepDone(step)) || messages.contains(&Msg::StepSkipped(step));
            assert!(ended, "{step:?} did not finish: {messages:?}");
        }
        assert!(messages.contains(&Msg::StepSkipped(Step::CloseKindle)), "Kindle was already closed");
    }

    /// The preparation and the books, in the order they start and end.
    fn events(messages: &[Msg]) -> Vec<String> {
        messages
            .iter()
            .filter_map(|m| match m {
                Msg::StepStarted(s) => Some(format!("start {s:?}")),
                Msg::StepDone(s) => Some(format!("done {s:?}")),
                Msg::StepSkipped(s) => Some(format!("skip {s:?}")),
                Msg::PreparationFailed(_) => Some("preparation failed".to_string()),
                Msg::BookStarted { asin, stage } => Some(format!("{asin} start {stage:?}")),
                Msg::BookProgress { asin, state } => Some(format!("{asin} {state:?}")),
                Msg::BookFailed { asin, stage, .. } => Some(format!("{asin} failed {stage:?}")),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn each_book_goes_through_its_own_stages() {
        let fake = Fake { unlock: Ok(vec!["A", "C"]), ..Fake::ready() };
        let books = vec![book("A", BookState::Locked), book("B", BookState::Unlocked), book("C", BookState::Locked)];
        let mut messages = Vec::new();
        let plan = plan(&["A", "B", "C"], true);
        run(&fake, &plan, &books, Path::new("out"), Language::En, &mut |m| messages.push(m), &mut || true);
        assert_eq!(
            events(&messages),
            vec![
                "start Check",
                "done Check",
                "start Download",
                "done Download",
                "start CloseKindle",
                "skip CloseKindle",
                // The tool unlocks every locked book in one go.
                "A start Unlock",
                "C start Unlock",
                "A Unlocked",
                "C Unlocked",
                // Then they are converted: one by one here, several at once on a backend
                // that can be shared between threads.
                "A start Convert",
                "A Converted",
                "B start Convert",
                "B Converted",
                "C start Convert",
                "C Converted",
            ]
        );
    }

    #[test]
    fn a_run_without_unlocking_has_no_preparation() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&["B"], false), true);
        assert_eq!(events(&messages), vec!["B start Convert", "B Converted"]);
    }

    #[test]
    fn a_failed_preparation_is_reported_once_and_not_per_book() {
        let fake = Fake { running: true, close_ok: false, ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);
        let failures: Vec<&Msg> = messages.iter().filter(|m| matches!(m, Msg::PreparationFailed(_))).collect();
        assert_eq!(failures.len(), 1, "{messages:?}");
        let Msg::PreparationFailed(reason) = failures[0] else { unreachable!() };
        assert!(reason.starts_with("Could not close the Kindle app"), "{reason}");
        assert!(
            !messages.iter().any(|m| matches!(m, Msg::BookStarted { asin, .. } | Msg::BookFailed { asin, .. } if asin == "A")),
            "the locked book never started: {messages:?}"
        );
        assert_eq!(finished(&messages), (1, vec!["A".to_string()]), "it still counts as not converted");
    }

    #[test]
    fn an_unlock_error_fails_each_locked_book_in_the_unlock_stage() {
        let fake = Fake { unlock: Err(Error::UnlockFailed { code: -1, tail: "no session".into() }), ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);
        let failed = messages.iter().find_map(|m| match m {
            Msg::BookFailed { asin, stage: Stage::Unlock, reason } if asin == "A" => Some(reason.clone()),
            _ => None,
        });
        assert!(failed.is_some_and(|r| r.contains("no session")), "{messages:?}");
    }

    #[test]
    fn a_failed_conversion_is_reported_before_the_next_book_starts() {
        let fake = Fake { unlock: Ok(vec!["A"]), failing_conversions: vec!["A"], ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);
        let seen = events(&messages);
        let failed = seen.iter().position(|e| e == "A failed Convert").expect("A fails on its own");
        let next = seen.iter().position(|e| e == "B start Convert").expect("B still runs");
        assert!(failed < next, "{seen:?}");
    }

    #[test]
    fn a_conversion_is_held_less_than_a_step() {
        let mut pacer = Pacer::new(Duration::from_millis(90));
        let start = Instant::now();
        pacer.wait_before(&Msg::BookStarted { asin: "A".into(), stage: Stage::Convert });
        pacer.wait_before(&Msg::BookProgress { asin: "A".into(), state: BookState::Converted });
        let held = start.elapsed();
        assert!(held >= Duration::from_millis(30) && held < Duration::from_millis(80), "{held:?}");
    }

    #[test]
    fn each_book_is_timed_on_its_own() {
        let min = Duration::from_millis(60);
        let mut pacer = Pacer::new(min);
        pacer.wait_before(&Msg::BookStarted { asin: "A".into(), stage: Stage::Unlock });
        pacer.wait_before(&Msg::BookStarted { asin: "B".into(), stage: Stage::Unlock });
        let start = Instant::now();
        pacer.wait_before(&Msg::BookProgress { asin: "A".into(), state: BookState::Unlocked });
        assert!(start.elapsed() >= Duration::from_millis(40), "A is held: {:?}", start.elapsed());
        let second = Instant::now();
        pacer.wait_before(&Msg::BookProgress { asin: "B".into(), state: BookState::Unlocked });
        assert!(second.elapsed() < Duration::from_millis(20), "B already ran long enough: {:?}", second.elapsed());
    }

    #[test]
    fn the_path_of_each_epub_is_reported() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&["B"], false), true);
        assert!(messages.contains(&Msg::Saved { asin: "B".into(), path: Path::new("out").join("B.epub"), renamed: false }), "{messages:?}");
    }

    #[test]
    fn a_skipped_step_is_held_too() {
        let min = Duration::from_millis(60);
        let mut pacer = Pacer::new(min);
        let start = Instant::now();
        pacer.wait_before(&Msg::StepStarted(Step::Check));
        pacer.wait_before(&Msg::StepSkipped(Step::Check));
        assert!(start.elapsed() >= min, "{:?}", start.elapsed());
    }

    #[test]
    fn incompatibility_found_at_run_time_blocks_the_unlock() {
        let fake = Fake { kindle_version: Some("1.1.6000.0"), ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        assert!(!called(&fake, "unlock"), "{:?}", fake.calls());
        assert!(!called(&fake, "close"));
        assert!(messages.iter().any(|m| matches!(m, Msg::Health(Health::Blocked(BlockReason::TooNew { .. })))));
        assert_eq!(finished(&messages), (1, vec!["A".to_string()]), "the already unlocked book is still converted");
        assert!(called(&fake, "convert B"));
        assert!(!called(&fake, "convert A"));
    }

    #[test]
    fn a_tampered_download_blocks_the_unlock() {
        let fake = Fake { tool: Err(Error::ChecksumMismatch), ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A"], true), true);

        assert!(!called(&fake, "unlock"));
        assert!(messages.contains(&Msg::Health(Health::Blocked(BlockReason::BadDownload))));
        assert_eq!(finished(&messages), (0, vec!["A".to_string()]));
    }

    #[test]
    fn kindle_is_never_closed_without_consent() {
        let fake = Fake { running: true, ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), false);

        assert!(messages.contains(&Msg::NeedCloseKindle));
        assert!(!called(&fake, "close"), "{:?}", fake.calls());
        assert!(!called(&fake, "unlock"));
        assert_eq!(finished(&messages), (1, vec!["A".to_string()]));
    }

    #[test]
    fn with_consent_kindle_is_closed_before_unlocking() {
        let fake = Fake { running: true, ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A"], true), true);

        let calls = fake.calls();
        let close = calls.iter().position(|c| c == "close").expect("it must close");
        let unlock = calls.iter().position(|c| c.starts_with("unlock")).expect("it must unlock");
        assert!(close < unlock, "{calls:?}");
        assert_eq!(finished(&messages), (1, vec![]));
    }

    #[test]
    fn kindle_that_does_not_close_stops_the_unlock() {
        let fake = Fake { running: true, close_ok: false, ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A"], true), true);

        assert!(!called(&fake, "unlock"));
        assert_eq!(finished(&messages), (0, vec!["A".to_string()]));
    }

    #[test]
    fn kindle_is_not_asked_about_when_it_is_already_closed() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&["A"], true), false);
        assert!(!messages.contains(&Msg::NeedCloseKindle));
        assert_eq!(finished(&messages), (1, vec![]));
    }

    #[test]
    fn a_book_the_tool_could_not_unlock_fails_alone() {
        let fake = Fake { unlock: Ok(vec![]), ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        assert_eq!(finished(&messages), (1, vec!["A".to_string()]));
        assert!(!called(&fake, "convert A"));
        assert!(called(&fake, "convert B"));
    }

    #[test]
    fn closing_kindle_reports_why_it_failed() {
        let fake = Fake { running: true, close_ok: false, ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A"], true), true);
        let Some(Msg::Finished { failed, .. }) = messages.last() else { panic!("Finished") };
        assert_eq!(failed[0].1, "Could not close the Kindle app: it was still running 60 s after it was asked to close");
        let spanish = run_in(&fake, &plan(&["A"], true), true, Language::Es);
        let Some(Msg::Finished { failed, .. }) = spanish.last() else { panic!("Finished") };
        assert_eq!(failed[0].1, "No se pudo cerrar la app Kindle: seguía abierta 60 s después de pedirle que se cerrara");
    }

    #[test]
    fn an_unlock_error_fails_the_locked_books_with_its_message() {
        let fake = Fake { unlock: Err(Error::UnlockFailed { code: -1, tail: "no session".into() }), ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        let Some(Msg::Finished { converted, failed }) = messages.last() else { panic!("Finished") };
        assert_eq!(*converted, 1);
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, "A");
        assert!(failed[0].1.contains("no session"), "{failed:?}");
    }

    #[test]
    fn one_failed_conversion_does_not_stop_the_rest() {
        let fake = Fake { unlock: Ok(vec!["A"]), failing_conversions: vec!["A"], ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        assert_eq!(finished(&messages), (1, vec!["A".to_string()]));
        assert!(called(&fake, "convert B"));
        assert!(!called(&fake, "mark A"), "a book that failed is not marked as converted");
    }

    #[test]
    fn tool_output_is_forwarded_to_the_log() {
        let fake = Fake { unlock_log: vec!["Decrypting DRMION...", "done"], ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A"], true), true);
        assert!(messages.contains(&Msg::Log("Decrypting DRMION...".into())));
        assert!(messages.contains(&Msg::Log("done".into())));
    }

    #[test]
    fn conversion_warnings_are_shown_in_the_log() {
        let fake = Fake { conversion_warnings: vec!["unknown node type $276"], ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["B"], false), true);
        assert!(messages.contains(&Msg::Log("warning: unknown node type $276".into())), "{messages:?}");
    }

    #[test]
    fn the_run_speaks_the_chosen_language() {
        let fake = Fake {
            running: true,
            conversion_warnings: vec!["unknown node type $276"],
            unlock_log: vec![anzu_core::unlock::WITHHELD],
            ..Fake::ready()
        };
        let declined = run_in(&fake, &plan(&["A", "B"], true), false, Language::Es);
        let Some(Msg::Finished { failed, .. }) = declined.last() else { panic!("Finished") };
        assert_eq!(failed[0].1, "La app Kindle está abierta y no se cerró");
        assert!(declined.contains(&Msg::Log("aviso: unknown node type $276".into())), "{declined:?}");
        assert!(called(&fake, "convert B [Sin título]"), "book labels follow the language: {:?}", fake.calls());

        let accepted = run_in(&fake, &plan(&["A"], true), true, Language::Es);
        assert!(
            accepted.contains(&Msg::Log("[línea de la herramienta omitida por seguridad]".into())),
            "the filter's placeholder is translated: {accepted:?}"
        );
    }

    #[test]
    fn a_quick_step_is_held_for_the_minimum_time() {
        let min = Duration::from_millis(60);
        let mut pacer = Pacer::new(min);
        let start = Instant::now();
        pacer.wait_before(&Msg::StepStarted(Step::Download));
        pacer.wait_before(&Msg::Log("line".into()));
        assert!(start.elapsed() < min, "only the end of a step waits");
        pacer.wait_before(&Msg::StepDone(Step::Download));
        assert!(start.elapsed() >= min, "{:?}", start.elapsed());
    }

    #[test]
    fn a_slow_step_is_not_held_any_longer() {
        let mut pacer = Pacer::new(Duration::from_millis(30));
        pacer.wait_before(&Msg::StepStarted(Step::CloseKindle));
        std::thread::sleep(Duration::from_millis(40));
        let before_done = Instant::now();
        pacer.wait_before(&Msg::StepDone(Step::CloseKindle));
        assert!(before_done.elapsed() < Duration::from_millis(20), "{:?}", before_done.elapsed());
    }

    #[test]
    fn an_empty_plan_finishes_at_once() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&[], false), true);
        assert!(fake.calls().is_empty());
        assert_eq!(finished(&messages), (0, vec![]));
    }

    #[test]
    fn books_converted_at_once_are_each_reported_once() {
        let fake = Fake { failing_conversions: vec!["C"], parallel: true, ..Fake::ready() };
        let asins = ["B", "C", "D", "E", "F", "G"];
        let books: Vec<Book> = asins.iter().map(|a| book(a, BookState::Unlocked)).collect();
        let mut messages = Vec::new();
        run(&fake, &plan(&asins, false), &books, Path::new("out"), Language::En, &mut |m| messages.push(m), &mut || true);
        let seen = events(&messages);
        for asin in asins {
            let start = seen.iter().position(|e| *e == format!("{asin} start Convert")).expect("every book starts");
            let end = seen
                .iter()
                .position(|e| *e == format!("{asin} Converted") || *e == format!("{asin} failed Convert"))
                .expect("every book ends");
            assert!(start < end, "{seen:?}");
        }
        assert_eq!(finished(&messages), (5, vec!["C".to_string()]), "one failure does not stop the others");
        let marks = fake.calls().iter().filter(|c| c.starts_with("mark ")).count();
        assert_eq!(marks, 5);
    }

    /// Holds a conversion that starts at `start` and really ends at `end`, in milliseconds;
    /// returns when it is shown ending.
    fn shown_end(holds: &mut Holds, asin: &str, start: u64, end: u64) -> Duration {
        holds.hold(&Msg::BookStarted { asin: asin.into(), stage: Stage::Convert }, Duration::from_millis(start));
        let now = Duration::from_millis(end);
        now + holds.hold(&Msg::BookProgress { asin: asin.into(), state: BookState::Converted }, now)
    }

    #[test]
    fn a_quick_batch_is_spread_over_its_floor() {
        let mut holds = Holds::new(MIN_STEP).for_books(2, 4);
        let first = shown_end(&mut holds, "A", 0, 10);
        let second = shown_end(&mut holds, "B", 0, 12);
        assert!(first > Duration::from_millis(500), "{first:?}");
        assert!(second >= BATCH_LEAD + batch_floor(2) - Duration::from_millis(1), "{second:?}");
        assert!(second > first);
        let finish = holds.hold(&Msg::Finished { converted: 2, failed: vec![] }, second);
        assert_eq!(finish, FINISH_HOLD, "every book shows done for a moment before the end");
    }

    #[test]
    fn a_slow_conversion_is_never_held() {
        let mut holds = Holds::new(MIN_STEP).for_books(1, 1);
        let end = shown_end(&mut holds, "A", 0, 9000);
        assert_eq!(end, Duration::from_millis(9000), "the pacer never shows a book done before it is");
    }

    #[test]
    fn a_large_batch_is_not_slowed_down() {
        let mut holds = Holds::new(MIN_STEP).for_books(200, 4);
        let mut now = 0;
        let mut last = Duration::ZERO;
        // Four at a time, each really taking 5 ms; the pacer sleeps between messages.
        for i in 0..200u64 {
            let start = now;
            last = shown_end(&mut holds, &i.to_string(), start, start + 5);
            if i % 4 == 3 {
                now = last.as_millis() as u64;
            }
        }
        // Each round of four waits for its slowest book here, so a little past the floor.
        assert!(last <= BATCH_LEAD + BATCH_FLOOR_MAX + Duration::from_millis(300), "{last:?}");
    }

    #[test]
    fn one_book_at_a_time_still_fits_the_floor() {
        // A computer with one core to spare, or the web demo: the books go one after another.
        let mut holds = Holds::new(MIN_STEP).for_books(12, 1);
        let mut now = 0;
        for i in 0..12u64 {
            now = shown_end(&mut holds, &i.to_string(), now, now + 5).as_millis() as u64;
        }
        let last = Duration::from_millis(now);
        assert!(last >= batch_floor(12), "{last:?}");
        assert!(last <= BATCH_LEAD + batch_floor(12) + Duration::from_millis(200), "{last:?}");
    }
}

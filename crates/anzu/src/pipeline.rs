//! El flujo de trabajo: comprobar, descargar, cerrar Kindle, desbloquear y convertir.
//!
//! Las operaciones reales (red, procesos, disco) están detrás de [`Backend`], de modo que
//! las reglas del flujo se pueden probar sin tocar el sistema:
//!
//! - no se desbloquea si la compatibilidad no se confirma en ese mismo momento;
//! - la app Kindle no se cierra sin el consentimiento del usuario;
//! - un libro que falla no detiene a los demás.

use std::path::{Path, PathBuf};

use anzu_core::Error;
use anzu_core::kindle::KindleInstall;
use anzu_core::library::{Book, BookState};
use anzu_core::releases::Release;
use anzu_core::tool::Tool;
use anzu_kfx::Converted;

use crate::app::{self, Health, RunPlan};

/// Cuántos avisos de conversión de un libro se muestran en el registro.
const MAX_WARNINGS_SHOWN: usize = 20;

/// Las operaciones del sistema que necesita el flujo.
pub trait Backend {
    fn detect_kindle(&self) -> Option<KindleInstall>;
    fn fetch_release(&self) -> Result<Release, Error>;
    fn ensure_tool(&self, release: &Release) -> Result<Tool, Error>;
    fn kindle_running(&self) -> bool;
    fn close_kindle(&self) -> Result<(), Error>;
    /// Desbloquea los libros de `content_dir`; devuelve `(ASIN, ruta)` por libro.
    fn unlock(
        &self,
        exe: &Path,
        content_dir: &Path,
        on_line: &mut dyn FnMut(&str),
    ) -> Result<Vec<(String, PathBuf)>, Error>;
    fn unlocked_path(&self, asin: &str) -> PathBuf;
    fn convert(&self, input: &Path, out_dir: &Path) -> Result<Converted, anzu_kfx::Error>;
    fn mark_converted(&self, asin: &str, epub: &Path);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Check,
    Download,
    CloseKindle,
    Unlock,
    Convert,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Health(Health),
    StepStarted(Step),
    StepDone(Step),
    BookProgress { asin: String, state: BookState },
    Log(String),
    /// La app Kindle está abierta: hay que preguntar antes de cerrarla.
    NeedCloseKindle,
    Finished { converted: usize, failed: Vec<(String, String)> },
}

/// Lo necesario para lanzar el desbloqueo una vez confirmada la compatibilidad.
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
    // Sin huella publicada no hay nada que verificar: ni siquiera se descarga.
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

/// Comprueba si el sistema está listo para desbloquear.
pub fn check(backend: &dyn Backend) -> (Health, Option<Ready>) {
    check_with(backend, &mut |_, _| {})
}

/// Desbloquea. Devuelve los ASIN desbloqueados, o el motivo por el que no se hizo.
fn unlock_phase(
    backend: &dyn Backend,
    send: &mut dyn FnMut(Msg),
    confirm_close: &mut dyn FnMut() -> bool,
) -> Result<Vec<String>, String> {
    let (health, ready) =
        check_with(backend, &mut |step, start| send(if start { Msg::StepStarted(step) } else { Msg::StepDone(step) }));
    let Some(ready) = ready else {
        let reason = match &health {
            Health::Blocked(reason) => reason.message().0,
            _ => "No se pudo confirmar la compatibilidad".to_string(),
        };
        send(Msg::Health(health));
        return Err(reason);
    };
    send(Msg::Health(health));

    send(Msg::StepStarted(Step::CloseKindle));
    if backend.kindle_running() {
        send(Msg::NeedCloseKindle);
        if !confirm_close() {
            return Err("La app Kindle está abierta y no se cerró".to_string());
        }
        backend.close_kindle().map_err(|e| format!("No se pudo cerrar la app Kindle: {e}"))?;
    }
    send(Msg::StepDone(Step::CloseKindle));

    send(Msg::StepStarted(Step::Unlock));
    let unlocked = backend
        .unlock(&ready.exe, &ready.content_dir, &mut |line| send(Msg::Log(line.to_string())))
        .map_err(|e| e.to_string())?;
    send(Msg::StepDone(Step::Unlock));
    Ok(unlocked.into_iter().map(|(asin, _)| asin).collect())
}

/// Ejecuta el plan. Emite mensajes de progreso y termina siempre con [`Msg::Finished`].
pub fn run(
    backend: &dyn Backend,
    plan: &RunPlan,
    books: &[Book],
    out_dir: &Path,
    send: &mut dyn FnMut(Msg),
    confirm_close: &mut dyn FnMut() -> bool,
) {
    let mut failed: Vec<(String, String)> = Vec::new();
    let locked: Vec<&String> = plan
        .asins
        .iter()
        .filter(|asin| books.iter().any(|b| &b.asin == *asin && b.state == BookState::Locked))
        .collect();

    if plan.needs_unlock && !locked.is_empty() {
        match unlock_phase(backend, send, confirm_close) {
            Ok(unlocked) => {
                for asin in &locked {
                    if unlocked.contains(asin) {
                        send(Msg::BookProgress { asin: (*asin).clone(), state: BookState::Unlocked });
                    } else {
                        failed.push(((*asin).clone(), "La herramienta no pudo desbloquear este libro".to_string()));
                    }
                }
            }
            Err(reason) => {
                send(Msg::Log(format!("Desbloqueo no realizado: {reason}")));
                failed.extend(locked.iter().map(|asin| ((*asin).clone(), reason.clone())));
            }
        }
    }

    let mut converted = 0;
    let pending: Vec<&String> = plan.asins.iter().filter(|a| !failed.iter().any(|(f, _)| f == *a)).collect();
    if !pending.is_empty() {
        send(Msg::StepStarted(Step::Convert));
        for asin in pending {
            match backend.convert(&backend.unlocked_path(asin), out_dir) {
                Ok(result) => {
                    backend.mark_converted(asin, &result.path);
                    converted += 1;
                    for warning in result.warnings.iter().take(MAX_WARNINGS_SHOWN) {
                        send(Msg::Log(format!("aviso: {warning}")));
                    }
                    if result.warnings.len() > MAX_WARNINGS_SHOWN {
                        send(Msg::Log(format!("… y {} avisos más", result.warnings.len() - MAX_WARNINGS_SHOWN)));
                    }
                    send(Msg::Log(format!("EPUB guardado: {}", result.path.display())));
                    send(Msg::BookProgress { asin: asin.clone(), state: BookState::Converted });
                }
                Err(error) => {
                    send(Msg::Log(format!("{asin}: no se pudo convertir ({error})")));
                    failed.push((asin.clone(), error.to_string()));
                }
            }
        }
        send(Msg::StepDone(Step::Convert));
    }
    // Los fallos se informan en el orden del plan.
    failed.sort_by_key(|(asin, _)| plan.asins.iter().position(|a| a == asin));
    send(Msg::Finished { converted, failed });
}

/// El sistema real.
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

    fn convert(&self, input: &Path, out_dir: &Path) -> Result<Converted, anzu_kfx::Error> {
        anzu_kfx::convert(input, out_dir)
    }

    fn mark_converted(&self, asin: &str, epub: &Path) {
        let mut settings = anzu_core::settings::Settings::load(&self.data_dir);
        settings.converted.insert(asin.to_string(), epub.to_path_buf());
        let _ = settings.save(&self.data_dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{BlockReason, Health, RunPlan};
    use anzu_core::library::BookState;
    use anzu_core::releases::Asset;
    use std::sync::Mutex;

    /// Un sistema simulado que anota cada operación que se le pide.
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
            if self.close_ok { Ok(()) } else { Err(Error::KindleRunning) }
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

        fn convert(&self, input: &Path, out_dir: &Path) -> Result<Converted, anzu_kfx::Error> {
            let asin = input.file_stem().unwrap().to_string_lossy().to_string();
            self.note(&format!("convert {asin}"));
            if self.failing_conversions.contains(&asin.as_str()) {
                return Err(anzu_kfx::Error::NotKfx);
            }
            Ok(Converted { path: out_dir.join(format!("{asin}.epub")), title: format!("Libro {asin}"), warnings: self.conversion_warnings.iter().map(|w| w.to_string()).collect() })
        }

        fn mark_converted(&self, asin: &str, epub: &Path) {
            self.note(&format!("mark {asin} {}", epub.file_name().unwrap().to_string_lossy()));
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

    /// Ejecuta el flujo y devuelve los mensajes emitidos.
    fn run_with(fake: &Fake, plan: &RunPlan, accept_close: bool) -> Vec<Msg> {
        let mut messages = Vec::new();
        run(fake, plan, &library(), Path::new("out"), &mut |m| messages.push(m), &mut || accept_close);
        messages
    }

    fn finished(messages: &[Msg]) -> (usize, Vec<String>) {
        match messages.last() {
            Some(Msg::Finished { converted, failed }) => (*converted, failed.iter().map(|(a, _)| a.clone()).collect()),
            other => panic!("el último mensaje debe ser Finished, llegó {other:?}"),
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
        let ready = ready.expect("datos para desbloquear");
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
        assert!(!called(&offline, "ensure"), "sin publicación no se descarga nada");

        let tampered = Fake { tool: Err(Error::ChecksumMismatch), ..Fake::ready() };
        assert_eq!(check(&tampered).0, Health::Blocked(BlockReason::BadDownload));

        let no_digest = Fake { release: Ok(None), ..Fake::ready() };
        assert_eq!(check(&no_digest).0, Health::Blocked(BlockReason::NoDigest));
        assert!(!called(&no_digest, "ensure"), "sin huella no se descarga nada");

        let too_new = Fake { kindle_version: Some("1.1.6000.0"), ..Fake::ready() };
        assert!(matches!(check(&too_new).0, Health::Blocked(BlockReason::TooNew { .. })));
    }

    #[test]
    fn converting_unlocked_books_needs_nothing_else() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&["B"], false), true);

        assert_eq!(fake.calls(), vec!["convert B", "mark B B.epub"]);
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
                "convert A",
                "mark A A.epub",
                "convert B",
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
        assert_eq!(steps, vec![Step::Check, Step::Download, Step::CloseKindle, Step::Unlock, Step::Convert]);
        for step in steps {
            assert!(messages.contains(&Msg::StepDone(step)), "{step:?} no terminó: {messages:?}");
        }
    }

    #[test]
    fn incompatibility_found_at_run_time_blocks_the_unlock() {
        let fake = Fake { kindle_version: Some("1.1.6000.0"), ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        assert!(!called(&fake, "unlock"), "{:?}", fake.calls());
        assert!(!called(&fake, "close"));
        assert!(messages.iter().any(|m| matches!(m, Msg::Health(Health::Blocked(BlockReason::TooNew { .. })))));
        assert_eq!(finished(&messages), (1, vec!["A".to_string()]), "el libro ya desbloqueado sí se convierte");
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
        let close = calls.iter().position(|c| c == "close").expect("debe cerrar");
        let unlock = calls.iter().position(|c| c.starts_with("unlock")).expect("debe desbloquear");
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
    fn an_unlock_error_fails_the_locked_books_with_its_message() {
        let fake = Fake { unlock: Err(Error::UnlockFailed { code: -1, tail: "sin sesión".into() }), ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        let Some(Msg::Finished { converted, failed }) = messages.last() else { panic!("Finished") };
        assert_eq!(*converted, 1);
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].0, "A");
        assert!(failed[0].1.contains("sin sesión"), "{failed:?}");
    }

    #[test]
    fn one_failed_conversion_does_not_stop_the_rest() {
        let fake = Fake { unlock: Ok(vec!["A"]), failing_conversions: vec!["A"], ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A", "B"], true), true);

        assert_eq!(finished(&messages), (1, vec!["A".to_string()]));
        assert!(called(&fake, "convert B"));
        assert!(!called(&fake, "mark A"), "un libro que falló no se marca como convertido");
    }

    #[test]
    fn tool_output_is_forwarded_to_the_log() {
        let fake = Fake { unlock_log: vec!["Decrypting DRMION...", "listo"], ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["A"], true), true);
        assert!(messages.contains(&Msg::Log("Decrypting DRMION...".into())));
        assert!(messages.contains(&Msg::Log("listo".into())));
    }

    #[test]
    fn conversion_warnings_are_shown_in_the_log() {
        let fake = Fake { conversion_warnings: vec!["tipo de nodo desconocido $276"], ..Fake::ready() };
        let messages = run_with(&fake, &plan(&["B"], false), true);
        assert!(messages.contains(&Msg::Log("aviso: tipo de nodo desconocido $276".into())), "{messages:?}");
    }

    #[test]
    fn an_empty_plan_finishes_at_once() {
        let fake = Fake::ready();
        let messages = run_with(&fake, &plan(&[], false), true);
        assert!(fake.calls().is_empty());
        assert_eq!(finished(&messages), (0, vec![]));
    }
}

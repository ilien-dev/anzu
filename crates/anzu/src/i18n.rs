//! Interface text, in English (the default) and Spanish.
//!
//! Every fixed string is a field of [`Strings`], so a language that misses one does not
//! compile. Text with numbers or names in it is built by the methods.

use std::path::Path;

use anzu_core::library::BookState;
use anzu_core::settings::Language;

use crate::app::BlockReason;
use crate::cli::CliError;
use crate::pipeline::Step;

pub struct Strings {
    pub lang: Language,
    pub tagline: &'static str,
    pub window_title: &'static str,
    pub theme: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub language: &'static str,
    pub recheck: &'static str,
    pub checking: &'static str,
    pub checking_long: &'static str,
    pub ready: &'static str,
    pub fact_kindle: &'static str,
    pub fact_tool: &'static str,
    pub fact_digest: &'static str,
    pub verified: &'static str,
    pub filter_all: &'static str,
    pub filter_locked: &'static str,
    pub filter_unlocked: &'static str,
    pub filter_converted: &'static str,
    pub col_title: &'static str,
    pub col_asin: &'static str,
    pub col_size: &'static str,
    pub col_state: &'static str,
    pub sort_hint: &'static str,
    pub select_all: &'static str,
    pub clear_selection: &'static str,
    pub empty_group: &'static str,
    pub empty_title: &'static str,
    pub empty_steps: [&'static str; 3],
    pub empty_note: &'static str,
    pub save_in: &'static str,
    pub change_folder: &'static str,
    pub folder_dialog: &'static str,
    pub pick_hint: &'static str,
    pub convert: &'static str,
    pub unlock_and_convert: &'static str,
    pub state_locked: &'static str,
    pub state_locked_waiting: &'static str,
    pub state_unlocked: &'static str,
    pub state_converted: &'static str,
    pub blocked_note: &'static str,
    pub releases_link: &'static str,
    pub close_title: &'static str,
    pub close_body: &'static str,
    pub close_confirm: &'static str,
    pub cancel: &'static str,
    pub steps_title: &'static str,
    pub step_not_completed: &'static str,
    pub step_pending: &'static str,
    pub step_not_needed: &'static str,
    pub failed_unlock: &'static str,
    pub failed_convert: &'static str,
    pub converting: &'static str,
    pub unlocking: &'static str,
    pub queued: &'static str,
    pub log: &'static str,
    pub copy: &'static str,
    pub saved_in: &'static str,
    pub back: &'static str,
    pub wait_hint: &'static str,
    pub open_folder: &'static str,
    pub window_error: &'static str,
    pub withheld: &'static str,
    pub tool_could_not_unlock: &'static str,
    pub kindle_left_open: &'static str,
    pub compatibility_unconfirmed: &'static str,
    pub usage: &'static str,
}

pub const EN: Strings = Strings {
    lang: Language::En,
    tagline: "Kindle to EPUB",
    window_title: "Anzu: Kindle to EPUB",
    theme: "Theme",
    theme_system: "System",
    theme_light: "Light",
    theme_dark: "Dark",
    language: "Language",
    recheck: "Check again",
    checking: "Checking…",
    checking_long: "Checking the Kindle app and the latest version of the tool…",
    ready: "Ready to unlock",
    fact_kindle: "Kindle",
    fact_tool: "Tool",
    fact_digest: "Checksum",
    verified: "verified",
    filter_all: "All",
    filter_locked: "With DRM",
    filter_unlocked: "Unlocked",
    filter_converted: "Converted",
    col_title: "TITLE",
    col_asin: "ASIN",
    col_size: "SIZE",
    col_state: "STATE",
    sort_hint: "Sort by this column",
    select_all: "Select all",
    clear_selection: "Clear the selection",
    empty_group: "No books in this group.",
    empty_title: "No books to convert yet",
    empty_steps: [
        "1. Open the Kindle app from the Microsoft Store and sign in.",
        "2. Download in it the books you want to convert.",
        "3. Come back here and press “Check again”.",
    ],
    empty_note: "Anzu only reads that folder: it never changes or deletes your Kindle books.",
    save_in: "Save to",
    change_folder: "Change folder",
    folder_dialog: "Folder for the EPUB files",
    pick_hint: "Tick the books you want to convert",
    convert: "Convert",
    unlock_and_convert: "Unlock and convert",
    state_locked: "With DRM",
    state_locked_waiting: "With DRM, waiting",
    state_unlocked: "Unlocked",
    state_converted: "EPUB ready",
    blocked_note: "Books that are already unlocked can still be converted.",
    releases_link: "See the tool's releases",
    close_title: "The Kindle app is open",
    close_body: "It has to be closed to unlock. Your reading position is saved when it closes.",
    close_confirm: "Close Kindle and continue",
    cancel: "Cancel",
    steps_title: "Steps",
    step_not_completed: "Not completed",
    step_pending: "Pending",
    step_not_needed: "Not needed",
    failed_unlock: "Could not unlock",
    failed_convert: "Could not convert",
    converting: "Converting…",
    unlocking: "Unlocking…",
    queued: "Queued",
    log: "Log",
    copy: "Copy",
    saved_in: "EPUB files are saved to",
    back: "Back to the books",
    wait_hint: "Wait until it finishes",
    open_folder: "Open the folder",
    window_error: "could not open the window",
    withheld: "[tool output line withheld for safety]",
    tool_could_not_unlock: "The tool could not unlock this book",
    kindle_left_open: "The Kindle app is open and was not closed",
    compatibility_unconfirmed: "Compatibility could not be confirmed",
    usage: "\
Anzu: Kindle to EPUB

Usage:
  anzu                                Open the app
  anzu convert <file> [-o <folder>]   Convert a DRM-free KFX (.kfx-zip) to EPUB
  anzu --help | --version

Options for convert:
  -o, --output <folder>               Where to save the EPUB (default: the current folder)",
};

pub const ES: Strings = Strings {
    lang: Language::Es,
    tagline: "de Kindle a EPUB",
    window_title: "Anzu: de Kindle a EPUB",
    theme: "Tema",
    theme_system: "Sistema",
    theme_light: "Claro",
    theme_dark: "Oscuro",
    language: "Idioma",
    recheck: "Volver a comprobar",
    checking: "Comprobando…",
    checking_long: "Comprobando la app Kindle y la última versión de la herramienta…",
    ready: "Lista para desbloquear",
    fact_kindle: "Kindle",
    fact_tool: "Herramienta",
    fact_digest: "Huella",
    verified: "verificada",
    filter_all: "Todos",
    filter_locked: "Con DRM",
    filter_unlocked: "Desbloqueados",
    filter_converted: "Convertidos",
    col_title: "TÍTULO",
    col_asin: "ASIN",
    col_size: "TAMAÑO",
    col_state: "ESTADO",
    sort_hint: "Ordenar por esta columna",
    select_all: "Seleccionar todos",
    clear_selection: "Quitar la selección",
    empty_group: "No hay libros en este grupo.",
    empty_title: "Aún no hay libros que convertir",
    empty_steps: [
        "1. Abre la app Kindle de Microsoft Store e inicia sesión.",
        "2. Descarga en ella los libros que quieras convertir.",
        "3. Vuelve aquí y pulsa «Volver a comprobar».",
    ],
    empty_note: "Anzu solo lee esa carpeta: nunca cambia ni borra tus libros de Kindle.",
    save_in: "Guardar en",
    change_folder: "Cambiar carpeta",
    folder_dialog: "Carpeta para los EPUB",
    pick_hint: "Marca los libros que quieras convertir",
    convert: "Convertir",
    unlock_and_convert: "Desbloquear y convertir",
    state_locked: "Con DRM",
    state_locked_waiting: "Con DRM, en espera",
    state_unlocked: "Desbloqueado",
    state_converted: "EPUB listo",
    blocked_note: "Los libros ya desbloqueados se pueden convertir igualmente.",
    releases_link: "Ver las publicaciones de la herramienta",
    close_title: "La app Kindle está abierta",
    close_body: "Para desbloquear hay que cerrarla. Tu posición de lectura se guarda al cerrar.",
    close_confirm: "Cerrar Kindle y continuar",
    cancel: "Cancelar",
    steps_title: "Pasos",
    step_not_completed: "No se completó",
    step_pending: "Pendiente",
    step_not_needed: "No hizo falta",
    failed_unlock: "No se pudo desbloquear",
    failed_convert: "No se pudo convertir",
    converting: "Convirtiendo…",
    unlocking: "Desbloqueando…",
    queued: "En cola",
    log: "Registro",
    copy: "Copiar",
    saved_in: "Los EPUB se guardan en",
    back: "Volver a los libros",
    wait_hint: "Espera a que termine",
    open_folder: "Abrir la carpeta",
    window_error: "no se pudo abrir la ventana",
    withheld: "[línea de la herramienta omitida por seguridad]",
    tool_could_not_unlock: "La herramienta no pudo desbloquear este libro",
    kindle_left_open: "La app Kindle está abierta y no se cerró",
    compatibility_unconfirmed: "No se pudo confirmar la compatibilidad",
    usage: "\
Anzu: de Kindle a EPUB

Uso:
  anzu                                  Abre la app
  anzu convert <archivo> [-o <carpeta>] Convierte un KFX sin DRM (.kfx-zip) a EPUB
  anzu --help | --version

Opciones de convert:
  -o, --output <carpeta>                Dónde guardar el EPUB (por defecto, la carpeta actual)",
};

/// The text of a language.
pub fn strings(lang: Language) -> &'static Strings {
    match lang {
        Language::En => &EN,
        Language::Es => &ES,
    }
}

impl Strings {
    fn es(&self) -> bool {
        self.lang == Language::Es
    }

    /// "1 book", "3 books".
    pub fn books(&self, n: usize) -> String {
        match (self.es(), n) {
            (false, 1) => "1 book".to_string(),
            (false, n) => format!("{n} books"),
            (true, 1) => "1 libro".to_string(),
            (true, n) => format!("{n} libros"),
        }
    }

    pub fn selected(&self, n: usize) -> String {
        match (self.es(), n) {
            (false, 0) => "None selected".to_string(),
            (false, n) => format!("{n} selected"),
            (true, 0) => "Ninguno seleccionado".to_string(),
            (true, 1) => "1 seleccionado".to_string(),
            (true, n) => format!("{n} seleccionados"),
        }
    }

    /// The main button: what will happen, and to how many books.
    pub fn run_label(&self, needs_unlock: bool, n: usize) -> String {
        let action = if needs_unlock { self.unlock_and_convert } else { self.convert };
        if n == 0 { action.to_string() } else { format!("{action} {}", self.books(n)) }
    }

    pub fn book_fallback(&self, asin: &str) -> String {
        if self.es() { format!("Libro {asin}") } else { format!("Book {asin}") }
    }

    pub fn state(&self, state: BookState, ready: bool) -> &'static str {
        match state {
            BookState::Locked if ready => self.state_locked,
            BookState::Locked => self.state_locked_waiting,
            BookState::Unlocked => self.state_unlocked,
            BookState::Converted => self.state_converted,
        }
    }

    /// A size people can read, with the decimal separator of the language.
    pub fn human_size(&self, bytes: u64) -> String {
        const KB: f64 = 1024.0;
        let value = bytes as f64;
        let text = if value < KB * KB {
            format!("{} KB", (value / KB).floor())
        } else if value < KB * KB * KB {
            format!("{:.1} MB", value / (KB * KB))
        } else {
            format!("{:.1} GB", value / (KB * KB * KB))
        };
        if self.es() { text.replace('.', ",") } else { text }
    }

    /// Title and detail of a step of a run.
    pub fn step(&self, step: Step) -> (&'static str, &'static str) {
        match (self.es(), step) {
            (false, Step::Check) => ("Check versions", "Installed Kindle and the latest published tool"),
            (false, Step::Download) => ("Download the tool", "With its SHA-256 checksum verified"),
            (false, Step::CloseKindle) => ("Close the Kindle app", "Only if you confirm it"),
            (false, Step::Unlock) => ("Unlock", "It can take a minute per book"),
            (false, Step::Convert) => ("Convert to EPUB", "With Anzu's own converter"),
            (true, Step::Check) => ("Comprobar versiones", "Kindle instalada y última herramienta publicada"),
            (true, Step::Download) => ("Descargar la herramienta", "Con su huella SHA-256 verificada"),
            (true, Step::CloseKindle) => ("Cerrar la app Kindle", "Solo si la confirmas"),
            (true, Step::Unlock) => ("Desbloquear", "Puede tardar un minuto por libro"),
            (true, Step::Convert) => ("Convertir a EPUB", "Con el conversor de Anzu"),
        }
    }

    pub fn processing(&self, n: usize) -> String {
        if self.es() {
            format!("Procesando {}. No abras la app Kindle hasta que termine.", self.books(n))
        } else {
            format!("Processing {}. Do not open the Kindle app until it finishes.", self.books(n))
        }
    }

    pub fn finished(&self, converted: usize, failed: usize) -> String {
        match (self.es(), failed) {
            (false, 0) => format!("Done: {converted} converted"),
            (false, f) => format!("Done: {converted} converted, {f} with problems"),
            (true, 0) => format!("Terminado: {converted} convertidos"),
            (true, f) => format!("Terminado: {converted} convertidos, {f} con problemas"),
        }
    }

    /// Why unlocking is blocked: a title and an explanation.
    pub fn block_message(&self, reason: &BlockReason) -> (String, String) {
        let (title, body): (&str, String) = if self.es() {
            match reason {
                BlockReason::KindleMissing => (
                    "No se encontró la app Kindle",
                    "Instala «Amazon Kindle» desde Microsoft Store, inicia sesión y descarga tus libros. \
                     Después pulsa «Volver a comprobar»."
                        .to_string(),
                ),
                BlockReason::Offline => (
                    "Sin conexión",
                    "No se pudo consultar la última versión de la herramienta de desbloqueo. Sin esa \
                     comprobación no se desbloquea nada."
                        .to_string(),
                ),
                BlockReason::RateLimited => (
                    "GitHub ha limitado las consultas",
                    "Se hicieron demasiadas consultas en poco tiempo. Espera unos minutos y pulsa \
                     «Volver a comprobar»."
                        .to_string(),
                ),
                BlockReason::NoDigest => (
                    "La descarga no se puede verificar",
                    "La última publicación de la herramienta no trae huella SHA-256, así que no hay forma \
                     de comprobar que el archivo es el original. Por seguridad no se ejecutará."
                        .to_string(),
                ),
                BlockReason::BadDownload => (
                    "La descarga no coincide con la original",
                    "La huella del archivo descargado no es la que publica GitHub. Puede ser una descarga \
                     corrupta o manipulada, y no se ejecutará. Pulsa «Volver a comprobar» para reintentarlo."
                        .to_string(),
                ),
                BlockReason::TooNew { kindle, max_supported } => (
                    "Tu app Kindle es más nueva que la herramienta de desbloqueo",
                    format!(
                        "Tienes Kindle {kindle} y la herramienta admite hasta la versión {max_supported}. \
                         No se intentará desbloquear nada hasta que exista una versión compatible; \
                         se descargará sola en cuanto se publique."
                    ),
                ),
                BlockReason::Unknown => (
                    "No se puede confirmar la compatibilidad",
                    "No fue posible determinar si la herramienta admite tu versión de Kindle. Ante la \
                     duda, el desbloqueo queda bloqueado."
                        .to_string(),
                ),
            }
        } else {
            match reason {
                BlockReason::KindleMissing => (
                    "The Kindle app was not found",
                    "Install “Amazon Kindle” from the Microsoft Store, sign in and download your books. \
                     Then press “Check again”."
                        .to_string(),
                ),
                BlockReason::Offline => (
                    "No connection",
                    "The latest version of the unlock tool could not be checked. Nothing is unlocked \
                     without that check."
                        .to_string(),
                ),
                BlockReason::RateLimited => (
                    "GitHub is limiting requests",
                    "Too many requests were made in a short time. Wait a few minutes and press \
                     “Check again”."
                        .to_string(),
                ),
                BlockReason::NoDigest => (
                    "The download cannot be verified",
                    "The tool's latest release has no SHA-256 checksum, so there is no way to confirm \
                     the file is the original. For safety it will not be run."
                        .to_string(),
                ),
                BlockReason::BadDownload => (
                    "The download does not match the original",
                    "The checksum of the downloaded file is not the one GitHub publishes. The download \
                     may be corrupt or tampered with, and it will not be run. Press “Check again” to retry."
                        .to_string(),
                ),
                BlockReason::TooNew { kindle, max_supported } => (
                    "Your Kindle app is newer than the unlock tool",
                    format!(
                        "You have Kindle {kindle} and the tool supports up to version {max_supported}. \
                         Nothing will be unlocked until a compatible version exists; it will be \
                         downloaded automatically once it is published."
                    ),
                ),
                BlockReason::Unknown => (
                    "Compatibility cannot be confirmed",
                    "It was not possible to tell whether the tool supports your Kindle version. When in \
                     doubt, unlocking stays blocked."
                        .to_string(),
                ),
            }
        };
        (title.to_string(), body)
    }

    /// An error from the core library. English uses the library's own wording.
    pub fn core_error(&self, error: &anzu_core::Error) -> String {
        use anzu_core::Error as E;
        if !self.es() {
            return error.to_string();
        }
        match error {
            E::BadVersion(v) => format!("versión no válida: {v}"),
            E::Network(detail) => format!("sin conexión o error de red: {detail}"),
            E::RateLimited => "GitHub ha limitado las peticiones; inténtalo más tarde".to_string(),
            E::NoRelease => "no hay ninguna publicación de la herramienta".to_string(),
            E::BadResponse(detail) => format!("respuesta de GitHub no válida: {detail}"),
            E::NoDigest => "la publicación no trae huella SHA-256; no se ejecutará".to_string(),
            E::ChecksumMismatch => "la huella SHA-256 de la descarga no coincide".to_string(),
            E::KindleRunning => "la app Kindle está abierta".to_string(),
            E::UnlockFailed { code, tail } => format!("el desbloqueo falló (código {code}):\n{tail}"),
            E::Io(detail) => format!("error de lectura o escritura: {detail}"),
            E::Other(detail) => detail.clone(),
        }
    }

    /// An error from the converter. English uses the library's own wording.
    pub fn kfx_error(&self, error: &anzu_kfx::Error) -> String {
        use anzu_kfx::Error as E;
        if !self.es() {
            return error.to_string();
        }
        match error {
            E::Ion(detail) => format!("datos Ion no válidos: {detail}"),
            E::NotKfx => "el archivo no es KFX".to_string(),
            E::StillEncrypted => "el libro sigue cifrado; hay que desbloquearlo antes".to_string(),
            E::Container(detail) => format!("contenedor KFX no válido: {detail}"),
            E::NoReadingOrder => "el libro no declara un orden de lectura".to_string(),
            E::Io(detail) => format!("error de lectura o escritura: {detail}"),
            E::Other(detail) => detail.clone(),
        }
    }

    /// The words the converter writes into a book.
    pub fn labels(&self) -> anzu_kfx::Labels {
        if self.es() {
            anzu_kfx::Labels {
                contents: "Índice".to_string(),
                chapter: "Capítulo".to_string(),
                untitled: "Sin título".to_string(),
            }
        } else {
            anzu_kfx::Labels::default()
        }
    }

    pub fn unlock_not_done(&self, reason: &str) -> String {
        if self.es() { format!("Desbloqueo no realizado: {reason}") } else { format!("Unlock not done: {reason}") }
    }

    pub fn could_not_close_kindle(&self, detail: &str) -> String {
        if self.es() {
            format!("No se pudo cerrar la app Kindle: {detail}")
        } else {
            format!("Could not close the Kindle app: {detail}")
        }
    }

    pub fn epub_saved(&self, path: &Path) -> String {
        if self.es() { format!("EPUB guardado: {}", path.display()) } else { format!("EPUB saved: {}", path.display()) }
    }

    pub fn could_not_convert(&self, asin: &str, detail: &str) -> String {
        if self.es() {
            format!("{asin}: no se pudo convertir ({detail})")
        } else {
            format!("{asin}: could not convert ({detail})")
        }
    }

    pub fn warning(&self, text: &str) -> String {
        if self.es() { format!("aviso: {text}") } else { format!("warning: {text}") }
    }

    pub fn more_warnings(&self, n: usize) -> String {
        if self.es() { format!("… y {n} avisos más") } else { format!("… and {n} more warnings") }
    }

    pub fn cli_error(&self, error: &CliError) -> String {
        match (self.es(), error) {
            (false, CliError::UnknownCommand(c)) => format!("unknown command: {c}"),
            (false, CliError::UnknownOption(o)) => format!("unknown option: {o}"),
            (false, CliError::MissingValue(o)) => format!("missing folder after {o}"),
            (false, CliError::MissingInput) => "missing the file to convert".to_string(),
            (false, CliError::TooManyInputs) => "only one file can be converted at a time".to_string(),
            (true, CliError::UnknownCommand(c)) => format!("orden desconocida: {c}"),
            (true, CliError::UnknownOption(o)) => format!("opción desconocida: {o}"),
            (true, CliError::MissingValue(o)) => format!("falta la carpeta después de {o}"),
            (true, CliError::MissingInput) => "falta el archivo que convertir".to_string(),
            (true, CliError::TooManyInputs) => "solo se puede convertir un archivo cada vez".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_is_the_default_language() {
        assert_eq!(strings(Language::default()).lang, Language::En);
        assert_eq!(strings(Language::En).recheck, "Check again");
        assert_eq!(strings(Language::Es).recheck, "Volver a comprobar");
    }

    #[test]
    fn counts_use_the_right_plural() {
        assert_eq!(EN.books(1), "1 book");
        assert_eq!(EN.books(3), "3 books");
        assert_eq!(ES.books(1), "1 libro");
        assert_eq!(ES.books(3), "3 libros");
        assert_eq!(EN.selected(0), "None selected");
        assert_eq!(EN.selected(1), "1 selected");
        assert_eq!(ES.selected(1), "1 seleccionado");
        assert_eq!(ES.selected(2), "2 seleccionados");
    }

    #[test]
    fn the_button_says_what_will_happen_and_to_how_many() {
        assert_eq!(EN.run_label(false, 0), "Convert");
        assert_eq!(EN.run_label(false, 1), "Convert 1 book");
        assert_eq!(EN.run_label(true, 3), "Unlock and convert 3 books");
        assert_eq!(ES.run_label(true, 3), "Desbloquear y convertir 3 libros");
    }

    #[test]
    fn sizes_use_the_decimal_separator_of_the_language() {
        assert_eq!(EN.human_size(0), "0 KB");
        assert_eq!(EN.human_size(871_790), "851 KB");
        assert_eq!(EN.human_size(2_973_494), "2.8 MB");
        assert_eq!(ES.human_size(2_973_494), "2,8 MB");
        assert_eq!(EN.human_size(1_500_000_000), "1.4 GB");
    }

    #[test]
    fn every_block_reason_explains_itself_in_both_languages() {
        let reasons = [
            BlockReason::KindleMissing,
            BlockReason::Offline,
            BlockReason::RateLimited,
            BlockReason::NoDigest,
            BlockReason::BadDownload,
            BlockReason::TooNew { kindle: "1.1.6000.0".into(), max_supported: "1.1.5634".into() },
            BlockReason::Unknown,
        ];
        for t in [&EN, &ES] {
            let mut titles = std::collections::HashSet::new();
            for reason in &reasons {
                let (title, body) = t.block_message(reason);
                assert!(!title.is_empty() && !body.is_empty(), "{reason:?}");
                assert!(titles.insert(title.clone()), "repeated title: {title}");
            }
            let (_, body) = t.block_message(&reasons[5]);
            assert!(body.contains("1.1.6000.0") && body.contains("1.1.5634"), "{body}");
        }
        assert_ne!(EN.block_message(&reasons[0]), ES.block_message(&reasons[0]));
    }

    #[test]
    fn library_errors_are_translated_and_keep_their_detail() {
        let error = anzu_core::Error::UnlockFailed { code: -1, tail: "no session".into() };
        assert!(EN.core_error(&error).contains("no session"));
        let spanish = ES.core_error(&error);
        assert!(spanish.contains("no session") && spanish.contains("-1"), "{spanish}");
        assert_ne!(EN.core_error(&error), spanish);
        assert_ne!(EN.kfx_error(&anzu_kfx::Error::NotKfx), ES.kfx_error(&anzu_kfx::Error::NotKfx));
    }

    #[test]
    fn book_labels_follow_the_language() {
        assert_eq!(EN.labels(), anzu_kfx::Labels::default());
        assert_eq!(ES.labels().untitled, "Sin título");
        assert_eq!(ES.labels().chapter, "Capítulo");
    }

    #[test]
    fn every_step_has_a_title_and_a_detail() {
        for t in [&EN, &ES] {
            for step in [Step::Check, Step::Download, Step::CloseKindle, Step::Unlock, Step::Convert] {
                let (title, detail) = t.step(step);
                assert!(!title.is_empty() && !detail.is_empty());
            }
        }
    }

    #[test]
    fn the_withheld_line_matches_what_the_filter_emits() {
        assert_eq!(EN.withheld, anzu_core::unlock::WITHHELD);
    }
}

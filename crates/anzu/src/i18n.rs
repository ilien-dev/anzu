//! Interface text, in English (the default) and Spanish.
//!
//! Every fixed string is a field of [`Strings`], so a language that misses one does not
//! compile. Text with numbers or names in it is built by the methods.

use std::path::Path;

use anzu_core::library::BookState;
use anzu_core::settings::Language;

use crate::app::BlockReason;
use crate::cli::CliError;
use crate::pipeline::{Stage, Step};

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
    pub shelf: &'static str,
    pub shelf_locked: &'static str,
    pub shelf_unlocked: &'static str,
    pub shelf_converted: &'static str,
    pub shelf_show: &'static str,
    pub shelf_hide: &'static str,
    pub unmark_all: &'static str,
    pub title_after_unlock: &'static str,
    pub sort_by: &'static str,
    pub sort_title: &'static str,
    pub sort_asin: &'static str,
    pub sort_size: &'static str,
    /// The hover text of the button that reverses the order.
    pub sort_reverse: &'static str,
    /// The direction of the order, in words: by text, then by size.
    pub sort_a_to_z: &'static str,
    pub sort_z_to_a: &'static str,
    pub sort_smallest: &'static str,
    pub sort_largest: &'static str,
    pub plan_unlock: &'static str,
    pub plan_convert: &'static str,
    pub plan_again: &'static str,
    pub kindle_note: &'static str,
    pub working: &'static str,
    pub finished_band: &'static str,
    pub run_books: &'static str,
    pub failure_note: &'static str,
    pub failed_stays: &'static str,
    /// The last step of a run, while the EPUB files are being written down.
    pub save_step: &'static str,
    pub hide_books: &'static str,
    pub select_all: &'static str,
    pub clear_selection: &'static str,
    pub empty_title: &'static str,
    pub empty_steps: [&'static str; 3],
    pub empty_note: &'static str,
    pub refresh: &'static str,
    pub refresh_hint: &'static str,
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
    pub step_not_completed: &'static str,
    pub step_pending: &'static str,
    pub failed_unlock: &'static str,
    pub failed_convert: &'static str,
    pub converting: &'static str,
    pub unlocking: &'static str,
    pub queued: &'static str,
    pub not_started: &'static str,
    pub preparation: &'static str,
    pub preparation_note: &'static str,
    pub log: &'static str,
    pub copy: &'static str,
    pub back: &'static str,
    pub wait_hint: &'static str,
    pub step_done: &'static str,
    pub step_skipped: &'static str,
    pub step_running: &'static str,
    pub show_in_folder: &'static str,
    pub open_epub: &'static str,
    pub convert_more: &'static str,
    pub again_badge: &'static str,
    pub leave_out_one: &'static str,
    pub leave_out_many: &'static str,
    pub window_error: &'static str,
    pub withheld: &'static str,
    pub tool_could_not_unlock: &'static str,
    pub kindle_left_open: &'static str,
    pub compatibility_unconfirmed: &'static str,
    pub usage: &'static str,
    pub settings: &'static str,
    pub version: &'static str,
    pub check_updates: &'static str,
    pub update_checking: &'static str,
    pub update_current: &'static str,
    pub update_schedule: &'static str,
    pub update_new: &'static str,
    pub update_body: &'static str,
    pub update_download: &'static str,
    pub update_later: &'static str,
    pub update_notes: &'static str,
    pub update_ready_body: &'static str,
    pub update_restart: &'static str,
    pub update_wait_run: &'static str,
    pub update_retry: &'static str,
    pub update_check_failed: &'static str,
    pub update_download_failed: &'static str,
    pub update_install_failed: &'static str,
    /// Each language named in itself, so it can be found from either one.
    pub language_en: &'static str,
    pub language_es: &'static str,
    pub setup_window_title: &'static str,
    pub setup_badge: &'static str,
    pub setup_step_language: &'static str,
    pub setup_step_folder: &'static str,
    pub setup_step_install: &'static str,
    pub setup_step_done: &'static str,
    pub setup_title: &'static str,
    pub setup_intro: &'static str,
    pub setup_language: &'static str,
    pub setup_language_note: &'static str,
    pub setup_folder_title: &'static str,
    pub setup_folder_intro: &'static str,
    pub setup_folder_label: &'static str,
    pub setup_folder_dialog: &'static str,
    pub setup_change_folder: &'static str,
    pub setup_desktop: &'static str,
    pub setup_start_menu: &'static str,
    pub setup_installing_title: &'static str,
    pub setup_installing_intro: &'static str,
    pub setup_task_copy: &'static str,
    pub setup_task_shortcuts: &'static str,
    pub setup_task_register: &'static str,
    pub setup_state_done: &'static str,
    pub setup_state_running: &'static str,
    pub setup_state_pending: &'static str,
    pub setup_done_title: &'static str,
    pub setup_done_body: &'static str,
    pub setup_installed_in: &'static str,
    pub setup_failed_title: &'static str,
    pub setup_continue: &'static str,
    pub setup_back: &'static str,
    pub setup_install: &'static str,
    pub setup_installing_button: &'static str,
    pub setup_open: &'static str,
    pub setup_close: &'static str,
    pub setup_retry: &'static str,
    pub uninstall_title: &'static str,
    pub uninstall_confirm: &'static str,
    pub uninstall_data: &'static str,
    pub uninstall_done: &'static str,
    pub update_failed_title: &'static str,
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
    shelf: "Your shelf",
    shelf_locked: "To unlock",
    shelf_unlocked: "Ready for EPUB",
    shelf_converted: "In EPUB",
    shelf_show: "Show",
    shelf_hide: "Hide",
    unmark_all: "Unmark all",
    title_after_unlock: "title once unlocked",
    sort_by: "Sort by",
    sort_title: "Title",
    sort_asin: "ASIN",
    sort_size: "Size",
    sort_reverse: "Reverse the order",
    sort_a_to_z: "A to Z",
    sort_z_to_a: "Z to A",
    sort_smallest: "Smallest first",
    sort_largest: "Largest first",
    plan_unlock: "unlock + EPUB",
    plan_convert: "EPUB only",
    plan_again: "EPUB again",
    kindle_note: "Unlocking needs the Kindle app closed. Anzu asks you first.",
    working: "Working · do not open the Kindle app until it finishes",
    finished_band: "Finished · you can open the Kindle app again",
    run_books: "Books in this run",
    failure_note: "If a book fails, it is marked with the reason and the others go on.",
    failed_stays: "A book with an error stays on your shelf: you can try it again whenever you like.",
    save_step: "Save to your folder",
    hide_books: "Hide the books",
    select_all: "Select all",
    clear_selection: "Clear the selection",
    empty_title: "No books to convert yet",
    empty_steps: [
        "1. Open the Kindle app from the Microsoft Store and sign in.",
        "2. Download in it the books you want to convert.",
        "3. Come back here and press “Refresh list”.",
    ],
    empty_note: "Anzu only reads that folder: it never changes or deletes your Kindle books.",
    refresh: "Refresh list",
    refresh_hint: "Reads the Kindle folder again to show the books you have just downloaded.",
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
    step_not_completed: "Not completed",
    step_pending: "Pending",
    failed_unlock: "Could not unlock",
    failed_convert: "Could not convert",
    converting: "Converting…",
    unlocking: "Unlocking…",
    queued: "Queued",
    not_started: "Not started",
    preparation: "Preparation",
    preparation_note: "Once for all the books",
    log: "Log",
    copy: "Copy",
    back: "Back to the books",
    wait_hint: "Wait until it finishes",
    step_done: "Done",
    step_skipped: "Skipped",
    step_running: "Running",
    show_in_folder: "Show in folder",
    open_epub: "Open EPUB",
    convert_more: "Convert more books",
    again_badge: "AGAIN",
    leave_out_one: "Leave it out",
    leave_out_many: "Leave them out",
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
    settings: "Settings",
    version: "Version",
    check_updates: "Check for updates",
    update_checking: "Checking for updates…",
    update_current: "You have the latest version",
    update_schedule: "Anzu checks every hour while it is open",
    update_new: "New update",
    update_body: "It downloads while you keep using Anzu. Then you only need to restart.",
    update_download: "Download",
    update_later: "Later",
    update_notes: "What's new",
    update_ready_body: "File verified. Anzu will close, update and open again.",
    update_restart: "Restart and update",
    update_wait_run: "You can restart when the current run finishes.",
    update_retry: "Try again",
    update_check_failed: "Could not check for updates",
    update_download_failed: "The download failed",
    update_install_failed: "The installer could not be started",
    language_en: "English",
    language_es: "Español",
    setup_window_title: "Install Anzu",
    setup_badge: "installer",
    setup_step_language: "Language",
    setup_step_folder: "Folder",
    setup_step_install: "Install",
    setup_step_done: "Done",
    setup_title: "Let's install Anzu",
    setup_intro: "It converts the books you download with the Kindle app to EPUB. Installing takes a few seconds and needs no administrator rights.",
    setup_language: "Language of Anzu",
    setup_language_note: "This installer and Anzu will use this language. You can change it later in Settings.",
    setup_folder_title: "Where should it go?",
    setup_folder_intro: "The proposed folder belongs to your user only, so Anzu can update itself.",
    setup_folder_label: "Installation folder",
    setup_folder_dialog: "Choose where to install Anzu",
    setup_change_folder: "Change folder",
    setup_desktop: "Shortcut on the desktop",
    setup_start_menu: "Shortcut in the Start menu",
    setup_installing_title: "Installing Anzu",
    setup_installing_intro: "Just a few seconds.",
    setup_task_copy: "Copy Anzu to the folder",
    setup_task_shortcuts: "Create the shortcuts",
    setup_task_register: "Register in the apps of Windows",
    setup_state_done: "done",
    setup_state_running: "in progress",
    setup_state_pending: "pending",
    setup_done_title: "Anzu is installed",
    setup_done_body: "You will find it in the Start menu. When there is a new version, Anzu will let you know and update itself with one click.",
    setup_installed_in: "Installed in",
    setup_failed_title: "The installation did not finish",
    setup_continue: "Continue",
    setup_back: "Back",
    setup_install: "Install",
    setup_installing_button: "Installing…",
    setup_open: "Open Anzu",
    setup_close: "Close",
    setup_retry: "Try again",
    uninstall_title: "Uninstall Anzu",
    uninstall_confirm: "Remove Anzu from this computer? Your EPUB files are not touched.",
    uninstall_data: "Also delete the data of Anzu? It holds the keys of your Amazon account and the unlocked copies of your books. Choose No to keep them for a later installation.",
    uninstall_done: "Anzu was uninstalled.",
    update_failed_title: "Anzu could not be updated",
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
    shelf: "Tu estantería",
    shelf_locked: "Por desbloquear",
    shelf_unlocked: "Listos para EPUB",
    shelf_converted: "En EPUB",
    shelf_show: "Mostrar",
    shelf_hide: "Ocultar",
    unmark_all: "Desmarcar todos",
    title_after_unlock: "título al desbloquear",
    sort_by: "Ordenar por",
    sort_title: "Título",
    sort_asin: "ASIN",
    sort_size: "Tamaño",
    sort_reverse: "Invertir el orden",
    sort_a_to_z: "De la A a la Z",
    sort_z_to_a: "De la Z a la A",
    sort_smallest: "Más pequeños primero",
    sort_largest: "Más grandes primero",
    plan_unlock: "desbloquear + EPUB",
    plan_convert: "solo EPUB",
    plan_again: "EPUB de nuevo",
    kindle_note: "Para desbloquear hay que cerrar la app Kindle. Anzu te lo pregunta antes.",
    working: "Trabajando · no abras la app Kindle hasta que termine",
    finished_band: "Terminado · ya puedes volver a abrir la app Kindle",
    run_books: "Libros de la tanda",
    failure_note: "Si un libro falla, se marca con el motivo y los demás siguen.",
    failed_stays: "Un libro con error sigue en tu estantería: puedes volver a intentarlo cuando quieras.",
    save_step: "Guardar en tu carpeta",
    hide_books: "Ocultar los libros",
    select_all: "Seleccionar todos",
    clear_selection: "Quitar la selección",
    empty_title: "Aún no hay libros que convertir",
    empty_steps: [
        "1. Abre la app Kindle de Microsoft Store e inicia sesión.",
        "2. Descarga en ella los libros que quieras convertir.",
        "3. Vuelve aquí y pulsa «Actualizar lista».",
    ],
    empty_note: "Anzu solo lee esa carpeta: nunca cambia ni borra tus libros de Kindle.",
    refresh: "Actualizar lista",
    refresh_hint: "Vuelve a leer la carpeta de Kindle para mostrar los libros que acabas de descargar.",
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
    step_not_completed: "No se completó",
    step_pending: "Pendiente",
    failed_unlock: "No se pudo desbloquear",
    failed_convert: "No se pudo convertir",
    converting: "Convirtiendo…",
    unlocking: "Desbloqueando…",
    queued: "En cola",
    not_started: "Sin empezar",
    preparation: "Preparación",
    preparation_note: "Una sola vez para todos los libros",
    log: "Registro",
    copy: "Copiar",
    back: "Volver a los libros",
    wait_hint: "Espera a que termine",
    step_done: "Hecho",
    step_skipped: "Saltado",
    step_running: "En curso",
    show_in_folder: "Mostrar en la carpeta",
    open_epub: "Abrir EPUB",
    convert_more: "Convertir más libros",
    again_badge: "DE NUEVO",
    leave_out_one: "Dejarlo fuera",
    leave_out_many: "Dejarlos fuera",
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
    settings: "Ajustes",
    version: "Versión",
    check_updates: "Buscar actualizaciones",
    update_checking: "Buscando actualizaciones…",
    update_current: "Tienes la última versión",
    update_schedule: "Anzu lo comprueba cada hora mientras está abierta",
    update_new: "Nueva actualización",
    update_body: "Se descarga mientras sigues usando Anzu. Después solo tienes que reiniciar.",
    update_download: "Descargar",
    update_later: "Más tarde",
    update_notes: "Novedades",
    update_ready_body: "Archivo verificado. Anzu se cerrará, se actualizará y volverá a abrirse.",
    update_restart: "Reiniciar y actualizar",
    update_wait_run: "Podrás reiniciar cuando termine la conversión en curso.",
    update_retry: "Reintentar",
    update_check_failed: "No se pudo buscar actualizaciones",
    update_download_failed: "La descarga falló",
    update_install_failed: "No se pudo abrir el instalador",
    language_en: "English",
    language_es: "Español",
    setup_window_title: "Instalar Anzu",
    setup_badge: "instalador",
    setup_step_language: "Idioma",
    setup_step_folder: "Carpeta",
    setup_step_install: "Instalar",
    setup_step_done: "Listo",
    setup_title: "Vamos a instalar Anzu",
    setup_intro: "Convierte a EPUB los libros que descargas con la app Kindle. La instalación tarda unos segundos y no pide permisos de administrador.",
    setup_language: "Idioma de Anzu",
    setup_language_note: "Este instalador y Anzu se mostrarán en este idioma. Puedes cambiarlo después en Ajustes.",
    setup_folder_title: "¿Dónde la instalamos?",
    setup_folder_intro: "La carpeta propuesta es solo para tu usuario, así Anzu puede actualizarse sola.",
    setup_folder_label: "Carpeta de instalación",
    setup_folder_dialog: "Elige dónde instalar Anzu",
    setup_change_folder: "Cambiar carpeta",
    setup_desktop: "Acceso directo en el escritorio",
    setup_start_menu: "Acceso en el menú Inicio",
    setup_installing_title: "Instalando Anzu",
    setup_installing_intro: "Solo unos segundos.",
    setup_task_copy: "Copiar Anzu a la carpeta",
    setup_task_shortcuts: "Crear accesos directos",
    setup_task_register: "Registrar en Aplicaciones de Windows",
    setup_state_done: "hecho",
    setup_state_running: "en curso",
    setup_state_pending: "pendiente",
    setup_done_title: "Anzu está instalada",
    setup_done_body: "La encontrarás en el menú Inicio. Cuando haya una versión nueva, Anzu te avisará y se actualizará con un clic.",
    setup_installed_in: "Instalada en",
    setup_failed_title: "La instalación no terminó",
    setup_continue: "Continuar",
    setup_back: "Atrás",
    setup_install: "Instalar",
    setup_installing_button: "Instalando…",
    setup_open: "Abrir Anzu",
    setup_close: "Cerrar",
    setup_retry: "Reintentar",
    uninstall_title: "Desinstalar Anzu",
    uninstall_confirm: "¿Quitar Anzu de este equipo? Tus archivos EPUB no se tocan.",
    uninstall_data: "¿Borrar también los datos de Anzu? Guardan las claves de tu cuenta de Amazon y las copias desbloqueadas de tus libros. Elige No para conservarlos para otra instalación.",
    uninstall_done: "Anzu se desinstaló.",
    update_failed_title: "No se pudo actualizar Anzu",
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

    /// What the last "Refresh list" found: "No new books", "1 new book", "3 new books".
    pub fn new_books(&self, n: usize) -> String {
        match (self.es(), n) {
            (false, 0) => "No new books".to_string(),
            (false, 1) => "1 new book".to_string(),
            (false, n) => format!("{n} new books"),
            (true, 0) => "No hay libros nuevos".to_string(),
            (true, 1) => "1 libro nuevo".to_string(),
            (true, n) => format!("{n} libros nuevos"),
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

    /// "Anzu 0.4.0 is available. You have 0.3.0."
    pub fn update_available(&self, version: &str, current: &str) -> String {
        if self.es() {
            format!("Anzu {version} está disponible. Tienes la {current}.")
        } else {
            format!("Anzu {version} is available. You have {current}.")
        }
    }

    pub fn update_downloading(&self, version: &str) -> String {
        if self.es() { format!("Descargando Anzu {version}") } else { format!("Downloading Anzu {version}") }
    }

    /// "7.2 of 12.4 MB", or only what has arrived when the size is unknown.
    pub fn update_progress(&self, done: u64, total: u64) -> String {
        if total == 0 {
            return self.human_size(done);
        }
        let of = if self.es() { "de" } else { "of" };
        format!("{} {of} {}", self.human_size(done), self.human_size(total))
    }

    pub fn update_ready(&self, version: &str) -> String {
        if self.es() { format!("Anzu {version} está lista") } else { format!("Anzu {version} is ready") }
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
            (true, Step::Check) => ("Comprobar versiones", "Kindle instalada y última herramienta publicada"),
            (true, Step::Download) => ("Descargar la herramienta", "Con su huella SHA-256 verificada"),
            (true, Step::CloseKindle) => ("Cerrar la app Kindle", "Solo si la confirmas"),
        }
    }

    /// Title and detail of what is done to each book.
    pub fn stage(&self, stage: Stage) -> (&'static str, &'static str) {
        match (self.es(), stage) {
            (false, Stage::Unlock) => ("Unlock", "It can take a minute per book"),
            (false, Stage::Convert) => ("Convert to EPUB", "With Anzu's own converter"),
            (true, Stage::Unlock) => ("Desbloquear", "Puede tardar un minuto por libro"),
            (true, Stage::Convert) => ("Convertir a EPUB", "Con el conversor de Anzu"),
        }
    }

    /// "Unlocking 3 books": the tool works on all of them at once.
    pub fn unlocking_books(&self, n: usize) -> String {
        if self.es() { format!("Desbloqueando {}", self.books(n)) } else { format!("Unlocking {}", self.books(n)) }
    }

    /// "Converting Dune".
    pub fn converting_book(&self, title: &str) -> String {
        if self.es() { format!("Convirtiendo {title}") } else { format!("Converting {title}") }
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

    /// "Mark all 3".
    pub fn mark_all(&self, n: usize) -> String {
        match (self.es(), n) {
            (false, 1) => "Mark it".to_string(),
            (false, n) => format!("Mark all {n}"),
            (true, 1) => "Marcarlo".to_string(),
            (true, n) => format!("Marcar los {n}"),
        }
    }

    /// "and 3 more", below a list cut short.
    pub fn and_more(&self, n: usize) -> String {
        if self.es() { format!("y {n} más") } else { format!("and {n} more") }
    }

    /// "Step 2 of 5".
    pub fn step_of(&self, n: usize, total: usize) -> String {
        if self.es() { format!("Paso {n} de {total}") } else { format!("Step {n} of {total}") }
    }

    /// The unlock step of a run: "Unlock 2 books".
    pub fn unlock_step(&self, n: usize) -> String {
        if self.es() { format!("Desbloquear {}", self.books(n)) } else { format!("Unlock {}", self.books(n)) }
    }

    /// The convert step of a run: "Convert 3 books to EPUB".
    pub fn convert_step(&self, n: usize) -> String {
        if self.es() { format!("Convertir {} a EPUB", self.books(n)) } else { format!("Convert {} to EPUB", self.books(n)) }
    }

    /// How far a step of the run has got: "1 of 2 done".
    pub fn done_of(&self, done: usize, total: usize) -> String {
        if self.es() { format!("{done} de {total} hechos") } else { format!("{done} of {total} done") }
    }

    /// "Preparing: step 2 of 3".
    pub fn preparing(&self, n: usize, total: usize) -> String {
        if self.es() { format!("Preparando: paso {n} de {total}") } else { format!("Preparing: step {n} of {total}") }
    }

    /// "Book 2 of 4".
    pub fn book_of(&self, n: usize, total: usize) -> String {
        if self.es() { format!("Libro {n} de {total}") } else { format!("Book {n} of {total}") }
    }

    /// How many books are converted at the same time: "4 at a time".
    pub fn at_a_time(&self, n: usize) -> String {
        match (self.es(), n) {
            (false, 1) => "One at a time".to_string(),
            (false, n) => format!("{n} at a time"),
            (true, 1) => "Uno a uno".to_string(),
            (true, n) => format!("{n} a la vez"),
        }
    }

    /// The EPUB files that got a number because their name was taken.
    pub fn copies_note(&self, n: usize) -> String {
        match (self.es(), n) {
            (false, 1) => "1 was saved with a number at the end: an EPUB with that name was already there.".to_string(),
            (false, n) => format!("{n} were saved with a number at the end: an EPUB with that name was already there."),
            (true, 1) => "1 se guardó con un número al final: ya había un EPUB con ese nombre.".to_string(),
            (true, n) => format!("{n} se guardaron con un número al final: ya había un EPUB con ese nombre."),
        }
    }

    /// The link that unfolds the list of books of a finished run.
    pub fn see_books(&self, n: usize) -> String {
        if self.es() { format!("Ver los {n} libros") } else { format!("See the {n} books") }
    }

    /// The heading of the result: one EPUB or several.
    pub fn epubs_ready(&self, n: usize) -> String {
        match (self.es(), n) {
            (false, 1) => "Your EPUB is ready".to_string(),
            (false, n) => format!("{n} EPUBs are ready"),
            (true, 1) => "Tu EPUB está listo".to_string(),
            (true, n) => format!("{n} EPUB listos"),
        }
    }

    /// "in C:\Users\…\EPUB": the folder a file was saved in.
    pub fn in_folder(&self, dir: &Path) -> String {
        if self.es() { format!("en {}", dir.display()) } else { format!("in {}", dir.display()) }
    }

    /// The link that shows or hides the log.
    pub fn log_toggle(&self, open: bool, lines: usize) -> String {
        match (self.es(), open, lines) {
            (false, true, _) => "Hide the technical log".to_string(),
            (false, false, 1) => "Show the technical log (1 line)".to_string(),
            (false, false, n) => format!("Show the technical log ({n} lines)"),
            (true, true, _) => "Ocultar el registro técnico".to_string(),
            (true, false, 1) => "Ver el registro técnico (1 línea)".to_string(),
            (true, false, n) => format!("Ver el registro técnico ({n} líneas)"),
        }
    }

    /// The main button, saying how many of the books already have an EPUB.
    pub fn run_button(&self, needs_unlock: bool, n: usize, again: usize) -> String {
        let label = self.run_label(needs_unlock, n);
        match (self.es(), again) {
            (_, 0) => label,
            (false, a) => format!("{label} · {a} again"),
            (true, a) => format!("{label} · {a} de nuevo"),
        }
    }

    /// "A, B and C", or "A, B and 3 more" when the list is long.
    fn name_list(&self, names: &[String]) -> String {
        let and = if self.es() { "y" } else { "and" };
        match names {
            [] => String::new(),
            [one] => one.clone(),
            [rest @ .., last] if names.len() <= 3 => format!("{} {and} {last}", rest.join(", ")),
            _ => {
                let more = if self.es() { format!("{} más", names.len() - 2) } else { format!("{} more", names.len() - 2) };
                format!("{} {and} {more}", names[..2].join(", "))
            }
        }
    }

    /// The notice above the main button when some chosen books already have an EPUB.
    pub fn already_epub(&self, names: &[String], selected: usize) -> (String, &'static str) {
        let all = names.len() == selected;
        let title = match (self.es(), names.len()) {
            (false, 1) => format!("{} already has an EPUB.", names[0]),
            (true, 1) => format!("{} ya tiene un EPUB.", names[0]),
            (false, n) if all => format!("The {n} chosen books already have an EPUB: {}.", self.name_list(names)),
            (true, n) if all => format!("Los {n} libros elegidos ya tienen un EPUB: {}.", self.name_list(names)),
            (false, n) => format!("{n} of the chosen books already have an EPUB: {}.", self.name_list(names)),
            (true, n) => format!("{n} de los libros elegidos ya tienen un EPUB: {}.", self.name_list(names)),
        };
        let body = match (self.es(), names.len() == 1, all) {
            (false, true, true) => "Converting it again saves a second copy next to the old one.",
            (false, false, true) => "Converting them again saves a second copy next to each old one.",
            (false, true, false) => "Converting it again saves a second copy. Leave it out to convert only the new ones.",
            (false, false, false) => {
                "Converting them again saves a second copy of each. Leave them out to convert only the new ones."
            }
            (true, true, true) => "Convertirlo de nuevo guarda una segunda copia junto a la anterior.",
            (true, false, true) => "Convertirlos de nuevo guarda una segunda copia junto a cada anterior.",
            (true, true, false) => "Convertirlo de nuevo guarda una segunda copia. Déjalo fuera para convertir solo los nuevos.",
            (true, false, false) => {
                "Convertirlos de nuevo guarda una segunda copia de cada uno. Déjalos fuera para convertir solo los nuevos."
            }
        };
        (title, body)
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
            E::CloseRefused(said) => format!("taskkill no aceptó la petición: {said}"),
            E::CloseTimedOut(secs) => format!("seguía abierta {secs} s después de pedirle que se cerrara"),
            E::InstallerTooLarge => "el instalador es más grande de lo esperado".to_string(),
            E::InstallInUse => "la copia instalada de Anzu está en uso; ciérrala y vuelve a intentarlo".to_string(),
            E::Shortcut(detail) => format!("no se pudo crear el acceso directo: {detail}"),
            E::Registry(detail) => format!("no se pudo registrar la app en Windows: {detail}"),
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
        assert_eq!(EN.new_books(0), "No new books");
        assert_eq!(EN.new_books(1), "1 new book");
        assert_eq!(EN.new_books(4), "4 new books");
        assert_eq!(ES.new_books(0), "No hay libros nuevos");
        assert_eq!(ES.new_books(1), "1 libro nuevo");
        assert_eq!(ES.new_books(4), "4 libros nuevos");
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
    fn the_notice_names_the_books_that_already_have_an_epub() {
        let names = |n: &[&str]| n.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(EN.already_epub(&names(&["Dune"]), 2).0, "Dune already has an EPUB.");
        assert_eq!(
            EN.already_epub(&names(&["A", "B", "C"]), 5).0,
            "3 of the chosen books already have an EPUB: A, B and C."
        );
        assert_eq!(
            ES.already_epub(&names(&["A", "B", "C", "D"]), 4).0,
            "Los 4 libros elegidos ya tienen un EPUB: A, B y 2 más."
        );
        assert!(EN.already_epub(&names(&["A"]), 2).1.contains("Leave it out"));
        assert!(!EN.already_epub(&names(&["A"]), 1).1.contains("Leave"), "nothing new to keep when all are converted");
        assert_eq!(EN.run_button(false, 2, 1), "Convert 2 books · 1 again");
        assert_eq!(ES.run_button(false, 2, 0), "Convertir 2 libros");
    }

    #[test]
    fn every_step_has_a_title_and_a_detail() {
        for t in [&EN, &ES] {
            for step in [Step::Check, Step::Download, Step::CloseKindle] {
                let (title, detail) = t.step(step);
                assert!(!title.is_empty() && !detail.is_empty());
            }
            for stage in [Stage::Unlock, Stage::Convert] {
                let (title, detail) = t.stage(stage);
                assert!(!title.is_empty() && !detail.is_empty());
            }
        }
    }

    #[test]
    fn the_withheld_line_matches_what_the_filter_emits() {
        assert_eq!(EN.withheld, anzu_core::unlock::WITHHELD);
    }
}

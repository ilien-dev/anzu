//! Estado de la app y las reglas que no dependen de la interfaz.

use seshat_core::compat::{self, Compat};
use seshat_core::kindle::KindleInstall;
use seshat_core::library::{Book, BookState};
use seshat_core::releases::Release;
use seshat_core::{Error, KindleVersion};

/// Si el sistema está en condiciones de desbloquear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Checking,
    Ready { kindle: KindleVersion, tool_tag: String, exe_name: String },
    Blocked(BlockReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockReason {
    KindleMissing,
    Offline,
    RateLimited,
    NoDigest,
    BadDownload,
    TooNew { kindle: String, max_supported: String },
    Unknown,
}

impl BlockReason {
    /// Título y explicación para mostrar al usuario.
    pub fn message(&self) -> (String, String) {
        let (title, body) = match self {
            BlockReason::KindleMissing => (
                "No se encontró la app Kindle",
                "Instala «Amazon Kindle» desde Microsoft Store, inicia sesión y descarga tus libros. \
                 Después pulsa «Volver a comprobar»."
                    .to_string(),
            ),
            BlockReason::Offline => (
                "Sin conexión",
                "No se pudo consultar la última versión de la herramienta de desbloqueo. Sin esa comprobación \
                 no se desbloquea nada. Los libros ya desbloqueados se pueden convertir igualmente."
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
                "La última publicación de la herramienta no trae huella SHA-256, así que no hay forma de \
                 comprobar que el archivo es el original. Por seguridad no se ejecutará."
                    .to_string(),
            ),
            BlockReason::BadDownload => (
                "La descarga no coincide con la original",
                "La huella del archivo descargado no es la que publica GitHub. Puede ser una descarga \
                 corrupta o manipulada, y no se ejecutará. Pulsa «Volver a comprobar» para intentarlo de nuevo."
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
                "No fue posible determinar si la herramienta admite tu versión de Kindle. \
                 Ante la duda, el desbloqueo queda bloqueado."
                    .to_string(),
            ),
        };
        (title.to_string(), body)
    }
}

fn evaluate(
    kindle: Option<&KindleInstall>,
    release: Result<&Release, &Error>,
    exe_names: &[String],
) -> Result<(KindleVersion, String, String), BlockReason> {
    let kindle = kindle.ok_or(BlockReason::KindleMissing)?;
    let release = release.map_err(|e| match e {
        Error::Network(_) => BlockReason::Offline,
        Error::RateLimited => BlockReason::RateLimited,
        Error::NoDigest => BlockReason::NoDigest,
        Error::ChecksumMismatch => BlockReason::BadDownload,
        _ => BlockReason::Unknown,
    })?;
    let zip = release.tools_zip().ok_or(BlockReason::Unknown)?;
    if zip.sha256.is_none() {
        return Err(BlockReason::NoDigest);
    }
    match compat::check(&kindle.version, exe_names) {
        Compat::Compatible { exe_name } => Ok((kindle.version, release.tag.clone(), exe_name)),
        Compat::TooNew { max_supported } => {
            Err(BlockReason::TooNew { kindle: kindle.version.to_string(), max_supported })
        }
        Compat::Unknown => Err(BlockReason::Unknown),
    }
}

/// Por qué no se puede desbloquear, o `None` si todo está en orden.
#[cfg_attr(not(test), allow(dead_code))] // la app usa `health`; esto queda como consulta directa
pub fn block_reason(
    kindle: Option<&KindleInstall>,
    release: Result<&Release, &Error>,
    exe_names: &[String],
) -> Option<BlockReason> {
    evaluate(kindle, release, exe_names).err()
}

pub fn health(kindle: Option<&KindleInstall>, release: Result<&Release, &Error>, exe_names: &[String]) -> Health {
    match evaluate(kindle, release, exe_names) {
        Ok((kindle, tool_tag, exe_name)) => Health::Ready { kindle, tool_tag, exe_name },
        Err(reason) => Health::Blocked(reason),
    }
}

/// Un libro con DRM solo se puede elegir si el sistema está listo para desbloquear.
pub fn selectable(state: BookState, ready: bool) -> bool {
    ready || state != BookState::Locked
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPlan {
    pub needs_unlock: bool,
    /// Libros que se procesarán, en el orden de la biblioteca.
    pub asins: Vec<String>,
}

pub fn plan_run(books: &[Book], selected: &[String]) -> RunPlan {
    let chosen: Vec<&Book> = books.iter().filter(|b| selected.contains(&b.asin)).collect();
    RunPlan {
        needs_unlock: chosen.iter().any(|b| b.state == BookState::Locked),
        asins: chosen.iter().map(|b| b.asin.clone()).collect(),
    }
}

pub fn action_label(plan: &RunPlan) -> &'static str {
    if plan.needs_unlock { "Desbloquear y convertir" } else { "Convertir" }
}

/// Tamaño legible, con coma decimal.
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let value = bytes as f64;
    if value < KB * KB {
        format!("{} KB", (value / KB).floor())
    } else if value < KB * KB * KB {
        format!("{:.1} MB", value / (KB * KB)).replace('.', ",")
    } else {
        format!("{:.1} GB", value / (KB * KB * KB)).replace('.', ",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seshat_core::releases::Asset;
    use std::path::PathBuf;

    fn kindle(version: &str) -> KindleInstall {
        KindleInstall { version: version.parse().unwrap(), content_dir: PathBuf::from("Content") }
    }

    fn release(sha256: Option<&str>) -> Release {
        Release {
            tag: "v10.0.35".into(),
            published_at: String::new(),
            notes: String::new(),
            assets: vec![Asset { name: "DeDRM_tools.zip".into(), url: "u".into(), sha256: sha256.map(String::from) }],
        }
    }

    fn names() -> Vec<String> {
        ["KFXArchiver291.exe", "MSIXKFXArchiver1_23620_final.exe", "MSIXKFXArchiver_x64_1_5634.exe"]
            .map(String::from)
            .to_vec()
    }

    fn book(asin: &str, state: BookState) -> Book {
        Book {
            asin: asin.into(),
            title: None,
            source: PathBuf::from("x.azw"),
            size: 1,
            state,
            unlocked: None,
            epub: None,
        }
    }

    #[test]
    fn this_machine_is_ready() {
        let k = kindle("1.1.5634.0");
        let r = release(Some("abc"));
        assert_eq!(block_reason(Some(&k), Ok(&r), &names()), None);
        assert_eq!(
            health(Some(&k), Ok(&r), &names()),
            Health::Ready {
                kindle: k.version,
                tool_tag: "v10.0.35".into(),
                exe_name: "MSIXKFXArchiver_x64_1_5634.exe".into()
            }
        );
    }

    #[test]
    fn missing_kindle_blocks_first() {
        let r = release(Some("abc"));
        assert_eq!(block_reason(None, Ok(&r), &names()), Some(BlockReason::KindleMissing));
        assert_eq!(block_reason(None, Err(&Error::Network("x".into())), &[]), Some(BlockReason::KindleMissing));
    }

    #[test]
    fn network_problems_block() {
        let k = kindle("1.1.5634.0");
        assert_eq!(block_reason(Some(&k), Err(&Error::Network("sin red".into())), &[]), Some(BlockReason::Offline));
        assert_eq!(block_reason(Some(&k), Err(&Error::RateLimited), &[]), Some(BlockReason::RateLimited));
        assert_eq!(block_reason(Some(&k), Err(&Error::NoRelease), &[]), Some(BlockReason::Unknown));
    }

    #[test]
    fn unverifiable_downloads_block() {
        let k = kindle("1.1.5634.0");
        assert_eq!(block_reason(Some(&k), Ok(&release(None)), &names()), Some(BlockReason::NoDigest));
        assert_eq!(block_reason(Some(&k), Err(&Error::NoDigest), &[]), Some(BlockReason::NoDigest));
        assert_eq!(block_reason(Some(&k), Err(&Error::ChecksumMismatch), &[]), Some(BlockReason::BadDownload));
    }

    #[test]
    fn newer_kindle_blocks_with_both_versions() {
        let k = kindle("1.1.6000.0");
        let r = release(Some("abc"));
        assert_eq!(
            block_reason(Some(&k), Ok(&r), &names()),
            Some(BlockReason::TooNew { kindle: "1.1.6000.0".into(), max_supported: "1.1.5634".into() })
        );
        assert!(matches!(health(Some(&k), Ok(&r), &names()), Health::Blocked(BlockReason::TooNew { .. })));
    }

    #[test]
    fn uninterpretable_tool_blocks() {
        let k = kindle("1.1.5634.0");
        let r = release(Some("abc"));
        assert_eq!(block_reason(Some(&k), Ok(&r), &["NewTool.exe".to_string()]), Some(BlockReason::Unknown));
        assert_eq!(block_reason(Some(&k), Ok(&r), &[]), Some(BlockReason::Unknown));
    }

    #[test]
    fn every_block_reason_explains_itself() {
        let reasons = [
            BlockReason::KindleMissing,
            BlockReason::Offline,
            BlockReason::RateLimited,
            BlockReason::NoDigest,
            BlockReason::BadDownload,
            BlockReason::TooNew { kindle: "1.1.6000.0".into(), max_supported: "1.1.5634".into() },
            BlockReason::Unknown,
        ];
        let mut titles = std::collections::HashSet::new();
        for reason in &reasons {
            let (title, body) = reason.message();
            assert!(!title.is_empty() && !body.is_empty(), "{reason:?}");
            assert!(titles.insert(title.clone()), "título repetido: {title}");
        }
        let (_, body) = reasons[5].message();
        assert!(body.contains("1.1.6000.0") && body.contains("1.1.5634"), "{body}");
    }

    #[test]
    fn locked_books_need_a_ready_system() {
        assert!(selectable(BookState::Locked, true));
        assert!(!selectable(BookState::Locked, false));
        for ready in [true, false] {
            assert!(selectable(BookState::Unlocked, ready));
            assert!(selectable(BookState::Converted, ready), "se puede volver a convertir");
        }
    }

    #[test]
    fn a_run_unlocks_only_when_a_selected_book_is_locked() {
        let books = [book("A", BookState::Locked), book("B", BookState::Unlocked), book("C", BookState::Converted)];
        let selected: Vec<String> = vec!["B".into(), "C".into()];
        assert_eq!(plan_run(&books, &selected), RunPlan { needs_unlock: false, asins: vec!["B".into(), "C".into()] });

        let selected: Vec<String> = vec!["A".into(), "B".into()];
        assert_eq!(plan_run(&books, &selected), RunPlan { needs_unlock: true, asins: vec!["A".into(), "B".into()] });
    }

    #[test]
    fn a_run_ignores_selections_that_are_no_longer_in_the_library() {
        let books = [book("A", BookState::Unlocked)];
        let selected: Vec<String> = vec!["GONE".into(), "A".into()];
        assert_eq!(plan_run(&books, &selected), RunPlan { needs_unlock: false, asins: vec!["A".into()] });
        assert_eq!(plan_run(&books, &[]), RunPlan { needs_unlock: false, asins: vec![] });
    }

    #[test]
    fn action_label_says_what_will_happen() {
        assert_eq!(action_label(&RunPlan { needs_unlock: true, asins: vec!["A".into()] }), "Desbloquear y convertir");
        assert_eq!(action_label(&RunPlan { needs_unlock: false, asins: vec!["A".into()] }), "Convertir");
    }

    #[test]
    fn sizes_are_readable() {
        assert_eq!(human_size(0), "0 KB");
        assert_eq!(human_size(871_790), "851 KB");
        assert_eq!(human_size(2_973_494), "2,8 MB");
        assert_eq!(human_size(1_500_000_000), "1,4 GB");
    }
}

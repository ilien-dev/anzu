//! App state and the rules that do not depend on the interface.

use anzu_core::compat::{self, Compat};
use anzu_core::kindle::KindleInstall;
use anzu_core::library::{Book, BookState};
use anzu_core::releases::Release;
use anzu_core::{Error, KindleVersion};

/// Whether the system is in a state where unlocking is allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Checking,
    Ready { kindle: KindleVersion, tool_tag: String, exe_name: String },
    Blocked(BlockReason),
}

/// Why unlocking is blocked. The text shown for each lives in `i18n`.
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

/// Why unlocking is not allowed, or `None` when everything is in order.
#[cfg_attr(not(test), allow(dead_code))] // the app uses `health`; this stays as a direct query
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

/// A book with DRM can only be picked when the system is ready to unlock.
pub fn selectable(state: BookState, ready: bool) -> bool {
    ready || state != BookState::Locked
}

/// How many of `books` were not among the ASINs listed before.
pub fn new_books(before: &[String], books: &[Book]) -> usize {
    books.iter().filter(|b| !before.contains(&b.asin)).count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPlan {
    pub needs_unlock: bool,
    /// The books to process, in library order.
    pub asins: Vec<String>,
}

pub fn plan_run(books: &[Book], selected: &[String]) -> RunPlan {
    let chosen: Vec<&Book> = books.iter().filter(|b| selected.contains(&b.asin)).collect();
    RunPlan {
        needs_unlock: chosen.iter().any(|b| b.state == BookState::Locked),
        asins: chosen.iter().map(|b| b.asin.clone()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anzu_core::releases::Asset;
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
        Book { asin: asin.into(), title: None, source: PathBuf::from("x.azw"), size: 1, state, unlocked: None, epub: None }
    }

    #[test]
    fn a_supported_kindle_with_a_verified_tool_is_ready() {
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
        assert_eq!(block_reason(Some(&k), Err(&Error::Network("offline".into())), &[]), Some(BlockReason::Offline));
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
    fn locked_books_need_a_ready_system() {
        assert!(selectable(BookState::Locked, true));
        assert!(!selectable(BookState::Locked, false));
        for ready in [true, false] {
            assert!(selectable(BookState::Unlocked, ready));
            assert!(selectable(BookState::Converted, ready), "a book can be converted again");
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
    fn a_refresh_counts_only_the_books_that_appeared() {
        let books = [book("A", BookState::Locked), book("B", BookState::Unlocked), book("C", BookState::Locked)];
        assert_eq!(new_books(&["A".into(), "B".into()], &books), 1);
        assert_eq!(new_books(&["A".into(), "B".into(), "C".into()], &books), 0);
        assert_eq!(new_books(&[], &books), 3);
        assert_eq!(new_books(&["GONE".into()], &[]), 0, "a book that disappears is not new");
    }

    #[test]
    fn a_run_ignores_selections_that_are_no_longer_in_the_library() {
        let books = [book("A", BookState::Unlocked)];
        let selected: Vec<String> = vec!["GONE".into(), "A".into()];
        assert_eq!(plan_run(&books, &selected), RunPlan { needs_unlock: false, asins: vec!["A".into()] });
        assert_eq!(plan_run(&books, &[]), RunPlan { needs_unlock: false, asins: vec![] });
    }
}

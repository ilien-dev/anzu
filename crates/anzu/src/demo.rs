//! The demo system: a Kindle library, a tool release and a converter that live only in
//! memory, so the real interface can run on a web page without disk, network or
//! processes. The rules of the run stay the real ones: [`DemoBackend`] is one more
//! [`Backend`], driven by the same [`crate::pipeline::run`] as the app.
//!
//! The data is illustrative: fake ASINs and public-domain titles.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anzu_core::Error;
use anzu_core::kindle::KindleInstall;
use anzu_core::library::{self, Book, BookState};
use anzu_core::releases::{Asset, Release};
use anzu_core::tool::Tool;
use anzu_kfx::{Converted, Labels};

use crate::pipeline::Backend;

/// Where the demo pretends Anzu keeps its data.
pub const DATA_DIR: &str = r"C:\Users\you\AppData\Local\anzu";
/// Where the demo pretends the Kindle app keeps the books.
pub const CONTENT_DIR: &str = r"C:\Users\you\AppData\Local\Amazon\Kindle\My Kindle Content";
/// The destination folder shown in the demo: the default one, for a user called "you".
pub const OUTPUT_DIR: &str = r"C:\Users\you\Documents\EPUB";

const KINDLE_VERSION: &str = "1.1.5634.0";
const TOOL_TAG: &str = "v10.0.35";
const TOOL_EXE: &str = "MSIXKFXArchiver_x64_1_5634.exe";

/// One book of the demo library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoBook {
    pub asin: &'static str,
    pub title: &'static str,
    pub size: u64,
    pub state: BookState,
    /// Whether the Kindle app has downloaded it yet.
    pub downloaded: bool,
}

/// The library shared by the backend and the window.
#[derive(Debug, Clone)]
pub struct DemoSystem {
    books: RefCell<Vec<DemoBook>>,
    kindle_running: RefCell<bool>,
}

impl DemoSystem {
    /// The starting point of every loop: one book already converted, two still in the
    /// Kindle app's store, the Kindle app open.
    pub fn new() -> Rc<DemoSystem> {
        Rc::new(DemoSystem {
            books: RefCell::new(vec![
                DemoBook {
                    asin: "B0DEMO0003",
                    title: "Walden",
                    size: 912_384,
                    state: BookState::Converted,
                    downloaded: true,
                },
                DemoBook {
                    asin: "B0DEMO0001",
                    title: "Moby-Dick",
                    size: 2_516_582,
                    state: BookState::Locked,
                    downloaded: false,
                },
                DemoBook {
                    asin: "B0DEMO0002",
                    title: "Pride and Prejudice",
                    size: 1_153_434,
                    state: BookState::Locked,
                    downloaded: false,
                },
            ]),
            kindle_running: RefCell::new(true),
        })
    }

    /// The Kindle app finishes downloading every book of the library.
    pub fn download_all(&self) {
        for book in self.books.borrow_mut().iter_mut() {
            book.downloaded = true;
        }
    }

    /// The ASINs that are in the Kindle folder and still have DRM.
    pub fn locked(&self) -> Vec<&'static str> {
        self.books.borrow().iter().filter(|b| b.downloaded && b.state == BookState::Locked).map(|b| b.asin).collect()
    }

    /// What a scan of the Kindle folder finds, as `anzu_core::library::scan` reports it.
    pub fn scan(&self) -> Vec<Book> {
        let data_dir = Path::new(DATA_DIR);
        self.books
            .borrow()
            .iter()
            .filter(|b| b.downloaded)
            .map(|b| Book {
                asin: b.asin.to_string(),
                // As in the app, which reads it from the Kindle app's catalog: the title is
                // known before the book is unlocked.
                title: Some(b.title.to_string()),
                source: Path::new(CONTENT_DIR).join(format!("{}_EBOK", b.asin)).join(format!("{}_EBOK.azw", b.asin)),
                size: b.size,
                state: b.state,
                unlocked: (b.state != BookState::Locked).then(|| library::unlocked_path(data_dir, b.asin)),
                epub: (b.state == BookState::Converted).then(|| epub_path(Path::new(OUTPUT_DIR), b.title)),
            })
            .collect()
    }

    /// A copy to run ahead on, so the window only sees each change when it is shown.
    pub fn rehearsal(&self) -> Rc<DemoSystem> {
        Rc::new(self.clone())
    }

    /// Records that `asin` reached `state`, as the window is told.
    pub fn set_state(&self, asin: &str, state: BookState) {
        if let Some(book) = self.books.borrow_mut().iter_mut().find(|b| b.asin == asin) {
            book.state = state;
        }
    }

    fn title_of(&self, asin: &str) -> Option<&'static str> {
        self.books.borrow().iter().find(|b| b.asin == asin).map(|b| b.title)
    }
}

/// Where the converter writes the EPUB of a book called `title`.
fn epub_path(out_dir: &Path, title: &str) -> PathBuf {
    out_dir.join(format!("{}.epub", anzu_kfx::epub::safe_file_name(title)))
}

/// A [`Backend`] over the demo library: the Kindle app is found, the tool is the latest
/// release and supports it, and every book unlocks and converts.
pub struct DemoBackend {
    pub system: Rc<DemoSystem>,
}

impl Backend for DemoBackend {
    fn detect_kindle(&self) -> Option<KindleInstall> {
        Some(KindleInstall { version: KINDLE_VERSION.parse().ok()?, content_dir: PathBuf::from(CONTENT_DIR) })
    }

    fn fetch_release(&self) -> Result<Release, Error> {
        Ok(Release {
            tag: TOOL_TAG.into(),
            published_at: String::new(),
            notes: String::new(),
            assets: vec![Asset {
                name: "DeDRM_tools.zip".into(),
                url: String::new(),
                sha256: Some("0".repeat(64)),
            }],
        })
    }

    fn ensure_tool(&self, release: &Release) -> Result<Tool, Error> {
        Ok(Tool {
            tag: release.tag.clone(),
            dir: Path::new(DATA_DIR).join("tool").join(&release.tag),
            exe_names: vec![TOOL_EXE.into()],
        })
    }

    fn kindle_running(&self) -> bool {
        *self.system.kindle_running.borrow()
    }

    fn close_kindle(&self) -> Result<(), Error> {
        *self.system.kindle_running.borrow_mut() = false;
        Ok(())
    }

    fn unlock(
        &self,
        _exe: &Path,
        _content_dir: &Path,
        on_line: &mut dyn FnMut(&str),
    ) -> Result<Vec<(String, PathBuf)>, Error> {
        let mut unlocked = Vec::new();
        for asin in self.system.locked() {
            // The tool's output reaches the screen only through the same filter as in the app.
            for line in [format!("Read file of {asin}_EBOK.azw"), "Decrypting DRMION".into(), "DRMION decrypted".into()] {
                on_line(&anzu_core::unlock::redact(&line));
            }
            self.system.set_state(asin, BookState::Unlocked);
            unlocked.push((asin.to_string(), self.unlocked_path(asin)));
        }
        Ok(unlocked)
    }

    fn unlocked_path(&self, asin: &str) -> PathBuf {
        library::unlocked_path(Path::new(DATA_DIR), asin)
    }

    fn convert(&self, input: &Path, out_dir: &Path, labels: &Labels) -> Result<Converted, anzu_kfx::Error> {
        let asin = input.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let title = self.system.title_of(&asin).map_or_else(|| labels.untitled.clone(), str::to_string);
        Ok(Converted { path: epub_path(out_dir, &title), title, warnings: Vec::new(), renamed: false })
    }

    fn mark_converted(&self, asin: &str, _epub: &Path) {
        self.system.set_state(asin, BookState::Converted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{self, Health};
    use crate::pipeline::{self, Msg};
    use anzu_core::settings::Language;

    #[test]
    fn the_demo_system_passes_the_real_check() {
        let backend = DemoBackend { system: DemoSystem::new() };
        let (health, ready) = pipeline::check(&backend);
        assert!(matches!(health, Health::Ready { .. }), "{health:?}");
        assert!(ready.is_some());
    }

    #[test]
    fn books_appear_only_once_downloaded() {
        let system = DemoSystem::new();
        assert_eq!(system.scan().len(), 1);
        system.download_all();
        let books = system.scan();
        assert_eq!(books.len(), 3);
        assert!(books.iter().all(|b| b.title.is_some()));
    }

    #[test]
    fn a_real_run_unlocks_converts_and_asks_before_closing_kindle() {
        let system = DemoSystem::new();
        system.download_all();
        let backend = DemoBackend { system: system.clone() };
        let books = system.scan();
        let plan = app::plan_run(&books, &["B0DEMO0001".into(), "B0DEMO0002".into()]);
        let mut messages = Vec::new();
        let mut asked = false;
        pipeline::run(
            &backend,
            &plan,
            &books,
            Path::new(OUTPUT_DIR),
            Language::En,
            &mut |m| messages.push(m),
            &mut || {
                asked = true;
                true
            },
        );
        assert!(asked);
        assert!(messages.contains(&Msg::NeedCloseKindle));
        assert!(matches!(messages.last(), Some(Msg::Finished { converted: 2, failed }) if failed.is_empty()));
        assert!(messages.contains(&Msg::Saved {
            asin: "B0DEMO0001".into(),
            path: Path::new(OUTPUT_DIR).join("Moby-Dick.epub"),
            renamed: false,
        }));
        assert!(system.scan().iter().all(|b| b.state == BookState::Converted));
        // Nothing the tool said reaches the log unless the filter lets it through.
        assert!(messages.iter().any(|m| matches!(m, Msg::Log(l) if l.starts_with("Decrypting DRMION"))));
    }
}

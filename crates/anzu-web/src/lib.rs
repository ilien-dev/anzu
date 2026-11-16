//! The Anzu window on a web page: the real interface of the app, compiled to WebAssembly
//! and played by the demo script (`anzu::ui::demo`). Only built for `wasm32`; on other
//! targets this crate is empty.
//!
//! From JavaScript:
//!
//! ```js
//! import init, { Demo } from "./anzu_web.js";
//! await init();
//! const demo = new Demo();
//! await demo.start(canvas, "en", "dark", (event) => console.log(event));
//! demo.set_language("es");
//! demo.set_theme("light");
//! ```

#[cfg(target_arch = "wasm32")]
mod web {
    use anzu::ui::demo::{Remote, RemoteHandle};
    use anzu_core::settings::{Language, ThemeChoice};
    use wasm_bindgen::prelude::*;

    fn language(code: &str) -> Language {
        if code.starts_with("es") { Language::Es } else { Language::En }
    }

    fn theme(name: &str) -> ThemeChoice {
        if name == "light" { ThemeChoice::Light } else { ThemeChoice::Dark }
    }

    /// One demo window on one canvas.
    #[wasm_bindgen]
    pub struct Demo {
        runner: eframe::WebRunner,
        remote: RemoteHandle,
    }

    #[wasm_bindgen]
    impl Demo {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Demo {
            Demo { runner: eframe::WebRunner::new(), remote: RemoteHandle::default() }
        }

        /// Starts the window on `canvas`. `on_event` hears "download", "saved",
        /// "finished" and "loop".
        pub async fn start(
            &self,
            canvas: web_sys::HtmlCanvasElement,
            lang: String,
            theme_name: String,
            on_event: js_sys::Function,
        ) -> Result<(), JsValue> {
            *self.remote.borrow_mut() = Remote {
                on_event: Some(Box::new(move |event: &str| {
                    let _ = on_event.call1(&JsValue::NULL, &JsValue::from_str(event));
                })),
                ..Remote::default()
            };
            let remote = self.remote.clone();
            let (lang, choice) = (language(&lang), theme(&theme_name));
            self.runner
                .start(
                    canvas,
                    eframe::WebOptions::default(),
                    Box::new(move |cc| Ok(anzu::ui::demo_app(&cc.egui_ctx, remote, lang, choice))),
                )
                .await
        }

        /// Switches the interface language: "en" or "es".
        pub fn set_language(&self, lang: &str) {
            self.remote.borrow_mut().language = Some(language(lang));
        }

        /// Switches the theme: "light" or "dark".
        pub fn set_theme(&self, name: &str) {
            self.remote.borrow_mut().theme = Some(theme(name));
        }

        /// Stops the window and frees what it holds.
        pub fn destroy(&self) {
            self.runner.destroy();
        }
    }

    impl Default for Demo {
        fn default() -> Self {
            Demo::new()
        }
    }
}

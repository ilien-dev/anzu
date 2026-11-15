//! The notice that explains why unlocking is blocked.

use eframe::egui;

use super::{AnzuApp, theme};
use crate::app::BlockReason;

const RELEASES_URL: &str = "https://github.com/Satsuoni/DeDRM_tools/releases";

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui, reason: &BlockReason) {
    let p = theme::Palette::of(ui.ctx());
    let t = app.t();
    let (title, body) = t.block_message(reason);
    theme::warn_panel(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::bold(title).color(p.warn).size(15.0));
        ui.label(body);
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            if theme::button(ui, theme::Surface::Page, true, egui::Button::new(t.recheck)).clicked() {
                let ctx = ui.ctx().clone();
                app.start_check(&ctx);
            }
            if matches!(reason, BlockReason::TooNew { .. } | BlockReason::Unknown | BlockReason::NoDigest) {
                ui.hyperlink_to(t.releases_link, RELEASES_URL);
            }
        });
        ui.label(theme::muted(p, t.blocked_note).size(theme::SMALL));
    });
}

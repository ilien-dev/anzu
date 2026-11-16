//! The notice that explains why unlocking is blocked.

use eframe::egui;

use super::icons::{self, Icon};
use super::theme::{self, Kind};
use super::AnzuApp;
use crate::app::BlockReason;

const RELEASES_URL: &str = "https://github.com/Satsuoni/DeDRM_tools/releases";

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui, reason: &BlockReason) {
    let p = theme::Palette::of(ui.ctx());
    let t = app.t();
    let (title, body) = t.block_message(reason);
    theme::warn_panel(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            icons::tile(ui, Icon::Alert, 30.0, theme::DRM, theme::INK);
            ui.add(egui::Label::new(theme::display(title).size(17.0)).wrap());
        });
        ui.label(body);
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            if theme::icon_button(ui, Kind::Secondary, true, Icon::Refresh, t.recheck).clicked() {
                let ctx = ui.ctx().clone();
                app.start_check(&ctx);
            }
            if matches!(reason, BlockReason::TooNew { .. } | BlockReason::Unknown | BlockReason::NoDigest) {
                icons::show(ui, Icon::External, 14.0, p.fg);
                ui.hyperlink_to(t.releases_link, RELEASES_URL);
            }
        });
        ui.horizontal(|ui| {
            icons::show(ui, Icon::Info, 14.0, p.muted);
            ui.label(theme::muted(p, t.blocked_note).size(theme::SMALL));
        });
    });
}

//! El aviso que explica por qué el desbloqueo está bloqueado.

use eframe::egui;

use super::{AnzuApp, theme};
use crate::app::BlockReason;

const RELEASES_URL: &str = "https://github.com/Satsuoni/DeDRM_tools/releases";

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui, reason: &BlockReason) {
    let p = theme::Palette::of(ui.ctx());
    let (title, body) = reason.message();
    theme::warn_panel(p).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(theme::bold(title).color(p.warn).size(15.0));
        ui.label(body);
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Volver a comprobar").clicked() {
                let ctx = ui.ctx().clone();
                app.start_check(&ctx);
            }
            if matches!(reason, BlockReason::TooNew { .. } | BlockReason::Unknown | BlockReason::NoDigest) {
                ui.hyperlink_to("Ver las publicaciones de la herramienta", RELEASES_URL);
            }
        });
        ui.label(
            theme::muted(p, "Los libros ya desbloqueados se pueden convertir igualmente.").size(theme::SMALL),
        );
    });
}

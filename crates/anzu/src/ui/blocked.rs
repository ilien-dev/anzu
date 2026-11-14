//! El aviso que explica por qué el desbloqueo está bloqueado.

use eframe::egui::{self, RichText};

use super::{AnzuApp, theme};
use crate::app::BlockReason;

const RELEASES_URL: &str = "https://github.com/Satsuoni/DeDRM_tools/releases";

pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui, reason: &BlockReason) {
    let (title, body) = reason.message();
    theme::card(theme::WARN_BG, theme::WARN_LINE).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).color(theme::WARN).size(18.0).strong());
        ui.add_space(2.0);
        ui.label(body);
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            let recheck = egui::Button::new(RichText::new("Volver a comprobar ahora").color(theme::ON_ACCENT).strong())
                .fill(theme::WARN);
            if ui.add(recheck).clicked() {
                let ctx = ui.ctx().clone();
                app.start_check(&ctx);
            }
            if matches!(reason, BlockReason::TooNew { .. } | BlockReason::Unknown | BlockReason::NoDigest) {
                ui.hyperlink_to("Ver publicaciones de la herramienta", RELEASES_URL);
            }
        });
        ui.label(theme::muted("La compatibilidad se comprueba al abrir la app y antes de cada desbloqueo."));
    });
}

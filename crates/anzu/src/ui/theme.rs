//! Colores y estilo de la interfaz.

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

pub const BG: Color32 = Color32::from_rgb(0x11, 0x13, 0x15);
pub const PANEL: Color32 = Color32::from_rgb(0x18, 0x1B, 0x1F);
pub const LINE: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x36);
pub const TEXT: Color32 = Color32::from_rgb(0xE7, 0xEA, 0xEE);
pub const MUTED: Color32 = Color32::from_rgb(0xA0, 0xA8, 0xB3);
pub const ACCENT: Color32 = Color32::from_rgb(0x5A, 0xA9, 0xFF);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x0B, 0x16, 0x20);
pub const OK: Color32 = Color32::from_rgb(0x7B, 0xE0, 0xB0);
pub const OK_BG: Color32 = Color32::from_rgb(0x14, 0x24, 0x1D);
pub const OK_LINE: Color32 = Color32::from_rgb(0x2C, 0x5A, 0x45);
pub const WARN: Color32 = Color32::from_rgb(0xF2, 0xA6, 0x5A);
pub const WARN_BG: Color32 = Color32::from_rgb(0x2A, 0x1F, 0x14);
pub const WARN_LINE: Color32 = Color32::from_rgb(0x6B, 0x4A, 0x22);
pub const DONE: Color32 = Color32::from_rgb(0x8C, 0xC4, 0xFF);
pub const LOG_BG: Color32 = Color32::from_rgb(0x0C, 0x0E, 0x10);

pub const RADIUS: CornerRadius = CornerRadius::same(8);

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = PANEL;
    visuals.override_text_color = Some(TEXT);
    visuals.window_stroke = Stroke::new(1.0, LINE);
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
    visuals.widgets.inactive.bg_fill = PANEL;
    visuals.widgets.inactive.weak_bg_fill = PANEL;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
    // La app es siempre oscura: sin esto, con Windows en modo claro los controles
    // tomarían los colores claros sobre nuestro fondo oscuro.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.set_visuals_of(egui::Theme::Dark, visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = 32.0;
    });
}

pub fn muted(text: impl Into<String>) -> RichText {
    RichText::new(text).color(MUTED).size(12.5)
}

pub fn mono(text: impl Into<String>) -> RichText {
    RichText::new(text).monospace()
}

/// Un recuadro con borde, como las tarjetas de estado.
pub fn card(fill: Color32, line: Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, line))
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::symmetric(14, 12))
}

/// El botón de la acción principal.
pub fn primary_button(label: &str) -> egui::Button<'static> {
    egui::Button::new(RichText::new(label.to_string()).color(ON_ACCENT).strong())
        .fill(ACCENT)
        .min_size(egui::vec2(0.0, 38.0))
}

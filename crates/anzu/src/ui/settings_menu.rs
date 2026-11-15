//! The settings button of the header and the floating panel it opens: theme, language
//! and updates. They used to sit in the header itself.

use anzu_core::settings::{Language, ThemeChoice};
use eframe::egui::{self, RectAlign, RichText, Stroke};

use super::{AnzuApp, icons, theme, update};
use crate::i18n::Strings;

const BUTTON: f32 = 30.0;
const PANEL_WIDTH: f32 = 300.0;

/// Draws the button and, while it is open, the panel below it.
pub fn show(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &'static Strings) {
    let id = egui::Id::unique("settings-panel");
    let open = egui::Popup::is_id_open(ui.ctx(), id);
    let (fill, ink, edge) =
        if open { (p.shell_accent, p.on_shell_accent, p.shell_accent) } else { (p.shell, p.on_shell, p.shell_line) };
    let button = egui::Button::new("").fill(fill).stroke(Stroke::new(1.0, edge)).min_size(egui::vec2(BUTTON, BUTTON));
    let response = theme::button(ui, theme::Surface::Shell, true, button).on_hover_text(t.settings);
    icons::gear(ui.painter(), response.rect.shrink(7.5), ink);
    if app.updates.has_news() {
        let dot = response.rect.right_top() + egui::vec2(-2.0, 2.0);
        ui.painter().circle(dot, 4.0, p.shell_accent, Stroke::new(2.0, p.shell));
    }

    egui::Popup::from_toggle_button_response(&response)
        .id(id)
        .align(RectAlign::BOTTOM_END)
        .gap(6.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .frame(theme::panel(p).stroke(Stroke::new(1.0, p.fg)).inner_margin(egui::Margin::same(16)))
        .width(PANEL_WIDTH)
        .show(|ui| panel(app, ui, p, t));
}

fn panel(app: &mut AnzuApp, ui: &mut egui::Ui, p: theme::Palette, t: &'static Strings) {
    ui.set_width(PANEL_WIDTH);
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.label(theme::bold(t.settings).size(14.0));
    ui.add_space(6.0);

    ui.label(theme::muted(p, t.theme).size(theme::SMALL));
    let themes = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];
    let current = themes.iter().position(|c| *c == app.settings.theme).unwrap_or(0);
    if let Some(i) = segmented(ui, p, &[t.theme_system, t.theme_light, t.theme_dark], current) {
        let ctx = ui.ctx().clone();
        app.set_theme(&ctx, themes[i]);
    }
    ui.add_space(8.0);

    ui.label(theme::muted(p, t.language).size(theme::SMALL));
    let languages = [Language::En, Language::Es];
    let current = languages.iter().position(|l| *l == app.settings.language).unwrap_or(0);
    if let Some(i) = segmented(ui, p, &[t.language_en, t.language_es], current) {
        let ctx = ui.ctx().clone();
        app.set_language(&ctx, languages[i]);
    }
    ui.add_space(10.0);
    let (line, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, p.line);
    ui.add_space(8.0);

    let busy = app.busy();
    update::settings_section(&mut app.updates, ui, t, busy);
}

/// A row of joined buttons where one is chosen. Returns the one clicked, if another.
pub(super) fn segmented(ui: &mut egui::Ui, p: theme::Palette, labels: &[&str], selected: usize) -> Option<usize> {
    let mut clicked = None;
    let width = ui.available_width() / labels.len() as f32;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (i, label) in labels.iter().enumerate() {
            let chosen = i == selected;
            let text = if chosen {
                theme::bold(label.to_string()).color(p.on_accent).size(12.0)
            } else {
                RichText::new(label.to_string()).color(p.fg).size(12.0)
            };
            let button = egui::Button::new(text)
                .fill(if chosen { p.accent } else { p.panel })
                .stroke(Stroke::new(1.0, if chosen { p.accent } else { p.line }))
                .corner_radius(0)
                .min_size(egui::vec2(width, 30.0));
            if theme::button(ui, theme::Surface::Page, true, button).clicked() && !chosen {
                clicked = Some(i);
            }
        }
    });
    clicked
}

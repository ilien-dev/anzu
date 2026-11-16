//! The settings button of the header and the floating panel it opens: theme, language
//! and updates. They used to sit in the header itself.

use anzu_core::settings::{Language, ThemeChoice};
use eframe::egui::{self, RectAlign, Stroke};

use super::icons::{self, Icon};
use super::{AnzuApp, theme, update};
use crate::i18n::Strings;

const BUTTON: f32 = 30.0;
const PANEL_WIDTH: f32 = 300.0;
/// How long the pill of a row of options takes to slide to the one chosen, in seconds.
const SLIDE_TIME: f32 = 0.18;

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
    ui.horizontal(|ui| {
        icons::tile(ui, Icon::Gear, 26.0, p.quiet, p.fg);
        ui.label(theme::display(t.settings).size(17.0));
    });
    ui.add_space(6.0);

    ui.label(theme::muted(p, t.theme).size(theme::SMALL));
    let themes = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];
    let current = themes.iter().position(|c| *c == app.settings.theme).unwrap_or(0);
    let options = [(Icon::Monitor, t.theme_system), (Icon::Sun, t.theme_light), (Icon::Moon, t.theme_dark)];
    if let Some(i) = segmented(ui, p, "theme", &options, current) {
        let ctx = ui.ctx().clone();
        app.set_theme(&ctx, themes[i]);
    }
    ui.add_space(8.0);

    ui.label(theme::muted(p, t.language).size(theme::SMALL));
    let languages = [Language::En, Language::Es];
    let current = languages.iter().position(|l| *l == app.settings.language).unwrap_or(0);
    if let Some(i) = segmented(ui, p, "language", &[(Icon::Globe, t.language_en), (Icon::Globe, t.language_es)], current) {
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

/// A row of options on a quiet track where one is chosen, each with its icon. Returns the
/// one clicked, if another. `id_salt` must differ between the rows of one panel: segments
/// with the same id share their clicks.
pub(super) fn segmented(
    ui: &mut egui::Ui,
    p: theme::Palette,
    id_salt: &str,
    options: &[(Icon, &str)],
    selected: usize,
) -> Option<usize> {
    let mut clicked = None;
    let (track, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 38.0), egui::Sense::hover());
    ui.painter().rect_filled(track, theme::RADIUS, p.quiet);
    let inner = track.shrink(3.0);
    let width = inner.width() / options.len() as f32;
    let segment = |at: f32| egui::Rect::from_min_size(egui::pos2(inner.left() + at * width, inner.top()), egui::vec2(width, inner.height()));
    // The chosen option sits on a white pill that slides to the next one chosen.
    // Without animations in Windows it jumps there.
    let slide = if theme::motion_allowed() { SLIDE_TIME } else { 0.0 };
    let at = ui.ctx().animate_value_with_time(ui.scope_id().with(("pill", id_salt)), selected as f32, slide);
    let pill = segment(at);
    ui.painter().rect_filled(pill, egui::CornerRadius::same(8), p.panel);
    ui.painter().rect_stroke(pill, egui::CornerRadius::same(8), Stroke::new(1.0, p.line_strong), egui::StrokeKind::Inside);
    for (i, (icon, label)) in options.iter().enumerate() {
        let rect = segment(i as f32);
        let chosen = i == selected;
        let response = ui.interact(rect, ui.scope_id().with(("segment", id_salt, i)), egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, chosen, *label));
        // The one chosen already is not a thing to click: the arrow stays over it.
        if !chosen {
            theme::pointer(&response, true);
        }
        let painter = ui.painter();
        if !chosen && response.hovered() && (at - i as f32).abs() >= 1.0 {
            painter.rect_filled(rect, egui::CornerRadius::same(8), p.hover_bg);
        }
        let ink = if chosen { p.fg } else { p.muted };
        let font = if chosen { theme::bold_font(12.5) } else { theme::regular_font(12.5) };
        let galley = painter.layout_no_wrap(label.to_string(), font, ink);
        let content = 14.0 + 6.0 + galley.size().x;
        let left = rect.center().x - content / 2.0;
        icons::paint(painter, *icon, egui::Rect::from_min_size(egui::pos2(left, rect.center().y - 7.0), egui::vec2(14.0, 14.0)), ink);
        painter.galley(egui::pos2(left + 20.0, rect.center().y - galley.size().y / 2.0), galley, ink);
        if response.clicked() && !chosen {
            clicked = Some(i);
        }
    }
    clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two rows like the theme and language rows of the panel; a click on the second option
    /// of the lower row. Returns what each row reported on the frame of the release.
    fn click_lower_row(upper: &str, lower: &str) -> (Option<usize>, Option<usize>) {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0));
        let options = [(Icon::Sun, "A"), (Icon::Moon, "B")];
        let mut target = egui::Pos2::ZERO;
        let mut result = (None, None);
        for frame in 0..4 {
            let pos = target;
            let events = match frame {
                1 => vec![egui::Event::PointerMoved(pos)],
                2 => vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                }],
                3 => vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
                _ => Vec::new(),
            };
            let raw = egui::RawInput { time: Some(frame as f64 / 60.0), screen_rect: Some(screen), events, ..Default::default() };
            let mut output = ctx.run_ui(raw, |ui| {
                let p = theme::Palette::of(ui.ctx());
                let a = segmented(ui, p, upper, &options, 0);
                let top = ui.next_widget_position().y;
                let b = segmented(ui, p, lower, &options, 0);
                let rect = ui.min_rect();
                target = egui::pos2(rect.left() + rect.width() * 0.75, top + 19.0);
                if frame == 3 {
                    result = (a, b);
                }
            });
            output.textures_delta.clear();
        }
        result
    }

    #[test]
    fn a_click_on_one_row_does_not_reach_the_other() {
        assert_eq!(click_lower_row("theme", "language"), (None, Some(1)));
    }
}

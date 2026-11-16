//! Colours, type and style of the interface, light and dark: "Coloured paper".
//!
//! Ink on cool white, or light ink on night, with four pastels that name the state of a
//! book: peach for DRM, lilac for unlocked, mint for EPUB, butter for what is selected.
//! Pastels are fills only, never the colour of text.

use std::sync::Arc;

use anzu_core::settings::ThemeChoice;
use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, RichText, Stroke};

use super::icons::{self, Icon};

/// Maximum width of the content: on large monitors it stays centred, not stretched.
pub const MAX_WIDTH: f32 = 1240.0;
/// Corners of buttons and small controls.
pub const RADIUS: CornerRadius = CornerRadius::same(10);
/// Corners of panels and cards.
pub const CARD_RADIUS: CornerRadius = CornerRadius::same(14);
pub const BODY: f32 = 14.0;
pub const SMALL: f32 = 12.0;

const BOLD_FAMILY: &str = "bold";
const DISPLAY_FAMILY: &str = "display";

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// The pastels of the book states, the same in both themes. Text on them is always [`INK`].
pub const DRM: Color32 = rgb(0xFFB4A2);
pub const UNLOCKED: Color32 = rgb(0xC9B8FF);
pub const EPUB: Color32 = rgb(0xC8F0A0);
pub const INFO: Color32 = rgb(0xA8E1F5);
pub const BUTTER: Color32 = rgb(0xFFF27A);
pub const INK: Color32 = rgb(0x15161A);

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    /// The side panel of the main screen.
    pub aside: Color32,
    pub line: Color32,
    /// Borders of controls, a step stronger than `line`.
    pub line_strong: Color32,
    pub fg: Color32,
    pub muted: Color32,
    /// Quiet fills: chips, empty track stages, collapsed shelves.
    pub quiet: Color32,
    /// The main action: ink in light, butter in dark.
    pub accent: Color32,
    pub on_accent: Color32,
    /// A ticked box.
    pub check: Color32,
    pub on_check: Color32,
    /// The header band, dark in both themes.
    pub shell: Color32,
    pub shell_chip: Color32,
    pub on_shell: Color32,
    pub shell_muted: Color32,
    pub shell_line: Color32,
    pub shell_accent: Color32,
    pub on_shell_accent: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub warn_bg: Color32,
    pub warn_line: Color32,
    /// What is selected, and the thing running now.
    pub select_bg: Color32,
    pub select_line: Color32,
    pub hover_bg: Color32,
    pub log_bg: Color32,
    /// Soft fills of the three states, for notes and chips.
    pub drm_tint: Color32,
    pub epub_tint: Color32,
    /// The empty part of a progress bar, and its fill.
    pub progress_track: Color32,
    pub progress: Color32,
}

pub const LIGHT: Palette = Palette {
    bg: rgb(0xF4F5F7),
    panel: rgb(0xFFFFFF),
    aside: rgb(0xFAFBFC),
    line: rgb(0xE4E7EC),
    line_strong: rgb(0xC9CDD5),
    fg: INK,
    muted: rgb(0x565A64),
    quiet: rgb(0xEDEFF2),
    accent: INK,
    on_accent: rgb(0xFFFFFF),
    check: INK,
    on_check: rgb(0xFFFFFF),
    shell: INK,
    shell_chip: rgb(0x26282E),
    on_shell: rgb(0xFFFFFF),
    shell_muted: rgb(0xC9CCD3),
    shell_line: rgb(0x3A3D45),
    shell_accent: BUTTER,
    on_shell_accent: INK,
    ok: rgb(0x2F7A1F),
    warn: rgb(0x8A2E14),
    warn_bg: rgb(0xFFF4F1),
    warn_line: rgb(0xF2B8A8),
    select_bg: rgb(0xFFFBD6),
    select_line: INK,
    hover_bg: rgb(0xF7F8FA),
    log_bg: rgb(0xF7F8FA),
    drm_tint: rgb(0xFFE3DB),
    epub_tint: rgb(0xE6F8D4),
    progress_track: rgb(0xEFE6A0),
    progress: INK,
};

pub const DARK: Palette = Palette {
    bg: rgb(0x0E0F12),
    panel: rgb(0x16171B),
    aside: rgb(0x131418),
    line: rgb(0x2A2C33),
    line_strong: rgb(0x3A3D45),
    fg: rgb(0xECEDEF),
    muted: rgb(0xA3A7B0),
    quiet: rgb(0x1E2025),
    accent: BUTTER,
    on_accent: INK,
    check: BUTTER,
    on_check: INK,
    shell: rgb(0x060708),
    shell_chip: rgb(0x1E2025),
    on_shell: rgb(0xFFFFFF),
    shell_muted: rgb(0xA3A7B0),
    shell_line: rgb(0x3A3D45),
    shell_accent: BUTTER,
    on_shell_accent: INK,
    ok: rgb(0xA9E57A),
    warn: rgb(0xFFB4A2),
    warn_bg: rgb(0x331C17),
    warn_line: rgb(0x7A3A2A),
    select_bg: rgb(0x24220F),
    select_line: BUTTER,
    hover_bg: rgb(0x1B1D22),
    log_bg: rgb(0x0A0B0D),
    drm_tint: rgb(0x3A2620),
    epub_tint: rgb(0x22301B),
    progress_track: rgb(0x3E3A1C),
    progress: BUTTER,
};

pub use super::mark::{ICON_BACKGROUND, ICON_MARK};

impl Palette {
    /// The palette of the theme that is active right now.
    pub fn of(ctx: &egui::Context) -> Palette {
        if ctx.theme() == egui::Theme::Dark { DARK } else { LIGHT }
    }
}

fn visuals(p: Palette, dark: bool) -> egui::Visuals {
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.panel;
    v.window_corner_radius = CARD_RADIUS;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.extreme_bg_color = p.bg;
    v.faint_bg_color = p.panel;
    v.override_text_color = Some(p.fg);
    v.window_stroke = Stroke::new(1.0, p.line);
    v.selection.bg_fill = p.select_bg;
    v.selection.stroke = Stroke::new(1.0, p.fg);
    v.hyperlink_color = p.fg;
    // Every enabled button shows the hand while hovered.
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
    v.widgets.noninteractive.corner_radius = RADIUS;
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        w.bg_fill = p.panel;
        w.weak_bg_fill = p.panel;
        w.corner_radius = RADIUS;
        w.fg_stroke = Stroke::new(1.0, p.fg);
    }
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.line_strong);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.fg);
    v.widgets.hovered.weak_bg_fill = p.hover_bg;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.fg);
    v
}

/// Registers the typefaces, the colours of both themes and the spacing. Once, at start-up.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font = |bytes: &'static [u8]| Arc::new(egui::FontData::from_static(bytes));
    fonts.font_data.insert("inter".to_owned(), font(include_bytes!("../../assets/fonts/Inter-Regular.otf")));
    fonts.font_data.insert("inter-semibold".to_owned(), font(include_bytes!("../../assets/fonts/Inter-SemiBold.otf")));
    fonts
        .font_data
        .insert("space-grotesk-bold".to_owned(), font(include_bytes!("../../assets/fonts/SpaceGrotesk-Bold.ttf")));
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "inter".to_owned());
    // Each family falls back to Inter, then to egui's own fonts, for the characters it lacks.
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let with_fallback = |first: &str| std::iter::once(first.to_owned()).chain(fallback.iter().cloned()).collect::<Vec<_>>();
    fonts.families.insert(FontFamily::Name(BOLD_FAMILY.into()), with_fallback("inter-semibold"));
    fonts.families.insert(FontFamily::Name(DISPLAY_FAMILY.into()), with_fallback("space-grotesk-bold"));
    ctx.set_fonts(fonts);

    ctx.set_visuals_of(egui::Theme::Dark, visuals(DARK, true));
    ctx.set_visuals_of(egui::Theme::Light, visuals(LIGHT, false));
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = 34.0;
        for (text_style, size) in [
            (egui::TextStyle::Body, BODY),
            (egui::TextStyle::Button, BODY),
            (egui::TextStyle::Monospace, 12.5),
            (egui::TextStyle::Small, SMALL),
            (egui::TextStyle::Heading, 20.0),
        ] {
            let family =
                if text_style == egui::TextStyle::Monospace { FontFamily::Monospace } else { FontFamily::Proportional };
            style.text_styles.insert(text_style, FontId::new(size, family));
        }
    });
}

/// Applies the user's choice: follow the system, or force light or dark.
pub fn set_choice(ctx: &egui::Context, choice: ThemeChoice) {
    ctx.set_theme(match choice {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Light => egui::ThemePreference::Light,
        ThemeChoice::Dark => egui::ThemePreference::Dark,
    });
}

pub fn bold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD_FAMILY.into()))
}

pub fn regular_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY_FAMILY.into()))
}

/// Semibold Inter, for labels that lead.
pub fn bold(text: impl Into<String>) -> RichText {
    RichText::new(text).family(FontFamily::Name(BOLD_FAMILY.into()))
}

/// Space Grotesk, for headings and the name.
pub fn display(text: impl Into<String>) -> RichText {
    RichText::new(text).family(FontFamily::Name(DISPLAY_FAMILY.into()))
}

pub fn muted(p: Palette, text: impl Into<String>) -> RichText {
    RichText::new(text).color(p.muted)
}

/// A panel with a hairline border.
pub fn panel(p: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(CARD_RADIUS)
        .inner_margin(egui::Margin::symmetric(16, 12))
}

pub fn warn_panel(p: Palette) -> egui::Frame {
    panel(p).fill(p.warn_bg).stroke(Stroke::new(1.0, p.warn_line)).inner_margin(egui::Margin::symmetric(18, 16))
}

/// Where a button sits, which decides the colours of its hover feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// The page and its panels.
    Page,
    /// The header band, dark in both themes.
    Shell,
}

/// How long the hover and press feedback takes to fade in or out, in seconds.
const HOVER_TIME: f32 = 0.12;
const PRESS_TIME: f32 = 0.06;

/// Adds a button with a short animated hover and press feedback. egui gives no hover change
/// to a button with its own fill, so the feedback is painted over every button the same way.
pub fn button(ui: &mut egui::Ui, surface: Surface, enabled: bool, widget: impl egui::Widget) -> egui::Response {
    let response = ui.add_enabled(enabled, widget);
    feedback(ui, surface, &response);
    response
}

/// Paints the animated hover and press feedback over a clickable widget already added.
pub fn feedback(ui: &egui::Ui, surface: Surface, response: &egui::Response) {
    let p = Palette::of(ui.ctx());
    let (tint, edge) = match surface {
        Surface::Page => (p.fg, p.fg),
        Surface::Shell => (p.on_shell, p.shell_accent),
    };
    let ctx = ui.ctx();
    let hover = ctx.animate_bool_with_time(response.id.with("hover"), response.hovered(), HOVER_TIME);
    let press = ctx.animate_bool_with_time(response.id.with("press"), response.is_pointer_button_down_on(), PRESS_TIME);
    if hover <= 0.0 && press <= 0.0 {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(response.rect, RADIUS, tint.gamma_multiply(0.06 * hover + 0.08 * press));
    painter.rect_stroke(
        response.rect,
        RADIUS,
        Stroke::new(1.0, edge.gamma_multiply(0.7 * hover)),
        egui::StrokeKind::Inside,
    );
}

/// The look of a button with an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The one main action of a screen.
    Primary,
    /// A bordered button.
    Secondary,
    /// Borderless, for links and actions that should not compete.
    Quiet,
    /// A bordered button on the dark header band.
    Shell,
}

/// A button with an icon to the left of its label.
pub fn icon_button(ui: &mut egui::Ui, kind: Kind, enabled: bool, icon: Icon, label: &str) -> egui::Response {
    draw_icon_button(ui, kind, enabled, icon, Some(label), label, 0.0)
}

/// A button with an icon and a label, `width` wide, its content centred. A label too long
/// for it is set smaller.
pub fn icon_button_wide(
    ui: &mut egui::Ui,
    kind: Kind,
    enabled: bool,
    icon: Icon,
    label: &str,
    width: f32,
) -> egui::Response {
    draw_icon_button(ui, kind, enabled, icon, Some(label), label, width)
}

fn draw_icon_button(
    ui: &mut egui::Ui,
    kind: Kind,
    enabled: bool,
    icon: Icon,
    label: Option<&str>,
    name: &str,
    min_width: f32,
) -> egui::Response {
    let p = Palette::of(ui.ctx());
    let (height, font, icon_size) = match kind {
        Kind::Primary => (42.0, display_font(15.0), 18.0),
        _ => (34.0, bold_font(13.0), 16.0),
    };
    let (fill, ink, edge) = match (kind, enabled) {
        // A main action that cannot run yet steps back to a quiet fill, not a faded accent.
        (Kind::Primary, false) => (p.quiet, p.muted, Color32::TRANSPARENT),
        (Kind::Primary, true) => (p.accent, p.on_accent, Color32::TRANSPARENT),
        (Kind::Secondary, _) => (p.panel, p.fg, p.line_strong),
        (Kind::Quiet, _) => (Color32::TRANSPARENT, p.fg, Color32::TRANSPARENT),
        (Kind::Shell, _) => (Color32::TRANSPARENT, p.on_shell, p.shell_line),
    };
    let disabled_primary = kind == Kind::Primary && !enabled;
    let pad = if kind == Kind::Quiet { 8.0 } else { 14.0 };
    let mut galley = label.map(|label| ui.painter().layout_no_wrap(label.to_string(), font.clone(), ink));
    // Shrinks a label that does not fit the width asked for, down to 11 points.
    if let (Some(text), true) = (label, min_width > 0.0) {
        let mut size = font.size;
        while galley.as_ref().is_some_and(|g| pad + icon_size + 8.0 + g.size().x + pad > min_width) && size > 11.0 {
            size -= 0.5;
            galley = Some(ui.painter().layout_no_wrap(text.to_string(), FontId::new(size, font.family.clone()), ink));
        }
    }
    let content = match &galley {
        Some(g) => icon_size + 8.0 + g.size().x,
        None => icon_size,
    };
    let width = match &galley {
        Some(_) => (pad + content + pad).max(min_width),
        None => height,
    };
    let sense = if enabled { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), sense);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, name));
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let alpha = if enabled || disabled_primary { 1.0 } else { 0.4 };
        painter.rect_filled(rect, RADIUS, fill.gamma_multiply(alpha));
        if edge != Color32::TRANSPARENT {
            painter.rect_stroke(rect, RADIUS, Stroke::new(1.0, edge), egui::StrokeKind::Inside);
        }
        let ink = ink.gamma_multiply(if enabled || disabled_primary { 1.0 } else { 0.55 });
        let icon_left = rect.center().x - content / 2.0;
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(icon_left, rect.center().y - icon_size / 2.0),
            egui::vec2(icon_size, icon_size),
        );
        icons::paint(painter, icon, icon_rect, ink);
        if let Some(g) = galley {
            let pos = egui::pos2(icon_rect.right() + 8.0, rect.center().y - g.size().y / 2.0);
            painter.galley(pos, g, ink);
        }
    }
    if enabled {
        let surface = if kind == Kind::Shell { Surface::Shell } else { Surface::Page };
        feedback(ui, surface, &response);
    }
    response
}

/// A small chip: an icon and a short text on a quiet fill.
pub fn chip(ui: &mut egui::Ui, icon: Icon, fill: Color32, ink: Color32, text: &str) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), bold_font(12.0), ink);
    let size = egui::vec2(10.0 + 14.0 + 6.0 + galley.size().x + 12.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(14), fill);
    let icon_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 10.0, rect.center().y - 7.0), egui::vec2(14.0, 14.0));
    icons::paint(painter, icon, icon_rect, ink);
    painter.galley(egui::pos2(icon_rect.right() + 6.0, rect.center().y - galley.size().y / 2.0), galley, ink);
    response
}

/// A borderless band background with the given vertical margin.
pub fn band(fill: Color32, vertical: i8) -> egui::Frame {
    egui::Frame::new().fill(fill).inner_margin(egui::Margin::symmetric(24, vertical))
}

/// Lays the content out in a centred column of at most [`MAX_WIDTH`].
pub fn centered<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let full = ui.available_width();
    let width = full.min(MAX_WIDTH);
    let pad = ((full - width) / 2.0).max(0.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.x = 8.0;
            add(ui)
        })
        .inner
    })
    .inner
}

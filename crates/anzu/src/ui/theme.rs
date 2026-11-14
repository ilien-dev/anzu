//! Colores, tipografía y estilo de la interfaz, en claro y en oscuro.

use std::sync::Arc;

use anzu_core::settings::ThemeChoice;
use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, RichText, Stroke};

/// Ancho máximo del contenido: en monitores grandes queda centrado, no estirado.
pub const MAX_WIDTH: f32 = 1160.0;
pub const RADIUS: CornerRadius = CornerRadius::same(3);
pub const BODY: f32 = 13.0;
pub const SMALL: f32 = 11.0;

const BOLD_FAMILY: &str = "negrita";

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub line: Color32,
    pub fg: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    /// La banda de cabecera, oscura en ambos temas.
    pub shell: Color32,
    pub on_shell: Color32,
    pub shell_muted: Color32,
    pub shell_line: Color32,
    pub shell_accent: Color32,
    pub on_shell_accent: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub warn_bg: Color32,
    pub warn_line: Color32,
    pub select_bg: Color32,
    pub hover_bg: Color32,
    pub log_bg: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// «Papel frío»: blanco frío y tinta. El ámbar solo aparece como acento.
pub const LIGHT: Palette = Palette {
    bg: rgb(0xF3F5F7),
    panel: rgb(0xFFFFFF),
    line: rgb(0xCBD2D9),
    fg: rgb(0x14171A),
    muted: rgb(0x55606B),
    accent: rgb(0xB25E00),
    on_accent: rgb(0xFFFFFF),
    shell: rgb(0x14171A),
    on_shell: rgb(0xF3F5F7),
    shell_muted: rgb(0xA9B3BD),
    shell_line: rgb(0x3A424A),
    shell_accent: rgb(0xFFB000),
    on_shell_accent: rgb(0x14171A),
    ok: rgb(0x1F6B3A),
    warn: rgb(0xA63A12),
    warn_bg: rgb(0xFDEDE6),
    warn_line: rgb(0xE3B6A3),
    select_bg: rgb(0xFFF1D6),
    hover_bg: rgb(0xF7F9FA),
    log_bg: rgb(0xF7F9FA),
};

pub const DARK: Palette = Palette {
    bg: rgb(0x0E0F0C),
    panel: rgb(0x161812),
    line: rgb(0x34382B),
    fg: rgb(0xE9E6D6),
    muted: rgb(0xA7A48F),
    accent: rgb(0xFFB000),
    on_accent: rgb(0x1A1200),
    shell: rgb(0x1C1F17),
    on_shell: rgb(0xE9E6D6),
    shell_muted: rgb(0xA7A48F),
    shell_line: rgb(0x3B4031),
    shell_accent: rgb(0xFFB000),
    on_shell_accent: rgb(0x1A1200),
    ok: rgb(0xA2D975),
    warn: rgb(0xFF9166),
    warn_bg: rgb(0x2B1A12),
    warn_line: rgb(0x6B3A26),
    select_bg: rgb(0x2A2410),
    hover_bg: rgb(0x1B1D16),
    log_bg: rgb(0x0A0B08),
};

/// Colores del icono de la ventana: la tinta de la cabecera con el logotipo en ámbar.
pub const ICON_BACKGROUND: [u8; 3] = [0x14, 0x17, 0x1A];
pub const ICON_MARK: [u8; 3] = [0xFF, 0xB0, 0x00];

impl Palette {
    /// La paleta del tema que está activo ahora mismo.
    pub fn of(ctx: &egui::Context) -> Palette {
        if ctx.theme() == egui::Theme::Dark { DARK } else { LIGHT }
    }
}

fn visuals(p: Palette, dark: bool) -> egui::Visuals {
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.panel;
    v.window_corner_radius = RADIUS;
    v.extreme_bg_color = p.bg;
    v.faint_bg_color = p.panel;
    v.override_text_color = Some(p.fg);
    v.window_stroke = Stroke::new(1.0, p.line);
    v.selection.bg_fill = p.select_bg;
    v.selection.stroke = Stroke::new(1.0, p.fg);
    v.hyperlink_color = p.accent;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
    v.widgets.noninteractive.corner_radius = RADIUS;
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
        w.bg_fill = p.panel;
        w.weak_bg_fill = p.panel;
        w.corner_radius = RADIUS;
        w.fg_stroke = Stroke::new(1.0, p.fg);
    }
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.line);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    v.widgets.hovered.weak_bg_fill = p.hover_bg;
    v.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    v
}

/// Registra la tipografía, los colores de ambos temas y el espaciado. Una vez, al arrancar.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "jetbrains-mono".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "jetbrains-mono-bold".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"))),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts.families.entry(family).or_default().insert(0, "jetbrains-mono".to_owned());
    }
    // La negrita cae a la regular para los caracteres que no tenga.
    fonts
        .families
        .insert(FontFamily::Name(BOLD_FAMILY.into()), vec!["jetbrains-mono-bold".to_owned(), "jetbrains-mono".to_owned()]);
    ctx.set_fonts(fonts);

    ctx.set_visuals_of(egui::Theme::Dark, visuals(DARK, true));
    ctx.set_visuals_of(egui::Theme::Light, visuals(LIGHT, false));
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size.y = 32.0;
        for (text_style, size) in [
            (egui::TextStyle::Body, BODY),
            (egui::TextStyle::Button, BODY),
            (egui::TextStyle::Monospace, BODY),
            (egui::TextStyle::Small, SMALL),
            (egui::TextStyle::Heading, 18.0),
        ] {
            style.text_styles.insert(text_style, FontId::new(size, FontFamily::Proportional));
        }
    });
}

/// Aplica la elección del usuario: seguir al sistema, o forzar claro u oscuro.
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

pub fn bold(text: impl Into<String>) -> RichText {
    RichText::new(text).family(FontFamily::Name(BOLD_FAMILY.into()))
}

pub fn muted(p: Palette, text: impl Into<String>) -> RichText {
    RichText::new(text).color(p.muted)
}

/// Un panel con borde fino.
pub fn panel(p: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0, p.line))
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::symmetric(14, 10))
}

pub fn warn_panel(p: Palette) -> egui::Frame {
    panel(p).fill(p.warn_bg).stroke(Stroke::new(1.0, p.warn_line)).inner_margin(egui::Margin::symmetric(16, 14))
}

pub fn primary_button(p: Palette, label: &str) -> egui::Button<'static> {
    egui::Button::new(bold(label.to_string()).color(p.on_accent).size(14.0))
        .fill(p.accent)
        .stroke(Stroke::NONE)
        .min_size(egui::vec2(0.0, 38.0))
}

/// Un botón para la banda de cabecera, que es oscura en ambos temas.
pub fn shell_button(p: Palette, label: &str, selected: bool) -> egui::Button<'static> {
    let (fill, text) = if selected { (p.shell_accent, p.on_shell_accent) } else { (p.shell, p.on_shell) };
    egui::Button::new(RichText::new(label.to_string()).color(text).size(12.0))
        .fill(fill)
        .stroke(Stroke::new(1.0, if selected { p.shell_accent } else { p.shell_line }))
}

/// Un fondo de banda sin borde, con el margen vertical dado.
pub fn band(fill: Color32, vertical: i8) -> egui::Frame {
    egui::Frame::new().fill(fill).inner_margin(egui::Margin::symmetric(24, vertical))
}

/// Coloca el contenido en una columna centrada de como mucho [`MAX_WIDTH`].
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

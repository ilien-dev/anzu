//! The Anzu logo: an A made of two open wings over a tablet.
//!
//! The shape is defined once, on a 64 x 64 grid, in [`super::mark`]: the drawing on screen,
//! the window icon and the icon embedded in the executable all come from it. Each part has
//! its own colour: a lilac wing, a peach wing and a butter tablet.

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke};

pub use super::mark::icon_rgba;
use super::mark::{GRID, LEFT_WING, MarkColors, RIGHT_WING, TABLET};

/// The colours of the logo on screen.
pub fn colors(mark: MarkColors) -> [Color32; 3] {
    let c = |rgb: [u8; 3]| Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    [c(mark.left_wing), c(mark.right_wing), c(mark.tablet)]
}

/// Draws the logo inside `rect`, each part in its colour: left wing, right wing, tablet.
pub fn paint(painter: &egui::Painter, rect: Rect, [left, right, tablet_color]: [Color32; 3]) {
    let scale = rect.width().min(rect.height()) / GRID;
    let at = |(x, y): (f32, f32)| Pos2::new(rect.min.x + x * scale, rect.min.y + y * scale);
    for (wing, color) in [(LEFT_WING, left), (RIGHT_WING, right)] {
        painter.add(Shape::convex_polygon(wing.iter().map(|p| at(*p)).collect(), color, Stroke::NONE));
    }
    let (tx, ty, tw, th) = TABLET;
    let tablet = Rect::from_min_max(at((tx, ty)), at((tx + tw, ty + th)));
    painter.rect_filled(tablet, CornerRadius::same((2.0 * scale).round() as u8), tablet_color);
}

/// The logo on a rounded tile, as in the app icon.
pub fn badge(ui: &mut egui::Ui, size: f32, background: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same((size * 0.28) as u8), background);
    paint(ui.painter(), rect.shrink(size * 0.17), colors(super::mark::ICON_MARK));
}

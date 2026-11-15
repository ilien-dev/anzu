//! The Anzu logo: an A made of two open wings over a tablet.
//!
//! The shape is defined once, on a 64 x 64 grid, in [`super::mark`]: the drawing on screen,
//! the window icon and the icon embedded in the executable all come from it.

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke};

pub use super::mark::icon_rgba;
use super::mark::{GRID, LEFT_WING, RIGHT_WING, TABLET};

/// Draws the logo inside `rect`.
pub fn paint(painter: &egui::Painter, rect: Rect, color: Color32) {
    let scale = rect.width().min(rect.height()) / GRID;
    let at = |(x, y): (f32, f32)| Pos2::new(rect.min.x + x * scale, rect.min.y + y * scale);
    for wing in [LEFT_WING, RIGHT_WING] {
        painter.add(Shape::convex_polygon(wing.iter().map(|p| at(*p)).collect(), color, Stroke::NONE));
    }
    let (tx, ty, tw, th) = TABLET;
    let tablet = Rect::from_min_max(at((tx, ty)), at((tx + tw, ty + th)));
    painter.rect_filled(tablet, CornerRadius::same((1.5 * scale).round() as u8), color);
}

/// The logo on a rounded tile, as in the app icon.
pub fn badge(ui: &mut egui::Ui, size: f32, background: Color32, mark: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same((size * 0.23) as u8), background);
    paint(ui.painter(), rect.shrink(size * 0.17), mark);
}

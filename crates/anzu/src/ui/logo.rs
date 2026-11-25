//! The Anzu logo: a solid block that turns into lines of text, a locked book becoming an EPUB.
//!
//! The shape is defined once, on a 64 x 64 grid, in [`super::mark`]: the drawing on screen,
//! the window icon and the icon embedded in the executable all come from it. Each part has
//! its own colour: a peach block and mint lines, the colours of a book with DRM and in EPUB.

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect};

pub use super::mark::icon_rgba;
use super::mark::{BLOCK_RADIUS, BLOCK_X, MarkColors, Shape};

/// The colours of the logo on screen: block, lines.
pub fn colors(mark: MarkColors) -> [Color32; 2] {
    let c = |rgb: [u8; 3]| Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    [c(mark.block), c(mark.lines)]
}

/// Draws the logo inside `rect`, each part in its colour: block, lines.
pub fn paint(painter: &egui::Painter, rect: Rect, [block, lines]: [Color32; 2]) {
    let side = rect.width().min(rect.height());
    let shape = Shape::for_size(side);
    let scale = side / super::mark::GRID;
    let at = |x: f32, y: f32| Pos2::new(rect.min.x + x * scale, rect.min.y + y * scale);
    let radius = |r: f32| (r * scale).round() as u8;
    // The lines first, starting under the block, so no seam shows where the two colours meet.
    let h = shape.line_height;
    for &(top, end) in shape.lines {
        let line = Rect::from_min_max(at(BLOCK_X.1 - 2.0, top), at(end, top + h));
        let round = radius(h / 2.0);
        painter.rect_filled(line, CornerRadius { nw: 0, ne: round, sw: 0, se: round }, lines);
    }
    let (top, bottom) = shape.block_y();
    let body = Rect::from_min_max(at(BLOCK_X.0, top), at(BLOCK_X.1, bottom));
    let round = radius(BLOCK_RADIUS);
    painter.rect_filled(body, CornerRadius { nw: round, ne: 0, sw: round, se: 0 }, block);
}

/// The logo on a rounded tile, as in the app icon.
pub fn badge(ui: &mut egui::Ui, size: f32, background: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same((size * 0.28) as u8), background);
    paint(ui.painter(), rect.shrink(size * super::mark::TILE_INSET), colors(super::mark::ICON_MARK));
}

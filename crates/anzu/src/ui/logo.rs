//! The Anzu logo: an A made of two open wings over a tablet.
//!
//! The shape is defined once, on a 64 x 64 grid, and both the drawing on screen and the
//! window icon come from it.

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke};

const GRID: f32 = 64.0;
/// Left wing and right wing: two convex quadrilaterals that meet at the apex.
const LEFT_WING: [(f32, f32); 4] = [(32.0, 6.0), (10.0, 58.0), (20.0, 58.0), (32.0, 28.0)];
const RIGHT_WING: [(f32, f32); 4] = [(32.0, 6.0), (32.0, 28.0), (44.0, 58.0), (54.0, 58.0)];
/// The tablet: x, y, width, height.
const TABLET: (f32, f32, f32, f32) = (28.5, 46.0, 7.0, 12.0);

fn in_quad(quad: &[(f32, f32); 4], x: f32, y: f32) -> bool {
    let mut sign = 0.0f32;
    for i in 0..4 {
        let (ax, ay) = quad[i];
        let (bx, by) = quad[(i + 1) % 4];
        let cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        if cross != 0.0 {
            if sign != 0.0 && cross.signum() != sign {
                return false;
            }
            sign = cross.signum();
        }
    }
    true
}

/// Whether a point of the 64 x 64 grid belongs to the logo.
pub fn in_mark(x: f32, y: f32) -> bool {
    let (tx, ty, tw, th) = TABLET;
    in_quad(&LEFT_WING, x, y) || in_quad(&RIGHT_WING, x, y) || (x >= tx && x <= tx + tw && y >= ty && y <= ty + th)
}

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

/// The window icon: a rounded tile with the logo, as RGBA.
pub fn icon_rgba(size: u32, background: [u8; 3], mark: [u8; 3]) -> Vec<u8> {
    let n = size as f32;
    let radius = n * 0.23;
    let inset = n * 0.17;
    let scale = (n - 2.0 * inset) / GRID;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for py in 0..size {
        for px in 0..size {
            let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
            // Distance to the inner rectangle of the rounded tile.
            let dx = (radius - x).max(x - (n - radius)).max(0.0);
            let dy = (radius - y).max(y - (n - radius)).max(0.0);
            let inside_tile = dx * dx + dy * dy <= radius * radius;
            let rgba = if !inside_tile {
                [0, 0, 0, 0]
            } else if in_mark((x - inset) / scale, (y - inset) / scale) {
                [mark[0], mark[1], mark[2], 255]
            } else {
                [background[0], background[1], background[2], 255]
            };
            pixels.extend_from_slice(&rgba);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_an_a_with_a_tablet_under_it() {
        assert!(in_mark(32.0, 12.0), "apex");
        assert!(in_mark(15.0, 55.0), "foot of the left wing");
        assert!(in_mark(49.0, 55.0), "foot of the right wing");
        assert!(in_mark(32.0, 50.0), "tablet");
        assert!(!in_mark(32.0, 38.0), "gap between the wings and the tablet");
        assert!(!in_mark(25.0, 55.0), "gap between a wing and the tablet");
        assert!(!in_mark(5.0, 5.0), "outside");
        assert!(!in_mark(60.0, 30.0), "outside");
    }

    #[test]
    fn the_mark_is_symmetric() {
        for (x, y) in [(14.0, 52.0), (26.0, 30.0), (30.0, 12.0), (20.0, 40.0), (29.0, 50.0)] {
            assert_eq!(in_mark(x, y), in_mark(64.0 - x, y), "({x}, {y})");
        }
    }

    #[test]
    fn icon_has_transparent_corners_and_both_colours() {
        let size = 64;
        let px = icon_rgba(size, [10, 20, 30], [200, 210, 220]);
        assert_eq!(px.len(), (size * size * 4) as usize);
        let at = |x: u32, y: u32| {
            let i = ((y * size + x) * 4) as usize;
            [px[i], px[i + 1], px[i + 2], px[i + 3]]
        };
        assert_eq!(at(0, 0)[3], 0, "transparent corner");
        assert_eq!(at(32, 3), [10, 20, 30, 255], "tile background");
        assert_eq!(at(32, 16), [200, 210, 220, 255], "apex of the A");
    }
}

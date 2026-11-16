//! The geometry of the Anzu logo and the app icon, with no dependency on egui.
//!
//! `build.rs` includes this file too, to embed the same icon in the executable.

pub const GRID: f32 = 64.0;
/// Left wing and right wing: two convex quadrilaterals that meet at the apex.
pub const LEFT_WING: [(f32, f32); 4] = [(32.0, 6.0), (10.0, 58.0), (20.0, 58.0), (32.0, 28.0)];
pub const RIGHT_WING: [(f32, f32); 4] = [(32.0, 6.0), (32.0, 28.0), (44.0, 58.0), (54.0, 58.0)];
/// The tablet: x, y, width, height.
pub const TABLET: (f32, f32, f32, f32) = (27.0, 44.0, 10.0, 14.0);

/// The three parts of the logo, each painted in its own colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkColors {
    pub left_wing: [u8; 3],
    pub right_wing: [u8; 3],
    pub tablet: [u8; 3],
}

/// Colours of the app icon: ink tile, lilac and peach wings, butter tablet.
pub const ICON_BACKGROUND: [u8; 3] = [0x15, 0x16, 0x1A];
pub const ICON_MARK: MarkColors =
    MarkColors { left_wing: [0xC9, 0xB8, 0xFF], right_wing: [0xFF, 0xB4, 0xA2], tablet: [0xFF, 0xF2, 0x7A] };

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

/// A part of the logo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    LeftWing,
    RightWing,
    Tablet,
}

/// The part of the logo a point of the 64 x 64 grid belongs to, if any.
pub fn part_at(x: f32, y: f32) -> Option<Part> {
    let (tx, ty, tw, th) = TABLET;
    if in_quad(&LEFT_WING, x, y) {
        Some(Part::LeftWing)
    } else if in_quad(&RIGHT_WING, x, y) {
        Some(Part::RightWing)
    } else if x >= tx && x <= tx + tw && y >= ty && y <= ty + th {
        Some(Part::Tablet)
    } else {
        None
    }
}

/// Whether a point of the 64 x 64 grid belongs to the logo.
#[cfg(test)]
pub fn in_mark(x: f32, y: f32) -> bool {
    part_at(x, y).is_some()
}

/// The app icon: a rounded tile with the logo, as RGBA.
pub fn icon_rgba(size: u32, background: [u8; 3], mark: MarkColors) -> Vec<u8> {
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
            } else {
                let c = match part_at((x - inset) / scale, (y - inset) / scale) {
                    Some(Part::LeftWing) => mark.left_wing,
                    Some(Part::RightWing) => mark.right_wing,
                    Some(Part::Tablet) => mark.tablet,
                    None => background,
                };
                [c[0], c[1], c[2], 255]
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
    fn icon_has_transparent_corners_and_every_colour() {
        let size = 64;
        let colors = MarkColors { left_wing: [200, 210, 220], right_wing: [1, 2, 3], tablet: [4, 5, 6] };
        let px = icon_rgba(size, [10, 20, 30], colors);
        assert_eq!(px.len(), (size * size * 4) as usize);
        let at = |x: u32, y: u32| {
            let i = ((y * size + x) * 4) as usize;
            [px[i], px[i + 1], px[i + 2], px[i + 3]]
        };
        assert_eq!(at(0, 0)[3], 0, "transparent corner");
        assert_eq!(at(32, 3), [10, 20, 30, 255], "tile background");
        assert_eq!(at(28, 26), [200, 210, 220, 255], "left wing");
        assert_eq!(at(35, 26), [1, 2, 3, 255], "right wing");
        assert_eq!(at(32, 47), [4, 5, 6, 255], "tablet");
    }
}

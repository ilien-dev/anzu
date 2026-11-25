//! The geometry of the Anzu logo and the app icon, with no dependency on egui.
//!
//! The logo is a solid block that turns into lines of text as it moves right: a locked book
//! becoming an EPUB. `build.rs` includes this file too, to embed the same icon in the
//! executable, and `assets/brand/logo/` holds the same drawing as SVG.

pub const GRID: f32 = 64.0;

/// One cut of the logo on the 64 x 64 grid. The block spans `BLOCK_X` across and the full
/// height of the lines; each line starts at the right edge of the block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    /// Top and right end of each line.
    pub lines: &'static [(f32, f32)],
    /// Height of every line; its right end is a half circle of half this height.
    pub line_height: f32,
    /// Radius of the round start of each gap between lines, cut into the block.
    pub notch: f32,
}

/// Left and right edge of the block.
pub const BLOCK_X: (f32, f32) = (6.0, 28.0);
/// Radius of the two left corners of the block.
pub const BLOCK_RADIUS: f32 = 7.0;

/// The full logo: four lines with ragged ends, like a page of text.
pub const FULL: Shape =
    Shape { lines: &[(14.0, 58.0), (24.0, 48.0), (34.0, 54.0), (44.0, 40.0)], line_height: 6.0, notch: 2.0 };
/// The small cut, for icons of 32 px and below: three thicker lines and wider gaps.
pub const SMALL: Shape = Shape { lines: &[(14.0, 58.0), (28.0, 46.0), (42.0, 52.0)], line_height: 8.0, notch: 3.0 };

impl Shape {
    /// The cut to draw when the whole logo is `size` pixels wide.
    pub fn for_size(size: f32) -> Shape {
        if size <= 22.0 { SMALL } else { FULL }
    }

    /// Top and bottom of the block: those of the first and the last line.
    pub fn block_y(&self) -> (f32, f32) {
        let top = self.lines.first().map_or(0.0, |l| l.0);
        let bottom = self.lines.last().map_or(0.0, |l| l.0 + self.line_height);
        (top, bottom)
    }

    /// The part of the logo a point of the 64 x 64 grid belongs to, if any.
    pub fn part_at(&self, x: f32, y: f32) -> Option<Part> {
        let h = self.line_height;
        let r = h / 2.0;
        for &(top, end) in self.lines {
            if y >= top && y <= top + h && x >= BLOCK_X.1 && x <= end {
                let (cx, cy) = (end - r, top + r);
                if x <= cx || (x - cx).powi(2) + (y - cy).powi(2) <= r * r {
                    return Some(Part::Lines);
                }
            }
        }
        let (top, bottom) = self.block_y();
        if x < BLOCK_X.0 || x > BLOCK_X.1 || y < top || y > bottom {
            return None;
        }
        let (left, radius) = (BLOCK_X.0 + BLOCK_RADIUS, BLOCK_RADIUS);
        for cy in [top + radius, bottom - radius] {
            let outside_corner = x < left && (if cy < 32.0 { y < cy } else { y > cy });
            if outside_corner && (x - left).powi(2) + (y - cy).powi(2) > radius * radius {
                return None;
            }
        }
        for pair in self.lines.windows(2) {
            let cy = (pair[0].0 + h + pair[1].0) / 2.0;
            if (x - BLOCK_X.1).powi(2) + (y - cy).powi(2) < self.notch * self.notch {
                return None;
            }
        }
        Some(Part::Block)
    }
}

/// The two parts of the logo, each painted in its own colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkColors {
    pub block: [u8; 3],
    pub lines: [u8; 3],
}

/// Colours of the app icon: ink tile, a peach block (the colour of a book with DRM) and mint
/// lines (the colour of a book in EPUB).
pub const ICON_BACKGROUND: [u8; 3] = [0x15, 0x16, 0x1A];
pub const ICON_MARK: MarkColors = MarkColors { block: [0xFF, 0xB4, 0xA2], lines: [0xC8, 0xF0, 0xA0] };

/// A part of the logo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Block,
    Lines,
}

/// Inset of the logo inside the rounded tile of the icon, as a share of the tile.
pub const TILE_INSET: f32 = 0.17;

/// The app icon: a rounded tile with the logo, as RGBA. The cut follows `size`.
pub fn icon_rgba(size: u32, background: [u8; 3], mark: MarkColors) -> Vec<u8> {
    icon_rgba_with(size, Shape::for_size(size as f32 * (1.0 - 2.0 * TILE_INSET)), background, mark)
}

/// The app icon drawn with a given cut, for when it is drawn larger and scaled down.
pub fn icon_rgba_with(size: u32, shape: Shape, background: [u8; 3], mark: MarkColors) -> Vec<u8> {
    let n = size as f32;
    let radius = n * 0.23;
    let inset = n * TILE_INSET;
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
                let c = match shape.part_at((x - inset) / scale, (y - inset) / scale) {
                    Some(Part::Block) => mark.block,
                    Some(Part::Lines) => mark.lines,
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
    fn the_mark_is_a_block_that_turns_into_lines() {
        let at = |x, y| FULL.part_at(x, y);
        assert_eq!(at(10.0, 32.0), Some(Part::Block), "middle of the block");
        assert_eq!(at(27.0, 16.0), Some(Part::Block), "block, in the row of the first line");
        assert_eq!(at(40.0, 17.0), Some(Part::Lines), "first line");
        assert_eq!(at(52.0, 47.0), None, "the last line is the shortest but one");
        assert_eq!(at(38.0, 47.0), Some(Part::Lines), "last line");
        assert_eq!(at(40.0, 22.0), None, "gap between the first two lines");
        assert_eq!(at(27.5, 22.0), None, "the gap starts inside the block");
        assert_eq!(at(20.0, 22.0), Some(Part::Block), "the block is solid behind the gaps");
        assert_eq!(at(6.5, 14.5), None, "rounded top left corner");
        assert_eq!(at(6.5, 49.5), None, "rounded bottom left corner");
        assert_eq!(at(31.0, 8.0), None, "above the logo");
        assert_eq!(at(61.0, 32.0), None, "right of the logo");
    }

    #[test]
    fn the_lines_end_ragged_with_round_ends() {
        for &(top, end) in FULL.lines {
            let mid = top + FULL.line_height / 2.0;
            assert_eq!(FULL.part_at(end - 0.5, mid), Some(Part::Lines), "end of the line at {top}");
            assert_eq!(FULL.part_at(end + 0.5, mid), None, "past the end of the line at {top}");
            assert_eq!(FULL.part_at(end - 0.3, top + 0.2), None, "round end of the line at {top}");
        }
        let ends: Vec<f32> = FULL.lines.iter().map(|l| l.1).collect();
        assert!(ends.windows(2).all(|w| w[0] != w[1]), "no two lines end together");
    }

    #[test]
    fn the_small_cut_has_three_lines_and_the_same_block() {
        assert_eq!(SMALL.lines.len(), 3);
        assert_eq!(SMALL.block_y(), FULL.block_y());
        assert_eq!(Shape::for_size(10.0), SMALL);
        assert_eq!(Shape::for_size(21.0), SMALL, "a 32 px icon");
        assert_eq!(Shape::for_size(32.0), FULL, "a 48 px icon");
    }

    #[test]
    fn icon_has_transparent_corners_and_every_colour() {
        let size = 64;
        let colors = MarkColors { block: [200, 210, 220], lines: [1, 2, 3] };
        let px = icon_rgba(size, [10, 20, 30], colors);
        assert_eq!(px.len(), (size * size * 4) as usize);
        let at = |x: u32, y: u32| {
            let i = ((y * size + x) * 4) as usize;
            [px[i], px[i + 1], px[i + 2], px[i + 3]]
        };
        // The logo spans 64 * 0.66 = 42.24 px from an inset of 10.88 px: one grid unit is 0.66 px.
        let to_px = |g: f32| (10.88 + g * 0.66) as u32;
        assert_eq!(at(0, 0)[3], 0, "transparent corner");
        assert_eq!(at(32, 3), [10, 20, 30, 255], "tile background");
        assert_eq!(at(to_px(14.0), to_px(32.0)), [200, 210, 220, 255], "block");
        assert_eq!(at(to_px(40.0), to_px(17.0)), [1, 2, 3, 255], "first line");
        assert_eq!(at(to_px(44.0), to_px(32.0)), [10, 20, 30, 255], "gap between lines");
    }
}

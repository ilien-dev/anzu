//! Icons drawn with strokes, all of the same weight.

use anzu_core::library::BookState;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke, StrokeKind};

use super::theme::Palette;

const STROKE: f32 = 1.8;

/// A check mark inside `rect`.
pub fn check(painter: &egui::Painter, rect: Rect, color: Color32) {
    let at = |x: f32, y: f32| Pos2::new(rect.left() + rect.width() * x, rect.top() + rect.height() * y);
    painter.add(Shape::line(vec![at(0.18, 0.54), at(0.42, 0.76), at(0.84, 0.26)], Stroke::new(STROKE, color)));
}

/// A cog with eight teeth and a hole, for the settings button.
pub fn gear(painter: &egui::Painter, rect: Rect, color: Color32) {
    use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0 - 0.75;
    let at = |angle: f32, radius: f32| c + egui::vec2(angle.cos(), angle.sin()) * radius;
    let (outer, inner) = (r, r * 0.72);
    let (tip, root) = (FRAC_PI_8 * 0.42, FRAC_PI_8 * 0.62);
    let mut outline = Vec::with_capacity(32);
    for i in 0..8 {
        let a = i as f32 * FRAC_PI_4;
        outline.extend([at(a - root, inner), at(a - tip, outer), at(a + tip, outer), at(a + root, inner)]);
    }
    let stroke = Stroke::new(1.5, color);
    painter.add(Shape::closed_line(outline, stroke));
    painter.circle_stroke(c, r * 0.3, stroke);
}

/// An arrow pointing up inside a ring: a new version.
pub fn update(painter: &egui::Painter, rect: Rect, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0;
    let stroke = Stroke::new(1.5, color);
    painter.circle_stroke(c, r - 0.75, stroke);
    let (top, bottom) = (c.y - r * 0.45, c.y + r * 0.45);
    painter.line_segment([Pos2::new(c.x, top), Pos2::new(c.x, bottom)], stroke);
    let wing = r * 0.38;
    painter.add(Shape::line(
        vec![Pos2::new(c.x - wing, top + wing), Pos2::new(c.x, top), Pos2::new(c.x + wing, top + wing)],
        stroke,
    ));
}

/// A selection box.
pub fn checkbox(painter: &egui::Painter, rect: Rect, checked: bool, enabled: bool, p: Palette) {
    let radius = CornerRadius::same(2);
    if checked {
        painter.rect_filled(rect, radius, p.accent);
        check(painter, rect.shrink(2.0), p.on_accent);
    } else {
        let line = if enabled { p.muted } else { p.line };
        painter.rect_stroke(rect, radius, Stroke::new(1.2, line), StrokeKind::Inside);
    }
}

/// The sort arrow next to a column header.
pub fn sort_arrow(painter: &egui::Painter, center: Pos2, ascending: bool, color: Color32) {
    let (w, h) = (4.0, 3.0);
    let points = if ascending {
        vec![Pos2::new(center.x - w, center.y + h), Pos2::new(center.x + w, center.y + h), Pos2::new(center.x, center.y - h)]
    } else {
        vec![Pos2::new(center.x - w, center.y - h), Pos2::new(center.x + w, center.y - h), Pos2::new(center.x, center.y + h)]
    };
    painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

/// How many stages of the track a book has done, and the colour they are painted in.
pub fn track_progress(state: BookState, p: Palette) -> (usize, Color32) {
    match state {
        BookState::Locked => (1, p.warn),
        BookState::Unlocked => (2, p.accent),
        BookState::Converted => (3, p.ok),
    }
}

pub const TRACK_WIDTH: f32 = 3.0 * 18.0 + 2.0 * 4.0;

/// The track of a book: with DRM, unlocked, EPUB. Returns where it ends.
pub fn track(painter: &egui::Painter, left_center: Pos2, state: BookState, enabled: bool, p: Palette) -> f32 {
    let (done, color) = track_progress(state, p);
    let color = if enabled { color } else { p.muted };
    let (w, h, gap) = (18.0, 6.0, 4.0);
    for i in 0..3 {
        let x = left_center.x + i as f32 * (w + gap);
        let segment = Rect::from_min_size(Pos2::new(x, left_center.y - h / 2.0), egui::vec2(w, h));
        if i < done {
            painter.rect_filled(segment, CornerRadius::same(1), color);
        } else {
            painter.rect_stroke(segment, CornerRadius::same(1), Stroke::new(1.0, p.line), StrokeKind::Inside);
        }
    }
    left_center.x + TRACK_WIDTH
}

/// A segment of the step bar for a step that was not needed: diagonal stripes.
pub fn skipped_bar(painter: &egui::Painter, rect: Rect, p: Palette) {
    painter.rect_filled(rect, CornerRadius::same(2), p.hover_bg);
    let clipped = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let stroke = Stroke::new(2.0, p.line);
    let mut x = rect.left() - rect.height();
    while x < rect.right() {
        clipped.line_segment([Pos2::new(x, rect.bottom()), Pos2::new(x + rect.height(), rect.top())], stroke);
        x += 6.0;
    }
}

/// A document with a folded corner, inside `rect`.
pub fn file(painter: &egui::Painter, rect: Rect, color: Color32) {
    let fold = rect.width() * 0.35;
    let (l, t, r, b) = (rect.left(), rect.top(), rect.right(), rect.bottom());
    let stroke = Stroke::new(1.4, color);
    let outline = vec![Pos2::new(l, t), Pos2::new(r - fold, t), Pos2::new(r, t + fold), Pos2::new(r, b), Pos2::new(l, b)];
    painter.add(Shape::closed_line(outline, stroke));
    painter.add(Shape::line(vec![Pos2::new(r - fold, t), Pos2::new(r - fold, t + fold), Pos2::new(r, t + fold)], stroke));
}

/// A circled exclamation mark.
pub fn alert(painter: &egui::Painter, center: Pos2, color: Color32) {
    painter.circle_stroke(center, 8.0, Stroke::new(1.5, color));
    painter.line_segment([Pos2::new(center.x, center.y - 4.0), Pos2::new(center.x, center.y + 1.0)], Stroke::new(STROKE, color));
    painter.circle_filled(Pos2::new(center.x, center.y + 4.0), 1.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::LIGHT;

    #[test]
    fn the_track_fills_one_more_stage_per_state() {
        assert_eq!(track_progress(BookState::Locked, LIGHT).0, 1);
        assert_eq!(track_progress(BookState::Unlocked, LIGHT).0, 2);
        assert_eq!(track_progress(BookState::Converted, LIGHT).0, 3);
    }

    #[test]
    fn each_state_has_its_own_colour() {
        let colours: std::collections::HashSet<_> =
            [BookState::Locked, BookState::Unlocked, BookState::Converted].map(|s| track_progress(s, LIGHT).1).into();
        assert_eq!(colours.len(), 3);
    }
}

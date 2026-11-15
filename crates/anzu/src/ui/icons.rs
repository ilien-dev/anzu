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

/// The mark of a step: done, running, pending or not completed.
pub enum StepMark {
    Done,
    Running,
    Pending,
    Failed,
}

pub fn step(painter: &egui::Painter, center: Pos2, mark: StepMark, p: Palette) {
    let r = 7.0;
    match mark {
        StepMark::Done => {
            painter.circle_filled(center, r, p.ok);
            check(painter, Rect::from_center_size(center, egui::vec2(11.0, 11.0)), p.panel);
        }
        StepMark::Running => {
            painter.circle_stroke(center, r, Stroke::new(STROKE, p.accent));
            painter.circle_filled(center, 3.0, p.accent);
        }
        StepMark::Pending => {
            painter.circle_stroke(center, r, Stroke::new(1.2, p.line));
        }
        StepMark::Failed => {
            painter.circle_stroke(center, r, Stroke::new(STROKE, p.warn));
            painter.line_segment(
                [Pos2::new(center.x, center.y - 3.5), Pos2::new(center.x, center.y + 0.8)],
                Stroke::new(STROKE, p.warn),
            );
            painter.circle_filled(Pos2::new(center.x, center.y + 3.6), 1.0, p.warn);
        }
    }
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

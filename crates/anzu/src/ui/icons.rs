//! Icons drawn with strokes, all of the same weight, on a 24 x 24 grid.

use anzu_core::library::BookState;
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Shape, Stroke, StrokeKind};

use super::theme::{self, Palette};

/// Line weight on the 24 x 24 grid.
const WEIGHT: f32 = 1.8;

/// Every icon of the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Check,
    Lock,
    Unlock,
    File,
    Folder,
    Refresh,
    Shield,
    Device,
    Book,
    Package,
    Hash,
    Sun,
    Moon,
    Monitor,
    Globe,
    Copy,
    External,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Download,
    Close,
    ChevronDown,
    ChevronRight,
    Clock,
    Info,
    Terminal,
    Alert,
    Gear,
    Update,
}

/// A pen that draws on the 24 x 24 grid of `rect`.
struct Pen<'a> {
    painter: &'a egui::Painter,
    rect: Rect,
    stroke: Stroke,
}

impl Pen<'_> {
    fn scale(&self) -> f32 {
        self.rect.width().min(self.rect.height()) / 24.0
    }

    fn at(&self, x: f32, y: f32) -> Pos2 {
        let s = self.scale();
        let origin = self.rect.center() - egui::vec2(12.0, 12.0) * s;
        origin + egui::vec2(x, y) * s
    }

    fn line(&self, points: &[(f32, f32)]) {
        let points = points.iter().map(|(x, y)| self.at(*x, *y)).collect();
        self.painter.add(Shape::line(points, self.stroke));
    }

    fn closed(&self, points: &[(f32, f32)]) {
        let points = points.iter().map(|(x, y)| self.at(*x, *y)).collect();
        self.painter.add(Shape::closed_line(points, self.stroke));
    }

    fn circle(&self, x: f32, y: f32, r: f32) {
        self.painter.circle_stroke(self.at(x, y), r * self.scale(), self.stroke);
    }

    fn dot(&self, x: f32, y: f32, r: f32) {
        self.painter.circle_filled(self.at(x, y), r * self.scale(), self.stroke.color);
    }

    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) {
        let r = Rect::from_min_max(self.at(x0, y0), self.at(x1, y1));
        let radius = CornerRadius::same((radius * self.scale()).round() as u8);
        self.painter.rect_stroke(r, radius, self.stroke, StrokeKind::Middle);
    }

    /// Points of an arc around (cx, cy), from `from` to `to` degrees (y down, clockwise).
    fn arc_points(cx: f32, cy: f32, r: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
        let steps = (((to - from).abs() / 10.0).ceil() as usize).max(2);
        (0..=steps)
            .map(|i| {
                let a = (from + (to - from) * i as f32 / steps as f32).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }

    fn arc(&self, cx: f32, cy: f32, r: f32, from: f32, to: f32) {
        self.line(&Self::arc_points(cx, cy, r, from, to));
    }
}

/// Draws `icon` inside `rect` in `color`.
pub fn paint(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    let scale = rect.width().min(rect.height()) / 24.0;
    let pen = Pen { painter, rect, stroke: Stroke::new((WEIGHT * scale).max(1.2), color) };
    match icon {
        Icon::Check => pen.line(&[(5.0, 12.5), (10.0, 17.0), (19.0, 7.0)]),
        Icon::Lock | Icon::Unlock => {
            pen.rect(5.0, 11.0, 19.0, 21.0, 2.0);
            let mut shackle = vec![(8.0, 11.0)];
            shackle.extend(Pen::arc_points(12.0, 7.5, 4.0, 180.0, 360.0));
            shackle.push(if icon == Icon::Lock { (16.0, 11.0) } else { (16.0, 8.5) });
            if icon == Icon::Unlock {
                // The shackle swings open: the right foot stays above the body.
                shackle = shackle.iter().map(|(x, y)| (*x, *y - if *x > 12.0 { 2.0 } else { 0.0 })).collect();
                shackle[0] = (8.0, 11.0);
            }
            pen.line(&shackle);
        }
        Icon::File => {
            pen.closed(&[(6.0, 3.0), (14.0, 3.0), (19.0, 8.0), (19.0, 21.0), (6.0, 21.0)]);
            pen.line(&[(14.0, 3.0), (14.0, 8.0), (19.0, 8.0)]);
            pen.line(&[(9.0, 13.0), (16.0, 13.0)]);
            pen.line(&[(9.0, 17.0), (16.0, 17.0)]);
        }
        Icon::Folder => pen.closed(&[(3.0, 5.5), (9.0, 5.5), (11.0, 8.0), (21.0, 8.0), (21.0, 19.0), (3.0, 19.0)]),
        Icon::Refresh => {
            pen.arc(12.0, 12.0, 7.5, -30.0, 270.0);
            pen.line(&[(13.5, 1.5), (12.0, 4.5), (15.0, 6.3)]);
        }
        Icon::Shield => {
            pen.closed(&[
                (12.0, 2.5),
                (19.5, 5.5),
                (19.5, 11.5),
                (17.0, 16.5),
                (12.0, 21.5),
                (7.0, 16.5),
                (4.5, 11.5),
                (4.5, 5.5),
            ]);
            pen.line(&[(8.5, 12.0), (11.0, 14.5), (15.5, 9.5)]);
        }
        Icon::Device => {
            pen.rect(6.0, 2.5, 18.0, 21.5, 2.5);
            pen.line(&[(10.0, 18.0), (14.0, 18.0)]);
        }
        Icon::Book => {
            pen.line(&[(12.0, 7.0), (12.0, 20.0)]);
            pen.closed(&[(12.0, 7.0), (9.0, 5.0), (3.0, 5.0), (3.0, 18.0), (9.0, 18.0), (12.0, 20.0)]);
            pen.closed(&[(12.0, 7.0), (15.0, 5.0), (21.0, 5.0), (21.0, 18.0), (15.0, 18.0), (12.0, 20.0)]);
        }
        Icon::Package => {
            pen.closed(&[(12.0, 2.5), (20.5, 7.0), (20.5, 17.0), (12.0, 21.5), (3.5, 17.0), (3.5, 7.0)]);
            pen.line(&[(3.5, 7.0), (12.0, 11.5), (20.5, 7.0)]);
            pen.line(&[(12.0, 11.5), (12.0, 21.5)]);
        }
        Icon::Hash => {
            pen.line(&[(4.5, 9.0), (19.5, 9.0)]);
            pen.line(&[(4.5, 15.0), (19.5, 15.0)]);
            pen.line(&[(10.0, 4.0), (8.0, 20.0)]);
            pen.line(&[(16.0, 4.0), (14.0, 20.0)]);
        }
        Icon::Sun => {
            pen.circle(12.0, 12.0, 4.0);
            for i in 0..8 {
                let a = (i as f32 * 45.0).to_radians();
                let (c, s) = (a.cos(), a.sin());
                pen.line(&[(12.0 + 7.0 * c, 12.0 + 7.0 * s), (12.0 + 9.5 * c, 12.0 + 9.5 * s)]);
            }
        }
        Icon::Moon => {
            let mut points = Pen::arc_points(12.0, 12.0, 9.0, 0.0, 270.0);
            points.extend(Pen::arc_points(16.5, 7.5, 6.36, 225.0, 45.0));
            pen.closed(&points);
        }
        Icon::Monitor => {
            pen.rect(3.0, 4.0, 21.0, 16.0, 2.0);
            pen.line(&[(12.0, 16.0), (12.0, 20.0)]);
            pen.line(&[(8.0, 20.0), (16.0, 20.0)]);
        }
        Icon::Globe => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line(&[(3.0, 12.0), (21.0, 12.0)]);
            let mut meridian = Pen::arc_points(0.0, 0.0, 1.0, -90.0, 270.0);
            for (x, y) in &mut meridian {
                (*x, *y) = (12.0 + 4.0 * *x, 12.0 + 9.0 * *y);
            }
            pen.line(&meridian);
        }
        Icon::Copy => {
            pen.rect(8.0, 8.0, 20.0, 20.0, 2.0);
            pen.line(&[(4.0, 16.0), (4.0, 4.0), (16.0, 4.0)]);
        }
        Icon::External => {
            pen.line(&[(19.0, 13.0), (19.0, 19.0), (5.0, 19.0), (5.0, 5.0), (11.0, 5.0)]);
            pen.line(&[(14.0, 4.0), (20.0, 4.0), (20.0, 10.0)]);
            pen.line(&[(20.0, 4.0), (11.0, 13.0)]);
        }
        Icon::ArrowLeft => {
            pen.line(&[(19.0, 12.0), (5.0, 12.0)]);
            pen.line(&[(11.0, 6.0), (5.0, 12.0), (11.0, 18.0)]);
        }
        Icon::ArrowRight => {
            pen.line(&[(5.0, 12.0), (19.0, 12.0)]);
            pen.line(&[(13.0, 6.0), (19.0, 12.0), (13.0, 18.0)]);
        }
        Icon::ArrowUp => {
            pen.line(&[(12.0, 19.0), (12.0, 5.0)]);
            pen.line(&[(6.0, 11.0), (12.0, 5.0), (18.0, 11.0)]);
        }
        Icon::ArrowDown => {
            pen.line(&[(12.0, 5.0), (12.0, 19.0)]);
            pen.line(&[(6.0, 13.0), (12.0, 19.0), (18.0, 13.0)]);
        }
        Icon::Download => {
            pen.line(&[(12.0, 3.5), (12.0, 15.0)]);
            pen.line(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]);
            pen.line(&[(4.5, 20.0), (19.5, 20.0)]);
        }
        Icon::Close => {
            pen.line(&[(6.0, 6.0), (18.0, 18.0)]);
            pen.line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icon::ChevronDown => pen.line(&[(6.0, 9.0), (12.0, 15.0), (18.0, 9.0)]),
        Icon::ChevronRight => pen.line(&[(9.0, 6.0), (15.0, 12.0), (9.0, 18.0)]),
        Icon::Clock => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line(&[(12.0, 7.0), (12.0, 12.0), (15.5, 14.0)]);
        }
        Icon::Info => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line(&[(12.0, 11.0), (12.0, 16.5)]);
            pen.dot(12.0, 7.8, 1.2);
        }
        Icon::Terminal => {
            pen.rect(3.0, 4.0, 21.0, 20.0, 2.0);
            pen.line(&[(7.0, 9.0), (10.0, 12.0), (7.0, 15.0)]);
            pen.line(&[(12.5, 15.0), (17.0, 15.0)]);
        }
        Icon::Alert => {
            pen.circle(12.0, 12.0, 9.0);
            pen.line(&[(12.0, 7.0), (12.0, 13.0)]);
            pen.dot(12.0, 16.5, 1.2);
        }
        Icon::Gear => gear(painter, rect.shrink(rect.width() * 0.08), color),
        Icon::Update => update(painter, rect.shrink(rect.width() * 0.08), color),
    }
}

/// Allocates a square of `size` and draws `icon` in it.
pub fn show(ui: &mut egui::Ui, icon: Icon, size: f32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    paint(ui.painter(), icon, rect, color);
    response
}

/// An icon on a small rounded tile of `fill`, as in the section headers and status chips.
pub fn tile(ui: &mut egui::Ui, icon: Icon, size: f32, fill: Color32, color: Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same((size * 0.3) as u8), fill);
    paint(ui.painter(), icon, rect.shrink(size * 0.22), color);
    response
}

/// A check mark inside `rect`.
pub fn check(painter: &egui::Painter, rect: Rect, color: Color32) {
    paint(painter, Icon::Check, rect, color);
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

/// A selection box, `checked` of the way from empty (0) to ticked (1): while it changes, the
/// fill grows from the centre and the tick fades in.
pub fn checkbox(painter: &egui::Painter, rect: Rect, checked: f32, enabled: bool, p: Palette) {
    let radius = CornerRadius::same(5);
    let checked = checked.clamp(0.0, 1.0);
    if checked < 1.0 {
        let line = if enabled { p.muted } else { p.line };
        painter.rect_filled(rect, radius, p.panel);
        painter.rect_stroke(rect, radius, Stroke::new(1.5, line), StrokeKind::Inside);
    }
    if checked > 0.0 {
        let grown = Rect::from_center_size(rect.center(), rect.size() * (0.6 + 0.4 * checked));
        painter.rect_filled(grown, radius, p.check.gamma_multiply(checked));
        check(painter, rect.shrink(2.5), p.on_check.gamma_multiply(checked));
    }
}

/// Where a state sits on the track: with DRM, unlocked, EPUB.
fn stage_index(state: BookState) -> usize {
    match state {
        BookState::Locked => 0,
        BookState::Unlocked => 1,
        BookState::Converted => 2,
    }
}

/// The pastel of each state, for fills only: never for text.
pub fn state_fill(state: BookState) -> Color32 {
    match state {
        BookState::Locked => theme::DRM,
        BookState::Unlocked => theme::UNLOCKED,
        BookState::Converted => theme::EPUB,
    }
}

/// The icon of each state.
pub fn state_icon(state: BookState) -> Icon {
    match state {
        BookState::Locked => Icon::Lock,
        BookState::Unlocked => Icon::Unlock,
        BookState::Converted => Icon::File,
    }
}

pub const TRACK_WIDTH: f32 = 3.0 * 18.0 + 2.0 * 4.0;

/// The track of a book: with DRM, unlocked, EPUB, each stage done in its own pastel.
/// Returns where it ends.
pub fn track(painter: &egui::Painter, left_center: Pos2, state: BookState, enabled: bool, p: Palette) -> f32 {
    let done = stage_index(state) + 1;
    let (w, h, gap) = (18.0, 6.0, 4.0);
    let stages = [BookState::Locked, BookState::Unlocked, BookState::Converted];
    for (i, stage) in stages.into_iter().enumerate() {
        let x = left_center.x + i as f32 * (w + gap);
        let segment = Rect::from_min_size(Pos2::new(x, left_center.y - h / 2.0), egui::vec2(w, h));
        if i < done {
            let fill = if enabled { state_fill(stage) } else { p.line };
            painter.rect_filled(segment, CornerRadius::same(3), fill);
        } else {
            painter.rect_filled(segment, CornerRadius::same(3), p.quiet);
        }
    }
    left_center.x + TRACK_WIDTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_track_fills_one_more_stage_per_state() {
        assert_eq!(stage_index(BookState::Locked), 0);
        assert_eq!(stage_index(BookState::Unlocked), 1);
        assert_eq!(stage_index(BookState::Converted), 2);
    }

    #[test]
    fn each_state_has_its_own_colour_and_icon() {
        let states = [BookState::Locked, BookState::Unlocked, BookState::Converted];
        let colours: std::collections::HashSet<_> = states.map(state_fill).into();
        assert_eq!(colours.len(), 3);
        let icons: std::collections::HashSet<_> = states.map(|s| format!("{:?}", state_icon(s))).into();
        assert_eq!(icons.len(), 3);
    }
}

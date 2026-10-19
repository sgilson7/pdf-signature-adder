//! Drawing a signature on the pad. The pad is `PAD_W` by `PAD_H` units (3:1,
//! the shape of most signature lines); pointer positions arrive in CSS
//! pixels with the pad's on-screen size and become units here. Strokes are
//! smoothed into Bézier curves, written once as SVG path data for the screen
//! and once as PDF operators for the file, from the same points.

use crate::num::mul_div;

pub const PAD_W: i64 = 3000;
pub const PAD_H: i64 = 1000;

/// A point closer than this to the last kept one adds nothing but noise.
const MIN_STEP: i64 = 6;
/// Space kept around the strokes when the drawing is trimmed, in units.
const MARGIN: i64 = 12;

/// Line weights, in pad units. At the starting width of 180 points a unit is
/// 0.06 points, so these are about 1.1, 1.8 and 2.7 points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Fine,
    Medium,
    Bold,
}

impl Weight {
    pub fn units(self) -> i64 {
        match self {
            Weight::Fine => 18,
            Weight::Medium => 30,
            Weight::Bold => 45,
        }
    }

    pub fn all() -> [Weight; 3] {
        [Weight::Fine, Weight::Medium, Weight::Bold]
    }

    pub fn key(self) -> &'static str {
        match self {
            Weight::Fine => "fine",
            Weight::Medium => "medium",
            Weight::Bold => "bold",
        }
    }
}

/// Ink colours (PS-Q12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    Black,
    Blue,
}

impl Ink {
    /// For the screen.
    pub fn css(self) -> &'static str {
        match self {
            Ink::Black => "#13233f",
            Ink::Blue => "#1d3f9c",
        }
    }

    /// For the file: the same colours as PDF RGB operands.
    pub fn pdf(self) -> &'static str {
        match self {
            Ink::Black => "0.075 0.137 0.247",
            Ink::Blue => "0.114 0.247 0.612",
        }
    }

    pub fn all() -> [Ink; 2] {
        [Ink::Black, Ink::Blue]
    }

    pub fn key(self) -> &'static str {
        match self {
            Ink::Black => "black",
            Ink::Blue => "blue",
        }
    }
}

/// The pad: finished strokes, the stroke in progress, and which pointer is
/// drawing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pad {
    pub strokes: Vec<Vec<(i64, i64)>>,
    pub active: Option<(String, Vec<(i64, i64)>)>,
    /// Once a pen has touched the pad, touches are a resting hand.
    pub pen_seen: bool,
    pub weight: Weight,
    pub ink: Ink,
}

impl Default for Pad {
    fn default() -> Pad {
        Pad {
            strokes: Vec::new(),
            active: None,
            pen_seen: false,
            weight: Weight::Medium,
            ink: Ink::Black,
        }
    }
}

/// A pointer event on the pad, already parsed: pointer id and type, the
/// position inside the pad and the pad's on-screen size, in thousandths of
/// a CSS pixel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadPoint {
    pub id: String,
    pub kind: String,
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Pad {
    fn to_units(p: &PadPoint) -> (i64, i64) {
        let x = mul_div(p.x, PAD_W, p.w.max(1)).clamp(0, PAD_W);
        let y = mul_div(p.y, PAD_H, p.h.max(1)).clamp(0, PAD_H);
        (x, y)
    }

    /// A pointer went down. Returns whether a stroke began.
    pub fn down(&mut self, p: &PadPoint) -> bool {
        if p.kind == "pen" {
            self.pen_seen = true;
        }
        if (self.pen_seen && p.kind == "touch") || self.active.is_some() {
            return false;
        }
        self.active = Some((p.id.clone(), vec![Pad::to_units(p)]));
        true
    }

    /// The drawing pointer moved.
    pub fn moved(&mut self, p: &PadPoint) {
        let Some((id, points)) = &mut self.active else {
            return;
        };
        if *id != p.id {
            return;
        }
        let next = Pad::to_units(p);
        let last = points[points.len() - 1];
        let (dx, dy) = (next.0 - last.0, next.1 - last.1);
        if dx * dx + dy * dy >= MIN_STEP * MIN_STEP {
            points.push(next);
        }
    }

    /// The drawing pointer lifted or was cancelled; the stroke is kept.
    pub fn up(&mut self, p: &PadPoint) {
        if let Some((id, _)) = &self.active {
            if *id != p.id {
                return;
            }
        }
        if let Some((_, points)) = self.active.take() {
            self.strokes.push(points);
        }
    }

    pub fn undo(&mut self) {
        self.active = None;
        self.strokes.pop();
    }

    pub fn clear(&mut self) {
        self.active = None;
        self.strokes.clear();
    }

    fn all_strokes(&self) -> Vec<&Vec<(i64, i64)>> {
        let mut v: Vec<&Vec<(i64, i64)>> = self.strokes.iter().collect();
        if let Some((_, a)) = &self.active {
            v.push(a);
        }
        v
    }

    pub fn is_empty(&self) -> bool {
        self.all_strokes().is_empty()
    }

    /// SVG path data for every stroke, in pad units, for the pad on screen.
    pub fn svg_paths(&self) -> Vec<String> {
        self.all_strokes()
            .into_iter()
            .map(|s| path(s, Syntax::Svg, (0, 0), 0))
            .collect()
    }

    /// The finished drawing as art: its box (trimmed to the strokes plus a
    /// margin) and the strokes inside it.
    pub fn drawing(&self) -> Option<Drawing> {
        if self.strokes.is_empty() {
            return None;
        }
        let pad = self.weight.units() / 2 + MARGIN;
        let xs = self.strokes.iter().flatten().map(|p| p.0);
        let ys = self.strokes.iter().flatten().map(|p| p.1);
        let x0 = xs.clone().min()? - pad;
        let x1 = xs.max()? + pad;
        let y0 = ys.clone().min()? - pad;
        let y1 = ys.max()? + pad;
        Some(Drawing {
            strokes: self.strokes.clone(),
            origin: (x0, y0),
            width: x1 - x0,
            height: y1 - y0,
            weight: self.weight,
            ink: self.ink,
        })
    }
}

/// A finished drawn signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawing {
    pub strokes: Vec<Vec<(i64, i64)>>,
    /// The top-left of its box, in pad units.
    pub origin: (i64, i64),
    pub width: i64,
    pub height: i64,
    pub weight: Weight,
    pub ink: Ink,
}

impl Drawing {
    /// The SVG `viewBox` that shows exactly the box.
    pub fn view_box(&self) -> String {
        format!(
            "{} {} {} {}",
            self.origin.0, self.origin.1, self.width, self.height
        )
    }

    pub fn svg_paths(&self) -> Vec<String> {
        self.strokes
            .iter()
            .map(|s| path(s, Syntax::Svg, (0, 0), 0))
            .collect()
    }

    /// PDF operators drawing the strokes in a form whose box is
    /// [0 0 width height], with y turned to point up.
    pub fn pdf_content(&self) -> Vec<u8> {
        let mut out = format!("1 J 1 j {} w {} RG\n", self.weight.units(), self.ink.pdf());
        let flip = self.origin.1 + self.height;
        for s in &self.strokes {
            out.push_str(&path(s, Syntax::Pdf, (self.origin.0, flip), 1));
            out.push_str(" S\n");
        }
        out.into_bytes()
    }

    /// A standalone SVG file of the drawing, for "Save as image file".
    pub fn svg_file(&self, px_w: i64, px_h: i64) -> String {
        let paths: String = self
            .svg_paths()
            .iter()
            .map(|d| format!("<path d=\"{d}\"/>"))
            .collect();
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{px_w}\" height=\"{px_h}\" viewBox=\"{}\">\
             <g fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{paths}</g></svg>",
            self.view_box(),
            self.ink.css(),
            self.weight.units()
        )
    }
}

#[derive(Clone, Copy)]
enum Syntax {
    Svg,
    Pdf,
}

/// One stroke as a path. For PDF, `shift` is the form's left edge and the
/// pad y of its top, and `flip` 1 turns y to point up: (x - sx, sy - y).
fn path(points: &[(i64, i64)], syntax: Syntax, shift: (i64, i64), flip: i64) -> String {
    let at = |p: (i64, i64)| -> (i64, i64) {
        if flip == 1 {
            (p.0 - shift.0, shift.1 - p.1)
        } else {
            p
        }
    };
    let (m, l, c) = match syntax {
        Syntax::Svg => ("M{} {}", "L{} {}", "C{} {} {} {} {} {}"),
        Syntax::Pdf => ("{} {} m", "{} {} l", "{} {} {} {} {} {} c"),
    };
    let fill = |template: &str, vals: &[i64]| -> String {
        let mut s = template.to_string();
        for v in vals {
            s = s.replacen("{}", &v.to_string(), 1);
        }
        s
    };
    let mut parts = Vec::new();
    let p0 = at(points[0]);
    parts.push(fill(m, &[p0.0, p0.1]));
    if points.len() == 1 {
        // A dot: a short line with round caps shows as a round mark.
        parts.push(fill(l, &[p0.0 + 1, p0.1]));
        return parts.join(" ");
    }
    for i in 0..points.len() - 1 {
        let q0 = points[i.saturating_sub(1)];
        let q1 = points[i];
        let q2 = points[i + 1];
        let q3 = points[(i + 2).min(points.len() - 1)];
        // Catmull-Rom through q1 and q2, as a cubic Bézier.
        let c1 = (q1.0 + (q2.0 - q0.0) / 6, q1.1 + (q2.1 - q0.1) / 6);
        let c2 = (q2.0 - (q3.0 - q1.0) / 6, q2.1 - (q3.1 - q1.1) / 6);
        let (c1, c2, end) = (at(c1), at(c2), at(q2));
        parts.push(fill(c, &[c1.0, c1.1, c2.0, c2.1, end.0, end.1]));
    }
    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(id: &str, kind: &str, x: i64, y: i64) -> PadPoint {
        // A pad shown 300 by 100 CSS pixels: 10 units a pixel.
        PadPoint {
            id: id.into(),
            kind: kind.into(),
            x: x * 1000,
            y: y * 1000,
            w: 300_000,
            h: 100_000,
        }
    }

    #[test]
    fn pixels_become_pad_units() {
        let mut pad = Pad::default();
        pad.down(&pt("1", "mouse", 30, 40));
        pad.moved(&pt("1", "mouse", 31, 40));
        pad.up(&pt("1", "mouse", 31, 40));
        assert_eq!(pad.strokes, vec![vec![(300, 400), (310, 400)]]);
    }

    #[test]
    fn a_step_under_the_minimum_is_dropped() {
        let mut pad = Pad::default();
        pad.down(&pt("1", "pen", 10, 10));
        // 0.5 px = 5 units: under 6, dropped.
        pad.moved(&PadPoint {
            x: 10_500,
            ..pt("1", "pen", 10, 10)
        });
        pad.up(&pt("1", "pen", 10, 10));
        assert_eq!(pad.strokes, vec![vec![(100, 100)]]);
    }

    #[test]
    fn once_a_pen_is_seen_touches_draw_nothing() {
        let mut pad = Pad::default();
        assert!(pad.down(&pt("1", "touch", 5, 5)));
        pad.up(&pt("1", "touch", 5, 5));
        assert!(pad.down(&pt("2", "pen", 5, 5)));
        pad.up(&pt("2", "pen", 5, 5));
        assert!(!pad.down(&pt("3", "touch", 50, 50)));
        assert_eq!(pad.strokes.len(), 2);
    }

    #[test]
    fn a_second_pointer_does_not_join_the_stroke() {
        let mut pad = Pad::default();
        pad.down(&pt("1", "mouse", 0, 0));
        assert!(!pad.down(&pt("2", "touch", 50, 50)));
        pad.moved(&pt("2", "touch", 60, 60));
        pad.up(&pt("2", "touch", 60, 60));
        assert!(pad.active.is_some());
        pad.up(&pt("1", "mouse", 0, 0));
        assert_eq!(pad.strokes, vec![vec![(0, 0)]]);
    }

    #[test]
    fn undo_and_clear() {
        let mut pad = Pad::default();
        for k in 0..3 {
            pad.down(&pt("1", "mouse", k * 10, 0));
            pad.up(&pt("1", "mouse", k * 10, 0));
        }
        pad.undo();
        assert_eq!(pad.strokes.len(), 2);
        pad.clear();
        assert!(pad.is_empty());
    }

    #[test]
    fn three_points_become_two_curves_through_them() {
        // Points (0,0), (60,0), (120,60). Segment 1: q0 = q1 = (0,0),
        // c1 = (0 + 60/6, 0) = (10, 0); c2 = (60 - 120/6, 0 - 60/6) = (40, -10).
        // Segment 2: c1 = (60 + 120/6, 0 + 60/6) = (80, 10);
        // c2 = (120 - (120-60)/6, 60 - (60-0)/6) = (110, 50).
        let s = vec![(0, 0), (60, 0), (120, 60)];
        assert_eq!(
            path(&s, Syntax::Svg, (0, 0), 0),
            "M0 0 C10 0 40 -10 60 0 C80 10 110 50 120 60"
        );
    }

    #[test]
    fn a_dot_is_a_short_line() {
        assert_eq!(path(&[(5, 7)], Syntax::Svg, (0, 0), 0), "M5 7 L6 7");
    }

    #[test]
    fn the_file_gets_the_same_curve_turned_upright() {
        let mut pad = Pad {
            weight: Weight::Fine,
            ..Pad::default()
        };
        pad.down(&pt("1", "mouse", 10, 20));
        pad.moved(&pt("1", "mouse", 16, 20));
        pad.up(&pt("1", "mouse", 16, 20));
        let d = pad.drawing().unwrap();
        // Margin 9 + 12 = 21 around x 100..160, y 200: box from (79, 179),
        // 102 wide, 42 high.
        assert_eq!((d.origin, d.width, d.height), ((79, 179), 102, 42));
        // In the form: x - 79, 221 - y. (100,200) -> (21, 21); (160,200) ->
        // (81, 21); controls (110,200) -> (31, 21) and (150,200) -> (71, 21).
        assert_eq!(
            String::from_utf8(d.pdf_content()).unwrap(),
            "1 J 1 j 18 w 0.075 0.137 0.247 RG\n21 21 m 31 21 71 21 81 21 c S\n"
        );
        assert_eq!(d.view_box(), "79 179 102 42");
    }

    #[test]
    fn nothing_drawn_is_no_drawing() {
        assert_eq!(Pad::default().drawing(), None);
    }
}

//! Where a signature sits (PS-2). A placement lives in the page's *display
//! space*: the visible box (crop box clipped to media box) turned by the
//! page's rotation, measured from its top-left corner, in thousandths of a
//! point. The page shows that space scaled to the screen; the file needs the
//! same box in the page's own coordinates, which `cm` works out for each of
//! the four rotations.

use crate::num::{format_milli, format_scaled, mul_div};

/// A page's geometry, from the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// [x0, y0, x1, y1] in thousandths of a point.
    pub view: [i64; 4],
    /// 0, 90, 180 or 270.
    pub rotate: i64,
}

impl Geometry {
    /// Width and height as shown, after rotation.
    pub fn display(&self) -> (i64, i64) {
        let w = self.view[2] - self.view[0];
        let h = self.view[3] - self.view[1];
        if self.rotate == 90 || self.rotate == 270 {
            (h, w)
        } else {
            (w, h)
        }
    }
}

/// A box in display space: left, top, width, height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Rect {
    pub fn contains(&self, px: i64, py: i64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    /// The four corners, in the order top-left, top-right, bottom-left,
    /// bottom-right.
    pub fn corners(&self) -> [(i64, i64); 4] {
        [
            (self.x, self.y),
            (self.x + self.w, self.y),
            (self.x, self.y + self.h),
            (self.x + self.w, self.y + self.h),
        ]
    }
}

/// The `cm` operands that draw art `art_w` by `art_h` units into `r` on a
/// page with geometry `g`. The art's own y axis points up, as PDF's does.
/// Scales are written to six decimals, offsets to three.
pub fn cm(g: &Geometry, r: &Rect, art_w: i64, art_h: i64) -> String {
    let [x0, y0, x1, y1] = g.view;
    // Scales in millionths of a point per art unit: r.w is in thousandths.
    let sx = mul_div(r.w, 1000, art_w.max(1));
    let sy = mul_div(r.h, 1000, art_h.max(1));
    let (a, b, c, d, e, f) = match g.rotate {
        90 => (0, sx, -sy, 0, x0 + r.y + r.h, y0 + r.x),
        180 => (-sx, 0, 0, -sy, x1 - r.x, y0 + r.y + r.h),
        270 => (0, -sx, sy, 0, x1 - r.y - r.h, y1 - r.x),
        _ => (sx, 0, 0, sy, x0 + r.x, y1 - r.y - r.h),
    };
    let s = |v: i64| format_scaled(v, 1_000_000, 6);
    format!(
        "{} {} {} {} {} {}",
        s(a),
        s(b),
        s(c),
        s(d),
        format_milli(e),
        format_milli(f)
    )
}

/// Keeps `r` on a page of display size `(pw, ph)`: a box larger than the page
/// shrinks with its shape kept, then the box moves inside.
pub fn clamp(r: Rect, pw: i64, ph: i64) -> Rect {
    let mut out = r;
    if out.w > pw {
        out.h = mul_div(out.h, pw, out.w.max(1));
        out.w = pw;
    }
    if out.h > ph {
        out.w = mul_div(out.w, ph, out.h.max(1));
        out.h = ph;
    }
    out.x = out.x.clamp(0, pw - out.w);
    out.y = out.y.clamp(0, ph - out.h);
    out
}

/// A new placement: `width` wide with the art's shape, centred left to right
/// on the tap with its bottom edge on the tap, so a tap on a signature line
/// puts the signature on the line.
pub fn at_tap(px: i64, py: i64, width: i64, art_w: i64, art_h: i64, pw: i64, ph: i64) -> Rect {
    let h = mul_div(width, art_h, art_w.max(1));
    clamp(
        Rect {
            x: px - width / 2,
            y: py - h,
            w: width,
            h,
        },
        pw,
        ph,
    )
}

/// Resizes `r` by dragging one corner to `(px, py)` while the opposite corner
/// `(ax, ay)` stays put. The shape is kept: the width follows whichever of
/// the two distances asks for the larger box, and is held between `min_w`
/// and the page.
#[allow(clippy::too_many_arguments)]
pub fn resize(
    ax: i64,
    ay: i64,
    px: i64,
    py: i64,
    art_w: i64,
    art_h: i64,
    min_w: i64,
    pw: i64,
    ph: i64,
) -> Rect {
    let dx = (px - ax).abs();
    let dy = (py - ay).abs();
    let from_dy = mul_div(dy, art_w, art_h.max(1));
    let mut w = dx.max(from_dy).max(min_w);
    // The room on the side the corner is being dragged towards.
    let room_x = if px >= ax { pw - ax } else { ax };
    let room_y = if py >= ay { ph - ay } else { ay };
    w = w.min(room_x).min(mul_div(room_y, art_w, art_h.max(1))).max(1);
    let h = mul_div(w, art_h, art_w.max(1));
    let x = if px >= ax { ax } else { ax - w };
    let y = if py >= ay { ay } else { ay - h };
    clamp(Rect { x, y, w, h }, pw, ph)
}

/// A CSS percentage of `whole`, to three decimals: "12.345%".
pub fn percent(part: i64, whole: i64) -> String {
    format!(
        "{}%",
        format_scaled(mul_div(part, 100_000, whole.max(1)), 1000, 3)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LETTER: [i64; 4] = [0, 0, 612_000, 792_000];

    fn r(x: i64, y: i64, w: i64, h: i64) -> Rect {
        Rect {
            x: x * 1000,
            y: y * 1000,
            w: w * 1000,
            h: h * 1000,
        }
    }

    // Each case: an image (art 1 x 1) placed at left 100, top 50, 180 wide,
    // 60 high, in display space. Worked by hand from the corner the image's
    // own origin (its bottom-left) must land on.

    #[test]
    fn upright_page_bottom_left_lands_at_left_and_bottom() {
        let g = Geometry {
            view: LETTER,
            rotate: 0,
        };
        // Bottom of the box is 110 from the top, so 792 - 110 = 682 up.
        assert_eq!(cm(&g, &r(100, 50, 180, 60), 1, 1), "180 0 0 60 100 682");
    }

    #[test]
    fn page_turned_90_maps_display_left_to_user_bottom() {
        let g = Geometry {
            view: LETTER,
            rotate: 90,
        };
        assert_eq!(g.display(), (792_000, 612_000));
        // Display (100, 110) is user (110, 100): x from the top distance,
        // y from the left distance.
        assert_eq!(cm(&g, &r(100, 50, 180, 60), 1, 1), "0 180 -60 0 110 100");
    }

    #[test]
    fn page_turned_180_counts_from_the_other_corner() {
        let g = Geometry {
            view: LETTER,
            rotate: 180,
        };
        assert_eq!(cm(&g, &r(100, 50, 180, 60), 1, 1), "-180 0 0 -60 512 110");
    }

    #[test]
    fn page_turned_270() {
        let g = Geometry {
            view: LETTER,
            rotate: 270,
        };
        // e = 612 - 50 - 60 = 502; f = 792 - 100 = 692.
        assert_eq!(cm(&g, &r(100, 50, 180, 60), 1, 1), "0 -180 60 0 502 692");
    }

    #[test]
    fn an_offset_crop_box_shifts_the_origin() {
        let g = Geometry {
            view: [36_000, 36_500, 576_000, 756_000],
            rotate: 0,
        };
        // x = 36 + 100; y = 756 - 50 - 60 = 646.
        assert_eq!(cm(&g, &r(100, 50, 180, 60), 1, 1), "180 0 0 60 136 646");
    }

    #[test]
    fn form_art_scales_by_its_own_units() {
        let g = Geometry {
            view: LETTER,
            rotate: 0,
        };
        // Art 3000 x 1000 units into 180 x 60 points: 0.06 per unit.
        assert_eq!(cm(&g, &r(100, 50, 180, 60), 3000, 1000), "0.06 0 0 0.06 100 682");
    }

    #[test]
    fn a_tap_puts_the_bottom_edge_on_the_tap() {
        // Art 3:1 at 180 wide is 60 high; tap at (300, 400).
        assert_eq!(
            at_tap(300_000, 400_000, 180_000, 3, 1, 612_000, 792_000),
            r(210, 340, 180, 60)
        );
    }

    #[test]
    fn a_tap_near_the_edge_keeps_the_box_on_the_page() {
        assert_eq!(
            at_tap(10_000, 20_000, 180_000, 3, 1, 612_000, 792_000),
            r(0, 0, 180, 60)
        );
    }

    #[test]
    fn a_box_wider_than_the_page_shrinks_with_its_shape() {
        assert_eq!(clamp(r(0, 0, 1224, 408), 612_000, 792_000), r(0, 0, 612, 204));
    }

    #[test]
    fn dragging_the_bottom_right_corner_keeps_the_top_left_and_the_shape() {
        // Anchor (100, 50); drag to (400, 90): dx = 300, dy = 40 -> 120 by
        // shape; width 300, height 100.
        let out = resize(100_000, 50_000, 400_000, 90_000, 3, 1, 36_000, 612_000, 792_000);
        assert_eq!(out, r(100, 50, 300, 100));
    }

    #[test]
    fn dragging_the_top_left_corner_grows_up_and_left() {
        // Anchor (280, 110); drag to (100, 100): dx = 180 -> 180 by 60.
        let out = resize(280_000, 110_000, 100_000, 100_000, 3, 1, 36_000, 612_000, 792_000);
        assert_eq!(out, r(100, 50, 180, 60));
    }

    #[test]
    fn resizing_stops_at_the_minimum_and_at_the_page_edge() {
        let small = resize(100_000, 50_000, 101_000, 51_000, 3, 1, 36_000, 612_000, 792_000);
        assert_eq!(small, r(100, 50, 36, 12));
        let big = resize(500_000, 50_000, 900_000, 60_000, 3, 1, 36_000, 612_000, 792_000);
        assert_eq!(big, r(500, 50, 112, 37).with_h(37_333));
    }

    impl Rect {
        fn with_h(mut self, h: i64) -> Rect {
            self.h = h;
            self
        }
    }

    #[test]
    fn percentages_have_three_decimals() {
        assert_eq!(percent(210_000, 612_000), "34.314%");
        assert_eq!(percent(0, 612_000), "0%");
        assert_eq!(percent(612_000, 612_000), "100%");
    }
}

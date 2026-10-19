//! A signature from a picture: a photo or scan of ink on paper, a PNG with
//! transparency, or an SVG the page has drawn to pixels (PS-Q10). The browser
//! decodes the file; the core decides which pixels are ink.

/// The largest picture kept, in pixels. At the starting width of 2.5 inches
/// this is 720 pixels an inch, past the 600 the design asked for.
pub const MAX_W: i64 = 1800;
pub const MAX_H: i64 = 900;
/// Pixels lighter than this are paper; darker than `INK` are ink; between,
/// they fade. Luma on 0..255.
const PAPER: i64 = 200;
const INK: i64 = 140;
/// Alpha at or under this counts as empty when trimming.
const EMPTY: u8 = 16;

/// The size the page should draw a picture at before handing over its
/// pixels: within MAX_W by MAX_H with its shape kept. A raster picture is
/// never enlarged; an SVG is drawn at the full size, which is four times its
/// placed size and more.
pub fn target_size(natural_w: i64, natural_h: i64, is_vector: bool) -> Option<(i64, i64)> {
    if natural_w <= 0 || natural_h <= 0 {
        return None;
    }
    let fit_w = MAX_W;
    let fit_h = crate::num::mul_div(natural_h, MAX_W, natural_w);
    let (w, h) = if fit_h <= MAX_H {
        (fit_w, fit_h)
    } else {
        (crate::num::mul_div(natural_w, MAX_H, natural_h), MAX_H)
    };
    if !is_vector && natural_w <= w {
        return Some((natural_w, natural_h));
    }
    Some((w.max(1), h.max(1)))
}

/// A picture ready to place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    /// RGBA, for showing it on screen.
    pub rgba: Vec<u8>,
}

impl Picture {
    pub fn rgb(&self) -> Vec<u8> {
        self.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect()
    }

    pub fn alpha(&self) -> Vec<u8> {
        self.rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PictureError {
    WrongSize,
    TooLarge,
    Blank,
}

/// Turns decoded pixels into a signature: light pixels made clear when
/// `clear_paper`, empty margins cut when `trim`.
pub fn prepare(
    width: u32,
    height: u32,
    rgba: &[u8],
    clear_paper: bool,
    trim: bool,
) -> Result<Picture, PictureError> {
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || rgba.len() != w * h * 4 {
        return Err(PictureError::WrongSize);
    }
    if i64::from(width) > MAX_W * 2 || i64::from(height) > MAX_H * 2 {
        return Err(PictureError::TooLarge);
    }
    let mut px = rgba.to_vec();
    if clear_paper {
        for p in px.as_chunks_mut::<4>().0 {
            let luma = (299 * i64::from(p[0]) + 587 * i64::from(p[1]) + 114 * i64::from(p[2])) / 1000;
            let keep = if luma >= PAPER {
                0
            } else if luma <= INK {
                255
            } else {
                (PAPER - luma) * 255 / (PAPER - INK)
            };
            p[3] = (i64::from(p[3]) * keep / 255) as u8;
        }
    }
    let (mut x0, mut y0, mut x1, mut y1) = (0, 0, w, h);
    if trim {
        let solid = |x: usize, y: usize| px[(y * w + x) * 4 + 3] > EMPTY;
        let rows: Vec<usize> = (0..h).filter(|&y| (0..w).any(|x| solid(x, y))).collect();
        let cols: Vec<usize> = (0..w).filter(|&x| (0..h).any(|y| solid(x, y))).collect();
        let (Some(&ry0), Some(&ry1), Some(&cx0), Some(&cx1)) =
            (rows.first(), rows.last(), cols.first(), cols.last())
        else {
            return Err(PictureError::Blank);
        };
        // Keep a margin of 2% of the longer side.
        let m = (w.max(h) / 50).max(1);
        x0 = cx0.saturating_sub(m);
        y0 = ry0.saturating_sub(m);
        x1 = (cx1 + 1 + m).min(w);
        y1 = (ry1 + 1 + m).min(h);
    } else if px.as_chunks::<4>().0.iter().all(|p| p[3] <= EMPTY) {
        return Err(PictureError::Blank);
    }
    let mut out = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
    for y in y0..y1 {
        out.extend_from_slice(&px[(y * w + x0) * 4..(y * w + x1) * 4]);
    }
    Ok(Picture {
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
        rgba: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAPER_PX: [u8; 4] = [247, 246, 238, 255];
    const INK_PX: [u8; 4] = [40, 45, 58, 255];

    fn image(rows: &[&str]) -> (u32, u32, Vec<u8>) {
        let mut v = Vec::new();
        for r in rows {
            for c in r.chars() {
                v.extend_from_slice(if c == '#' { &INK_PX } else { &PAPER_PX });
            }
        }
        (rows[0].len() as u32, rows.len() as u32, v)
    }

    #[test]
    fn paper_becomes_clear_and_ink_stays() {
        let (w, h, px) = image(&[".#"]);
        let p = prepare(w, h, &px, true, false).unwrap();
        // Paper luma (299*247 + 587*246 + 114*238)/1000 = 245 -> clear.
        // Ink luma (299*40 + 587*45 + 114*58)/1000 = 44 -> kept.
        assert_eq!(p.alpha(), vec![0, 255]);
        assert_eq!(p.rgb(), vec![247, 246, 238, 40, 45, 58]);
    }

    #[test]
    fn a_mid_grey_fades_in_proportion() {
        // Luma 170 is halfway between 140 and 200: alpha 255 * 30 / 60 = 127.
        let p = prepare(1, 1, &[170, 170, 170, 255], true, false).unwrap();
        assert_eq!(p.alpha(), vec![127]);
    }

    #[test]
    fn margins_are_trimmed_to_the_ink_plus_two_percent() {
        // 4 x 3, ink at (2, 1) only. Margin max(4,3)/50 = 0, so 1 pixel.
        let (w, h, px) = image(&["....", "..#.", "...."]);
        let p = prepare(w, h, &px, true, true).unwrap();
        // x from 1 to 4 (3 wide), y from 0 to 3 (3 high).
        assert_eq!((p.width, p.height), (3, 3));
        assert_eq!(p.alpha(), vec![0, 0, 0, 0, 255, 0, 0, 0, 0]);
    }

    #[test]
    fn a_blank_picture_is_refused() {
        let (w, h, px) = image(&["...", "..."]);
        assert_eq!(prepare(w, h, &px, true, true), Err(PictureError::Blank));
        assert_eq!(prepare(w, h, &px, true, false), Err(PictureError::Blank));
    }

    #[test]
    fn a_wrong_byte_count_is_refused() {
        assert_eq!(
            prepare(2, 2, &[0; 15], false, false),
            Err(PictureError::WrongSize)
        );
    }

    #[test]
    fn target_size_fits_and_never_enlarges_a_raster() {
        assert_eq!(target_size(1240, 520, false), Some((1240, 520)));
        assert_eq!(target_size(3600, 900, false), Some((1800, 450)));
        assert_eq!(target_size(1000, 1000, false), Some((900, 900)));
        // An SVG of 300 x 100 is drawn at the full 1800 x 600.
        assert_eq!(target_size(300, 100, true), Some((1800, 600)));
        assert_eq!(target_size(0, 100, true), None);
    }
}

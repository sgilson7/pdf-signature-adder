//! The door into the core. The shim holds one `Handle` and forwards each call
//! to it; the PDF and the signatures stay here, so nothing large crosses the
//! boundary on every pointer move (BUILDER-SPEC.md change table, R4).
//!
//! `ui()` describes the whole screen as a view model. Settings, screens,
//! pointer events and keys all come back through one function, `act(name,
//! value)`, with a dotted name such as `zoom.in`. Calls that carry bytes (a
//! PDF, a picture's pixels, the signed copy) have their own methods. Every
//! method returns `{"ok": ...}` or `{"error": "..."}` as JSON text, and an
//! error carries the message the page shows as it is. (After Peter's
//! Mandible, and Mp4 to Transcript's handle.)

use serde::Serialize;

use crate::glossary;
use crate::ink::{Drawing, Ink, Pad, PadPoint, Weight, PAD_H, PAD_W};
use crate::num::{format_scaled, mul_div, parse_milli, MILLI};
use crate::pdf::doc::{Document, Refusal};
use crate::pdf::write::{self, Art, Stamp, WriteError};
use crate::picture::{self, Picture, PictureError};
use crate::place::{self, Geometry, Rect};
use crate::strings::Strings;

/// A new placement is this wide, in thousandths of a point (2.5 inches).
const START_W: i64 = 180_000;
/// No placement is narrower than half an inch.
const MIN_W: i64 = 36_000;
/// Corner handles answer within this many CSS pixels (thousandths), the
/// smallest touch target Apple recommends, halved for a radius.
const HANDLE_RADIUS: i64 = 22_000;
/// Space either side of the pages, in CSS pixels.
const GUTTER: i64 = 16;
/// Zoom steps, as percentages of "fit width".
const ZOOMS: [i64; 7] = [50, 75, 100, 125, 150, 200, 300];
const FIT: usize = 2;
/// No page canvas holds more pixels than this; iPad Safari refuses a canvas
/// over about 16.7 million.
const MAX_CANVAS_PX: i64 = 12_000_000;
/// Arrow keys move a placement by one point, or ten with Shift.
const NUDGE: i64 = 1000;
const NUDGE_BIG: i64 = 10_000;

fn ok<T: Serialize>(value: &T) -> String {
    serde_json::json!({ "ok": value }).to_string()
}

fn error(message: String) -> String {
    serde_json::json!({ "error": message }).to_string()
}

/// One button: the action and value the page sends back through `act`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Action {
    pub name: String,
    pub value: String,
    pub label: String,
    pub enabled: bool,
    /// For a choice or a toggle: whether this one is on.
    pub pressed: bool,
    pub primary: bool,
}

fn action(name: &str, value: &str, label: String) -> Action {
    Action {
        name: name.into(),
        value: value.into(),
        label,
        enabled: true,
        pressed: false,
        primary: false,
    }
}

impl Action {
    fn enabled(mut self, on: bool) -> Action {
        self.enabled = on;
        self
    }
    fn pressed(mut self, on: bool) -> Action {
        self.pressed = on;
        self
    }
    fn primary(mut self) -> Action {
        self.primary = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Hint {
    pub term: String,
    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileView {
    pub name: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Nav {
    pub label: String,
    pub prev: Action,
    pub next: Action,
}

/// A signature on a page, as CSS percentages of the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Overlay {
    pub id: u32,
    pub art: usize,
    pub left: String,
    pub top: String,
    pub width: String,
    pub height: String,
    pub selected: bool,
}

/// One page: its label, its size on screen and on its canvas, the scale
/// pdf.js paints at, and the signatures on it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageView {
    pub index: usize,
    pub label: String,
    pub css_width: String,
    pub css_height: String,
    pub canvas_width: i64,
    pub canvas_height: i64,
    pub scale: serde_json::Value,
    pub overlays: Vec<Overlay>,
}

/// How to draw a signature on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArtView {
    pub id: usize,
    /// "drawing" (SVG paths) or "picture" (pixels from `art_rgba`).
    pub kind: String,
    pub view_box: String,
    pub paths: Vec<String>,
    pub stroke: String,
    pub stroke_width: i64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Selection {
    pub page: usize,
    pub left: String,
    pub top: String,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PadView {
    pub label: String,
    pub view_box: String,
    pub baseline: i64,
    pub paths: Vec<String>,
    pub stroke: String,
    pub stroke_width: i64,
    pub hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PictureView {
    pub width: u32,
    pub height: u32,
    /// Changes whenever the pixels do, so the page fetches them once.
    pub version: u32,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SheetView {
    pub title: String,
    pub tab: String,
    pub tabs: Vec<Action>,
    pub close: Action,
    pub hint: String,
    pub pad: Option<PadView>,
    pub ink_label: String,
    pub inks: Vec<Action>,
    pub pen_label: String,
    pub weights: Vec<Action>,
    pub edits: Vec<Action>,
    /// The button that asks the page for a picture: paste or choose a file.
    pub fetch: Option<Action>,
    pub picture: Option<PictureView>,
    pub empty: String,
    /// The file name "Save as image file" offers.
    pub image_file: String,
    pub options: Vec<Action>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedView {
    pub title: String,
    pub file_name: String,
    pub detail: String,
    pub note: String,
    pub actions: Vec<Action>,
}

/// Everything the page draws.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ui {
    pub screen: String,
    pub title: String,
    pub tagline: String,
    pub start: Vec<String>,
    pub drop_hint: String,
    pub message: String,
    pub note: String,
    pub status: String,
    /// Buttons beside the status line: Cancel while placing.
    pub status_actions: Vec<Action>,
    pub file: Option<FileView>,
    pub toolbar: Vec<Action>,
    pub zoom: Vec<Action>,
    pub nav: Option<Nav>,
    /// Changes when a new PDF is opened, so the page loads it into pdf.js.
    pub document: u32,
    /// Changes when the page should scroll `scroll_page` into view.
    pub scroll: u32,
    pub scroll_page: usize,
    pub placing: bool,
    pub pages: Vec<PageView>,
    pub arts: Vec<ArtView>,
    pub selection: Option<Selection>,
    pub sheet: Option<SheetView>,
    pub saved: Option<SavedView>,
    pub hints: Vec<Hint>,
}

/// A signature made in the sheet, ready to place.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtEntry {
    art: Art,
    /// Width and height for the box's shape.
    shape: (i64, i64),
    /// The art's own units, for `cm`: the form box, or 1 x 1 for a picture.
    units: (i64, i64),
    drawing: Option<Drawing>,
    picture: Option<Picture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Placement {
    id: u32,
    page: usize,
    art: usize,
    rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Drag {
    Move { id: u32, grab_x: i64, grab_y: i64 },
    Resize { id: u32, anchor_x: i64, anchor_y: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Draw,
    Paste,
    Open,
}

impl Tab {
    fn key(self) -> &'static str {
        match self {
            Tab::Draw => "draw",
            Tab::Paste => "paste",
            Tab::Open => "open",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Raw {
    name: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Sheet {
    tab: Tab,
}

struct Saved {
    bytes: Vec<u8>,
    name: String,
}

/// The tool's whole state. The shim holds one.
pub struct Handle {
    strings: Option<Strings>,
    hints: Vec<Hint>,
    start_error: Option<String>,
    doc: Option<Document>,
    file_name: String,
    document: u32,
    arts: Vec<ArtEntry>,
    placements: Vec<Placement>,
    next_id: u32,
    selected: Option<u32>,
    armed: Option<usize>,
    drag: Option<Drag>,
    sheet: Option<Sheet>,
    pad: Pad,
    raw: Option<Raw>,
    picture: Option<Picture>,
    picture_version: u32,
    clear_paper: bool,
    trim: bool,
    saved: Option<Saved>,
    message: String,
    view_width: i64,
    dpr: i64,
    zoom: usize,
    current: usize,
    scroll: u32,
    can_share: bool,
    can_read_clipboard: bool,
}

/// A pointer event as the page sends it: "id|type|x|y|left|top|width|height"
/// in CSS pixels, as text.
struct Pointer {
    id: String,
    kind: String,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

fn parse_pointer(value: &str) -> Result<Pointer, String> {
    let parts: Vec<&str> = value.split('|').collect();
    let [id, kind, x, y, left, top, w, h] = parts.as_slice() else {
        return Err(format!("A pointer event needs eight fields, not \"{value}\"."));
    };
    let n = |t: &str| css_number(t).ok_or(format!("\"{t}\" is not a number."));
    let (x, y, left, top, w, h) = (n(x)?, n(y)?, n(left)?, n(top)?, n(w)?, n(h)?);
    if w <= 0 || h <= 0 {
        return Err("A pointer event came from an element with no size.".into());
    }
    Ok(Pointer {
        id: id.to_string(),
        kind: kind.to_string(),
        x: x - left,
        y: y - top,
        w,
        h,
    })
}

/// A number as JavaScript writes it. Values under a millionth come out in
/// exponent form ("4.2e-7"); they are zero at the thousandths kept here.
fn css_number(text: &str) -> Option<i64> {
    match text.split_once("e-") {
        Some((mantissa, _)) if parse_milli(mantissa).is_some() => Some(0),
        _ => parse_milli(text),
    }
}

impl Handle {
    /// Checks the strings file and the glossary. A problem with either is
    /// kept and reported by every call, since the page cannot work without
    /// its words.
    pub fn new(strings_json: &str, glossary_md: &str) -> Handle {
        let mut start_error = None;
        let strings = match Strings::parse(strings_json) {
            Ok(s) => Some(s),
            Err(e) => {
                start_error = Some(e);
                None
            }
        };
        let hints = match glossary::parse(glossary_md) {
            Ok(entries) => entries
                .into_iter()
                .map(|e| Hint {
                    term: e.term,
                    definition: e.definition,
                })
                .collect(),
            Err(e) => {
                start_error.get_or_insert(e.message());
                Vec::new()
            }
        };
        Handle {
            strings,
            hints,
            start_error,
            doc: None,
            file_name: String::new(),
            document: 0,
            arts: Vec::new(),
            placements: Vec::new(),
            next_id: 1,
            selected: None,
            armed: None,
            drag: None,
            sheet: None,
            pad: Pad::default(),
            raw: None,
            picture: None,
            picture_version: 0,
            clear_paper: true,
            trim: true,
            saved: None,
            message: String::new(),
            view_width: 1000,
            dpr: MILLI,
            zoom: FIT,
            current: 0,
            scroll: 0,
            can_share: false,
            can_read_clipboard: false,
        }
    }

    fn s(&self) -> &Strings {
        self.strings.as_ref().expect("checked by respond")
    }

    fn respond(&self, outcome: Result<(), String>) -> String {
        if let Some(e) = &self.start_error {
            return error(e.clone());
        }
        match outcome {
            Ok(()) => ok(&self.view()),
            Err(e) => error(e),
        }
    }

    pub fn ui(&self) -> String {
        self.respond(Ok(()))
    }

    /// Opens a PDF. A refusal is shown as the screen's message, with the
    /// reason, and the previous document (if any) stays open.
    pub fn open_pdf(&mut self, name: &str, bytes: &[u8]) -> String {
        if self.start_error.is_some() {
            return self.respond(Ok(()));
        }
        match Document::open(bytes.to_vec()) {
            Ok(doc) => {
                self.doc = Some(doc);
                self.file_name = name.to_string();
                self.document += 1;
                self.placements.clear();
                self.selected = None;
                self.drag = None;
                self.saved = None;
                self.current = 0;
                self.zoom = FIT;
                self.message.clear();
            }
            Err(r) => self.message = self.refusal(&r),
        }
        self.respond(Ok(()))
    }

    fn refusal(&self, r: &Refusal) -> String {
        let (key, detail) = r.key();
        self.s().fill(&format!("refuse_{key}"), &[("detail", detail)])
    }

    /// The size the page should draw a picture at: `{"ok": [w, h]}`.
    pub fn picture_size(&self, natural_w: &str, natural_h: &str, is_vector: bool) -> String {
        let n = |t: &str| parse_milli(t).map(|v| v / MILLI).unwrap_or(0);
        match picture::target_size(n(natural_w), n(natural_h), is_vector) {
            Some((w, h)) => ok(&[w, h]),
            None => error(self.s().get("picture_unreadable")),
        }
    }

    /// A picture's pixels, decoded by the page, for the sheet.
    pub fn picture(&mut self, name: &str, width: u32, height: u32, rgba: &[u8]) -> String {
        if self.start_error.is_some() {
            return self.respond(Ok(()));
        }
        // A picture pasted or dropped while no sheet is open opens it.
        if self.doc.is_some() && self.sheet.is_none() {
            self.sheet = Some(Sheet { tab: Tab::Paste });
        }
        self.raw = Some(Raw {
            name: name.to_string(),
            width,
            height,
            rgba: rgba.to_vec(),
        });
        self.prepare_picture();
        self.respond(Ok(()))
    }

    fn prepare_picture(&mut self) {
        let Some(raw) = &self.raw else { return };
        match picture::prepare(raw.width, raw.height, &raw.rgba, self.clear_paper, self.trim) {
            Ok(p) => {
                self.picture = Some(p);
                self.message.clear();
            }
            Err(e) => {
                self.picture = None;
                let key = match e {
                    PictureError::Blank => "picture_blank",
                    PictureError::TooLarge => "picture_too_large",
                    PictureError::WrongSize => "picture_unreadable",
                };
                self.message = self.s().get(key);
            }
        }
        self.picture_version += 1;
    }

    /// The sheet's picture as RGBA, for the page to show.
    pub fn picture_rgba(&self) -> Vec<u8> {
        self.picture.as_ref().map(|p| p.rgba.clone()).unwrap_or_default()
    }

    /// A placed picture's RGBA.
    pub fn art_rgba(&self, id: usize) -> Vec<u8> {
        self.arts
            .get(id)
            .and_then(|a| a.picture.as_ref())
            .map(|p| p.rgba.clone())
            .unwrap_or_default()
    }

    /// The drawing on the pad as an SVG file and its pixel size, for "Save
    /// as image file": `{"ok": {"svg": "...", "width": w, "height": h}}`.
    pub fn drawing_file(&self) -> String {
        match self.pad.drawing() {
            Some(d) => {
                // 1200 pixels wide, which is 480 pixels an inch at the
                // starting width.
                let w = 1200;
                let h = mul_div(w, d.height, d.width.max(1)).max(1);
                ok(&serde_json::json!({ "svg": d.svg_file(w, h), "width": w, "height": h }))
            }
            None => error(self.s().get("picture_empty")),
        }
    }

    /// Writes the signed copy. The result's `saved` view describes it, and
    /// `saved_bytes` hands it over.
    pub fn save(&mut self) -> String {
        if self.start_error.is_some() {
            return self.respond(Ok(()));
        }
        let Some(doc) = &self.doc else {
            return self.respond(Err(self.s().get("nothing_placed")));
        };
        let mut stamps = Vec::new();
        for p in &self.placements {
            let page = &doc.pages[p.page];
            let g = Geometry {
                view: page.view(),
                rotate: page.rotate,
            };
            let a = &self.arts[p.art];
            stamps.push(Stamp {
                page: p.page,
                art: p.art,
                cm: place::cm(&g, &p.rect, a.units.0, a.units.1),
            });
        }
        let arts: Vec<Art> = self.arts.iter().map(|a| a.art.clone()).collect();
        match write::amend(doc, &arts, &stamps) {
            Ok(bytes) => {
                self.saved = Some(Saved {
                    bytes,
                    name: signed_name(&self.file_name),
                });
                self.selected = None;
                self.armed = None;
                self.message.clear();
            }
            Err(WriteError::NothingPlaced) => self.message = self.s().get("nothing_placed"),
            Err(WriteError::BadStamp) => {
                self.message = self
                    .s()
                    .fill("save_failed", &[("detail", "a placement is out of range".into())])
            }
            Err(WriteError::ReadBack(d)) => self.message = self.s().fill("save_failed", &[("detail", d)]),
        }
        self.respond(Ok(()))
    }

    pub fn saved_bytes(&self) -> Vec<u8> {
        self.saved.as_ref().map(|s| s.bytes.clone()).unwrap_or_default()
    }

    /// The one door for everything else.
    pub fn act(&mut self, name: &str, value: &str) -> String {
        if self.start_error.is_some() {
            return self.respond(Ok(()));
        }
        let outcome = self.dispatch(name, value);
        self.respond(outcome)
    }

    fn dispatch(&mut self, name: &str, value: &str) -> Result<(), String> {
        match name {
            "ability.share" => self.can_share = value == "true",
            "ability.clipboard" => self.can_read_clipboard = value == "true",
            "view.width" => {
                self.view_width = parse_milli(value).ok_or("The view width is not a number.")? / MILLI;
            }
            "view.dpr" => {
                self.dpr = parse_milli(value)
                    .ok_or("The pixel ratio is not a number.")?
                    .clamp(MILLI, 4 * MILLI)
            }
            // The pages on screen, as "2,3": the first one is the current
            // page for the "Page n of m" label.
            "view.visible" => {
                let first = value.split(',').filter_map(|v| v.parse::<usize>().ok()).min();
                if let Some(k) = first.filter(|k| *k < self.page_count()) {
                    self.current = k;
                }
            }
            "zoom.in" => self.zoom = (self.zoom + 1).min(ZOOMS.len() - 1),
            "zoom.out" => self.zoom = self.zoom.saturating_sub(1),
            "zoom.fit" => self.zoom = FIT,
            "page.go" => {
                let k: usize = value.parse().map_err(|_| "The page number is not a number.")?;
                if k < self.page_count() {
                    self.current = k;
                    self.scroll += 1;
                }
            }
            "page.down" => self.page_down(value)?,
            "page.move" => self.page_move(value)?,
            "page.up" => self.drag = None,
            "selection.remove" => {
                if let Some(id) = self.selected.take() {
                    self.placements.retain(|p| p.id != id);
                }
            }
            "selection.done" => self.selected = None,
            "place.cancel" => self.armed = None,
            "key" => self.key(value),
            "sheet.open" => {
                self.sheet = Some(Sheet { tab: Tab::Draw });
                self.armed = None;
                self.message.clear();
            }
            "sheet.close" => {
                self.sheet = None;
                self.message.clear();
            }
            "sheet.tab" => {
                let tab = match value {
                    "draw" => Tab::Draw,
                    "paste" => Tab::Paste,
                    "open" => Tab::Open,
                    other => return Err(format!("There is no tab \"{other}\".")),
                };
                if let Some(s) = &mut self.sheet {
                    s.tab = tab;
                }
                self.message.clear();
            }
            "pad.down" => {
                let p = self.pad_point(value)?;
                self.pad.down(&p);
            }
            // The page batches the samples a pen gives between frames,
            // separated by ";", so a fast stroke is one call a frame.
            "pad.move" => {
                for sample in value.split(';') {
                    let p = self.pad_point(sample)?;
                    self.pad.moved(&p);
                }
            }
            "pad.up" => {
                let p = self.pad_point(value)?;
                self.pad.up(&p);
            }
            "pad.ink" => {
                self.pad.ink = *Ink::all()
                    .iter()
                    .find(|i| i.key() == value)
                    .ok_or(format!("There is no ink \"{value}\"."))?;
            }
            "pad.weight" => {
                self.pad.weight = *Weight::all()
                    .iter()
                    .find(|w| w.key() == value)
                    .ok_or(format!("There is no pen \"{value}\"."))?;
            }
            "pad.undo" => self.pad.undo(),
            "pad.clear" => self.pad.clear(),
            "picture.clear_paper" => {
                self.clear_paper = !self.clear_paper;
                self.prepare_picture();
            }
            "picture.trim" => {
                self.trim = !self.trim;
                self.prepare_picture();
            }
            "clipboard.empty" => self.message = self.s().get("clipboard_empty"),
            "clipboard.refused" => self.message = self.s().get("clipboard_refused"),
            "picture.unreadable" => self.message = self.s().get("picture_unreadable"),
            "signature.use" => self.use_signature()?,
            "saved.close" => self.saved = None,
            "message.clear" => self.message.clear(),
            other => return Err(format!("Unknown action \"{other}\".")),
        }
        Ok(())
    }

    fn page_count(&self) -> usize {
        self.doc.as_ref().map(|d| d.pages.len()).unwrap_or(0)
    }

    fn pad_point(&self, value: &str) -> Result<PadPoint, String> {
        let p = parse_pointer(value)?;
        Ok(PadPoint {
            id: p.id,
            kind: p.kind,
            x: p.x,
            y: p.y,
            w: p.w,
            h: p.h,
        })
    }

    fn geometry(&self, page: usize) -> Option<Geometry> {
        let doc = self.doc.as_ref()?;
        let p = doc.pages.get(page)?;
        Some(Geometry {
            view: p.view(),
            rotate: p.rotate,
        })
    }

    /// A pointer on page `k`, in display thousandths of a point, and the
    /// handle radius in the same units.
    fn page_point(&self, value: &str) -> Result<(usize, i64, i64, i64), String> {
        let (k, rest) = value
            .split_once('|')
            .ok_or("A page pointer event names no page.")?;
        let k: usize = k.parse().map_err(|_| "The page number is not a number.")?;
        let g = self.geometry(k).ok_or("There is no such page.")?;
        let p = parse_pointer(rest)?;
        let (dw, dh) = g.display();
        let x = mul_div(p.x, dw, p.w);
        let y = mul_div(p.y, dh, p.h);
        let radius = mul_div(HANDLE_RADIUS, dw, p.w);
        Ok((k, x, y, radius))
    }

    fn page_down(&mut self, value: &str) -> Result<(), String> {
        let (k, x, y, radius) = self.page_point(value)?;
        let (pw, ph) = self.geometry(k).map(|g| g.display()).unwrap_or((1, 1));
        self.current = k;
        if let Some(art) = self.armed.take() {
            let a = &self.arts[art];
            let rect = place::at_tap(x, y, START_W, a.shape.0, a.shape.1, pw, ph);
            let id = self.next_id;
            self.next_id += 1;
            self.placements.push(Placement {
                id,
                page: k,
                art,
                rect,
            });
            self.selected = Some(id);
            self.drag = None;
            return Ok(());
        }
        // A corner of the selected signature resizes it. Inside the box only
        // the inner half of a corner's reach counts, so a small signature can
        // still be picked up and moved.
        if let Some(sel) = self
            .selected
            .and_then(|id| self.placements.iter().find(|p| p.id == id && p.page == k))
        {
            let corners = sel.rect.corners();
            let inside = sel.rect.contains(x, y);
            let reach = if inside { radius / 2 } else { radius };
            for (i, (cx, cy)) in corners.iter().enumerate() {
                if (x - cx).abs() <= reach && (y - cy).abs() <= reach {
                    let (ax, ay) = corners[3 - i];
                    self.drag = Some(Drag::Resize {
                        id: sel.id,
                        anchor_x: ax,
                        anchor_y: ay,
                    });
                    return Ok(());
                }
            }
        }
        // The topmost signature under the pointer is picked up.
        if let Some(p) = self
            .placements
            .iter()
            .rev()
            .find(|p| p.page == k && p.rect.contains(x, y))
        {
            self.selected = Some(p.id);
            self.drag = Some(Drag::Move {
                id: p.id,
                grab_x: x - p.rect.x,
                grab_y: y - p.rect.y,
            });
            return Ok(());
        }
        self.selected = None;
        self.drag = None;
        Ok(())
    }

    fn page_move(&mut self, value: &str) -> Result<(), String> {
        let Some(drag) = self.drag else {
            return Ok(());
        };
        let (k, x, y, _) = self.page_point(value)?;
        let (pw, ph) = self.geometry(k).map(|g| g.display()).unwrap_or((1, 1));
        let arts = &self.arts;
        let Some(p) = self.placements.iter_mut().find(|p| match drag {
            Drag::Move { id, .. } | Drag::Resize { id, .. } => p.id == id,
        }) else {
            return Ok(());
        };
        // A drag stays on the page it started on.
        if p.page != k {
            return Ok(());
        }
        let shape = arts[p.art].shape;
        p.rect = match drag {
            Drag::Move { grab_x, grab_y, .. } => place::clamp(
                Rect {
                    x: x - grab_x,
                    y: y - grab_y,
                    ..p.rect
                },
                pw,
                ph,
            ),
            Drag::Resize {
                anchor_x, anchor_y, ..
            } => place::resize(anchor_x, anchor_y, x, y, shape.0, shape.1, MIN_W, pw, ph),
        };
        Ok(())
    }

    fn key(&mut self, value: &str) {
        let (key, big) = match value.strip_suffix("+shift") {
            Some(k) => (k, true),
            None => (value, false),
        };
        if key == "Escape" {
            self.armed = None;
            self.selected = None;
            self.sheet = None;
            self.saved = None;
            return;
        }
        let Some(id) = self.selected else { return };
        if key == "Delete" || key == "Backspace" {
            self.placements.retain(|p| p.id != id);
            self.selected = None;
            return;
        }
        let step = if big { NUDGE_BIG } else { NUDGE };
        let (dx, dy) = match key {
            "ArrowLeft" => (-step, 0),
            "ArrowRight" => (step, 0),
            "ArrowUp" => (0, -step),
            "ArrowDown" => (0, step),
            _ => return,
        };
        let geometry: Vec<Option<Geometry>> = (0..self.page_count()).map(|k| self.geometry(k)).collect();
        if let Some(p) = self.placements.iter_mut().find(|p| p.id == id) {
            if let Some(g) = geometry[p.page] {
                let (pw, ph) = g.display();
                p.rect = place::clamp(
                    Rect {
                        x: p.rect.x + dx,
                        y: p.rect.y + dy,
                        ..p.rect
                    },
                    pw,
                    ph,
                );
            }
        }
    }

    fn use_signature(&mut self) -> Result<(), String> {
        let Some(sheet) = &self.sheet else {
            return Ok(());
        };
        let entry = match sheet.tab {
            Tab::Draw => {
                let Some(d) = self.pad.drawing() else {
                    return Ok(());
                };
                // The same drawing used again places the same art.
                if let Some(k) = self.arts.iter().position(|a| a.drawing.as_ref() == Some(&d)) {
                    self.armed = Some(k);
                    self.sheet = None;
                    return Ok(());
                }
                ArtEntry {
                    art: Art::Form {
                        width: d.width,
                        height: d.height,
                        content: d.pdf_content(),
                    },
                    shape: (d.width, d.height),
                    units: (d.width, d.height),
                    drawing: Some(d),
                    picture: None,
                }
            }
            Tab::Paste | Tab::Open => {
                let Some(p) = &self.picture else {
                    return Ok(());
                };
                if let Some(k) = self.arts.iter().position(|a| a.picture.as_ref() == Some(p)) {
                    self.armed = Some(k);
                    self.sheet = None;
                    return Ok(());
                }
                ArtEntry {
                    art: Art::Image {
                        width: p.width,
                        height: p.height,
                        rgb: p.rgb(),
                        alpha: p.alpha(),
                    },
                    shape: (i64::from(p.width), i64::from(p.height)),
                    units: (1, 1),
                    drawing: None,
                    picture: Some(p.clone()),
                }
            }
        };
        self.arts.push(entry);
        self.armed = Some(self.arts.len() - 1);
        self.sheet = None;
        self.selected = None;
        Ok(())
    }

    fn view(&self) -> Ui {
        let s = self.s();
        let has_doc = self.doc.is_some();
        let toolbar = vec![
            action("file.open", "", s.get("open_pdf")),
            action("sheet.open", "", s.get("add_signature"))
                .enabled(has_doc)
                .primary(),
            action("file.save", "", s.get("save_pdf")).enabled(!self.placements.is_empty()),
        ];
        let zoom = if has_doc {
            vec![
                action("zoom.out", "", s.get("zoom_out")).enabled(self.zoom > 0),
                action("zoom.fit", "", s.get("zoom_fit")).pressed(self.zoom == FIT),
                action("zoom.in", "", s.get("zoom_in")).enabled(self.zoom + 1 < ZOOMS.len()),
            ]
        } else {
            Vec::new()
        };
        let count = self.page_count();
        let nav = has_doc.then(|| Nav {
            label: s.fill(
                "page_of",
                &[
                    ("n", (self.current + 1).to_string()),
                    ("count", count.to_string()),
                ],
            ),
            prev: action(
                "page.go",
                &self.current.saturating_sub(1).to_string(),
                s.get("prev_page"),
            )
            .enabled(self.current > 0),
            next: action(
                "page.go",
                &(self.current + 1).min(count.saturating_sub(1)).to_string(),
                s.get("next_page"),
            )
            .enabled(self.current + 1 < count),
        });
        let file = has_doc.then(|| FileView {
            name: self.file_name.clone(),
            detail: s.fill(
                "file_detail",
                &[
                    ("pages", s.count("pages_one", "pages_many", count)),
                    ("signatures", self.signature_count()),
                ],
            ),
        });
        let status = if !has_doc {
            String::new()
        } else if self.armed.is_some() {
            s.get("status_placing")
        } else if self.selected.is_some() {
            s.get("status_selected")
        } else {
            s.get("status_ready")
        };
        let mut note = String::new();
        if self.doc.as_ref().map(|d| d.has_signatures).unwrap_or(false) {
            note = s.get("already_signed");
        }
        Ui {
            screen: if has_doc { "document" } else { "start" }.into(),
            title: s.get("title"),
            tagline: s.get("tagline"),
            start: vec![s.get("start_heading"), s.get("start_text"), s.get("privacy_note")],
            drop_hint: s.get("drop_hint"),
            message: self.message.clone(),
            note,
            status,
            status_actions: if self.armed.is_some() {
                vec![action("place.cancel", "", s.get("cancel_placing"))]
            } else {
                Vec::new()
            },
            file,
            toolbar,
            zoom,
            nav,
            document: self.document,
            scroll: self.scroll,
            scroll_page: self.current,
            placing: self.armed.is_some(),
            pages: self.page_views(),
            arts: self.art_views(),
            selection: self.selection_view(),
            sheet: self.sheet_view(),
            saved: self.saved_view(),
            hints: self.hints.clone(),
        }
    }

    fn signature_count(&self) -> String {
        let s = self.s();
        match self.placements.len() {
            0 => s.get("sigs_none"),
            n => s.count("sigs_one", "sigs_many", n),
        }
    }

    fn page_views(&self) -> Vec<PageView> {
        let Some(doc) = &self.doc else {
            return Vec::new();
        };
        let s = self.s();
        let widest = doc
            .pages
            .iter()
            .map(|p| {
                Geometry {
                    view: p.view(),
                    rotate: p.rotate,
                }
                .display()
                .0
            })
            .max()
            .unwrap_or(1)
            .max(1);
        let fit = (self.view_width - 2 * GUTTER).max(160);
        doc.pages
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let g = Geometry {
                    view: p.view(),
                    rotate: p.rotate,
                };
                let (dw, dh) = g.display();
                let css_w = mul_div(mul_div(fit, ZOOMS[self.zoom], 100), dw, widest).max(1);
                let css_h = mul_div(css_w, dh, dw.max(1)).max(1);
                let mut canvas_w = mul_div(css_w, self.dpr, MILLI).max(1);
                let mut canvas_h = mul_div(css_h, self.dpr, MILLI).max(1);
                if canvas_w * canvas_h > MAX_CANVAS_PX {
                    // Shrink both sides by the square root of the excess.
                    let area = canvas_w * canvas_h;
                    let root = crate::num::isqrt(mul_div(MAX_CANVAS_PX, 1_000_000, area));
                    canvas_w = mul_div(canvas_w, root, 1000).max(1);
                    canvas_h = mul_div(canvas_h, root, 1000).max(1);
                }
                // pdf.js paints at `scale` pixels per point.
                let scale_micro = mul_div(canvas_w * MILLI, 1_000_000, dw.max(1));
                let scale_text = format_scaled(scale_micro, 1_000_000, 6);
                let scale = serde_json::from_str(&scale_text).unwrap_or(serde_json::Value::Null);
                let signed = self.placements.iter().any(|q| q.page == k);
                let label = if signed {
                    s.fill("page_signed", &[("n", (k + 1).to_string())])
                } else {
                    s.fill("page_label", &[("n", (k + 1).to_string())])
                };
                let overlays = self
                    .placements
                    .iter()
                    .filter(|q| q.page == k)
                    .map(|q| Overlay {
                        id: q.id,
                        art: q.art,
                        left: place::percent(q.rect.x, dw),
                        top: place::percent(q.rect.y, dh),
                        width: place::percent(q.rect.w, dw),
                        height: place::percent(q.rect.h, dh),
                        selected: self.selected == Some(q.id),
                    })
                    .collect();
                PageView {
                    index: k,
                    label,
                    css_width: format!("{css_w}px"),
                    css_height: format!("{css_h}px"),
                    canvas_width: canvas_w,
                    canvas_height: canvas_h,
                    scale,
                    overlays,
                }
            })
            .collect()
    }

    fn art_views(&self) -> Vec<ArtView> {
        self.arts
            .iter()
            .enumerate()
            .map(|(id, a)| match (&a.drawing, &a.picture) {
                (Some(d), _) => ArtView {
                    id,
                    kind: "drawing".into(),
                    view_box: d.view_box(),
                    paths: d.svg_paths(),
                    stroke: d.ink.css().into(),
                    stroke_width: d.weight.units(),
                    width: 0,
                    height: 0,
                },
                (None, Some(p)) => ArtView {
                    id,
                    kind: "picture".into(),
                    view_box: String::new(),
                    paths: Vec::new(),
                    stroke: String::new(),
                    stroke_width: 0,
                    width: p.width,
                    height: p.height,
                },
                (None, None) => ArtView {
                    id,
                    kind: "none".into(),
                    view_box: String::new(),
                    paths: Vec::new(),
                    stroke: String::new(),
                    stroke_width: 0,
                    width: 0,
                    height: 0,
                },
            })
            .collect()
    }

    fn selection_view(&self) -> Option<Selection> {
        let id = self.selected?;
        let p = self.placements.iter().find(|p| p.id == id)?;
        let (dw, dh) = self.geometry(p.page)?.display();
        let s = self.s();
        Some(Selection {
            page: p.page,
            left: place::percent(p.rect.x, dw),
            top: place::percent(p.rect.y, dh),
            actions: vec![
                action("selection.remove", "", s.get("remove")),
                action("selection.done", "", s.get("done")),
            ],
        })
    }

    fn sheet_view(&self) -> Option<SheetView> {
        let sheet = self.sheet.as_ref()?;
        let s = self.s();
        let tabs = [
            (Tab::Draw, "tab_draw"),
            (Tab::Paste, "tab_paste"),
            (Tab::Open, "tab_open"),
        ]
        .iter()
        .map(|(t, key)| action("sheet.tab", t.key(), s.get(key)).pressed(sheet.tab == *t))
        .collect();
        let drawing = sheet.tab == Tab::Draw;
        let ready = if drawing {
            !self.pad.strokes.is_empty()
        } else {
            self.picture.is_some()
        };
        let mut actions = Vec::new();
        if drawing {
            actions.push(action("signature.save_image", "", s.get("save_image")).enabled(ready));
        }
        actions.push(
            action("signature.use", "", s.get("use_signature"))
                .enabled(ready)
                .primary(),
        );
        Some(SheetView {
            title: s.get("sheet_title"),
            tab: sheet.tab.key().into(),
            tabs,
            close: action("sheet.close", "", s.get("close")),
            hint: match sheet.tab {
                Tab::Draw => String::new(),
                Tab::Paste => s.get("paste_hint"),
                Tab::Open => s.get("open_image_hint"),
            },
            pad: drawing.then(|| PadView {
                label: s.get("pad_label"),
                view_box: format!("0 0 {PAD_W} {PAD_H}"),
                baseline: PAD_H * 4 / 5,
                paths: self.pad.svg_paths(),
                stroke: self.pad.ink.css().into(),
                stroke_width: self.pad.weight.units(),
                hint: s.get("pad_hint"),
            }),
            ink_label: s.get("ink_label"),
            inks: if drawing {
                Ink::all()
                    .iter()
                    .map(|i| {
                        action("pad.ink", i.key(), s.get(&format!("ink_{}", i.key())))
                            .pressed(self.pad.ink == *i)
                    })
                    .collect()
            } else {
                Vec::new()
            },
            pen_label: s.get("pen_label"),
            weights: if drawing {
                Weight::all()
                    .iter()
                    .map(|w| {
                        action("pad.weight", w.key(), s.get(&format!("weight_{}", w.key())))
                            .pressed(self.pad.weight == *w)
                    })
                    .collect()
            } else {
                Vec::new()
            },
            edits: if drawing {
                vec![
                    action("pad.undo", "", s.get("undo")).enabled(!self.pad.strokes.is_empty()),
                    action("pad.clear", "", s.get("clear")).enabled(!self.pad.strokes.is_empty()),
                ]
            } else {
                Vec::new()
            },
            fetch: match sheet.tab {
                Tab::Draw => None,
                Tab::Paste => {
                    Some(action("picture.paste", "", s.get("paste_button")).enabled(self.can_read_clipboard))
                }
                Tab::Open => Some(action("picture.open", "", s.get("open_image_button"))),
            },
            picture: if drawing {
                None
            } else {
                self.picture.as_ref().map(|p| PictureView {
                    width: p.width,
                    height: p.height,
                    version: self.picture_version,
                    source: self.raw.as_ref().map_or(String::new(), |r| {
                        s.fill(
                            "picture_from",
                            &[
                                ("name", r.name.clone()),
                                ("w", r.width.to_string()),
                                ("h", r.height.to_string()),
                            ],
                        )
                    }),
                })
            },
            empty: s.get("picture_empty"),
            image_file: s.get("signature_file"),
            options: if drawing {
                Vec::new()
            } else {
                vec![
                    action("picture.clear_paper", "", s.get("clear_paper")).pressed(self.clear_paper),
                    action("picture.trim", "", s.get("trim")).pressed(self.trim),
                ]
            },
            actions,
        })
    }

    fn saved_view(&self) -> Option<SavedView> {
        let saved = self.saved.as_ref()?;
        let s = self.s();
        let mut actions = vec![action("saved.download", "", s.get("download")).primary()];
        if self.can_share {
            actions.push(action("saved.share", "", s.get("share")));
        }
        actions.push(action("saved.close", "", s.get("keep_editing")));
        Some(SavedView {
            title: s.get("saved_title"),
            file_name: saved.name.clone(),
            detail: s.fill(
                "saved_detail",
                &[
                    ("name", saved.name.clone()),
                    ("pages", s.count("pages_one", "pages_many", self.page_count())),
                    ("size", size_text(s, saved.bytes.len())),
                ],
            ),
            note: s.get("saved_note"),
            actions,
        })
    }
}

/// The original's name with "-signed" before ".pdf".
pub fn signed_name(name: &str) -> String {
    let base = if name.to_ascii_lowercase().ends_with(".pdf") {
        &name[..name.len() - 4]
    } else {
        name
    };
    let base = if base.trim().is_empty() { "document" } else { base };
    format!("{base}-signed.pdf")
}

/// "412 KB", or "2.4 MB" from a megabyte up.
fn size_text(s: &Strings, bytes: usize) -> String {
    let b = bytes as i64;
    if b < 1024 * 1024 {
        s.fill("size_kb", &[("n", ((b + 512) / 1024).max(1).to_string())])
    } else {
        let tenths = mul_div(b, 10, 1024 * 1024);
        s.fill("size_mb", &[("n", format_scaled(tenths, 10, 1))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::fixture;

    const STRINGS: &str = include_str!("../../../data/strings.json");
    const GLOSSARY: &str = "### Signature\n\nThe mark you write.\n";

    fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap()
    }

    fn opened() -> Handle {
        let mut h = Handle::new(STRINGS, GLOSSARY);
        json(&h.act("view.width", "644"));
        h.open_pdf("lease.pdf", &fixture::sample());
        h
    }

    /// A pointer at (x, y) CSS pixels on page 0 shown 612 by 792 pixels:
    /// one pixel a point.
    fn on_page(x: i64, y: i64) -> String {
        format!("0|1|mouse|{x}|{y}|0|0|612|792")
    }

    fn draw_and_use(h: &mut Handle) {
        h.act("sheet.open", "");
        // Pad shown 300 by 100 pixels: a stroke from (30, 40) to (60, 40).
        h.act("pad.down", "1|mouse|30|40|0|0|300|100");
        h.act("pad.move", "1|mouse|45|40|0|0|300|100");
        h.act("pad.move", "1|mouse|60|40|0|0|300|100");
        h.act("pad.up", "1|mouse|60|40|0|0|300|100");
        h.act("signature.use", "");
    }

    #[test]
    fn a_bad_strings_file_is_reported_by_every_call() {
        let mut h = Handle::new("{}", GLOSSARY);
        let v = json(&h.act("zoom.in", ""));
        assert_eq!(v["error"], "The strings file has no \"title\".");
        assert_eq!(json(&h.ui())["error"], "The strings file has no \"title\".");
    }

    #[test]
    fn the_start_screen_offers_open_only() {
        let h = Handle::new(STRINGS, GLOSSARY);
        let v = json(&h.ui());
        assert_eq!(v["ok"]["screen"], "start");
        assert_eq!(v["ok"]["toolbar"][0]["name"], "file.open");
        assert_eq!(v["ok"]["toolbar"][1]["enabled"], false);
        assert_eq!(v["ok"]["toolbar"][2]["enabled"], false);
    }

    #[test]
    fn opening_a_pdf_shows_its_pages_at_fit_width() {
        let h = opened();
        let v = json(&h.ui());
        assert_eq!(v["ok"]["screen"], "document");
        assert_eq!(v["ok"]["file"]["detail"], "2 pages · no signatures yet");
        assert_eq!(v["ok"]["nav"]["label"], "Page 1 of 2");
        // View 644 wide less two 16 px gutters: 612 px for 612 points.
        assert_eq!(v["ok"]["pages"][0]["css_width"], "612px");
        assert_eq!(v["ok"]["pages"][0]["css_height"], "792px");
        assert_eq!(v["ok"]["pages"][0]["scale"], 1);
    }

    #[test]
    fn a_retina_screen_paints_at_twice_the_pixels() {
        let mut h = opened();
        let v = json(&h.act("view.dpr", "2"));
        assert_eq!(v["ok"]["pages"][0]["canvas_width"], 1224);
        assert_eq!(v["ok"]["pages"][0]["scale"], 2);
    }

    #[test]
    fn a_refused_pdf_is_a_message_and_nothing_opens() {
        let mut h = Handle::new(STRINGS, GLOSSARY);
        let v = json(&h.open_pdf("notes.txt", b"hello"));
        assert_eq!(v["ok"]["screen"], "start");
        assert_eq!(
            v["ok"]["message"],
            "This file is not a PDF. Choose a file whose name ends in .pdf."
        );
    }

    #[test]
    fn drawing_using_and_tapping_places_a_signature_on_the_line() {
        let mut h = opened();
        draw_and_use(&mut h);
        assert_eq!(json(&h.ui())["ok"]["placing"], true);
        let v = json(&h.act("page.down", &on_page(186, 627)));
        let o = &v["ok"]["pages"][0]["overlays"][0];
        // The drawing is 300..600 by 400 in pad units, with a 15 + 12 margin:
        // 354 by 54 units. At 180 points wide it is 180 * 54 / 354 = 27.458
        // high. Bottom on y 627, centred on x 186: left 96, top 599.542.
        assert_eq!(o["left"], place::percent(96_000, 612_000));
        assert_eq!(o["top"], place::percent(599_542, 792_000));
        assert_eq!(o["width"], place::percent(180_000, 612_000));
        assert_eq!(v["ok"]["placing"], false);
        assert_eq!(v["ok"]["file"]["detail"], "2 pages · 1 signature placed");
        assert_eq!(v["ok"]["pages"][0]["label"], "Page 1 · signed");
    }

    #[test]
    fn a_placed_signature_drags_and_nudges() {
        let mut h = opened();
        draw_and_use(&mut h);
        h.act("page.down", &on_page(186, 627));
        h.act("page.up", &on_page(186, 627));
        // Grab it in the middle (90 px from its left edge) and move 50
        // right, 20 up: left 96 + 50 = 146.
        h.act("page.down", &on_page(186, 615));
        h.act("page.move", &on_page(236, 595));
        let v = json(&h.act("page.up", &on_page(236, 595)));
        assert_eq!(
            v["ok"]["pages"][0]["overlays"][0]["left"],
            place::percent(146_000, 612_000)
        );
        let v = json(&h.act("key", "ArrowLeft+shift"));
        assert_eq!(
            v["ok"]["pages"][0]["overlays"][0]["left"],
            place::percent(136_000, 612_000)
        );
        let v = json(&h.act("key", "Delete"));
        assert_eq!(v["ok"]["pages"][0]["overlays"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn a_small_signature_grabbed_near_its_corner_moves() {
        let mut h = opened();
        draw_and_use(&mut h);
        h.act("page.down", &on_page(186, 627));
        h.act("page.up", &on_page(186, 627));
        // The box is (96, 599.542), 27.458 high. (111, 612) is inside it,
        // 15 and 12.458 from the top-left corner: within a corner's 22 px
        // reach but not its inner 11, so this is a move. 50 right: left 146.
        h.act("page.down", &on_page(111, 612));
        let v = json(&h.act("page.move", &on_page(161, 612)));
        let o = &v["ok"]["pages"][0]["overlays"][0];
        assert_eq!(o["left"], place::percent(146_000, 612_000));
        assert_eq!(o["width"], place::percent(180_000, 612_000));
    }

    #[test]
    fn a_corner_resizes_and_keeps_the_opposite_corner() {
        let mut h = opened();
        draw_and_use(&mut h);
        h.act("page.down", &on_page(186, 627));
        h.act("page.up", &on_page(186, 627));
        // Just outside the bottom-right corner (276, 627): drag it 90 right.
        h.act("page.down", &on_page(280, 630));
        let v = json(&h.act("page.move", &on_page(366, 640)));
        let o = &v["ok"]["pages"][0]["overlays"][0];
        // Anchor top-left (96, 599.542); dx = 270 wins over dy by shape.
        assert_eq!(o["left"], place::percent(96_000, 612_000));
        assert_eq!(o["top"], place::percent(599_542, 792_000));
        assert_eq!(o["width"], place::percent(270_000, 612_000));
    }

    #[test]
    fn a_tap_on_empty_page_clears_the_selection() {
        let mut h = opened();
        draw_and_use(&mut h);
        h.act("page.down", &on_page(186, 627));
        let v = json(&h.act("page.down", &on_page(500, 100)));
        assert!(v["ok"]["selection"].is_null());
    }

    #[test]
    fn saving_appends_to_the_original_and_names_the_copy() {
        let mut h = opened();
        draw_and_use(&mut h);
        h.act("page.down", &on_page(186, 627));
        let v = json(&h.save());
        assert_eq!(v["ok"]["saved"]["file_name"], "lease-signed.pdf");
        let bytes = h.saved_bytes();
        assert!(bytes.starts_with(&fixture::sample()));
        assert!(bytes.len() > fixture::sample().len());
    }

    #[test]
    fn save_with_nothing_placed_says_so() {
        let mut h = opened();
        let v = json(&h.save());
        assert_eq!(v["ok"]["message"], "Place a signature on a page before saving.");
        assert!(h.saved_bytes().is_empty());
    }

    #[test]
    fn a_pasted_picture_is_trimmed_and_placed_as_an_image() {
        let mut h = opened();
        h.act("sheet.open", "");
        h.act("sheet.tab", "paste");
        // 3 x 1: paper, ink, paper. Trimmed with a 1 px margin it stays 3 x 1.
        let px = [250, 250, 250, 255, 20, 20, 20, 255, 250, 250, 250, 255];
        let v = json(&h.picture("sig.png", 3, 1, &px));
        assert_eq!(v["ok"]["sheet"]["picture"]["width"], 3);
        assert_eq!(
            v["ok"]["sheet"]["picture"]["source"],
            "From sig.png, 3 × 1 pixels"
        );
        h.act("signature.use", "");
        h.act("page.down", &on_page(300, 400));
        let v = json(&h.ui());
        assert_eq!(v["ok"]["arts"][0]["kind"], "picture");
        assert_eq!(h.art_rgba(0)[7], 255);
        assert!(h.saved_bytes().is_empty());
        json(&h.save());
        assert!(h.saved_bytes().starts_with(&fixture::sample()));
    }

    #[test]
    fn a_blank_picture_is_refused_with_a_reason() {
        let mut h = opened();
        h.act("sheet.open", "");
        h.act("sheet.tab", "open");
        let v = json(&h.picture("white.png", 1, 1, &[255, 255, 255, 255]));
        assert!(v["ok"]["message"]
            .as_str()
            .unwrap()
            .starts_with("No ink was found"));
        assert_eq!(v["ok"]["sheet"]["actions"][0]["enabled"], false);
    }

    #[test]
    fn share_is_offered_only_where_the_browser_can_share_files() {
        let mut h = opened();
        draw_and_use(&mut h);
        h.act("page.down", &on_page(186, 627));
        let v = json(&h.save());
        let names: Vec<&str> = v["ok"]["saved"]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["saved.download", "saved.close"]);
        let v = json(&h.act("ability.share", "true"));
        assert_eq!(v["ok"]["saved"]["actions"][1]["name"], "saved.share");
    }

    #[test]
    fn act_refuses_an_unknown_action_by_name() {
        let mut h = opened();
        assert_eq!(
            json(&h.act("screen.jump", ""))["error"],
            "Unknown action \"screen.jump\"."
        );
        assert_eq!(
            json(&h.act("sheet.tab", "camera"))["error"],
            "There is no tab \"camera\"."
        );
    }

    #[test]
    fn pen_samples_arrive_batched_and_tiny_values_read_as_zero() {
        let mut h = opened();
        h.act("sheet.open", "");
        h.act("pad.down", "1|pen|0|0|0|0|300|100");
        h.act(
            "pad.move",
            "1|pen|1|0|0|0|300|100;1|pen|2|0|0|0|300|100;1|pen|3|4.2e-7|0|0|300|100",
        );
        let v = json(&h.act("pad.up", "1|pen|3|0|0|0|300|100"));
        // 0, 10, 20, 30 units along x; each step of 10 is over the minimum.
        // Controls, by Catmull-Rom with whole-unit division: 0 + 10/6 = 1 and
        // 10 - 20/6 = 7; 10 + 20/6 = 13 and 20 - 20/6 = 17; 20 + 20/6 = 23
        // and 30 - 10/6 = 29.
        assert_eq!(
            v["ok"]["sheet"]["pad"]["paths"][0],
            "M0 0 C1 0 7 0 10 0 C13 0 17 0 20 0 C23 0 29 0 30 0"
        );
    }

    #[test]
    fn the_first_page_on_screen_is_the_current_page() {
        let mut h = opened();
        let v = json(&h.act("view.visible", "1"));
        assert_eq!(v["ok"]["nav"]["label"], "Page 2 of 2");
        let v = json(&h.act("view.visible", "1,0"));
        assert_eq!(v["ok"]["nav"]["label"], "Page 1 of 2");
    }

    #[test]
    fn signed_names_keep_the_original_name() {
        assert_eq!(signed_name("Lease Renewal.PDF"), "Lease Renewal-signed.pdf");
        assert_eq!(signed_name("form"), "form-signed.pdf");
        assert_eq!(signed_name(".pdf"), "document-signed.pdf");
    }

    #[test]
    fn sizes_read_in_kilobytes_then_megabytes() {
        let s = Strings::parse(STRINGS).unwrap();
        assert_eq!(size_text(&s, 100), "1 KB");
        assert_eq!(size_text(&s, 421_888), "412 KB");
        assert_eq!(size_text(&s, 2_516_582), "2.4 MB");
    }
}

//! The incremental update (PS-1): the original bytes, untouched, then the
//! signatures as new objects, a new version of each signed page, and a
//! cross-reference section pointing back at the old one. Then the update is
//! read back with the same reader before it is offered (PS-6).

use std::collections::BTreeMap;

use super::doc::XrefKind;
use super::doc::{Document, Loaded};
use super::filter::deflate;
use super::object::{self, Dict, Obj};

/// What a signature is in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Art {
    /// Vector paths, drawn in a box `width` by `height` form units; `content`
    /// is the PDF drawing operators.
    Form {
        width: i64,
        height: i64,
        content: Vec<u8>,
    },
    /// Pixels, drawn in the unit square: RGB samples and an alpha mask.
    Image {
        width: u32,
        height: u32,
        rgb: Vec<u8>,
        alpha: Vec<u8>,
    },
}

/// One signature placed on one page: which art, and the `cm` matrix that
/// maps the art's space onto the page, already as PDF text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    pub page: usize,
    pub art: usize,
    pub cm: String,
}

/// Why an update could not be written or did not read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    NothingPlaced,
    BadStamp,
    ReadBack(String),
}

struct Out {
    bytes: Vec<u8>,
    offsets: BTreeMap<u32, (usize, u16)>,
}

impl Out {
    fn begin(&mut self, number: u32, generation: u16) {
        self.offsets.insert(number, (self.bytes.len(), generation));
        self.bytes
            .extend_from_slice(format!("{number} {generation} obj\n").as_bytes());
    }

    fn plain(&mut self, number: u32, generation: u16, obj: &Obj) {
        self.begin(number, generation);
        object::write(obj, &mut self.bytes);
        self.bytes.extend_from_slice(b"\nendobj\n");
    }

    fn stream(&mut self, number: u32, mut dict: Dict, data: &[u8]) {
        self.begin(number, 0);
        dict.set("Length", Obj::Int(data.len() as i64));
        object::write_dict(&dict, &mut self.bytes);
        self.bytes.extend_from_slice(b"stream\n");
        self.bytes.extend_from_slice(data);
        self.bytes.extend_from_slice(b"\nendstream\nendobj\n");
    }
}

fn dict(pairs: Vec<(&str, Obj)>) -> Dict {
    Dict(
        pairs
            .into_iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v))
            .collect(),
    )
}

/// The resource name for art `k` that the page does not already use.
fn art_name(k: usize, taken: &Dict) -> String {
    let mut name = format!("PSAsig{}", k + 1);
    while taken.get(&name).is_some() {
        name.push('x');
    }
    name
}

/// Appends `stamps` to `doc` and returns the new file.
pub fn amend(doc: &Document, arts: &[Art], stamps: &[Stamp]) -> Result<Vec<u8>, WriteError> {
    if stamps.is_empty() {
        return Err(WriteError::NothingPlaced);
    }
    if stamps
        .iter()
        .any(|s| s.page >= doc.pages.len() || s.art >= arts.len())
    {
        return Err(WriteError::BadStamp);
    }
    let mut out = Out {
        bytes: doc.bytes.clone(),
        offsets: BTreeMap::new(),
    };
    if !matches!(out.bytes.last(), Some(b'\n') | Some(b'\r')) {
        out.bytes.push(b'\n');
    }
    let mut next = doc.size;

    // One object (two for a picture) per art that is placed at least once,
    // shared by every placement of it.
    let mut art_obj: BTreeMap<usize, u32> = BTreeMap::new();
    for s in stamps {
        if art_obj.contains_key(&s.art) {
            continue;
        }
        let n = take(&mut next);
        art_obj.insert(s.art, n);
        match &arts[s.art] {
            Art::Form {
                width,
                height,
                content,
            } => {
                let d = dict(vec![
                    ("Type", Obj::name("XObject")),
                    ("Subtype", Obj::name("Form")),
                    (
                        "BBox",
                        Obj::Array(vec![
                            Obj::Int(0),
                            Obj::Int(0),
                            Obj::Int(*width),
                            Obj::Int(*height),
                        ]),
                    ),
                    ("Resources", Obj::Dict(Dict::default())),
                    ("Filter", Obj::name("FlateDecode")),
                ]);
                out.stream(n, d, &deflate(content));
            }
            Art::Image {
                width,
                height,
                rgb,
                alpha,
            } => {
                let mask = take(&mut next);
                let base = |space: &str| {
                    dict(vec![
                        ("Type", Obj::name("XObject")),
                        ("Subtype", Obj::name("Image")),
                        ("Width", Obj::Int(i64::from(*width))),
                        ("Height", Obj::Int(i64::from(*height))),
                        ("ColorSpace", Obj::name(space)),
                        ("BitsPerComponent", Obj::Int(8)),
                        ("Filter", Obj::name("FlateDecode")),
                    ])
                };
                let mut d = base("DeviceRGB");
                d.set("SMask", Obj::Ref(mask, 0));
                out.stream(n, d, &deflate(rgb));
                out.stream(mask, base("DeviceGray"), &deflate(alpha));
            }
        }
    }

    // Each signed page: a "q" stream before its old contents, and a stream
    // after them that restores the state and draws the signatures.
    let mut by_page: BTreeMap<usize, Vec<&Stamp>> = BTreeMap::new();
    for s in stamps {
        by_page.entry(s.page).or_default().push(s);
    }
    for (page_index, page_stamps) in &by_page {
        let page = &doc.pages[*page_index];
        let mut resources = page.resources.clone();
        let mut xobjects = match resources.get("XObject") {
            Some(o) => doc.resolve_dict(o).unwrap_or_default(),
            None => Dict::default(),
        };
        let mut names: BTreeMap<usize, String> = BTreeMap::new();
        for s in page_stamps {
            if names.contains_key(&s.art) {
                continue;
            }
            let name = art_name(s.art, &xobjects);
            xobjects.set(&name, Obj::Ref(art_obj[&s.art], 0));
            names.insert(s.art, name);
        }
        resources.set("XObject", Obj::Dict(xobjects));

        let open = take(&mut next);
        out.stream(open, Dict::default(), b"q\n");
        let close = take(&mut next);
        let mut draw = b"Q\n".to_vec();
        for s in page_stamps {
            draw.extend_from_slice(format!("q {} cm /{} Do Q\n", s.cm, names[&s.art]).as_bytes());
        }
        out.stream(close, Dict::default(), &draw);

        let old: Vec<Obj> = match page.dict.get("Contents") {
            None => Vec::new(),
            Some(Obj::Array(items)) => items.clone(),
            Some(r @ Obj::Ref(n, _)) => match doc.load(*n) {
                Ok(Loaded::Plain(Obj::Array(items))) => items,
                _ => vec![r.clone()],
            },
            Some(_) => Vec::new(),
        };
        let mut contents = vec![Obj::Ref(open, 0)];
        contents.extend(old);
        contents.push(Obj::Ref(close, 0));
        let mut new_page = page.dict.clone();
        new_page.set("Contents", Obj::Array(contents));
        new_page.set("Resources", Obj::Dict(resources));
        out.plain(page.number, page.generation, &Obj::Dict(new_page));
    }

    let mut trailer = Dict::default();
    for key in ["Root", "Info", "ID"] {
        if let Some(v) = doc.trailer.get(key) {
            trailer.set(key, v.clone());
        }
    }
    trailer.set("Prev", Obj::Int(doc.startxref as i64));
    match doc.xref_kind {
        XrefKind::Table => {
            let at = out.bytes.len();
            trailer.set("Size", Obj::Int(i64::from(next)));
            out.bytes.extend_from_slice(b"xref\n");
            for (start, run) in runs(&out.offsets) {
                out.bytes
                    .extend_from_slice(format!("{start} {}\n", run.len()).as_bytes());
                for (off, generation) in run {
                    out.bytes
                        .extend_from_slice(format!("{off:010} {generation:05} n\r\n").as_bytes());
                }
            }
            out.bytes.extend_from_slice(b"trailer\n");
            object::write_dict(&trailer, &mut out.bytes);
            out.bytes
                .extend_from_slice(format!("\nstartxref\n{at}\n%%EOF\n").as_bytes());
        }
        XrefKind::Stream => {
            let xref_no = take(&mut next);
            let at = out.bytes.len();
            out.offsets.insert(xref_no, (at, 0));
            let mut index = Vec::new();
            let mut data = Vec::new();
            for (start, run) in runs(&out.offsets) {
                index.push(Obj::Int(i64::from(start)));
                index.push(Obj::Int(run.len() as i64));
                for (off, generation) in run {
                    data.push(1u8);
                    data.extend_from_slice(&(off as u32).to_be_bytes());
                    data.extend_from_slice(&generation.to_be_bytes());
                }
            }
            trailer.set("Type", Obj::name("XRef"));
            trailer.set("Size", Obj::Int(i64::from(next)));
            trailer.set("W", Obj::Array(vec![Obj::Int(1), Obj::Int(4), Obj::Int(2)]));
            trailer.set("Index", Obj::Array(index));
            out.stream(xref_no, trailer, &data);
            out.bytes
                .extend_from_slice(format!("startxref\n{at}\n%%EOF\n").as_bytes());
        }
    }
    let bytes = out.bytes;
    read_back(doc, &bytes, &by_page.keys().copied().collect::<Vec<_>>())?;
    Ok(bytes)
}

/// The next free object number.
fn take(next: &mut u32) -> u32 {
    let n = *next;
    *next += 1;
    n
}

/// Consecutive object numbers grouped into cross-reference subsections.
fn runs(offsets: &BTreeMap<u32, (usize, u16)>) -> Vec<(u32, Vec<(usize, u16)>)> {
    let mut out: Vec<(u32, Vec<(usize, u16)>)> = Vec::new();
    for (&n, &entry) in offsets {
        match out.last_mut() {
            Some((start, run)) if *start + run.len() as u32 == n => run.push(entry),
            _ => out.push((n, vec![entry])),
        }
    }
    out
}

/// PS-6: the new file starts with the old one, reads with the same reader,
/// has the same pages, and each signed page's last content stream draws a
/// signature named in its resources.
pub fn read_back(original: &Document, bytes: &[u8], signed_pages: &[usize]) -> Result<(), WriteError> {
    let fail = |what: &str| Err(WriteError::ReadBack(what.to_string()));
    if !bytes.starts_with(&original.bytes) {
        return fail("the original bytes changed");
    }
    let doc = match Document::open(bytes.to_vec()) {
        Ok(d) => d,
        Err(e) => return Err(WriteError::ReadBack(format!("{e:?}"))),
    };
    if doc.pages.len() != original.pages.len() {
        return fail("the page count changed");
    }
    for &k in signed_pages {
        let page = &doc.pages[k];
        let Some(Obj::Array(contents)) = page.dict.get("Contents") else {
            return fail("a signed page has no contents array");
        };
        let Some(Obj::Ref(last, _)) = contents.last() else {
            return fail("a signed page's contents do not end in a stream");
        };
        let Ok(Loaded::Stream(s)) = doc.load(*last) else {
            return fail("a signed page's drawing stream does not load");
        };
        let xobjects = page
            .resources
            .get("XObject")
            .and_then(|o| doc.resolve_dict(o).ok())
            .unwrap_or_default();
        let text = String::from_utf8_lossy(&s.data).into_owned();
        let drawn: Vec<&str> = text
            .split('/')
            .skip(1)
            .filter_map(|t| t.strip_suffix(" Do Q\n").or_else(|| t.split(" Do").next()))
            .collect();
        if drawn.is_empty() {
            return fail("a signed page draws no signature");
        }
        for name in drawn {
            match xobjects.get(name) {
                Some(Obj::Ref(n, _)) => match doc.load(*n) {
                    Ok(Loaded::Stream(_)) => {}
                    _ => return fail("a signature object does not load"),
                },
                _ => return fail("a drawn signature is missing from the page's resources"),
            }
        }
    }
    Ok(())
}

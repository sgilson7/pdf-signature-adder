//! Reading a PDF far enough to amend it: the cross-reference chain, objects
//! (including those inside object streams), the page tree with its inherited
//! boxes, rotation and resources, and the reasons to refuse a file (PS-5).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use super::filter::{self, FilterError};
use super::object::{Dict, Lexer, Obj};

/// Where an object lives, from the newest cross-reference entry for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    Free,
    InFile { offset: usize, generation: u16 },
    InStream { stream: u32, index: u32 },
}

/// Why a file cannot be amended. Each has its own message (`message`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    NotPdf,
    Damaged(String),
    Encrypted,
    Certified,
    NoPages,
    Unsupported(String),
}

impl Refusal {
    /// A key into `data/strings.json`'s refusals and the detail, if any.
    pub fn key(&self) -> (&'static str, String) {
        match self {
            Refusal::NotPdf => ("not_pdf", String::new()),
            Refusal::Damaged(d) => ("damaged", d.clone()),
            Refusal::Encrypted => ("encrypted", String::new()),
            Refusal::Certified => ("certified", String::new()),
            Refusal::NoPages => ("no_pages", String::new()),
            Refusal::Unsupported(d) => ("unsupported", d.clone()),
        }
    }
}

fn damaged<T>(what: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal::Damaged(what.into()))
}

/// A stream object: its dictionary and its encoded data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub dict: Dict,
    pub data: Vec<u8>,
}

/// A loaded object, which may be a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Loaded {
    Plain(Obj),
    Stream(Stream),
}

/// One page, with what it inherits from the page tree already applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    pub number: u32,
    pub generation: u16,
    /// The page dictionary itself, as written in the file.
    pub dict: Dict,
    /// [x0, y0, x1, y1] in thousandths of a point, normalised so x0 < x1.
    pub media: [i64; 4],
    pub crop: [i64; 4],
    /// 0, 90, 180 or 270.
    pub rotate: i64,
    /// The page's effective resources, resolved to a dictionary.
    pub resources: Dict,
}

impl PageInfo {
    /// The visible area: the crop box clipped to the media box.
    pub fn view(&self) -> [i64; 4] {
        let [mx0, my0, mx1, my1] = self.media;
        let [cx0, cy0, cx1, cy1] = self.crop;
        let v = [cx0.max(mx0), cy0.max(my0), cx1.min(mx1), cy1.min(my1)];
        if v[2] > v[0] && v[3] > v[1] {
            v
        } else {
            self.media
        }
    }
}

/// Which kind of cross-reference section the file ends with; the update
/// writes the same kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrefKind {
    Table,
    Stream,
}

/// A PDF read far enough to amend.
#[derive(Debug)]
pub struct Document {
    pub bytes: Vec<u8>,
    pub entries: BTreeMap<u32, Entry>,
    pub trailer: Dict,
    pub startxref: usize,
    pub xref_kind: XrefKind,
    /// One more than the highest object number in use.
    pub size: u32,
    pub pages: Vec<PageInfo>,
    /// The file already carries a digital signature field that is signed.
    pub has_signatures: bool,
    stream_cache: RefCell<BTreeMap<u32, Vec<(u32, Obj)>>>,
    scanned: RefCell<Option<BTreeMap<u32, usize>>>,
}

/// More cross-reference sections than this is a loop or a crafted file.
const MAX_SECTIONS: usize = 4096;
/// More pages than this is refused; a page tree that large is not a form.
const MAX_PAGES: usize = 100_000;

impl Document {
    pub fn open(bytes: Vec<u8>) -> Result<Document, Refusal> {
        let head = &bytes[..bytes.len().min(1024)];
        if find(head, b"%PDF-").is_none() {
            return Err(Refusal::NotPdf);
        }
        let startxref = find_startxref(&bytes)?;
        let mut doc = Document {
            bytes,
            entries: BTreeMap::new(),
            trailer: Dict::default(),
            startxref,
            xref_kind: XrefKind::Table,
            size: 0,
            pages: Vec::new(),
            has_signatures: false,
            stream_cache: RefCell::new(BTreeMap::new()),
            scanned: RefCell::new(None),
        };
        doc.read_xref_chain()?;
        if doc.trailer.get("Encrypt").is_some() {
            return Err(Refusal::Encrypted);
        }
        let catalog = match doc.trailer.get("Root") {
            Some(r) => doc.resolve_dict(r)?,
            None => return damaged("the trailer names no catalog"),
        };
        if doc.certified_against_changes(&catalog)? {
            return Err(Refusal::Certified);
        }
        doc.has_signatures = doc.signed_already(&catalog);
        let pages_ref = match catalog.get("Pages") {
            Some(r @ Obj::Ref(..)) => r.clone(),
            _ => return Err(Refusal::NoPages),
        };
        let mut pages = Vec::new();
        let mut seen = BTreeSet::new();
        doc.walk(&pages_ref, &Inherited::default(), &mut pages, &mut seen, 0)?;
        if pages.is_empty() {
            return Err(Refusal::NoPages);
        }
        doc.pages = pages;
        Ok(doc)
    }

    fn read_xref_chain(&mut self) -> Result<(), Refusal> {
        let mut next = Some(self.startxref);
        let mut visited = BTreeSet::new();
        let mut first = true;
        while let Some(at) = next {
            if !visited.insert(at) || visited.len() > MAX_SECTIONS {
                break;
            }
            let (trailer, kind) = self.read_section(at)?;
            if first {
                self.xref_kind = kind;
                self.trailer = trailer.clone();
                first = false;
            }
            // A hybrid file's table section names a stream with the objects
            // kept in object streams; the table's own entries come first.
            if let Some(stm) = trailer.int("XRefStm") {
                let stm = usize::try_from(stm).unwrap_or(usize::MAX);
                if visited.insert(stm) {
                    let _ = self.read_section(stm)?;
                }
            }
            next = match trailer.get("Prev") {
                Some(Obj::Int(p)) if *p >= 0 => Some(*p as usize),
                _ => None,
            };
        }
        let size = self
            .trailer
            .int("Size")
            .unwrap_or(0)
            .clamp(0, i64::from(u32::MAX)) as u32;
        let highest = self.entries.keys().next_back().map(|n| n + 1).unwrap_or(0);
        self.size = size.max(highest);
        Ok(())
    }

    fn insert(&mut self, number: u32, entry: Entry) {
        self.entries.entry(number).or_insert(entry);
    }

    /// Reads one cross-reference section and returns its trailer.
    fn read_section(&mut self, at: usize) -> Result<(Dict, XrefKind), Refusal> {
        if at >= self.bytes.len() {
            return damaged("a cross-reference offset points past the end");
        }
        let bytes = std::mem::take(&mut self.bytes);
        let result = self.read_section_in(&bytes, at);
        self.bytes = bytes;
        result
    }

    fn read_section_in(&mut self, bytes: &[u8], at: usize) -> Result<(Dict, XrefKind), Refusal> {
        let mut lx = Lexer::new(bytes, at);
        if lx.keyword("xref") {
            loop {
                if lx.keyword("trailer") {
                    break;
                }
                let (Some(start), Some(count)) = (lx.uint(), lx.uint()) else {
                    return damaged("a cross-reference table cannot be read");
                };
                for k in 0..count {
                    let (Some(offset), Some(generation)) = (lx.uint(), lx.uint()) else {
                        return damaged("a cross-reference entry cannot be read");
                    };
                    let kind = lx.word();
                    let number = u32::try_from(start + k).unwrap_or(u32::MAX);
                    let entry = match kind {
                        b"n" => Entry::InFile {
                            offset: offset as usize,
                            generation: generation.min(u64::from(u16::MAX)) as u16,
                        },
                        b"f" => Entry::Free,
                        _ => return damaged("a cross-reference entry is neither in use nor free"),
                    };
                    self.insert(number, entry);
                }
            }
            return match lx.object() {
                Ok(Obj::Dict(d)) => Ok((d, XrefKind::Table)),
                _ => damaged("the trailer cannot be read"),
            };
        }
        // A cross-reference stream: an indirect object of /Type /XRef.
        let (_, _, loaded) = parse_indirect(bytes, at, |o| self.length_of(bytes, o))?;
        let Loaded::Stream(s) = loaded else {
            return damaged("the cross-reference offset does not point at a table or stream");
        };
        if s.dict.name("Type") != Some(b"XRef") {
            return damaged("the cross-reference stream has the wrong type");
        }
        let data = decode(&s)?;
        let w: Vec<usize> = match s.dict.get("W").and_then(Obj::as_array) {
            Some(items) if items.len() == 3 => items
                .iter()
                .map(|i| i.as_int().filter(|v| (0..=8).contains(v)).map(|v| v as usize))
                .collect::<Option<_>>()
                .ok_or(Refusal::Damaged(
                    "the cross-reference stream's /W is invalid".into(),
                ))?,
            _ => return damaged("the cross-reference stream has no /W"),
        };
        let size = s.dict.int("Size").unwrap_or(0);
        let index: Vec<i64> = match s.dict.get("Index").and_then(Obj::as_array) {
            Some(items) => items.iter().filter_map(Obj::as_int).collect(),
            None => vec![0, size],
        };
        let width = w[0] + w[1] + w[2];
        if width == 0 {
            return damaged("the cross-reference stream's /W is invalid");
        }
        let mut rows = data.chunks_exact(width);
        for pair in index.chunks(2) {
            let [start, count] = pair else { break };
            for k in 0..*count {
                let Some(row) = rows.next() else {
                    return damaged("the cross-reference stream is shorter than its /Index");
                };
                let field = |from: usize, len: usize| -> u64 {
                    row[from..from + len]
                        .iter()
                        .fold(0u64, |n, b| (n << 8) | u64::from(*b))
                };
                let kind = if w[0] == 0 { 1 } else { field(0, w[0]) };
                let a = field(w[0], w[1]);
                let b = field(w[0] + w[1], w[2]);
                let number = u32::try_from(start + k).unwrap_or(u32::MAX);
                let entry = match kind {
                    0 => Entry::Free,
                    1 => Entry::InFile {
                        offset: a as usize,
                        generation: b.min(u64::from(u16::MAX)) as u16,
                    },
                    2 => Entry::InStream {
                        stream: a.min(u64::from(u32::MAX)) as u32,
                        index: b.min(u64::from(u32::MAX)) as u32,
                    },
                    // Other types are reserved; readers treat them as null.
                    _ => Entry::Free,
                };
                self.insert(number, entry);
            }
        }
        Ok((s.dict, XrefKind::Stream))
    }

    /// The length of a stream whose /Length is an indirect object, while the
    /// cross-reference table is still being read.
    fn length_of(&self, bytes: &[u8], obj: &Obj) -> Option<usize> {
        match obj {
            Obj::Int(n) => usize::try_from(*n).ok(),
            Obj::Ref(n, _) => match self.entries.get(n) {
                Some(Entry::InFile { offset, .. }) => {
                    let (_, _, l) = parse_indirect(bytes, *offset, |_| None).ok()?;
                    match l {
                        Loaded::Plain(Obj::Int(v)) => usize::try_from(v).ok(),
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Loads object `number`, from the file or from its object stream.
    pub fn load(&self, number: u32) -> Result<Loaded, Refusal> {
        match self.entries.get(&number).copied() {
            None | Some(Entry::Free) => Ok(Loaded::Plain(Obj::Null)),
            Some(Entry::InFile { offset, .. }) => {
                let len = |o: &Obj| self.length_of(&self.bytes, o);
                match parse_indirect(&self.bytes, offset, len) {
                    Ok((n, _, l)) if n == number => Ok(l),
                    // The table points at the wrong place: look for the
                    // object by scanning the file, as other readers do.
                    _ => match self.scan_for(number) {
                        Some(at) => parse_indirect(&self.bytes, at, len).map(|(_, _, l)| l),
                        None => damaged(format!("object {number} is not where the file says")),
                    },
                }
            }
            Some(Entry::InStream { stream, index }) => {
                if !self.stream_cache.borrow().contains_key(&stream) {
                    let objs = self.read_object_stream(stream)?;
                    self.stream_cache.borrow_mut().insert(stream, objs);
                }
                let cache = self.stream_cache.borrow();
                let objs = &cache[&stream];
                let found = objs
                    .get(index as usize)
                    .filter(|(n, _)| *n == number)
                    .or_else(|| objs.iter().find(|(n, _)| *n == number));
                match found {
                    Some((_, o)) => Ok(Loaded::Plain(o.clone())),
                    None => damaged(format!("object {number} is missing from its object stream")),
                }
            }
        }
    }

    fn read_object_stream(&self, stream: u32) -> Result<Vec<(u32, Obj)>, Refusal> {
        let s = match self.entries.get(&stream) {
            Some(Entry::InFile { .. }) => match self.load(stream)? {
                Loaded::Stream(s) => s,
                Loaded::Plain(_) => return damaged(format!("object stream {stream} is not a stream")),
            },
            _ => return damaged(format!("object stream {stream} is not in the file")),
        };
        let data = decode(&s)?;
        let n = s.dict.int("N").unwrap_or(0).max(0) as usize;
        let first = s.dict.int("First").unwrap_or(0).max(0) as usize;
        if first > data.len() {
            return damaged(format!("object stream {stream} is shorter than its /First"));
        }
        let mut lx = Lexer::new(&data, 0);
        let mut heads = Vec::new();
        for _ in 0..n {
            match (lx.uint(), lx.uint()) {
                (Some(num), Some(off)) => heads.push((num as u32, off as usize)),
                _ => return damaged(format!("object stream {stream} has a bad header")),
            }
        }
        let mut out = Vec::new();
        for (num, off) in heads {
            let mut ox = Lexer::new(&data, first + off);
            match ox.object() {
                Ok(o) => out.push((num, o)),
                Err(_) => return damaged(format!("object {num} in object stream {stream} cannot be read")),
            }
        }
        Ok(out)
    }

    /// Finds "N G obj" for `number` anywhere in the file; the last one wins,
    /// as the newest definition.
    fn scan_for(&self, number: u32) -> Option<usize> {
        if self.scanned.borrow().is_none() {
            let mut found = BTreeMap::new();
            let b = &self.bytes;
            let mut i = 0;
            while let Some(k) = find(&b[i..], b"obj") {
                let at = i + k;
                i = at + 3;
                // Walk back over "G " and "N ".
                let mut j = at;
                let mut nums = Vec::new();
                for _ in 0..2 {
                    while j > 0 && super::object::is_white(b[j - 1]) {
                        j -= 1;
                    }
                    let end = j;
                    while j > 0 && b[j - 1].is_ascii_digit() {
                        j -= 1;
                    }
                    if j == end {
                        break;
                    }
                    nums.push(&b[j..end]);
                }
                if nums.len() == 2 && (j == 0 || !b[j - 1].is_ascii_alphanumeric()) {
                    let text = std::str::from_utf8(nums[1]).unwrap_or("");
                    if let Ok(n) = text.parse::<u32>() {
                        found.insert(n, j);
                    }
                }
            }
            *self.scanned.borrow_mut() = Some(found);
        }
        self.scanned
            .borrow()
            .as_ref()
            .and_then(|m| m.get(&number).copied())
    }

    /// Follows references until a direct object.
    pub fn resolve(&self, obj: &Obj) -> Result<Obj, Refusal> {
        let mut cur = obj.clone();
        for _ in 0..32 {
            match cur {
                Obj::Ref(n, _) => {
                    cur = match self.load(n)? {
                        Loaded::Plain(o) => o,
                        Loaded::Stream(s) => Obj::Dict(s.dict),
                    }
                }
                other => return Ok(other),
            }
        }
        damaged("a reference loops")
    }

    pub fn resolve_dict(&self, obj: &Obj) -> Result<Dict, Refusal> {
        match self.resolve(obj)? {
            Obj::Dict(d) => Ok(d),
            _ => damaged("an object that should be a dictionary is not"),
        }
    }

    fn walk(
        &self,
        node_ref: &Obj,
        inherited: &Inherited,
        out: &mut Vec<PageInfo>,
        seen: &mut BTreeSet<u32>,
        depth: usize,
    ) -> Result<(), Refusal> {
        let Obj::Ref(number, generation) = *node_ref else {
            return damaged("a page tree node is not an indirect object");
        };
        if depth > 64 || !seen.insert(number) {
            return damaged("the page tree loops");
        }
        if out.len() > MAX_PAGES {
            return Err(Refusal::Unsupported("more pages than the tool reads".into()));
        }
        let node = self.resolve_dict(node_ref)?;
        let mut here = inherited.clone();
        if let Some(m) = node.get("MediaBox") {
            here.media = Some(self.rect(m)?);
        }
        if let Some(c) = node.get("CropBox") {
            here.crop = Some(self.rect(c)?);
        }
        if let Some(r) = node.get("Rotate") {
            here.rotate = self.resolve(r)?.as_int();
        }
        if let Some(r) = node.get("Resources") {
            here.resources = Some(r.clone());
        }
        let is_pages = node.name("Type") == Some(b"Pages")
            || (node.name("Type").is_none() && node.get("Kids").is_some());
        if is_pages {
            let kids = match node.get("Kids") {
                Some(k) => self.resolve(k)?,
                None => return damaged("a page tree node has no /Kids"),
            };
            for kid in kids.as_array().unwrap_or(&[]) {
                self.walk(kid, &here, out, seen, depth + 1)?;
            }
            return Ok(());
        }
        // US Letter when a page names no media box, as readers assume.
        let media = here.media.unwrap_or([0, 0, 612_000, 792_000]);
        let crop = here.crop.unwrap_or(media);
        let rotate = here.rotate.unwrap_or(0).rem_euclid(360);
        let rotate = rotate - rotate % 90;
        let resources = match &here.resources {
            Some(r) => match self.resolve(r)? {
                Obj::Dict(d) => d,
                _ => Dict::default(),
            },
            None => Dict::default(),
        };
        out.push(PageInfo {
            number,
            generation,
            dict: node,
            media,
            crop,
            rotate,
            resources,
        });
        Ok(())
    }

    fn rect(&self, obj: &Obj) -> Result<[i64; 4], Refusal> {
        let arr = self.resolve(obj)?;
        let items = arr.as_array().unwrap_or(&[]);
        if items.len() != 4 {
            return damaged("a page box does not have four numbers");
        }
        let mut v = [0i64; 4];
        for (k, item) in items.iter().enumerate() {
            v[k] = match self.resolve(item)?.as_milli() {
                Some(m) => m,
                None => return damaged("a page box holds something other than a number"),
            };
        }
        Ok([v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])])
    }

    /// A DocMDP certification with /P 1 forbids any change (PS-Q11).
    fn certified_against_changes(&self, catalog: &Dict) -> Result<bool, Refusal> {
        let Some(perms) = catalog.get("Perms") else {
            return Ok(false);
        };
        let Ok(perms) = self.resolve_dict(perms) else {
            return Ok(false);
        };
        let Some(sig) = perms.get("DocMDP") else {
            return Ok(false);
        };
        let Ok(sig) = self.resolve_dict(sig) else {
            return Ok(false);
        };
        let refs = match sig.get("Reference") {
            Some(r) => self.resolve(r)?,
            None => return Ok(false),
        };
        for item in refs.as_array().unwrap_or(&[]) {
            let Ok(r) = self.resolve_dict(item) else { continue };
            if r.name("TransformMethod") != Some(b"DocMDP") {
                continue;
            }
            let p = match r.get("TransformParams") {
                Some(tp) => self.resolve_dict(tp).ok().and_then(|d| d.int("P")).unwrap_or(2),
                None => 2,
            };
            if p == 1 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Bit 1 of the form's /SigFlags: the document holds at least one signed
    /// signature field.
    fn signed_already(&self, catalog: &Dict) -> bool {
        catalog
            .get("AcroForm")
            .and_then(|a| self.resolve_dict(a).ok())
            .and_then(|a| a.get("SigFlags").cloned())
            .and_then(|f| self.resolve(&f).ok())
            .and_then(|f| f.as_int())
            .map(|f| f & 1 == 1)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Default)]
struct Inherited {
    media: Option<[i64; 4]>,
    crop: Option<[i64; 4]>,
    rotate: Option<i64>,
    resources: Option<Obj>,
}

fn decode(s: &Stream) -> Result<Vec<u8>, Refusal> {
    filter::decode(&s.dict, &s.data).map_err(|e| match e {
        FilterError::Unsupported(f) => Refusal::Unsupported(format!("a stream filter ({f})")),
        FilterError::Corrupt => Refusal::Damaged("a compressed stream cannot be read".into()),
    })
}

pub fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).rposition(|w| w == needle)
}

fn find_startxref(bytes: &[u8]) -> Result<usize, Refusal> {
    let tail_from = bytes.len().saturating_sub(4096);
    let Some(k) = rfind(&bytes[tail_from..], b"startxref") else {
        return damaged("the file has no startxref");
    };
    let mut lx = Lexer::new(bytes, tail_from + k + "startxref".len());
    match lx.uint() {
        Some(v) if (v as usize) < bytes.len() => Ok(v as usize),
        _ => damaged("startxref does not point inside the file"),
    }
}

/// Parses "N G obj ... endobj" at `at`. A stream's data is sliced by its
/// /Length (through `length`, which resolves an indirect length); when the
/// length is missing or wrong, the data runs to "endstream".
pub fn parse_indirect(
    bytes: &[u8],
    at: usize,
    length: impl Fn(&Obj) -> Option<usize>,
) -> Result<(u32, u16, Loaded), Refusal> {
    let mut lx = Lexer::new(bytes, at);
    let (Some(number), Some(generation)) = (lx.uint(), lx.uint()) else {
        return damaged("an object header cannot be read");
    };
    if !lx.keyword("obj") {
        return damaged("an object header cannot be read");
    }
    let obj = lx
        .object()
        .map_err(|e| Refusal::Damaged(format!("{} at byte {}", e.what, e.at)))?;
    let number = u32::try_from(number).unwrap_or(u32::MAX);
    let generation = generation.min(u64::from(u16::MAX)) as u16;
    if let (Obj::Dict(dict), true) = (&obj, lx.keyword("stream")) {
        let mut start = lx.pos;
        if bytes.get(start) == Some(&b'\r') {
            start += 1;
        }
        if bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
        let declared = dict.get("Length").and_then(&length);
        let fits = declared.and_then(|len| {
            let end = start.checked_add(len)?;
            if end > bytes.len() {
                return None;
            }
            let mut after = Lexer::new(bytes, end);
            after.keyword("endstream").then_some(end)
        });
        let end = match fits {
            Some(e) => e,
            None => {
                let Some(k) = find(&bytes[start..], b"endstream") else {
                    return damaged(format!("object {number}'s stream has no end"));
                };
                let mut e = start + k;
                if e > start && bytes[e - 1] == b'\n' {
                    e -= 1;
                }
                if e > start && bytes[e - 1] == b'\r' {
                    e -= 1;
                }
                e
            }
        };
        let data = bytes[start..end].to_vec();
        return Ok((
            number,
            generation,
            Loaded::Stream(Stream {
                dict: dict.clone(),
                data,
            }),
        ));
    }
    Ok((number, generation, Loaded::Plain(obj)))
}

//! PDFs built byte by byte, for tests and for the browser check's sample.
//! Each fixture is small enough to read by eye, so a test's expected value
//! can be worked out from the fixture's text.

use super::filter::deflate;

/// One object's number and the text between "N 0 obj" and "endobj".
pub type Body = (u32, Vec<u8>);

/// How the fixture records where objects are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// A classic table.
    Table,
    /// A cross-reference stream (PNG Up predictor), with every non-stream
    /// object except the catalog moved into one compressed object stream.
    Compressed,
}

/// Writes `bodies` as a PDF whose catalog is object `root`.
pub fn build(bodies: &[Body], root: u32, layout: Layout, trailer_extra: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let size = bodies.iter().map(|(n, _)| n + 1).max().unwrap_or(1);
    match layout {
        Layout::Table => {
            let mut offsets = vec![None; size as usize];
            for (n, body) in bodies {
                offsets[*n as usize] = Some(out.len());
                write_obj(&mut out, *n, body);
            }
            let xref = out.len();
            out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f\r\n").as_bytes());
            for off in offsets.iter().skip(1) {
                match off {
                    Some(o) => out.extend_from_slice(format!("{o:010} 00000 n\r\n").as_bytes()),
                    None => out.extend_from_slice(b"0000000000 00000 f\r\n"),
                }
            }
            out.extend_from_slice(
                format!(
                    "trailer\n<</Size {size}/Root {root} 0 R{trailer_extra}>>\nstartxref\n{xref}\n%%EOF\n"
                )
                .as_bytes(),
            );
        }
        Layout::Compressed => {
            let objstm = size;
            let xref_no = size + 1;
            let total = size + 2;
            // Entries: (type, field2, field3) per object number.
            let mut rows = vec![(0u8, 0u32, 65535u16); total as usize];
            let mut packed: Vec<&Body> = Vec::new();
            for b in bodies {
                let is_stream = b.1.windows(6).any(|w| w == b"stream");
                if b.0 == root || is_stream {
                    rows[b.0 as usize] = (1, out.len() as u32, 0);
                    write_obj(&mut out, b.0, &b.1);
                } else {
                    packed.push(b);
                }
            }
            let mut head = String::new();
            let mut body = Vec::new();
            for (k, (n, text)) in packed.iter().enumerate() {
                head.push_str(&format!("{n} {} ", body.len()));
                body.extend_from_slice(text);
                body.push(b'\n');
                rows[*n as usize] = (2, objstm, k as u16);
            }
            let mut plain = head.clone().into_bytes();
            plain.extend_from_slice(&body);
            let data = deflate(&plain);
            rows[objstm as usize] = (1, out.len() as u32, 0);
            let mut stm = format!(
                "<</Type/ObjStm/N {}/First {}/Filter/FlateDecode/Length {}>>stream\n",
                packed.len(),
                head.len(),
                data.len()
            )
            .into_bytes();
            stm.extend_from_slice(&data);
            stm.extend_from_slice(b"\nendstream");
            write_obj(&mut out, objstm, &stm);
            let at = out.len();
            rows[xref_no as usize] = (1, at as u32, 0);
            // Rows of 1 + 4 + 2 bytes, each preceded by PNG filter 2 (Up).
            let mut raw = Vec::new();
            let mut prev = [0u8; 7];
            for (t, a, b) in &rows {
                let mut row = [0u8; 7];
                row[0] = *t;
                row[1..5].copy_from_slice(&a.to_be_bytes());
                row[5..7].copy_from_slice(&b.to_be_bytes());
                raw.push(2);
                for i in 0..7 {
                    raw.push(row[i].wrapping_sub(prev[i]));
                }
                prev = row;
            }
            let data = deflate(&raw);
            let mut x = format!(
                "<</Type/XRef/Size {total}/W[1 4 2]/Root {root} 0 R{trailer_extra}/Filter/FlateDecode\
                 /DecodeParms<</Predictor 12/Columns 7>>/Length {}>>stream\n",
                data.len()
            )
            .into_bytes();
            x.extend_from_slice(&data);
            x.extend_from_slice(b"\nendstream");
            write_obj(&mut out, xref_no, &x);
            out.extend_from_slice(format!("startxref\n{at}\n%%EOF\n").as_bytes());
        }
    }
    out
}

fn write_obj(out: &mut Vec<u8>, n: u32, body: &[u8]) {
    out.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
}

fn stream_body(dict_extra: &str, content: &[u8]) -> Vec<u8> {
    let mut b = format!("<<{dict_extra}/Length {}>>stream\n", content.len()).into_bytes();
    b.extend_from_slice(content);
    b.extend_from_slice(b"\nendstream");
    b
}

/// What each page of `pages` looks like.
#[derive(Debug, Clone)]
pub struct PageSpec {
    pub media: &'static str,
    pub crop: Option<&'static str>,
    pub rotate: i64,
    pub label: &'static str,
}

impl PageSpec {
    pub fn letter(label: &'static str) -> PageSpec {
        PageSpec {
            media: "[0 0 612 792]",
            crop: None,
            rotate: 0,
            label,
        }
    }
}

/// A document of simple pages: a heading in Helvetica and a signature line,
/// so a reader that renders it shows something to sign. Objects: 1 catalog,
/// 2 page tree, 3 font, then for page k: 4 + 2k page, 5 + 2k contents.
pub fn pages(specs: &[PageSpec], layout: Layout) -> Vec<u8> {
    let mut bodies: Vec<Body> = Vec::new();
    let kids: Vec<String> = (0..specs.len()).map(|k| format!("{} 0 R", 4 + 2 * k)).collect();
    bodies.push((1, b"<</Type/Catalog/Pages 2 0 R>>".to_vec()));
    bodies.push((
        2,
        format!(
            "<</Type/Pages/Kids[{}]/Count {}/Resources<</Font<</F1 3 0 R>>>>>>",
            kids.join(" "),
            specs.len()
        )
        .into_bytes(),
    ));
    bodies.push((3, b"<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>".to_vec()));
    for (k, s) in specs.iter().enumerate() {
        let page = 4 + 2 * k as u32;
        let crop = s.crop.map(|c| format!("/CropBox{c}")).unwrap_or_default();
        let rotate = if s.rotate != 0 {
            format!("/Rotate {}", s.rotate)
        } else {
            String::new()
        };
        bodies.push((
            page,
            format!(
                "<</Type/Page/Parent 2 0 R/MediaBox{}{crop}{rotate}/Contents {} 0 R>>",
                s.media,
                page + 1
            )
            .into_bytes(),
        ));
        let content = format!(
            "BT /F1 18 Tf 72 700 Td ({}) Tj ET\nBT /F1 10 Tf 72 152 Td (Signature) Tj ET\n0.5 w 72 165 m 300 165 l S\n",
            s.label
        );
        bodies.push((page + 1, stream_body("", content.as_bytes())));
    }
    build(&bodies, 1, layout, "")
}

/// The sample the browser check opens: two Letter pages.
pub fn sample() -> Vec<u8> {
    pages(
        &[
            PageSpec::letter("Sample agreement, page 1"),
            PageSpec::letter("Sample agreement, page 2"),
        ],
        Layout::Table,
    )
}

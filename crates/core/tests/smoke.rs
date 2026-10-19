//! The smoke run on real PDFs (PS-9), outside the everyday suite. It signs
//! every PDF in `SMOKE_IN` on its first and last page and writes the copies
//! to `SMOKE_OUT`; `scripts/smoke.py` then has Poppler, a second reader,
//! render each copy. The person's files never enter the repository.
//!
//!     SMOKE_IN=~/Downloads SMOKE_OUT=/tmp/out cargo test --test smoke -- --ignored --nocapture

use std::fs;
use std::path::PathBuf;

use tool_core::ink::{Pad, PadPoint};
use tool_core::pdf::doc::Document;
use tool_core::pdf::fixture::{self, Layout, PageSpec};
use tool_core::pdf::write::{amend, Art, Stamp};
use tool_core::place::{at_tap, cm, Geometry};

fn drawing() -> (Art, (i64, i64)) {
    let mut pad = Pad::default();
    let pt = |id: &str, x: i64, y: i64| PadPoint {
        id: id.into(),
        kind: "mouse".into(),
        x: x * 1000,
        y: y * 1000,
        w: 300_000,
        h: 100_000,
    };
    pad.down(&pt("1", 20, 70));
    for (x, y) in [
        (40, 20),
        (60, 70),
        (90, 40),
        (120, 70),
        (150, 30),
        (180, 70),
        (260, 50),
    ] {
        pad.moved(&pt("1", x, y));
    }
    pad.up(&pt("1", 260, 50));
    let d = pad.drawing().unwrap();
    (
        Art::Form {
            width: d.width,
            height: d.height,
            // Bright red, so the change shows on a dark page too; the run
            // checks where the signature lands, not its colour.
            content: String::from_utf8(d.pdf_content())
                .unwrap()
                .replace(d.ink.pdf(), "1 0 0")
                .into_bytes(),
        },
        (d.width, d.height),
    )
}

#[test]
#[ignore]
fn sign_every_pdf_in_a_folder() {
    let input = PathBuf::from(std::env::var("SMOKE_IN").expect("set SMOKE_IN"));
    let output = PathBuf::from(std::env::var("SMOKE_OUT").expect("set SMOKE_OUT"));
    fs::create_dir_all(&output).unwrap();
    // Rotated and cropped pages, which real files in a folder may lack.
    for (name, rotate, crop) in [
        ("fixture-rotate-0.pdf", 0, None),
        ("fixture-rotate-90.pdf", 90, None),
        ("fixture-rotate-180.pdf", 180, None),
        ("fixture-rotate-270.pdf", 270, None),
        ("fixture-crop-rotate-90.pdf", 90, Some("[50 100 500 700]")),
    ] {
        let spec = PageSpec {
            media: "[0 0 612 792]",
            crop,
            rotate,
            label: "Fixture",
        };
        fs::write(input.join(name), fixture::pages(&[spec], Layout::Table)).unwrap();
    }
    let (art, (aw, ah)) = drawing();
    let mut names: Vec<PathBuf> = fs::read_dir(&input)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension()
                .map(|x| x.eq_ignore_ascii_case("pdf"))
                .unwrap_or(false)
        })
        .collect();
    names.sort();
    for path in names {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = fs::read(&path).unwrap();
        let doc = match Document::open(bytes) {
            Ok(d) => d,
            Err(e) => {
                println!("SMOKE|{name}|refused|{e:?}");
                continue;
            }
        };
        let last = doc.pages.len() - 1;
        let mut stamps = Vec::new();
        for k in [0, last] {
            let p = &doc.pages[k];
            let g = Geometry {
                view: p.view(),
                rotate: p.rotate,
            };
            let (dw, dh) = g.display();
            // Centred left to right, bottom edge 80% of the way down.
            let r = at_tap(dw / 2, dh * 4 / 5, 180_000, aw, ah, dw, dh);
            stamps.push(Stamp {
                page: k,
                art: 0,
                cm: cm(&g, &r, aw, ah),
            });
        }
        match amend(&doc, std::slice::from_ref(&art), &stamps) {
            Ok(out) => {
                fs::write(output.join(&name), &out).unwrap();
                println!(
                    "SMOKE|{name}|signed|{} pages|{:?}|rotate {}|{} bytes added",
                    doc.pages.len(),
                    doc.xref_kind,
                    doc.pages[0].rotate,
                    out.len() - doc.bytes.len()
                );
            }
            Err(e) => println!("SMOKE|{name}|failed|{e:?}"),
        }
    }
}

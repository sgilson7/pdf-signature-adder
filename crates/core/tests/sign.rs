//! PS-1 and PS-6, end to end: a signed copy starts with the original's bytes,
//! reads back with the same reader, and draws each signature on its page.
//! Fixtures are built byte by byte by `pdf::fixture`; real files are the
//! smoke script's and the person's (PS-9).

use std::fs;
use std::path::Path;

use tool_core::pdf::doc::{Document, Loaded, XrefKind};
use tool_core::pdf::fixture::{self, Layout, PageSpec};
use tool_core::pdf::object::Obj;
use tool_core::pdf::write::{amend, Art, Stamp};

fn form() -> Art {
    Art::Form {
        width: 300,
        height: 100,
        content: b"1 J 1 j 30 w 0 0 0 RG\n10 50 m 290 50 l S\n".to_vec(),
    }
}

fn image() -> Art {
    Art::Image {
        width: 2,
        height: 1,
        rgb: vec![0, 0, 0, 255, 255, 255],
        alpha: vec![255, 0],
    }
}

fn stamp(page: usize, art: usize) -> Stamp {
    Stamp {
        page,
        art,
        cm: "0.6 0 0 0.6 100 160".into(),
    }
}

/// The drawing stream that ends page `k`'s contents.
fn last_stream(doc: &Document, k: usize) -> String {
    let Some(Obj::Array(c)) = doc.pages[k].dict.get("Contents") else {
        panic!("page {k} has no contents array");
    };
    let Some(Obj::Ref(n, _)) = c.last() else {
        panic!("no last ref")
    };
    match doc.load(*n).unwrap() {
        Loaded::Stream(s) => String::from_utf8(s.data).unwrap(),
        Loaded::Plain(_) => panic!("not a stream"),
    }
}

#[test]
fn a_signed_copy_starts_with_the_original_byte_for_byte() {
    for layout in [Layout::Table, Layout::Compressed] {
        let original = fixture::pages(&[PageSpec::letter("A"), PageSpec::letter("B")], layout);
        let doc = Document::open(original.clone()).unwrap();
        let out = amend(&doc, &[form()], &[stamp(1, 0)]).unwrap();
        assert!(out.starts_with(&original), "{layout:?}");
        assert!(out.ends_with(b"%%EOF\n"));
    }
}

#[test]
fn the_update_uses_the_same_kind_of_cross_reference_as_the_original() {
    for (layout, kind) in [
        (Layout::Table, XrefKind::Table),
        (Layout::Compressed, XrefKind::Stream),
    ] {
        let original = fixture::pages(&[PageSpec::letter("A")], layout);
        let doc = Document::open(original.clone()).unwrap();
        let out = amend(&doc, &[form()], &[stamp(0, 0)]).unwrap();
        let again = Document::open(out.clone()).unwrap();
        assert_eq!(again.xref_kind, kind);
        assert!(again.startxref > original.len());
        let tail = String::from_utf8_lossy(&out[original.len()..]).into_owned();
        assert_eq!(tail.contains("/Type /XRef"), kind == XrefKind::Stream);
        assert!(tail.contains(&format!("/Prev {}", doc.startxref)));
    }
}

#[test]
fn the_signed_page_wraps_the_old_contents_and_then_draws() {
    let original = fixture::sample();
    let doc = Document::open(original).unwrap();
    let out = amend(&doc, &[form()], &[stamp(0, 0)]).unwrap();
    let again = Document::open(out).unwrap();
    let Some(Obj::Array(c)) = again.pages[0].dict.get("Contents") else {
        panic!()
    };
    // [q stream, the original contents (object 5), Q-and-draw stream].
    assert_eq!(c.len(), 3);
    assert_eq!(c[1], Obj::Ref(5, 0));
    assert_eq!(
        last_stream(&again, 0),
        "Q\nq 0.6 0 0 0.6 100 160 cm /PSAsig1 Do Q\n"
    );
    // The page's inherited font is still there beside the new XObject.
    assert!(again.pages[0].resources.get("Font").is_some());
    // The other page is untouched.
    assert_eq!(again.pages[1].dict, doc.pages[1].dict);
}

#[test]
fn one_art_placed_twice_is_written_once() {
    let doc = Document::open(fixture::sample()).unwrap();
    let out = amend(&doc, &[form()], &[stamp(0, 0), stamp(1, 0)]).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert_eq!(text.matches("/Subtype /Form").count(), 1);
}

#[test]
fn a_picture_is_an_image_with_a_soft_mask() {
    let doc = Document::open(fixture::sample()).unwrap();
    let out = amend(&doc, &[form(), image()], &[stamp(0, 0), stamp(0, 1)]).unwrap();
    let again = Document::open(out.clone()).unwrap();
    let drawn = last_stream(&again, 0);
    assert!(drawn.contains("/PSAsig1 Do") && drawn.contains("/PSAsig2 Do"));
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("/ColorSpace /DeviceRGB"));
    assert!(text.contains("/ColorSpace /DeviceGray"));
    assert!(text.contains("/SMask"));
}

#[test]
fn a_second_signing_builds_on_the_first() {
    let doc = Document::open(fixture::sample()).unwrap();
    let first = amend(&doc, &[form()], &[stamp(0, 0)]).unwrap();
    let doc2 = Document::open(first.clone()).unwrap();
    let second = amend(&doc2, &[image()], &[stamp(0, 0)]).unwrap();
    assert!(second.starts_with(&first));
    let again = Document::open(second).unwrap();
    let Some(Obj::Array(c)) = again.pages[0].dict.get("Contents") else {
        panic!()
    };
    // [q2, q1, original, Q1-draw, Q2-draw].
    assert_eq!(c.len(), 5);
    // The first signature is still named in the page's resources.
    let xo = again
        .resolve_dict(again.pages[0].resources.get("XObject").unwrap())
        .unwrap();
    assert!(xo.get("PSAsig1").is_some());
    assert!(xo.get("PSAsig1x").is_some());
}

#[test]
fn indirect_contents_arrays_and_resources_are_followed() {
    let bodies = vec![
        (1, b"<</Type/Catalog/Pages 2 0 R>>".to_vec()),
        (2, b"<</Type/Pages/Kids[3 0 R]/Count 1>>".to_vec()),
        (
            3,
            b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 100]/Contents 4 0 R/Resources 7 0 R>>".to_vec(),
        ),
        (4, b"[5 0 R 6 0 R]".to_vec()),
        (5, b"<</Length 2>>stream\nq\n\nendstream".to_vec()),
        (6, b"<</Length 2>>stream\nQ\n\nendstream".to_vec()),
        (7, b"<</XObject 8 0 R>>".to_vec()),
        (8, b"<</PSAsig1 9 0 R>>".to_vec()),
        (
            9,
            b"<</Type/XObject/Subtype/Form/BBox[0 0 1 1]/Length 0>>stream\n\nendstream".to_vec(),
        ),
    ];
    let original = fixture::build(&bodies, 1, Layout::Table, "");
    let doc = Document::open(original).unwrap();
    let out = amend(&doc, &[form()], &[stamp(0, 0)]).unwrap();
    let again = Document::open(out).unwrap();
    let Some(Obj::Array(c)) = again.pages[0].dict.get("Contents") else {
        panic!()
    };
    assert_eq!(&c[1..3], &[Obj::Ref(5, 0), Obj::Ref(6, 0)]);
    // The name already used by the page is kept, and the new one differs.
    let xo = again
        .resolve_dict(again.pages[0].resources.get("XObject").unwrap())
        .unwrap();
    assert_eq!(xo.get("PSAsig1"), Some(&Obj::Ref(9, 0)));
    assert!(last_stream(&again, 0).contains("/PSAsig1x Do"));
}

#[test]
fn nothing_placed_writes_nothing() {
    let doc = Document::open(fixture::sample()).unwrap();
    assert!(amend(&doc, &[form()], &[]).is_err());
}

/// The browser check opens this file. It is written by the fixture code, so
/// a change to the fixture shows up here first.
#[test]
fn the_browser_sample_matches_the_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/browser/sample.pdf");
    let want = fixture::sample();
    match fs::read(&path) {
        Ok(have) => assert!(
            have == want,
            "tests/browser/sample.pdf is stale; delete it and rerun"
        ),
        Err(_) => fs::write(&path, &want).expect("write sample.pdf"),
    }
}

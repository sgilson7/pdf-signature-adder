//! Reading and amending PDF files. `doc` reads a file far enough to amend
//! it; `write` appends the signatures as an incremental update (PS-1) and
//! reads the result back (PS-6).

pub mod doc;
pub mod filter;
pub mod fixture;
pub mod object;
pub mod write;

#[cfg(test)]
mod tests {
    use super::doc::{Document, Refusal, XrefKind};
    use super::fixture::{self, Layout, PageSpec};

    fn open(bytes: Vec<u8>) -> Document {
        Document::open(bytes).unwrap()
    }

    #[test]
    fn a_classic_table_file_reads_its_pages() {
        let d = open(fixture::sample());
        assert_eq!(d.pages.len(), 2);
        assert_eq!(d.xref_kind, XrefKind::Table);
        assert_eq!(d.pages[0].number, 4);
        assert_eq!(d.pages[1].number, 6);
        assert_eq!(d.pages[0].media, [0, 0, 612_000, 792_000]);
        assert_eq!(d.size, 8);
    }

    #[test]
    fn a_compressed_file_reads_pages_from_its_object_stream() {
        let bytes = fixture::pages(
            &[PageSpec::letter("A"), PageSpec::letter("B")],
            Layout::Compressed,
        );
        let d = open(bytes);
        assert_eq!(d.xref_kind, XrefKind::Stream);
        assert_eq!(d.pages.len(), 2);
        // Resources are inherited from the page tree (object 2) and reach
        // the page resolved.
        assert!(d.pages[1].resources.get("Font").is_some());
        assert_eq!(d.pages[1].media, [0, 0, 612_000, 792_000]);
    }

    #[test]
    fn rotation_and_crop_are_inherited_and_normalised() {
        let specs = [PageSpec {
            media: "[0 0 612 792]",
            crop: Some("[36 36.5 576 756]"),
            rotate: -90,
            label: "R",
        }];
        let d = open(fixture::pages(&specs, Layout::Table));
        assert_eq!(d.pages[0].rotate, 270);
        assert_eq!(d.pages[0].crop, [36_000, 36_500, 576_000, 756_000]);
        assert_eq!(d.pages[0].view(), [36_000, 36_500, 576_000, 756_000]);
    }

    #[test]
    fn a_crop_box_outside_the_media_box_is_clipped() {
        let specs = [PageSpec {
            media: "[0 0 612 792]",
            crop: Some("[-10 0 700 792]"),
            rotate: 0,
            label: "C",
        }];
        let d = open(fixture::pages(&specs, Layout::Table));
        assert_eq!(d.pages[0].view(), [0, 0, 612_000, 792_000]);
    }

    #[test]
    fn an_encrypted_file_is_refused() {
        let bodies = vec![
            (1, b"<</Type/Catalog/Pages 2 0 R>>".to_vec()),
            (2, b"<</Type/Pages/Kids[3 0 R]/Count 1>>".to_vec()),
            (3, b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 10 10]>>".to_vec()),
            (4, b"<</Filter/Standard/V 2/R 3>>".to_vec()),
        ];
        let bytes = fixture::build(&bodies, 1, Layout::Table, "/Encrypt 4 0 R");
        assert_eq!(Document::open(bytes).unwrap_err(), Refusal::Encrypted);
    }

    #[test]
    fn a_file_certified_against_changes_is_refused_and_p2_is_not() {
        let make =
            |p: i64| {
                let bodies =
                    vec![
                (1, b"<</Type/Catalog/Pages 2 0 R/Perms<</DocMDP 4 0 R>>>>".to_vec()),
                (2, b"<</Type/Pages/Kids[3 0 R]/Count 1>>".to_vec()),
                (3, b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 10 10]>>".to_vec()),
                (
                    4,
                    format!("<</Type/Sig/Reference[<</TransformMethod/DocMDP/TransformParams<</P {p}>>>>]>>")
                        .into_bytes(),
                ),
            ];
                fixture::build(&bodies, 1, Layout::Table, "")
            };
        assert_eq!(Document::open(make(1)).unwrap_err(), Refusal::Certified);
        assert!(Document::open(make(2)).is_ok());
    }

    #[test]
    fn a_signed_form_is_noted() {
        let bodies = vec![
            (
                1,
                b"<</Type/Catalog/Pages 2 0 R/AcroForm<</Fields[]/SigFlags 3>>>>".to_vec(),
            ),
            (2, b"<</Type/Pages/Kids[3 0 R]/Count 1>>".to_vec()),
            (3, b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 10 10]>>".to_vec()),
        ];
        let d = open(fixture::build(&bodies, 1, Layout::Table, ""));
        assert!(d.has_signatures);
        assert!(!open(fixture::sample()).has_signatures);
    }

    #[test]
    fn not_a_pdf_and_damaged_files_are_refused_by_name() {
        assert_eq!(Document::open(b"hello".to_vec()).unwrap_err(), Refusal::NotPdf);
        let mut cut = fixture::sample();
        cut.truncate(cut.len() - 40);
        assert!(matches!(Document::open(cut).unwrap_err(), Refusal::Damaged(_)));
    }

    #[test]
    fn a_page_tree_that_loops_is_refused() {
        let bodies = vec![
            (1, b"<</Type/Catalog/Pages 2 0 R>>".to_vec()),
            (2, b"<</Type/Pages/Kids[2 0 R]/Count 1>>".to_vec()),
        ];
        let e = Document::open(fixture::build(&bodies, 1, Layout::Table, "")).unwrap_err();
        assert_eq!(e, Refusal::Damaged("the page tree loops".into()));
    }

    #[test]
    fn a_wrong_offset_is_recovered_by_scanning() {
        // Shift every object by inserting a comment after the header; the
        // table's offsets are now 10 bytes short, and startxref is fixed up.
        let good = fixture::sample();
        let at = good.windows(5).position(|w| w == b"1 0 o").unwrap();
        let mut bad = good[..at].to_vec();
        bad.extend_from_slice(b"%padding\n\n");
        bad.extend_from_slice(&good[at..]);
        let sx = bad.windows(9).rposition(|w| w == b"startxref").unwrap();
        let old: usize = std::str::from_utf8(&bad[sx + 10..])
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let mut fixed = bad[..sx].to_vec();
        fixed.extend_from_slice(format!("startxref\n{}\n%%EOF\n", old + 10).as_bytes());
        let d = open(fixed);
        assert_eq!(d.pages.len(), 2);
    }

    #[test]
    fn an_update_chain_takes_the_newest_version_of_an_object() {
        // Append an update that rotates page 1 (object 4) by 90 degrees.
        let mut bytes = fixture::sample();
        let d = open(bytes.clone());
        let prev = d.startxref;
        let at = bytes.len();
        bytes.extend_from_slice(
            b"4 0 obj\n<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Rotate 90/Contents 5 0 R>>\nendobj\n",
        );
        let x = bytes.len();
        bytes.extend_from_slice(
            format!("xref\n4 1\n{at:010} 00000 n\r\ntrailer\n<</Size 8/Root 1 0 R/Prev {prev}>>\nstartxref\n{x}\n%%EOF\n").as_bytes(),
        );
        let d = open(bytes);
        assert_eq!(d.pages[0].rotate, 90);
        assert_eq!(d.pages[1].rotate, 0);
    }
}

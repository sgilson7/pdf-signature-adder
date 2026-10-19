//! The reader-visible strings. They live in `data/strings.json`, not in
//! source; the core checks the file names every key it uses, and fills each
//! `{placeholder}` from its rules (TONE.md rule 4).

use std::collections::BTreeMap;

/// Every key the core reads. A missing or empty one is refused at start, so
/// the page never shows a blank where a label belongs.
pub const KEYS: &[&str] = &[
    "title",
    "tagline",
    "start_heading",
    "start_text",
    "privacy_note",
    "drop_hint",
    "open_pdf",
    "add_signature",
    "save_pdf",
    "zoom_out",
    "zoom_fit",
    "zoom_in",
    "prev_page",
    "next_page",
    "page_of",
    "page_label",
    "page_signed",
    "pages_one",
    "pages_many",
    "sigs_none",
    "sigs_one",
    "sigs_many",
    "file_detail",
    "status_ready",
    "status_placing",
    "status_selected",
    "cancel_placing",
    "remove",
    "done",
    "sheet_title",
    "close",
    "tab_draw",
    "tab_paste",
    "tab_open",
    "pad_label",
    "pad_hint",
    "ink_label",
    "ink_black",
    "ink_blue",
    "pen_label",
    "weight_fine",
    "weight_medium",
    "weight_bold",
    "undo",
    "clear",
    "save_image",
    "signature_file",
    "use_signature",
    "paste_hint",
    "paste_button",
    "open_image_hint",
    "open_image_button",
    "clear_paper",
    "trim",
    "picture_from",
    "picture_empty",
    "saved_title",
    "saved_detail",
    "saved_note",
    "download",
    "share",
    "keep_editing",
    "already_signed",
    "size_kb",
    "size_mb",
    "refuse_not_pdf",
    "refuse_damaged",
    "refuse_encrypted",
    "refuse_certified",
    "refuse_no_pages",
    "refuse_unsupported",
    "picture_blank",
    "picture_unreadable",
    "picture_too_large",
    "clipboard_empty",
    "clipboard_refused",
    "nothing_placed",
    "save_failed",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strings(BTreeMap<String, String>);

impl Strings {
    /// Parses and checks the strings file. Keys starting with `_` are notes
    /// for people and are ignored.
    pub fn parse(json: &str) -> Result<Strings, String> {
        let map: BTreeMap<String, String> =
            serde_json::from_str(json).map_err(|e| format!("The strings file could not be read: {e}"))?;
        for key in KEYS {
            match map.get(*key) {
                Some(v) if !v.trim().is_empty() => {}
                Some(_) => return Err(format!("The strings file has an empty \"{key}\".")),
                None => return Err(format!("The strings file has no \"{key}\".")),
            }
        }
        if let Some(extra) = map
            .keys()
            .find(|k| !k.starts_with('_') && !KEYS.contains(&k.as_str()))
        {
            return Err(format!("The strings file has \"{extra}\", which nothing uses."));
        }
        Ok(Strings(map))
    }

    /// The string under `key`. Keys are checked at parse, so a key not in
    /// `KEYS` is a bug in the core, and shows as the key itself.
    pub fn get(&self, key: &str) -> String {
        self.0.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    /// The string under `key` with each `{name}` replaced.
    pub fn fill(&self, key: &str, values: &[(&str, String)]) -> String {
        let mut s = self.get(key);
        for (name, value) in values {
            s = s.replace(&format!("{{{name}}}"), value);
        }
        s
    }

    /// "1 page" or "{n} pages", by the count.
    pub fn count(&self, one: &str, many: &str, n: usize) -> String {
        if n == 1 {
            self.get(one)
        } else {
            self.fill(many, &[("n", n.to_string())])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> BTreeMap<String, String> {
        KEYS.iter().map(|k| (k.to_string(), format!("<{k}>"))).collect()
    }

    #[test]
    fn the_shipped_strings_file_names_every_key() {
        let shipped = include_str!("../../../data/strings.json");
        assert!(
            Strings::parse(shipped).is_ok(),
            "{:?}",
            Strings::parse(shipped).err()
        );
    }

    #[test]
    fn a_missing_or_empty_or_unused_key_is_refused_by_name() {
        let mut m = full();
        m.remove("save_pdf");
        let e = Strings::parse(&serde_json::to_string(&m).unwrap()).unwrap_err();
        assert_eq!(e, "The strings file has no \"save_pdf\".");
        let mut m = full();
        m.insert("undo".into(), " ".into());
        let e = Strings::parse(&serde_json::to_string(&m).unwrap()).unwrap_err();
        assert_eq!(e, "The strings file has an empty \"undo\".");
        let mut m = full();
        m.insert("extra".into(), "x".into());
        m.insert("_note".into(), "for people".into());
        let e = Strings::parse(&serde_json::to_string(&m).unwrap()).unwrap_err();
        assert_eq!(e, "The strings file has \"extra\", which nothing uses.");
    }

    #[test]
    fn placeholders_and_counts_fill() {
        let mut m = full();
        m.insert("page_of".into(), "Page {n} of {count}".into());
        m.insert("pages_one".into(), "1 page".into());
        m.insert("pages_many".into(), "{n} pages".into());
        let s = Strings::parse(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(
            s.fill("page_of", &[("n", "2".into()), ("count", "3".into())]),
            "Page 2 of 3"
        );
        assert_eq!(s.count("pages_one", "pages_many", 1), "1 page");
        assert_eq!(s.count("pages_one", "pages_many", 4), "4 pages");
    }
}

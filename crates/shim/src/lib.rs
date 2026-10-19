//! tool_shim: forwards calls between the page and `tool_core::api::Handle`.
//!
//! Builder requirement: the shim decides nothing. Each method is one call
//! into the core. `crates/core/tests/boundary.rs` fails if a branch, a loop or
//! the `?` operator appears here; if a feature seems to need one, the rule
//! belongs in the core.
//!
//! The template forwards free functions and the page holds the state as JSON.
//! This tool holds a whole PDF, so the shim holds the core's handle and the
//! PDF stays in the core (BUILDER-SPEC.md change table).

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Tool(tool_core::api::Handle);

#[wasm_bindgen]
impl Tool {
    #[wasm_bindgen(constructor)]
    pub fn new(strings_json: &str, glossary_md: &str) -> Tool {
        Tool(tool_core::api::Handle::new(strings_json, glossary_md))
    }

    pub fn ui(&self) -> String {
        self.0.ui()
    }

    pub fn act(&mut self, name: &str, value: &str) -> String {
        self.0.act(name, value)
    }

    pub fn open_pdf(&mut self, name: &str, bytes: &[u8]) -> String {
        self.0.open_pdf(name, bytes)
    }

    pub fn picture_size(&self, natural_w: &str, natural_h: &str, is_vector: bool) -> String {
        self.0.picture_size(natural_w, natural_h, is_vector)
    }

    pub fn picture(&mut self, name: &str, width: u32, height: u32, rgba: &[u8]) -> String {
        self.0.picture(name, width, height, rgba)
    }

    pub fn picture_rgba(&self) -> Vec<u8> {
        self.0.picture_rgba()
    }

    pub fn art_rgba(&self, id: u32) -> Vec<u8> {
        self.0.art_rgba(id as usize)
    }

    pub fn drawing_file(&self) -> String {
        self.0.drawing_file()
    }

    pub fn save(&mut self) -> String {
        self.0.save()
    }

    pub fn saved_bytes(&self) -> Vec<u8> {
        self.0.saved_bytes()
    }
}

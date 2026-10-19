//! tool_shim: forwards strings between the page and `tool_core::api`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start(settings_json: &str) -> String {
    tool_core::api::start(settings_json)
}

#[wasm_bindgen]
pub fn act(settings_json: &str, state_json: &str, action: &str) -> String {
    tool_core::api::act(settings_json, state_json, action)
}

//! tool_core: every rule of PDF Signature Adder.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `api.rs` is the one door the shim calls through. `pdf/` reads a PDF and
//! appends signatures to it; `place.rs` decides where they sit; `ink.rs`
//! and `picture.rs` make them; `strings.rs` holds the words' keys.

pub mod api;
pub mod glossary;
pub mod ink;
pub mod num;
pub mod pdf;
pub mod picture;
pub mod place;
pub mod strings;

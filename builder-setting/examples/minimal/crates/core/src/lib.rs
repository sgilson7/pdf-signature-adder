//! tool_core for the minimal example: a counter that stays between zero and a
//! maximum set in `data/counter.json`.
//!
//! Builder requirement: this crate performs no input or output.
//! `tests/boundary.rs` enforces that.

pub mod api;
pub mod counter;
pub mod glossary;

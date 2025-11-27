//! Blueprint Types - Core type definitions for the visual scripting system
//!
//! This crate contains the pure data structures used by the blueprint system.
//! It is designed to compile to WASM for use in the Electron app.
//!
//! ## Features
//!
//! - `wasm` - Enable WASM bindings via wasm-bindgen

mod behaviours;
mod functions;
mod structs;
mod types;

#[cfg(feature = "wasm")]
mod wasm;

pub use behaviours::*;
pub use functions::*;
pub use structs::*;
pub use types::*;

//! Blueprint Types - Core type definitions for the visual scripting system
//!
//! This crate contains the pure data structures used by the blueprint system.
//! It is designed to compile to WASM for use in the Electron app.
//!
//! ## Features
//!
//! - `wasm` - Enable WASM bindings via wasm-bindgen

mod behaviours;
mod exposed;
mod functions;
mod structs;
mod type_registry;
mod types;
mod value;

#[cfg(feature = "wasm")]
mod wasm;

pub use behaviours::*;
pub use exposed::*;
pub use functions::*;
pub use structs::*;
pub use type_registry::*;
pub use types::*;
pub use value::*;

//! Blueprint Runtime - Execution engine for visual scripts
//!
//! This crate provides the JS-driven blueprint execution engine and service lifecycle.

pub use blueprint_types;

// Re-export execution types from neo_js_runtime for convenience
pub use neo_js_runtime::{ExecutionTrigger, ExecutionResultJs};

mod blueprint_js_runtime;
mod js_node_library;
pub mod service;

pub use blueprint_js_runtime::*;
pub use js_node_library::*;
pub use service::*;

//! Blueprint Engine
//!
//! Provides blueprint execution and built-in node registration.

mod nodes;
mod executor;

pub use nodes::register_builtin_nodes;
pub use executor::BlueprintExecutor;

//! Code Generation Bridges
//!
//! This module provides bridges that sync the runtime type system with:
//! - TypeScript (.d.ts files for IDE support)
//! - Blueprint schemas (pushed via WebSocket)
//!
//! When user-defined types (events, objects) are registered at runtime,
//! these bridges automatically update the external representations.

mod typescript;
mod blueprint;

pub use typescript::TypeScriptBridge;
pub use blueprint::BlueprintBridge;

//! Neo - Building Automation System
//!
//! This crate provides the main application runtime for Neo, including:
//! - Service management via `blueprint_runtime::service`
//! - Type system via `blueprint_types`
//! - Visual scripting via blueprints
//! - Process-isolated JavaScript plugins via deno_core
//! - Code generation bridges (TypeScript, Blueprint schemas)
//! - Project management and WebSocket API

// Re-export core crates
pub use blueprint_runtime;
pub use blueprint_types;

// Process-isolated plugin system (deno_core)
pub mod plugin;

// Code generation bridges
pub mod bridges;

// Blueprint engine
pub mod engine;

// Project management
pub mod project;

// WebSocket server
pub mod server;

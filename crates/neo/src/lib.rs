//! Neo - Building Automation System
//!
//! This crate provides the main application runtime for Neo, including:
//! - Service management via `blueprint_runtime::service`
//! - Visual scripting via blueprints
//! - Process-isolated JavaScript plugins via deno_core
//! - Project management and WebSocket API

// Re-export core crates
pub use blueprint_runtime;

// Process-isolated plugin system (deno_core)
pub mod plugin;

// Blueprint engine
pub mod engine;

// Project management
pub mod project;

// WebSocket server
pub mod server;

// BACnet network service
pub mod bacnet;

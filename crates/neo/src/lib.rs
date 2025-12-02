//! Neo - Building Automation System
//!
//! This crate provides the main application runtime for Neo, including:
//! - Service management via `blueprint_runtime::service`
//! - Type system via `blueprint_types`
//! - Visual scripting via blueprints
//! - JavaScript runtime via QuickJS
//! - Code generation bridges (TypeScript, Blueprint schemas)
//! - Project management and WebSocket API

// Re-export core crates
pub use blueprint_runtime;
pub use blueprint_types;

// JavaScript runtime integration
pub mod js;

// Code generation bridges
pub mod bridges;

// Blueprint engine
pub mod engine;

// Project management
pub mod project;

// WebSocket server
pub mod server;

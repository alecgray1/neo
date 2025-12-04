//! Neo JavaScript Runtime
//!
//! This crate provides the JavaScript runtime infrastructure for Neo.
//! It follows Deno's worker pattern: each runtime runs in its own OS thread
//! with its own V8 isolate.
//!
//! # Architecture
//!
//! - Each service/blueprint runs in a dedicated runtime thread
//! - Runtimes are typed: `RuntimeHandle<BlueprintMode>` vs `RuntimeHandle<ServiceMode>`
//! - Type-state pattern ensures only valid operations can be called at compile time
//! - Commands are sent via channel, event loop runs until completion (like Deno)
//!
//! # Example
//!
//! ```ignore
//! // Blueprint runtime - can only call blueprint methods
//! let handle: RuntimeHandle<BlueprintMode> = spawn_blueprint_runtime("my-bp".to_string(), services)?;
//! handle.set_blueprint_for_execution(blueprint).await?;
//! handle.execute_blueprint(ExecutionTrigger::start()).await?;
//! // handle.start_service() <- compile error! Not available on BlueprintMode
//!
//! // Service runtime - can only call service methods
//! let handle: RuntimeHandle<ServiceMode> = spawn_service_runtime("my-svc".to_string(), code, id, services)?;
//! handle.start_service().await?;
//! // handle.execute_blueprint(...) <- compile error! Not available on ServiceMode
//! ```

mod command;
mod error;
mod handle;
mod mode;
mod ops;
mod services;
mod spawn;
mod types;
mod worker;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

// Error types
pub use error::{PointError, RuntimeError};

// Runtime handle and mode types
pub use handle::RuntimeHandle;
pub use mode::{BlueprintMode, RuntimeMode, ServiceMode};

// Spawn functions
pub use spawn::{spawn_blueprint_runtime, spawn_service_runtime};

// Services
pub use services::{Event, EventPublisher, PointStore, RuntimeServices};

// Types for JS communication
pub use types::{BlueprintJs, BlueprintNodeJs, ConnectionJs, ExecutionResultJs, ExecutionTrigger};

// Deno ops
pub use ops::{neo_runtime, BlueprintExecutionState};

// V8 initialization
pub use worker::init_platform;

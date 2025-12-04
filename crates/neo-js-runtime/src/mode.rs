//! Runtime mode markers for type-state pattern.
//!
//! This module provides the type-level markers that distinguish between
//! blueprint runtimes and service runtimes, enabling compile-time safety
//! for mode-specific operations.

/// Sealed trait pattern to prevent external implementations of RuntimeMode.
mod private {
    pub trait Sealed {}
}

/// Marker trait for runtime modes.
///
/// This trait is sealed and cannot be implemented outside this crate.
/// Only `BlueprintMode` and `ServiceMode` implement this trait.
pub trait RuntimeMode: private::Sealed + Send + Sync + 'static {}

/// Blueprint execution mode.
///
/// Runtimes in this mode are used for executing blueprint graphs.
/// They support operations like `execute_blueprint`, `set_blueprint`,
/// `load_node`, etc.
#[derive(Debug, Clone, Copy, Default)]
pub struct BlueprintMode;

/// Service execution mode.
///
/// Runtimes in this mode are used for long-running JavaScript services.
/// They support operations like `start_service` and `stop_service`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ServiceMode;

impl private::Sealed for BlueprintMode {}
impl private::Sealed for ServiceMode {}
impl RuntimeMode for BlueprintMode {}
impl RuntimeMode for ServiceMode {}

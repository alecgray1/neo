//! Runtime spawn functions.
//!
//! This module provides the public API for creating JavaScript runtimes.
//! It uses the type-state pattern to return appropriately typed handles.

use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use deno_core::v8;
use tokio::sync::{mpsc, watch};

use crate::error::RuntimeError;
use crate::handle::RuntimeHandle;
use crate::mode::{BlueprintMode, RuntimeMode, ServiceMode};
use crate::services::RuntimeServices;
use crate::worker::{init_platform, run_worker};

/// Spawn a runtime for blueprint execution.
///
/// Returns a `RuntimeHandle<BlueprintMode>` which only exposes blueprint-related
/// methods like `execute_blueprint`, `set_blueprint_for_execution`, `load_node`, etc.
pub fn spawn_blueprint_runtime(
    name: String,
    services: RuntimeServices,
) -> Result<RuntimeHandle<BlueprintMode>, RuntimeError> {
    spawn_runtime_inner(name, None, services)
}

/// Spawn a runtime for a JS service.
///
/// Returns a `RuntimeHandle<ServiceMode>` which only exposes service-related
/// methods like `start_service` and `stop_service`.
pub fn spawn_service_runtime(
    name: String,
    code: String,
    service_id: String,
    services: RuntimeServices,
) -> Result<RuntimeHandle<ServiceMode>, RuntimeError> {
    spawn_runtime_inner(name, Some((code, service_id)), services)
}

/// Internal spawn function that works for any mode.
fn spawn_runtime_inner<Mode: RuntimeMode>(
    name: String,
    initial_code: Option<(String, String)>,
    services: RuntimeServices,
) -> Result<RuntimeHandle<Mode>, RuntimeError> {
    tracing::debug!("[spawn_runtime] Starting for {}", name);
    init_platform();

    let terminated = Arc::new(AtomicBool::new(false));
    let terminated_clone = terminated.clone();

    // Command channel
    let (cmd_tx, cmd_rx) = mpsc::channel(32);

    // Shutdown signal (watch channel - multiple receivers can subscribe)
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Channel to receive isolate handle from the worker thread
    let (init_tx, init_rx) = std::sync::mpsc::sync_channel::<Result<v8::IsolateHandle, String>>(1);

    let name_clone = name.clone();
    let thread_handle = thread::Builder::new()
        .name(name.clone())
        .spawn(move || -> Result<(), RuntimeError> {
            tracing::debug!("[spawn_runtime:{}] Thread started", name_clone);

            // Create tokio runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(RuntimeError::SpawnFailed)?;

            // Run the worker
            let result = rt.block_on(run_worker(
                name_clone.clone(),
                initial_code,
                services,
                terminated_clone,
                cmd_rx,
                shutdown_rx,
                init_tx,
            ));

            rt.shutdown_background();
            tracing::debug!("[spawn_runtime:{}] Thread exiting", name_clone);
            result
        })?;

    // Wait for initialization
    let isolate_handle = init_rx
        .recv()
        .map_err(|_| RuntimeError::ChannelClosed)?
        .map_err(RuntimeError::JavaScript)?;

    tracing::debug!("[spawn_runtime] {} is ready", name);

    Ok(RuntimeHandle {
        cmd_tx,
        shutdown_tx,
        terminated,
        isolate_handle,
        thread_handle: std::sync::Mutex::new(Some(thread_handle)),
        _mode: PhantomData,
    })
}

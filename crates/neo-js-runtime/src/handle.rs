//! RuntimeHandle with type-state pattern.
//!
//! This module provides `RuntimeHandle<Mode>` which uses the type-state pattern
//! to enforce at compile-time that only valid operations can be performed on
//! a runtime based on its mode (blueprint vs service).

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use deno_core::v8;
use tokio::sync::{mpsc, oneshot, watch};

use crate::command::{BlueprintCommand, RuntimeCommand, ServiceCommand};
use crate::error::RuntimeError;
use crate::mode::{BlueprintMode, RuntimeMode, ServiceMode};
use crate::types::{BlueprintJs, ExecutionResultJs, ExecutionTrigger};

/// Handle to a spawned JavaScript runtime.
///
/// The type parameter `Mode` determines which operations are available:
/// - `RuntimeHandle<BlueprintMode>`: Blueprint operations (execute_blueprint, load_node, etc.)
/// - `RuntimeHandle<ServiceMode>`: Service operations (start_service, stop_service)
///
/// Common operations like `terminate()` and `is_terminated()` are available in all modes.
pub struct RuntimeHandle<Mode: RuntimeMode> {
    /// Command sender
    pub(crate) cmd_tx: mpsc::Sender<RuntimeCommand>,
    /// Shutdown signal sender
    pub(crate) shutdown_tx: watch::Sender<bool>,
    /// Whether the runtime has terminated
    pub(crate) terminated: Arc<AtomicBool>,
    /// V8 isolate handle for forced termination
    pub(crate) isolate_handle: v8::IsolateHandle,
    /// Thread join handle
    pub(crate) thread_handle: std::sync::Mutex<Option<thread::JoinHandle<Result<(), RuntimeError>>>>,
    /// Phantom data for the mode type
    pub(crate) _mode: PhantomData<Mode>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared methods (available in all modes)
// ─────────────────────────────────────────────────────────────────────────────

impl<Mode: RuntimeMode> RuntimeHandle<Mode> {
    /// Helper to send a command and wait for reply.
    async fn send_command<T, C, F>(&self, make_cmd: F) -> Result<T, RuntimeError>
    where
        C: Into<RuntimeCommand>,
        F: FnOnce(oneshot::Sender<Result<T, String>>) -> C,
    {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let (reply_tx, reply_rx) = oneshot::channel();
        self.cmd_tx
            .send(make_cmd(reply_tx).into())
            .await
            .map_err(|_| RuntimeError::ChannelClosed)?;

        reply_rx
            .await
            .map_err(|_| RuntimeError::ChannelClosed)?
            .map_err(RuntimeError::JavaScript)
    }

    /// Terminate the runtime.
    ///
    /// This signals the worker thread to shut down and forcefully terminates
    /// V8 execution if it's stuck.
    pub fn terminate(&self) {
        if self.terminated.swap(true, Ordering::SeqCst) {
            return; // Already terminated
        }
        // Signal shutdown - this will wake the worker's select!
        let _ = self.shutdown_tx.send(true);
        // Force terminate V8 execution if it's stuck
        self.isolate_handle.terminate_execution();
    }

    /// Check if the runtime has terminated.
    pub fn is_terminated(&self) -> bool {
        self.terminated.load(Ordering::SeqCst)
    }

    /// Wait for the runtime thread to finish.
    pub fn join(self) -> Result<(), RuntimeError> {
        if let Some(handle) = self.thread_handle.lock().unwrap().take() {
            handle.join().map_err(|_| RuntimeError::ThreadPanic)??;
        }
        Ok(())
    }
}

impl<Mode: RuntimeMode> Drop for RuntimeHandle<Mode> {
    fn drop(&mut self) {
        self.terminate();
        // Wait for the thread to finish to ensure clean V8 shutdown
        if let Some(handle) = self.thread_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

// SAFETY: RuntimeHandle is designed to be used from multiple threads.
// - mpsc::Sender is Send + Sync
// - watch::Sender is Send + Sync
// - v8::IsolateHandle is documented as Send + Sync
// - Arc<AtomicBool> is Send + Sync
// - Mutex<Option<JoinHandle>> is Send + Sync
// - PhantomData<Mode> is Send + Sync since Mode: Send + Sync
unsafe impl<Mode: RuntimeMode> Send for RuntimeHandle<Mode> {}
unsafe impl<Mode: RuntimeMode> Sync for RuntimeHandle<Mode> {}

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint mode methods
// ─────────────────────────────────────────────────────────────────────────────

impl RuntimeHandle<BlueprintMode> {
    /// Execute a blueprint and wait for the result.
    pub async fn execute_blueprint(
        &self,
        trigger: ExecutionTrigger,
    ) -> Result<ExecutionResultJs, RuntimeError> {
        self.send_command(|reply| BlueprintCommand::ExecuteBlueprint { trigger, reply })
            .await
    }

    /// Set the blueprint for execution.
    pub async fn set_blueprint_for_execution(
        &self,
        blueprint: BlueprintJs,
    ) -> Result<(), RuntimeError> {
        self.send_command(|reply| BlueprintCommand::SetBlueprint { blueprint, reply })
            .await
    }

    /// Execute the loaded node (single-node mode).
    pub async fn execute_node(&self, context_json: &str) -> Result<serde_json::Value, RuntimeError> {
        let context = context_json.to_string();
        self.send_command(|reply| BlueprintCommand::ExecuteNode {
            context_json: context,
            reply,
        })
        .await
    }

    /// Load a node definition into the runtime's node registry.
    pub async fn load_node(&self, node_id: &str, code: &str) -> Result<(), RuntimeError> {
        let node_id = node_id.to_string();
        let code = code.to_string();
        self.send_command(|reply| BlueprintCommand::LoadNode {
            node_id,
            code,
            reply,
        })
        .await
    }

    /// Execute a node by ID from the registry.
    pub async fn execute_node_by_id(
        &self,
        node_id: &str,
        context_json: &str,
    ) -> Result<serde_json::Value, RuntimeError> {
        let node_id = node_id.to_string();
        let context_json = context_json.to_string();
        self.send_command(|reply| BlueprintCommand::ExecuteNodeById {
            node_id,
            context_json,
            reply,
        })
        .await
    }

    /// Check if a node is registered in the runtime's node registry.
    pub async fn has_node(&self, node_id: &str) -> Result<bool, RuntimeError> {
        let node_id = node_id.to_string();
        self.send_command(|reply| BlueprintCommand::HasNode { node_id, reply })
            .await
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service mode methods
// ─────────────────────────────────────────────────────────────────────────────

impl RuntimeHandle<ServiceMode> {
    /// Start the loaded service (calls onStart).
    pub async fn start_service(&self) -> Result<(), RuntimeError> {
        self.send_command(|reply| ServiceCommand::Start { reply })
            .await
    }

    /// Stop the loaded service (calls onStop).
    pub async fn stop_service(&self) -> Result<(), RuntimeError> {
        self.send_command(|reply| ServiceCommand::Stop { reply })
            .await
    }
}

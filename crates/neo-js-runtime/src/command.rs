//! Commands sent to the runtime worker thread.
//!
//! Commands are split by mode:
//! - `BlueprintCommand` for blueprint operations
//! - `ServiceCommand` for service operations
//!
//! The internal `RuntimeCommand` wraps both for the unified worker channel.

use tokio::sync::oneshot;

use crate::types::{BlueprintJs, ExecutionResultJs, ExecutionTrigger};

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Commands for blueprint-mode runtimes.
pub(crate) enum BlueprintCommand {
    /// Execute a blueprint and return the result.
    ExecuteBlueprint {
        trigger: ExecutionTrigger,
        reply: oneshot::Sender<Result<ExecutionResultJs, String>>,
    },

    /// Set the blueprint for execution.
    SetBlueprint {
        blueprint: BlueprintJs,
        reply: oneshot::Sender<Result<(), String>>,
    },

    /// Load a node definition into the registry.
    LoadNode {
        node_id: String,
        code: String,
        reply: oneshot::Sender<Result<(), String>>,
    },

    /// Execute the loaded node (single-node mode).
    ExecuteNode {
        context_json: String,
        reply: oneshot::Sender<Result<serde_json::Value, String>>,
    },

    /// Execute a node by ID from the registry (multi-node mode).
    ExecuteNodeById {
        node_id: String,
        context_json: String,
        reply: oneshot::Sender<Result<serde_json::Value, String>>,
    },

    /// Check if a node exists in the registry.
    HasNode {
        node_id: String,
        reply: oneshot::Sender<Result<bool, String>>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Commands for service-mode runtimes.
pub(crate) enum ServiceCommand {
    /// Start the loaded service.
    Start {
        reply: oneshot::Sender<Result<(), String>>,
    },

    /// Stop the loaded service.
    Stop {
        reply: oneshot::Sender<Result<(), String>>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal Wrapper
// ─────────────────────────────────────────────────────────────────────────────

/// Internal wrapper for the worker's command channel.
///
/// The worker receives this unified type, but handles only get access
/// to their mode-specific command type.
pub(crate) enum RuntimeCommand {
    Blueprint(BlueprintCommand),
    Service(ServiceCommand),
}

impl From<BlueprintCommand> for RuntimeCommand {
    fn from(cmd: BlueprintCommand) -> Self {
        RuntimeCommand::Blueprint(cmd)
    }
}

impl From<ServiceCommand> for RuntimeCommand {
    fn from(cmd: ServiceCommand) -> Self {
        RuntimeCommand::Service(cmd)
    }
}

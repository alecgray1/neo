//! Blueprint JavaScript Runtime
//!
//! Manages a single V8 runtime shared by all JS nodes in a blueprint.
//! Multiple node definitions can be loaded and executed within the same runtime.

use std::collections::HashSet;
use std::sync::Arc;

use neo_js_runtime::{
    spawn_runtime_empty, BlueprintJs, ExecutionResultJs, ExecutionTrigger, RuntimeError,
    RuntimeHandle, RuntimeServices,
};

/// A JavaScript runtime dedicated to a single blueprint.
///
/// This holds one V8 isolate that can have multiple node definitions loaded into it.
/// All JS nodes in the blueprint share this runtime, reducing memory usage and
/// startup time compared to one runtime per node.
pub struct BlueprintJsRuntime {
    /// The underlying V8 runtime handle
    handle: Arc<RuntimeHandle>,
    /// Blueprint ID this runtime belongs to
    blueprint_id: String,
    /// Set of node type IDs that have been loaded
    loaded_nodes: HashSet<String>,
}

impl BlueprintJsRuntime {
    /// Create a new runtime for a blueprint.
    ///
    /// This spawns a new V8 isolate in its own OS thread.
    pub fn new(blueprint_id: &str, services: RuntimeServices) -> Result<Self, RuntimeError> {
        let name = format!("blueprint:{}", blueprint_id);
        let handle = spawn_runtime_empty(name, services)?;

        Ok(Self {
            handle: Arc::new(handle),
            blueprint_id: blueprint_id.to_string(),
            loaded_nodes: HashSet::new(),
        })
    }

    /// Get the blueprint ID this runtime belongs to.
    pub fn blueprint_id(&self) -> &str {
        &self.blueprint_id
    }

    /// Load a node definition into this runtime.
    ///
    /// The node code should use `export default defineNode({...})`.
    /// Once loaded, the node can be executed via `execute_node()`.
    ///
    /// Returns Ok(()) if already loaded (idempotent).
    pub async fn load_node(&mut self, node_id: &str, code: &str) -> Result<(), RuntimeError> {
        // Skip if already loaded
        if self.loaded_nodes.contains(node_id) {
            return Ok(());
        }

        // Load into the V8 runtime
        self.handle.load_node(node_id, code).await?;
        self.loaded_nodes.insert(node_id.to_string());

        tracing::debug!(
            "Loaded node {} into blueprint runtime {}",
            node_id,
            self.blueprint_id
        );

        Ok(())
    }

    /// Check if a node is loaded in this runtime.
    pub fn has_node(&self, node_id: &str) -> bool {
        self.loaded_nodes.contains(node_id)
    }

    /// Load a node definition into this runtime (async, no local tracking).
    ///
    /// This version doesn't require &mut self, suitable for use from
    /// borrowed contexts (e.g., DashMap refs). The JS runtime tracks
    /// whether a node is already loaded internally.
    pub async fn load_node_async(&self, node_id: &str, code: &str) -> Result<(), RuntimeError> {
        // Let the JS runtime handle deduplication
        self.handle.load_node(node_id, code).await?;

        tracing::debug!(
            "Loaded node {} into blueprint runtime {} (async)",
            node_id,
            self.blueprint_id
        );

        Ok(())
    }

    /// Execute a node by ID.
    ///
    /// The node must have been previously loaded via `load_node()`.
    /// Returns the deserialized result as a JSON value.
    pub async fn execute_node(&self, node_id: &str, context_json: &str) -> Result<serde_json::Value, RuntimeError> {
        if !self.loaded_nodes.contains(node_id) {
            return Err(RuntimeError::JavaScript(format!(
                "Node {} not loaded in blueprint runtime {}",
                node_id, self.blueprint_id
            )));
        }

        self.handle.execute_node_by_id(node_id, context_json).await
    }

    /// Get the number of loaded nodes.
    pub fn loaded_node_count(&self) -> usize {
        self.loaded_nodes.len()
    }

    /// Get an iterator over loaded node IDs.
    pub fn loaded_node_ids(&self) -> impl Iterator<Item = &str> {
        self.loaded_nodes.iter().map(|s| s.as_str())
    }

    /// Check if the runtime has terminated.
    pub fn is_terminated(&self) -> bool {
        self.handle.is_terminated()
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // JS-Driven Blueprint Execution
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the blueprint for JS-driven execution.
    ///
    /// Takes a pre-built BlueprintJs and stores it for the JS execution loop.
    pub async fn set_blueprint_for_execution(&self, blueprint: BlueprintJs) -> Result<(), RuntimeError> {
        self.handle.set_blueprint_for_execution(blueprint).await
    }

    /// Execute the loaded blueprint using the JS-driven execution loop.
    ///
    /// This runs the entire blueprint execution in JavaScript, including:
    /// - Finding entry nodes based on trigger type
    /// - Following execution flow
    /// - Evaluating pure nodes on demand
    /// - Handling flow control (branches, loops, sequences)
    ///
    /// The trigger determines which entry nodes to execute:
    /// - "start" -> event/OnStart nodes
    /// - "event" -> event/OnEvent nodes
    pub async fn execute_blueprint(&self, trigger: ExecutionTrigger) -> Result<ExecutionResultJs, RuntimeError> {
        self.handle.execute_blueprint(trigger).await
    }

    /// Execute the blueprint with a "start" trigger.
    pub async fn execute_start(&self) -> Result<ExecutionResultJs, RuntimeError> {
        self.execute_blueprint(ExecutionTrigger::start()).await
    }

    /// Execute the blueprint with an "event" trigger.
    pub async fn execute_event(&self, data: serde_json::Value) -> Result<ExecutionResultJs, RuntimeError> {
        self.execute_blueprint(ExecutionTrigger::event(data)).await
    }
}

#[cfg(test)]
mod tests {
    // Tests would require V8 initialization - run via examples instead
}

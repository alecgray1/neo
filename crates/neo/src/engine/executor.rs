//! Blueprint Executor Service
//!
//! A service that executes blueprint graphs in response to events.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dashmap::DashMap;

use blueprint_runtime::service::{
    Event, Service, ServiceContext, ServiceError, ServiceResult, ServiceSpec,
};
use blueprint_runtime::{BlueprintJsRuntime, ExecutionTrigger, JsNodeLibrary, NodeRegistry};
use blueprint_types::Blueprint;
use neo_js_runtime::RuntimeServices;

/// State of a running blueprint execution
#[derive(Debug)]
pub struct BlueprintState {
    /// The blueprint being executed
    pub blueprint: Blueprint,
    /// Current variable values
    pub variables: HashMap<String, serde_json::Value>,
    /// Whether execution is currently active
    pub active: bool,
}

/// Blueprint Executor Service
///
/// This service manages blueprint execution. It:
/// - Loads blueprints on startup
/// - Listens for trigger events
/// - Executes blueprint graphs when triggered
pub struct BlueprintExecutor {
    /// Unique service ID
    id: String,
    /// Human-readable name
    name: String,
    /// The node registry for looking up Rust node executors
    registry: Arc<NodeRegistry>,
    /// Library of JS node definitions (code, not runtimes)
    js_library: Arc<JsNodeLibrary>,
    /// Per-blueprint JS runtimes (created on blueprint load)
    js_runtimes: DashMap<String, BlueprintJsRuntime>,
    /// Loaded blueprints
    blueprints: Arc<DashMap<String, BlueprintState>>,
    /// Event patterns to listen for
    subscriptions: Vec<String>,
}

impl BlueprintExecutor {
    /// Create a new blueprint executor
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        registry: Arc<NodeRegistry>,
        js_library: Arc<JsNodeLibrary>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            registry,
            js_library,
            js_runtimes: DashMap::new(),
            blueprints: Arc::new(DashMap::new()),
            subscriptions: vec![
                "blueprint/*".to_string(),
                "device/point/*".to_string(),
                "schedule/*".to_string(),
            ],
        }
    }

    /// Add a subscription pattern
    pub fn subscribe(mut self, pattern: impl Into<String>) -> Self {
        self.subscriptions.push(pattern.into());
        self
    }

    /// Load a blueprint into the executor.
    ///
    /// The JS runtime is created on-demand when the blueprint is first executed.
    /// Built-in nodes are already registered in the JS runtime; only plugin nodes
    /// need to be loaded.
    pub fn load_blueprint(&mut self, blueprint: Blueprint) {
        let id = blueprint.id.clone();

        let state = BlueprintState {
            blueprint,
            variables: HashMap::new(),
            active: false,
        };
        self.blueprints.insert(id, state);
    }

    /// Execute a specific blueprint by ID using JS-driven execution.
    ///
    /// This uses the JS execution loop in bootstrap.js, which handles:
    /// - Finding entry nodes based on trigger type
    /// - Following execution flow
    /// - Evaluating pure nodes on demand
    /// - Flow control (branches, loops, sequences)
    pub async fn execute_blueprint(&self, blueprint_id: &str, trigger: &str) -> ServiceResult<()> {
        // Get the blueprint
        let blueprint = {
            let state = self.blueprints.get(blueprint_id).ok_or_else(|| {
                ServiceError::Internal(format!("Blueprint not found: {}", blueprint_id))
            })?;
            state.blueprint.clone()
        };

        tracing::debug!(
            blueprint_id = blueprint_id,
            trigger = trigger,
            "Executing blueprint (JS-driven)"
        );

        // Ensure runtime exists (creates it if needed)
        self.ensure_runtime_exists(blueprint_id)?;

        // Load any plugin JS nodes this blueprint uses
        self.load_plugin_nodes_for_blueprint(blueprint_id, &blueprint).await?;

        // Set the blueprint for execution (must drop ref before await)
        {
            let runtime = self.js_runtimes.get(blueprint_id).ok_or_else(|| {
                ServiceError::Internal("Runtime disappeared".to_string())
            })?;
            runtime
                .set_blueprint_for_execution(&blueprint)
                .await
                .map_err(|e| ServiceError::Internal(format!("Failed to set blueprint: {}", e)))?;
        }

        // Convert trigger string to ExecutionTrigger
        let execution_trigger = match trigger {
            "start" => ExecutionTrigger::start(),
            _ => ExecutionTrigger::event(serde_json::json!({ "type": trigger })),
        };

        // Execute via JS-driven execution loop (must get fresh ref)
        let result = {
            let runtime = self.js_runtimes.get(blueprint_id).ok_or_else(|| {
                ServiceError::Internal("Runtime disappeared".to_string())
            })?;
            runtime.execute_blueprint(execution_trigger).await
        };

        match result {
            Ok(result) => {
                tracing::debug!(
                    blueprint_id = blueprint_id,
                    status = %result.status,
                    "Blueprint execution completed"
                );

                // Update variables in state if needed
                if let Some(mut state) = self.blueprints.get_mut(blueprint_id) {
                    state.variables = result.variables;
                }

                Ok(())
            }
            Err(e) => {
                tracing::error!(
                    blueprint_id = blueprint_id,
                    error = %e,
                    "Blueprint execution failed"
                );
                Err(ServiceError::Internal(format!("Execution failed: {}", e)))
            }
        }
    }

    /// Ensure a JS runtime exists for a blueprint (creates it if needed).
    fn ensure_runtime_exists(&self, blueprint_id: &str) -> ServiceResult<()> {
        // Check if runtime already exists
        if self.js_runtimes.contains_key(blueprint_id) {
            return Ok(());
        }

        // Create new runtime
        match BlueprintJsRuntime::new(blueprint_id, RuntimeServices::default()) {
            Ok(runtime) => {
                self.js_runtimes.insert(blueprint_id.to_string(), runtime);
                tracing::debug!("Created JS runtime for blueprint {}", blueprint_id);
                Ok(())
            }
            Err(e) => Err(ServiceError::Internal(format!(
                "Failed to create JS runtime: {}",
                e
            ))),
        }
    }

    /// Load any plugin JS nodes that a blueprint uses.
    async fn load_plugin_nodes_for_blueprint(
        &self,
        blueprint_id: &str,
        blueprint: &Blueprint,
    ) -> ServiceResult<()> {
        // Collect node types that need loading
        let js_node_types: Vec<_> = blueprint
            .nodes
            .iter()
            .filter(|n| self.js_library.contains(&n.node_type))
            .map(|n| n.node_type.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if js_node_types.is_empty() {
            return Ok(());
        }

        // Load each node (need to get fresh ref for each await)
        for node_type in &js_node_types {
            if let Some(def) = self.js_library.get(node_type) {
                // Get mutable ref, check if loaded, then load
                let mut runtime = self.js_runtimes.get_mut(blueprint_id).ok_or_else(|| {
                    ServiceError::Internal("Runtime disappeared".to_string())
                })?;

                if !runtime.has_node(node_type) {
                    let code = def.code.clone();
                    let node_type_clone = node_type.clone();
                    drop(runtime); // Drop ref before await

                    // Get fresh ref for async operation
                    let runtime = self.js_runtimes.get(blueprint_id).ok_or_else(|| {
                        ServiceError::Internal("Runtime disappeared".to_string())
                    })?;

                    if let Err(e) = runtime.load_node_async(&node_type_clone, &code).await {
                        tracing::warn!(
                            "Failed to load plugin node {} into blueprint {}: {}",
                            node_type,
                            blueprint_id,
                            e
                        );
                    }
                }
            }
        }

        tracing::info!(
            "Loaded {} plugin nodes into JS runtime for blueprint {}",
            js_node_types.len(),
            blueprint_id
        );

        Ok(())
    }

    /// Get the number of loaded blueprints
    pub fn blueprint_count(&self) -> usize {
        self.blueprints.len()
    }

    /// Get the number of JS runtimes (one per blueprint with JS nodes)
    pub fn js_runtime_count(&self) -> usize {
        self.js_runtimes.len()
    }
}

#[async_trait]
impl Service for BlueprintExecutor {
    fn spec(&self) -> ServiceSpec {
        ServiceSpec::new(&self.id, &self.name)
            .with_subscriptions(self.subscriptions.clone())
            .with_description("Executes blueprint graphs in response to events")
    }

    async fn on_start(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.id,
            blueprint_count = self.blueprint_count(),
            "Blueprint executor started"
        );
        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(service_id = %self.id, "Blueprint executor stopped");
        Ok(())
    }

    async fn on_event(&mut self, _ctx: &ServiceContext, event: Event) -> ServiceResult<()> {
        tracing::debug!(
            service_id = %self.id,
            event_type = %event.event_type,
            "Received event"
        );

        // Execute all blueprints that might be triggered by this event
        let blueprint_ids: Vec<_> = self.blueprints.iter().map(|r| r.key().clone()).collect();
        for bp_id in blueprint_ids {
            if let Err(e) = self.execute_blueprint(&bp_id, &event.event_type).await {
                tracing::warn!(
                    blueprint_id = %bp_id,
                    error = %e,
                    "Error executing blueprint"
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::register_builtin_nodes;

    #[test]
    fn test_create_executor() {
        let mut registry = NodeRegistry::new();
        register_builtin_nodes(&mut registry);

        let executor = BlueprintExecutor::new(
            "test-executor",
            "Test Executor",
            Arc::new(registry),
            Arc::new(JsNodeLibrary::new()),
        );

        assert_eq!(executor.blueprint_count(), 0);
    }

    #[test]
    fn test_load_blueprint() {
        let mut registry = NodeRegistry::new();
        register_builtin_nodes(&mut registry);

        let mut executor = BlueprintExecutor::new(
            "test-executor",
            "Test Executor",
            Arc::new(registry),
            Arc::new(JsNodeLibrary::new()),
        );

        let blueprint = Blueprint::new("test-bp", "Test Blueprint");
        executor.load_blueprint(blueprint);

        assert_eq!(executor.blueprint_count(), 1);
    }
}

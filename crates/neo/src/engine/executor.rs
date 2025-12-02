//! Blueprint Executor Service
//!
//! A service that executes blueprint graphs in response to events.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::RwLock;

use blueprint_runtime::service::{
    Event, Service, ServiceContext, ServiceError, ServiceResult, ServiceSpec,
};
use blueprint_runtime::{NodeContext, NodeOutput, NodeRegistry};
use blueprint_types::Blueprint;

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
    /// The node registry for looking up node executors
    registry: Arc<NodeRegistry>,
    /// Loaded blueprints
    blueprints: Arc<RwLock<HashMap<String, BlueprintState>>>,
    /// Event patterns to listen for
    subscriptions: Vec<String>,
    /// Tick interval for periodic execution
    tick_interval: Option<Duration>,
}

impl BlueprintExecutor {
    /// Create a new blueprint executor
    pub fn new(id: impl Into<String>, name: impl Into<String>, registry: Arc<NodeRegistry>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            registry,
            blueprints: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: vec![
                "blueprint/*".to_string(),
                "device/point/*".to_string(),
                "schedule/*".to_string(),
            ],
            tick_interval: Some(Duration::from_secs(1)),
        }
    }

    /// Set tick interval
    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = Some(interval);
        self
    }

    /// Add a subscription pattern
    pub fn subscribe(mut self, pattern: impl Into<String>) -> Self {
        self.subscriptions.push(pattern.into());
        self
    }

    /// Load a blueprint into the executor
    pub fn load_blueprint(&self, blueprint: Blueprint) {
        let id = blueprint.id.clone();
        let state = BlueprintState {
            blueprint,
            variables: HashMap::new(),
            active: false,
        };
        self.blueprints.write().insert(id, state);
    }

    /// Execute a specific blueprint by ID
    pub async fn execute_blueprint(&self, blueprint_id: &str, trigger: &str) -> ServiceResult<()> {
        // Clone the necessary data while holding the lock, then drop it before async calls
        let (blueprint, variables, entry_node_ids) = {
            let blueprints = self.blueprints.read();
            let state = blueprints.get(blueprint_id).ok_or_else(|| {
                ServiceError::Internal(format!("Blueprint not found: {}", blueprint_id))
            })?;

            tracing::debug!(
                blueprint_id = blueprint_id,
                trigger = trigger,
                "Executing blueprint"
            );

            // Find entry points (nodes that match the trigger)
            let entry_node_ids: Vec<_> = state
                .blueprint
                .nodes
                .iter()
                .filter(|node| {
                    // Match event nodes like "event/OnTick", "event/OnPointChanged", etc.
                    node.node_type.starts_with("event/") || node.node_type.contains("On")
                })
                .map(|node| node.id.clone())
                .collect();

            if entry_node_ids.is_empty() {
                tracing::trace!(
                    blueprint_id = blueprint_id,
                    "No entry points found for trigger"
                );
                return Ok(());
            }

            (state.blueprint.clone(), state.variables.clone(), entry_node_ids)
        }; // Lock is dropped here

        // Execute from each entry point
        for entry_node_id in entry_node_ids {
            self.execute_from_node(&blueprint, &entry_node_id, &variables)
                .await?;
        }

        Ok(())
    }

    /// Execute a blueprint starting from a specific node
    async fn execute_from_node(
        &self,
        blueprint: &Blueprint,
        start_node_id: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> ServiceResult<()> {
        let mut current_node_id = Some(start_node_id.to_string());
        let mut node_outputs: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
        let mut variables = variables.clone();

        // Execute nodes following the exec flow
        while let Some(node_id) = current_node_id.take() {
            let node = blueprint.get_node(&node_id).ok_or_else(|| {
                ServiceError::Internal(format!("Node not found: {}", node_id))
            })?;

            // Skip event nodes - they're just entry points
            if node.node_type.starts_with("event/") || node.node_type.contains("On") {
                // Find the first output connection and follow it
                if let Some(conn) = blueprint.connections.iter().find(|c| {
                    c.from_parts()
                        .map(|(n, _)| n == node_id)
                        .unwrap_or(false)
                }) {
                    if let Some((to_node, _)) = conn.to_parts() {
                        current_node_id = Some(to_node.to_string());
                    }
                }
                continue;
            }

            // Get the node executor
            let executor = match self.registry.get_executor(&node.node_type) {
                Some(e) => e,
                None => {
                    tracing::warn!(
                        node_type = %node.node_type,
                        "Unknown node type, skipping"
                    );
                    break;
                }
            };

            // Gather inputs from connected nodes
            let mut inputs = HashMap::new();
            for conn in &blueprint.connections {
                if let (Some((from_node, from_pin)), Some((to_node, to_pin))) =
                    (conn.from_parts(), conn.to_parts())
                {
                    if to_node == node_id {
                        // Get output value from the source node
                        if let Some(outputs) = node_outputs.get(from_node) {
                            if let Some(value) = outputs.get(from_pin) {
                                inputs.insert(to_pin.to_string(), value.clone());
                            }
                        }
                    }
                }
            }

            // Create execution context
            let config = serde_json::to_value(&node.config).unwrap_or(serde_json::Value::Null);
            let mut ctx = NodeContext::new(node_id.clone(), config, inputs, variables.clone());

            // Execute the node
            let output = executor.execute(&mut ctx).await;

            // Store outputs for other nodes to use
            node_outputs.insert(node_id.clone(), output.values.clone());

            // Update variables if changed
            variables = ctx.variables;

            // Determine next node based on output
            match &output.result {
                blueprint_types::NodeResult::Continue(exec_pin) => {
                    // Find connection from this node's exec pin
                    let from_ref = format!("{}.{}", node_id, exec_pin);
                    for conn in &blueprint.connections {
                        if conn.from == from_ref {
                            if let Some((to_node, _)) = conn.to_parts() {
                                current_node_id = Some(to_node.to_string());
                                break;
                            }
                        }
                    }
                }
                blueprint_types::NodeResult::End => {
                    // Execution ended
                    break;
                }
                blueprint_types::NodeResult::Error(msg) => {
                    tracing::error!(
                        node_id = %node_id,
                        error = %msg,
                        "Node execution error"
                    );
                    break;
                }
                blueprint_types::NodeResult::Latent(_) => {
                    // Latent execution not yet supported
                    tracing::warn!(
                        node_id = %node_id,
                        "Latent execution not yet supported"
                    );
                    break;
                }
            }
        }

        Ok(())
    }

    /// Get the number of loaded blueprints
    pub fn blueprint_count(&self) -> usize {
        self.blueprints.read().len()
    }
}

#[async_trait]
impl Service for BlueprintExecutor {
    fn spec(&self) -> ServiceSpec {
        let mut spec = ServiceSpec::new(&self.id, &self.name)
            .with_subscriptions(self.subscriptions.clone())
            .with_description("Executes blueprint graphs in response to events");

        if let Some(interval) = self.tick_interval {
            spec = spec.with_tick_interval(interval);
        }

        spec
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
        let blueprint_ids: Vec<_> = self.blueprints.read().keys().cloned().collect();
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

    async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        // Execute blueprints that have tick-based entry points
        let blueprint_ids: Vec<_> = self.blueprints.read().keys().cloned().collect();
        for bp_id in blueprint_ids {
            if let Err(e) = self.execute_blueprint(&bp_id, "tick").await {
                tracing::warn!(
                    blueprint_id = %bp_id,
                    error = %e,
                    "Error executing blueprint on tick"
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
        );

        assert_eq!(executor.blueprint_count(), 0);
    }

    #[test]
    fn test_load_blueprint() {
        let mut registry = NodeRegistry::new();
        register_builtin_nodes(&mut registry);

        let executor = BlueprintExecutor::new(
            "test-executor",
            "Test Executor",
            Arc::new(registry),
        );

        let blueprint = Blueprint::new("test-bp", "Test Blueprint");
        executor.load_blueprint(blueprint);

        assert_eq!(executor.blueprint_count(), 1);
    }
}

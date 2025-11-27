// Blueprint Executor - Runs blueprint graphs
//
// The executor walks the graph following execution pins, evaluating data pins on demand.
// It handles pure node caching, latent node suspension, and variable management.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tracing::{debug, error, info};

use super::registry::{NodeContext, NodeRegistry};
use super::types::{
    Blueprint, BlueprintNode, ExecutionResult, ExecutionTrigger, LatentState, NodeResult,
};

// ─────────────────────────────────────────────────────────────────────────────
// Execution Context
// ─────────────────────────────────────────────────────────────────────────────

/// Context for a single blueprint execution
pub struct ExecutionContext {
    /// The blueprint being executed
    pub blueprint: Arc<Blueprint>,
    /// Current variable values
    pub variables: HashMap<String, Value>,
    /// Cached output values from executed nodes (node_id.pin_name -> value)
    pub node_outputs: HashMap<String, Value>,
    /// What triggered this execution
    pub trigger: ExecutionTrigger,
    /// Outputs from the execution (for external access)
    pub outputs: HashMap<String, Value>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(blueprint: Arc<Blueprint>, trigger: ExecutionTrigger) -> Self {
        // Initialize variables with their default values
        let variables = blueprint
            .variables
            .iter()
            .map(|(name, def)| {
                let value = def.default.clone().unwrap_or(Value::Null);
                (name.clone(), value)
            })
            .collect();

        Self {
            blueprint,
            variables,
            node_outputs: HashMap::new(),
            trigger,
            outputs: HashMap::new(),
        }
    }

    /// Get a cached node output value
    pub fn get_node_output(&self, node_id: &str, pin_name: &str) -> Option<&Value> {
        let key = format!("{}.{}", node_id, pin_name);
        self.node_outputs.get(&key)
    }

    /// Set a node output value
    pub fn set_node_output(&mut self, node_id: &str, pin_name: &str, value: Value) {
        let key = format!("{}.{}", node_id, pin_name);
        self.node_outputs.insert(key, value);
    }

    /// Get all output values from a node
    #[allow(dead_code)]
    pub fn get_all_node_outputs(&self, node_id: &str) -> HashMap<String, Value> {
        let prefix = format!("{}.", node_id);
        self.node_outputs
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, v)| {
                let pin_name = k.strip_prefix(&prefix).unwrap_or(k);
                (pin_name.to_string(), v.clone())
            })
            .collect()
    }

}

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint Executor
// ─────────────────────────────────────────────────────────────────────────────

/// Executor for running blueprint graphs
pub struct BlueprintExecutor {
    /// Node registry with definitions and executors
    registry: Arc<NodeRegistry>,
}

impl BlueprintExecutor {
    /// Create a new executor with the given registry
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        Self { registry }
    }

    /// Execute a blueprint starting from an event node
    pub async fn execute(
        &self,
        blueprint: Arc<Blueprint>,
        event_node_id: &str,
        trigger: ExecutionTrigger,
    ) -> ExecutionResult {
        let mut ctx = ExecutionContext::new(Arc::clone(&blueprint), trigger);

        info!(
            blueprint_id = %blueprint.id,
            event_node = %event_node_id,
            "Starting blueprint execution"
        );

        // Find the event node
        let event_node = match blueprint.get_node(event_node_id) {
            Some(node) => node.clone(),
            None => {
                return ExecutionResult::Failed {
                    error: format!("Event node '{}' not found", event_node_id),
                }
            }
        };

        // Set up initial outputs from the event trigger
        self.setup_event_outputs(&mut ctx, &event_node);

        // Execute starting from the event node's exec output
        match self.execute_from(&mut ctx, event_node_id, "exec").await {
            Ok(()) => ExecutionResult::Completed {
                outputs: ctx.outputs,
            },
            Err(ExecutionError::Suspended(state)) => ExecutionResult::Suspended { state },
            Err(ExecutionError::Failed(error)) => ExecutionResult::Failed { error },
        }
    }

    /// Set up outputs from the event trigger data
    fn setup_event_outputs(&self, ctx: &mut ExecutionContext, event_node: &BlueprintNode) {
        // Clone the trigger data to avoid borrow issues
        let trigger = ctx.trigger.clone();
        let node_id = event_node.id.clone();

        match trigger {
            ExecutionTrigger::Event { event_type, data } => {
                // Set event type and data as node outputs
                ctx.set_node_output(&node_id, "event_type", Value::String(event_type));
                ctx.set_node_output(&node_id, "data", data.clone());

                // If data is an object, expose its fields as outputs
                if let Some(obj) = data.as_object() {
                    for (key, value) in obj {
                        ctx.set_node_output(&node_id, key, value.clone());
                    }
                }
            }
            ExecutionTrigger::Schedule { schedule_id } => {
                ctx.set_node_output(&node_id, "schedule_id", Value::String(schedule_id));
            }
            ExecutionTrigger::Request { inputs } => {
                // Expose request inputs as node outputs
                if let Some(obj) = inputs.as_object() {
                    for (key, value) in obj {
                        ctx.set_node_output(&node_id, key, value.clone());
                    }
                }
            }
            ExecutionTrigger::ServiceStart => {
                // No special outputs for service start
            }
            ExecutionTrigger::ServiceStop => {
                // No special outputs for service stop
            }
            ExecutionTrigger::ServiceRequest { request_id, request } => {
                ctx.set_node_output(&node_id, "request_id", Value::String(request_id));
                if let Some(req) = request {
                    // Serialize the request for the blueprint
                    if let Ok(req_value) = serde_json::to_value(&req) {
                        ctx.set_node_output(&node_id, "request", req_value);
                    }
                }
            }
            ExecutionTrigger::ServiceEvent { event } => {
                if let Some(evt) = event {
                    // Serialize the event for the blueprint
                    if let Ok(evt_value) = serde_json::to_value(&evt) {
                        ctx.set_node_output(&node_id, "event", evt_value);
                    }
                }
            }
        }
    }

    /// Execute the graph starting from a specific node and exec pin
    fn execute_from<'a>(
        &'a self,
        ctx: &'a mut ExecutionContext,
        node_id: &'a str,
        exec_pin: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExecutionError>> + Send + 'a>> {
        Box::pin(async move {
        // Find connections from this exec pin and collect target node IDs
        let target_nodes: Vec<String> = ctx
            .blueprint
            .connections_from(node_id, exec_pin)
            .iter()
            .filter_map(|conn| conn.to_parts().map(|(id, _)| id.to_string()))
            .collect();

        for to_node_id in target_nodes {
            // Execute the connected node
            self.execute_node(ctx, &to_node_id).await?;
        }

        Ok(())
        })
    }

    /// Execute a single node
    fn execute_node<'a>(
        &'a self,
        ctx: &'a mut ExecutionContext,
        node_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExecutionError>> + Send + 'a>> {
        Box::pin(async move {
        let node = ctx
            .blueprint
            .get_node(node_id)
            .ok_or_else(|| ExecutionError::Failed(format!("Node '{}' not found", node_id)))?
            .clone();

        let node_def = self.registry.get_definition(&node.node_type).ok_or_else(|| {
            ExecutionError::Failed(format!("Unknown node type '{}'", node.node_type))
        })?;

        debug!(
            node_id = %node_id,
            node_type = %node.node_type,
            "Executing node"
        );

        // Gather input pin names first
        let input_pins: Vec<String> = node_def.data_inputs().map(|p| p.name.clone()).collect();

        // Gather input values
        let mut inputs = HashMap::new();
        for pin_name in input_pins {
            let value = self.evaluate_input(ctx, node_id, &pin_name).await?;
            inputs.insert(pin_name, value);
        }

        // Create node context
        let mut node_ctx = NodeContext {
            node_id: node_id.to_string(),
            config: node.config.clone(),
            inputs,
            variables: ctx.variables.clone(),
        };

        // Get and execute the node
        let executor = self.registry.get_executor(&node.node_type).ok_or_else(|| {
            ExecutionError::Failed(format!("No executor for node type '{}'", node.node_type))
        })?;

        let output = executor.execute(&mut node_ctx).await;

        // Update variables from node context
        ctx.variables = node_ctx.variables;

        // Store output values
        for (pin_name, value) in output.values {
            ctx.set_node_output(node_id, &pin_name, value);
        }

        // Handle the execution result
        match output.result {
            NodeResult::Continue(exec_pin) => {
                // Continue execution from the specified exec pin
                self.execute_from(ctx, node_id, &exec_pin).await?;
            }
            NodeResult::End => {
                // Execution ends at this node (no more exec flow)
            }
            NodeResult::Latent(state) => {
                // Node is suspended, return to caller
                return Err(ExecutionError::Suspended(state));
            }
            NodeResult::Error(err) => {
                error!(
                    node_id = %node_id,
                    error = %err,
                    "Node execution error"
                );
                return Err(ExecutionError::Failed(err));
            }
        }

        Ok(())
        })
    }

    /// Evaluate an input value for a node (resolves connections or uses defaults)
    fn evaluate_input<'a>(
        &'a self,
        ctx: &'a mut ExecutionContext,
        node_id: &'a str,
        pin_name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, ExecutionError>> + Send + 'a>> {
        Box::pin(async move {
        // Find connection to this input pin and extract source info
        let connection_source: Option<(String, String)> = ctx
            .blueprint
            .connections_to(node_id, pin_name)
            .first()
            .and_then(|conn| {
                conn.from_parts()
                    .map(|(n, p)| (n.to_string(), p.to_string()))
            });

        if let Some((from_node_id, from_pin_name)) = connection_source {
            // Check if we already have this value cached
            if let Some(value) = ctx.get_node_output(&from_node_id, &from_pin_name) {
                return Ok(value.clone());
            }

            // Need to evaluate the source node (must be a pure node)
            let from_node = ctx.blueprint.get_node(&from_node_id).ok_or_else(|| {
                ExecutionError::Failed(format!("Source node '{}' not found", from_node_id))
            })?;

            let from_node_def = self
                .registry
                .get_definition(&from_node.node_type)
                .ok_or_else(|| {
                    ExecutionError::Failed(format!(
                        "Unknown source node type '{}'",
                        from_node.node_type
                    ))
                })?;

            if !from_node_def.pure {
                // Non-pure nodes should have been executed already
                // Return the cached value or null
                return Ok(ctx
                    .get_node_output(&from_node_id, &from_pin_name)
                    .cloned()
                    .unwrap_or(Value::Null));
            }

            // Evaluate the pure node
            self.evaluate_pure_node(ctx, &from_node_id).await?;

            // Return the now-cached value
            Ok(ctx
                .get_node_output(&from_node_id, &from_pin_name)
                .cloned()
                .unwrap_or(Value::Null))
        } else {
            // No connection - use default value from node definition
            let node = ctx.blueprint.get_node(node_id).ok_or_else(|| {
                ExecutionError::Failed(format!("Node '{}' not found", node_id))
            })?;

            // First check if there's a default in the node's config
            if let Some(defaults) = node.config.get("defaults") {
                if let Some(value) = defaults.get(pin_name) {
                    return Ok(value.clone());
                }
            }

            // Then check the pin definition default
            let node_def = self.registry.get_definition(&node.node_type).ok_or_else(|| {
                ExecutionError::Failed(format!("Unknown node type '{}'", node.node_type))
            })?;

            if let Some(pin_def) = node_def.get_pin(pin_name) {
                if let Some(default) = &pin_def.default {
                    return Ok(default.clone());
                }
            }

            Ok(Value::Null)
        }
        })
    }

    /// Evaluate a pure node (called when its output is needed)
    fn evaluate_pure_node<'a>(
        &'a self,
        ctx: &'a mut ExecutionContext,
        node_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExecutionError>> + Send + 'a>> {
        Box::pin(async move {
        let node = ctx
            .blueprint
            .get_node(node_id)
            .ok_or_else(|| ExecutionError::Failed(format!("Node '{}' not found", node_id)))?
            .clone();

        let node_def = self.registry.get_definition(&node.node_type).ok_or_else(|| {
            ExecutionError::Failed(format!("Unknown node type '{}'", node.node_type))
        })?;

        debug!(
            node_id = %node_id,
            node_type = %node.node_type,
            "Evaluating pure node"
        );

        // Gather input pin names first
        let input_pins: Vec<String> = node_def.data_inputs().map(|p| p.name.clone()).collect();

        // Gather input values (recursively evaluates connected pure nodes)
        let mut inputs = HashMap::new();
        for pin_name in input_pins {
            let value = self.evaluate_input(ctx, node_id, &pin_name).await?;
            inputs.insert(pin_name, value);
        }

        // Create node context
        let mut node_ctx = NodeContext {
            node_id: node_id.to_string(),
            config: node.config.clone(),
            inputs,
            variables: ctx.variables.clone(),
        };

        // Execute the node
        let executor = self.registry.get_executor(&node.node_type).ok_or_else(|| {
            ExecutionError::Failed(format!("No executor for node type '{}'", node.node_type))
        })?;

        let output = executor.execute(&mut node_ctx).await;

        // Store output values
        for (pin_name, value) in output.values {
            ctx.set_node_output(node_id, &pin_name, value);
        }

        // Handle errors (pure nodes shouldn't return Continue or Latent)
        if let NodeResult::Error(err) = output.result {
            return Err(ExecutionError::Failed(err));
        }

        Ok(())
        })
    }

    /// Resume a suspended execution
    pub async fn resume(
        &self,
        ctx: &mut ExecutionContext,
        state: &LatentState,
    ) -> ExecutionResult {
        info!(
            blueprint_id = %ctx.blueprint.id,
            node_id = %state.node_id,
            resume_pin = %state.resume_pin,
            "Resuming blueprint execution"
        );

        match self
            .execute_from(ctx, &state.node_id, &state.resume_pin)
            .await
        {
            Ok(()) => ExecutionResult::Completed {
                outputs: ctx.outputs.clone(),
            },
            Err(ExecutionError::Suspended(new_state)) => {
                ExecutionResult::Suspended { state: new_state }
            }
            Err(ExecutionError::Failed(err)) => ExecutionResult::Failed { error: err },
        }
    }
}

/// Internal execution error type
enum ExecutionError {
    Suspended(LatentState),
    Failed(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints::types::{Connection, PinType, VariableDef};

    fn create_test_blueprint() -> Blueprint {
        Blueprint {
            id: "test".to_string(),
            name: "Test Blueprint".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            variables: {
                let mut vars = HashMap::new();
                vars.insert(
                    "threshold".to_string(),
                    VariableDef {
                        var_type: PinType::Real,
                        default: Some(Value::from(10.0)),
                        description: None,
                    },
                );
                vars
            },
            nodes: vec![
                BlueprintNode {
                    id: "event".to_string(),
                    node_type: "neo/OnEvent".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
                BlueprintNode {
                    id: "compare".to_string(),
                    node_type: "neo/Compare".to_string(),
                    position: Default::default(),
                    config: serde_json::json!({"operator": ">"}),
                },
                BlueprintNode {
                    id: "branch".to_string(),
                    node_type: "neo/Branch".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
                BlueprintNode {
                    id: "log_true".to_string(),
                    node_type: "neo/Log".to_string(),
                    position: Default::default(),
                    config: serde_json::json!({"defaults": {"message": "Condition is true!"}}),
                },
                BlueprintNode {
                    id: "log_false".to_string(),
                    node_type: "neo/Log".to_string(),
                    position: Default::default(),
                    config: serde_json::json!({"defaults": {"message": "Condition is false!"}}),
                },
            ],
            connections: vec![
                Connection::new("event", "exec", "branch", "exec"),
                Connection::new("event", "value", "compare", "a"),
                Connection::new("compare", "result", "branch", "condition"),
                Connection::new("branch", "true", "log_true", "exec"),
                Connection::new("branch", "false", "log_false", "exec"),
            ],
        }
    }

    #[tokio::test]
    async fn test_simple_execution() {
        let registry = Arc::new(NodeRegistry::with_builtins());
        let executor = BlueprintExecutor::new(registry);

        let mut blueprint = create_test_blueprint();
        // Add threshold value for compare node
        blueprint
            .connections
            .push(Connection::new("get_threshold", "value", "compare", "b"));
        blueprint.nodes.push(BlueprintNode {
            id: "get_threshold".to_string(),
            node_type: "neo/GetVariable".to_string(),
            position: Default::default(),
            config: serde_json::json!({"variable": "threshold"}),
        });

        let blueprint = Arc::new(blueprint);
        let trigger = ExecutionTrigger::Event {
            event_type: "test".to_string(),
            data: serde_json::json!({"value": 15.0}),
        };

        let result = executor.execute(blueprint, "event", trigger).await;

        match result {
            ExecutionResult::Completed { outputs } => {
                // Execution completed successfully
                println!("Execution completed with outputs: {:?}", outputs);
            }
            ExecutionResult::Failed { error } => {
                panic!("Execution failed: {}", error);
            }
            ExecutionResult::Suspended { state } => {
                panic!("Unexpected suspension: {:?}", state);
            }
        }
    }

    #[tokio::test]
    async fn test_math_chain() {
        let registry = Arc::new(NodeRegistry::with_builtins());
        let executor = BlueprintExecutor::new(registry);

        // Create a blueprint that does: (5 + 3) * 2 = 16
        let blueprint = Blueprint {
            id: "math-test".to_string(),
            name: "Math Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            variables: HashMap::new(),
            nodes: vec![
                BlueprintNode {
                    id: "event".to_string(),
                    node_type: "neo/OnEvent".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
                BlueprintNode {
                    id: "add".to_string(),
                    node_type: "neo/Add".to_string(),
                    position: Default::default(),
                    config: serde_json::json!({"defaults": {"a": 5.0, "b": 3.0}}),
                },
                BlueprintNode {
                    id: "multiply".to_string(),
                    node_type: "neo/Multiply".to_string(),
                    position: Default::default(),
                    config: serde_json::json!({"defaults": {"b": 2.0}}),
                },
                BlueprintNode {
                    id: "log".to_string(),
                    node_type: "neo/Log".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
            ],
            connections: vec![
                Connection::new("event", "exec", "log", "exec"),
                Connection::new("add", "result", "multiply", "a"),
                // Note: multiply.result would be 16.0, but we don't use it
            ],
        };

        let blueprint = Arc::new(blueprint);
        let trigger = ExecutionTrigger::Request {
            inputs: Value::Null,
        };

        let result = executor
            .execute(Arc::clone(&blueprint), "event", trigger)
            .await;

        match result {
            ExecutionResult::Completed { .. } => {
                // Test passed
            }
            ExecutionResult::Failed { error } => {
                panic!("Execution failed: {}", error);
            }
            _ => panic!("Unexpected result"),
        }
    }
}

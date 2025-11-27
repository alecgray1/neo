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
    Blueprint, BlueprintNode, ExecutionResult, ExecutionTrigger, FunctionDef, LatentState,
    NodeResult, FUNCTION_ENTRY_NODE, FUNCTION_EXIT_NODE,
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

    /// Execute a function within a blueprint
    pub async fn execute_function(
        &self,
        blueprint: Arc<Blueprint>,
        function: &FunctionDef,
        inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, String> {
        debug!(
            blueprint_id = %blueprint.id,
            function_name = ?function.name,
            "Executing function"
        );

        // Create a temporary context for function execution
        let trigger = ExecutionTrigger::Request {
            inputs: serde_json::to_value(&inputs).unwrap_or(Value::Null),
        };
        let mut ctx = FunctionExecutionContext::new(blueprint, function, trigger);

        // Set up input values on the entry node
        for (name, value) in &inputs {
            ctx.set_node_output(FUNCTION_ENTRY_NODE, name, value.clone());
        }

        // For pure functions, we evaluate data flow only (no exec pins)
        if function.pure {
            // Evaluate outputs by tracing data connections from exit node
            for output in &function.outputs {
                let value = self
                    .evaluate_function_input(&mut ctx, function, FUNCTION_EXIT_NODE, &output.name)
                    .await
                    .unwrap_or(Value::Null);
                ctx.set_node_output(FUNCTION_EXIT_NODE, &output.name, value);
            }
        } else {
            // Execute the function's graph starting from entry node
            if let Err(e) = self
                .execute_function_from(&mut ctx, function, FUNCTION_ENTRY_NODE, "exec")
                .await
            {
                return Err(format!("Function execution failed: {:?}", e));
            }
        }

        // Collect outputs from exit node
        let mut outputs = HashMap::new();
        for output in &function.outputs {
            let value = ctx
                .get_node_output(FUNCTION_EXIT_NODE, &output.name)
                .cloned()
                .unwrap_or(Value::Null);
            outputs.insert(output.name.clone(), value);
        }

        Ok(outputs)
    }

    /// Execute a function's graph starting from a specific node and exec pin
    fn execute_function_from<'a>(
        &'a self,
        ctx: &'a mut FunctionExecutionContext,
        function: &'a FunctionDef,
        node_id: &'a str,
        exec_pin: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExecutionError>> + Send + 'a>>
    {
        Box::pin(async move {
            // Find connections from this exec pin
            let target_nodes: Vec<String> = function
                .connections_from(node_id, exec_pin)
                .iter()
                .filter_map(|conn| conn.to_parts().map(|(id, _)| id.to_string()))
                .collect();

            for to_node_id in target_nodes {
                self.execute_function_node(ctx, function, &to_node_id)
                    .await?;
            }

            Ok(())
        })
    }

    /// Execute a single node within a function
    fn execute_function_node<'a>(
        &'a self,
        ctx: &'a mut FunctionExecutionContext,
        function: &'a FunctionDef,
        node_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExecutionError>> + Send + 'a>>
    {
        Box::pin(async move {
            // Skip exit node - it just collects values
            if node_id == FUNCTION_EXIT_NODE {
                // Evaluate all inputs to the exit node and store them
                for output in &function.outputs {
                    let value = self
                        .evaluate_function_input(ctx, function, FUNCTION_EXIT_NODE, &output.name)
                        .await?;
                    ctx.set_node_output(FUNCTION_EXIT_NODE, &output.name, value);
                }
                return Ok(());
            }

            let node = function
                .get_node(node_id)
                .ok_or_else(|| {
                    ExecutionError::Failed(format!("Node '{}' not found in function", node_id))
                })?
                .clone();

            let node_def = self.registry.get_definition(&node.node_type).ok_or_else(|| {
                ExecutionError::Failed(format!("Unknown node type '{}'", node.node_type))
            })?;

            debug!(
                node_id = %node_id,
                node_type = %node.node_type,
                "Executing function node"
            );

            // Gather input values
            let input_pins: Vec<String> = node_def.data_inputs().map(|p| p.name.clone()).collect();
            let mut inputs = HashMap::new();
            for pin_name in input_pins {
                let value = self
                    .evaluate_function_input(ctx, function, node_id, &pin_name)
                    .await?;
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

            // Update variables
            ctx.variables = node_ctx.variables;

            // Store output values
            for (pin_name, value) in output.values {
                ctx.set_node_output(node_id, &pin_name, value);
            }

            // Handle the execution result
            match output.result {
                NodeResult::Continue(exec_pin) => {
                    self.execute_function_from(ctx, function, node_id, &exec_pin)
                        .await?;
                }
                NodeResult::End => {}
                NodeResult::Latent(_) => {
                    return Err(ExecutionError::Failed(
                        "Latent nodes not supported in functions".to_string(),
                    ));
                }
                NodeResult::Error(err) => {
                    return Err(ExecutionError::Failed(err));
                }
            }

            Ok(())
        })
    }

    /// Evaluate an input value within a function context
    fn evaluate_function_input<'a>(
        &'a self,
        ctx: &'a mut FunctionExecutionContext,
        function: &'a FunctionDef,
        node_id: &'a str,
        pin_name: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, ExecutionError>> + Send + 'a>>
    {
        Box::pin(async move {
            // Find connection to this input pin
            let connection_source: Option<(String, String)> = function
                .connections_to(node_id, pin_name)
                .first()
                .and_then(|conn| {
                    conn.from_parts()
                        .map(|(n, p)| (n.to_string(), p.to_string()))
                });

            if let Some((from_node_id, from_pin_name)) = connection_source {
                // Check cache first
                if let Some(value) = ctx.get_node_output(&from_node_id, &from_pin_name) {
                    return Ok(value.clone());
                }

                // Entry node outputs are the function inputs
                if from_node_id == FUNCTION_ENTRY_NODE {
                    return Ok(ctx
                        .get_node_output(FUNCTION_ENTRY_NODE, &from_pin_name)
                        .cloned()
                        .unwrap_or(Value::Null));
                }

                // Need to evaluate the source node
                let from_node = function.get_node(&from_node_id).ok_or_else(|| {
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

                if from_node_def.pure {
                    // Evaluate pure node
                    self.evaluate_function_pure_node(ctx, function, &from_node_id)
                        .await?;
                }

                Ok(ctx
                    .get_node_output(&from_node_id, &from_pin_name)
                    .cloned()
                    .unwrap_or(Value::Null))
            } else {
                // No connection - use default
                if let Some(node) = function.get_node(node_id) {
                    if let Some(defaults) = node.config.get("defaults") {
                        if let Some(value) = defaults.get(pin_name) {
                            return Ok(value.clone());
                        }
                    }

                    if let Some(node_def) = self.registry.get_definition(&node.node_type) {
                        if let Some(pin_def) = node_def.get_pin(pin_name) {
                            if let Some(default) = &pin_def.default {
                                return Ok(default.clone());
                            }
                        }
                    }
                }

                Ok(Value::Null)
            }
        })
    }

    /// Evaluate a pure node within a function context
    fn evaluate_function_pure_node<'a>(
        &'a self,
        ctx: &'a mut FunctionExecutionContext,
        function: &'a FunctionDef,
        node_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ExecutionError>> + Send + 'a>>
    {
        Box::pin(async move {
            let node = function
                .get_node(node_id)
                .ok_or_else(|| ExecutionError::Failed(format!("Node '{}' not found", node_id)))?
                .clone();

            let node_def = self.registry.get_definition(&node.node_type).ok_or_else(|| {
                ExecutionError::Failed(format!("Unknown node type '{}'", node.node_type))
            })?;

            // Gather inputs
            let input_pins: Vec<String> = node_def.data_inputs().map(|p| p.name.clone()).collect();
            let mut inputs = HashMap::new();
            for pin_name in input_pins {
                let value = self
                    .evaluate_function_input(ctx, function, node_id, &pin_name)
                    .await?;
                inputs.insert(pin_name, value);
            }

            // Execute
            let mut node_ctx = NodeContext {
                node_id: node_id.to_string(),
                config: node.config.clone(),
                inputs,
                variables: ctx.variables.clone(),
            };

            let executor = self.registry.get_executor(&node.node_type).ok_or_else(|| {
                ExecutionError::Failed(format!("No executor for node type '{}'", node.node_type))
            })?;

            let output = executor.execute(&mut node_ctx).await;

            // Store outputs
            for (pin_name, value) in output.values {
                ctx.set_node_output(node_id, &pin_name, value);
            }

            if let NodeResult::Error(err) = output.result {
                return Err(ExecutionError::Failed(err));
            }

            Ok(())
        })
    }
}

/// Execution context for function calls
struct FunctionExecutionContext {
    /// The blueprint containing the function
    #[allow(dead_code)]
    blueprint: Arc<Blueprint>,
    /// Current variable values
    variables: HashMap<String, Value>,
    /// Cached output values from executed nodes
    node_outputs: HashMap<String, Value>,
    /// What triggered this execution
    #[allow(dead_code)]
    trigger: ExecutionTrigger,
}

impl FunctionExecutionContext {
    fn new(blueprint: Arc<Blueprint>, function: &FunctionDef, trigger: ExecutionTrigger) -> Self {
        // Initialize variables with defaults from the function's inputs
        let mut variables = HashMap::new();
        for input in &function.inputs {
            if let Some(default) = &input.default {
                variables.insert(input.name.clone(), default.clone());
            }
        }

        Self {
            blueprint,
            variables,
            node_outputs: HashMap::new(),
            trigger,
        }
    }

    fn get_node_output(&self, node_id: &str, pin_name: &str) -> Option<&Value> {
        let key = format!("{}.{}", node_id, pin_name);
        self.node_outputs.get(&key)
    }

    fn set_node_output(&mut self, node_id: &str, pin_name: &str, value: Value) {
        let key = format!("{}.{}", node_id, pin_name);
        self.node_outputs.insert(key, value);
    }
}

/// Internal execution error type
#[derive(Debug)]
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
            service: None,
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
            functions: HashMap::new(),
            imports: vec![],
            exports: vec![],
            implements: vec![],
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
            service: None,
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
            functions: HashMap::new(),
            imports: vec![],
            exports: vec![],
            implements: vec![],
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

    #[tokio::test]
    async fn test_pure_function_execution() {
        use crate::blueprints::types::{FunctionDef, FunctionParam, FUNCTION_ENTRY_NODE, FUNCTION_EXIT_NODE};

        let registry = Arc::new(NodeRegistry::with_builtins());
        let executor = BlueprintExecutor::new(registry);

        // Create a pure function that adds two numbers: add(a, b) -> result
        let add_function = FunctionDef {
            name: Some("add".to_string()),
            description: Some("Add two numbers".to_string()),
            inputs: vec![
                FunctionParam {
                    name: "a".to_string(),
                    param_type: PinType::Real,
                    default: None,
                    description: None,
                },
                FunctionParam {
                    name: "b".to_string(),
                    param_type: PinType::Real,
                    default: None,
                    description: None,
                },
            ],
            outputs: vec![FunctionParam {
                name: "result".to_string(),
                param_type: PinType::Real,
                default: None,
                description: None,
            }],
            pure: true,
            nodes: vec![
                BlueprintNode {
                    id: FUNCTION_ENTRY_NODE.to_string(),
                    node_type: "neo/FunctionEntry".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
                BlueprintNode {
                    id: "add_node".to_string(),
                    node_type: "neo/Add".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
                BlueprintNode {
                    id: FUNCTION_EXIT_NODE.to_string(),
                    node_type: "neo/FunctionExit".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
            ],
            connections: vec![
                // Entry outputs -> Add inputs
                Connection::new(FUNCTION_ENTRY_NODE, "a", "add_node", "a"),
                Connection::new(FUNCTION_ENTRY_NODE, "b", "add_node", "b"),
                // Add output -> Exit input
                Connection::new("add_node", "result", FUNCTION_EXIT_NODE, "result"),
            ],
        };

        // Create a blueprint with this function
        let mut blueprint = Blueprint {
            id: "function-test".to_string(),
            name: "Function Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            service: None,
            variables: HashMap::new(),
            nodes: vec![],
            connections: vec![],
            functions: HashMap::new(),
            imports: vec![],
            exports: vec!["add".to_string()],
            implements: vec![],
        };
        blueprint.functions.insert("add".to_string(), add_function.clone());

        let blueprint = Arc::new(blueprint);

        // Execute the function directly with inputs: a=5, b=3
        let mut inputs = HashMap::new();
        inputs.insert("a".to_string(), serde_json::json!(5.0));
        inputs.insert("b".to_string(), serde_json::json!(3.0));

        let result = executor
            .execute_function(Arc::clone(&blueprint), &add_function, inputs)
            .await;

        match result {
            Ok(outputs) => {
                let result_value = outputs.get("result").expect("Should have result output");
                assert_eq!(result_value.as_f64(), Some(8.0), "5 + 3 should equal 8");
                println!("Function executed successfully: 5 + 3 = {}", result_value);
            }
            Err(e) => {
                panic!("Function execution failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_function_with_call() {
        use crate::blueprints::types::{FunctionDef, FunctionParam, FUNCTION_ENTRY_NODE, FUNCTION_EXIT_NODE};

        let registry = Arc::new(NodeRegistry::with_builtins());
        let executor = BlueprintExecutor::new(registry);

        // Create a function that calculates error: error(current, setpoint) -> diff
        let error_function = FunctionDef {
            name: Some("calculate_error".to_string()),
            description: Some("Calculate the difference between current and setpoint".to_string()),
            inputs: vec![
                FunctionParam {
                    name: "current".to_string(),
                    param_type: PinType::Real,
                    default: None,
                    description: None,
                },
                FunctionParam {
                    name: "setpoint".to_string(),
                    param_type: PinType::Real,
                    default: None,
                    description: None,
                },
            ],
            outputs: vec![FunctionParam {
                name: "error".to_string(),
                param_type: PinType::Real,
                default: None,
                description: None,
            }],
            pure: true,
            nodes: vec![
                BlueprintNode {
                    id: FUNCTION_ENTRY_NODE.to_string(),
                    node_type: "neo/FunctionEntry".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
                BlueprintNode {
                    id: "subtract".to_string(),
                    node_type: "neo/Subtract".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
                BlueprintNode {
                    id: FUNCTION_EXIT_NODE.to_string(),
                    node_type: "neo/FunctionExit".to_string(),
                    position: Default::default(),
                    config: Value::Null,
                },
            ],
            connections: vec![
                Connection::new(FUNCTION_ENTRY_NODE, "current", "subtract", "a"),
                Connection::new(FUNCTION_ENTRY_NODE, "setpoint", "subtract", "b"),
                Connection::new("subtract", "result", FUNCTION_EXIT_NODE, "error"),
            ],
        };

        // Create blueprint with the function
        let mut blueprint = Blueprint {
            id: "error-calc-test".to_string(),
            name: "Error Calculation Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            service: None,
            variables: HashMap::new(),
            nodes: vec![],
            connections: vec![],
            functions: HashMap::new(),
            imports: vec![],
            exports: vec!["calculate_error".to_string()],
            implements: vec![],
        };
        blueprint.functions.insert("calculate_error".to_string(), error_function.clone());

        let blueprint = Arc::new(blueprint);

        // Test: current=75.5, setpoint=72.0 -> error=3.5
        let mut inputs = HashMap::new();
        inputs.insert("current".to_string(), serde_json::json!(75.5));
        inputs.insert("setpoint".to_string(), serde_json::json!(72.0));

        let result = executor
            .execute_function(Arc::clone(&blueprint), &error_function, inputs)
            .await;

        match result {
            Ok(outputs) => {
                let error_value = outputs.get("error").expect("Should have error output");
                let error = error_value.as_f64().expect("Should be a number");
                assert!((error - 3.5).abs() < 0.001, "75.5 - 72.0 should equal 3.5, got {}", error);
                println!("Error calculation: 75.5 - 72.0 = {}", error);
            }
            Err(e) => {
                panic!("Function execution failed: {}", e);
            }
        }
    }
}

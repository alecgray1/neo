// Node Registry - Stores node definitions and their executors
//
// The registry holds all available node types (built-in and plugin-provided).
// Each node type has a definition (pins, category, etc.) and an executor function.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::types::{LatentState, NodeDef, NodeResult, PinDef, PinType, WakeCondition};

// ─────────────────────────────────────────────────────────────────────────────
// Execution Context
// ─────────────────────────────────────────────────────────────────────────────

/// Context passed to node executors
pub struct NodeContext {
    /// Node instance ID
    pub node_id: String,
    /// Node configuration from blueprint JSON
    pub config: Value,
    /// Input values (pin_name -> value)
    pub inputs: HashMap<String, Value>,
    /// Blueprint variables (can be read/written)
    pub variables: HashMap<String, Value>,
}

impl NodeContext {
    /// Get an input value by pin name
    pub fn get_input(&self, name: &str) -> Option<&Value> {
        self.inputs.get(name)
    }

    /// Get input as f64
    pub fn get_input_real(&self, name: &str) -> Option<f64> {
        self.inputs.get(name).and_then(|v| v.as_f64())
    }

    /// Get input as i64
    pub fn get_input_integer(&self, name: &str) -> Option<i64> {
        self.inputs.get(name).and_then(|v| v.as_i64())
    }

    /// Get input as bool
    pub fn get_input_bool(&self, name: &str) -> Option<bool> {
        self.inputs.get(name).and_then(|v| v.as_bool())
    }

    /// Get input as string
    pub fn get_input_string(&self, name: &str) -> Option<&str> {
        self.inputs.get(name).and_then(|v| v.as_str())
    }

    /// Get a config value
    pub fn get_config(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }

    /// Get config as string
    pub fn get_config_string(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(|v| v.as_str())
    }

    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Set a variable value
    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node Executor Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Output from a node execution
pub struct NodeOutput {
    /// Output values (pin_name -> value)
    pub values: HashMap<String, Value>,
    /// Result of execution (which exec pin to follow, etc.)
    pub result: NodeResult,
}

impl NodeOutput {
    /// Create output that continues to the default "exec" pin
    pub fn continue_default(values: HashMap<String, Value>) -> Self {
        Self {
            values,
            result: NodeResult::Continue("exec".to_string()),
        }
    }

    /// Create output that continues to a specific exec pin
    pub fn continue_to(exec_pin: &str, values: HashMap<String, Value>) -> Self {
        Self {
            values,
            result: NodeResult::Continue(exec_pin.to_string()),
        }
    }

    /// Create output that ends execution (no more exec flow)
    pub fn end(values: HashMap<String, Value>) -> Self {
        Self {
            values,
            result: NodeResult::End,
        }
    }

    /// Create output for a pure node (just values, no exec flow)
    pub fn pure(values: HashMap<String, Value>) -> Self {
        Self {
            values,
            result: NodeResult::End,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            values: HashMap::new(),
            result: NodeResult::Error(message.into()),
        }
    }
}

/// Trait for node execution
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Execute the node with the given context
    async fn execute(&self, ctx: &mut NodeContext) -> NodeOutput;
}

/// Function-based node executor (for simple nodes)
pub struct FnNodeExecutor<F>
where
    F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync,
{
    func: F,
}

impl<F> FnNodeExecutor<F>
where
    F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[async_trait]
impl<F> NodeExecutor for FnNodeExecutor<F>
where
    F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync,
{
    async fn execute(&self, ctx: &mut NodeContext) -> NodeOutput {
        (self.func)(ctx)
    }
}

/// Async function-based node executor
#[allow(dead_code)]
pub struct AsyncFnNodeExecutor<F, Fut>
where
    F: Fn(NodeContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = NodeOutput> + Send,
{
    func: F,
    _phantom: std::marker::PhantomData<Fut>,
}

impl<F, Fut> AsyncFnNodeExecutor<F, Fut>
where
    F: Fn(NodeContext) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = NodeOutput> + Send,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: std::marker::PhantomData,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node Registry
// ─────────────────────────────────────────────────────────────────────────────

/// Entry in the node registry
struct NodeEntry {
    definition: NodeDef,
    executor: Arc<dyn NodeExecutor>,
}

/// Registry of all available node types
pub struct NodeRegistry {
    nodes: HashMap<String, NodeEntry>,
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Create a registry with built-in nodes registered
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        register_builtin_nodes(&mut registry);
        registry
    }

    /// Register a node type with its executor
    pub fn register(&mut self, definition: NodeDef, executor: Arc<dyn NodeExecutor>) {
        let id = definition.id.clone();
        self.nodes.insert(id, NodeEntry {
            definition,
            executor,
        });
    }

    /// Register a node with a sync function executor
    pub fn register_fn<F>(&mut self, definition: NodeDef, func: F)
    where
        F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync + 'static,
    {
        self.register(definition, Arc::new(FnNodeExecutor::new(func)));
    }

    /// Get a node definition by ID
    pub fn get_definition(&self, id: &str) -> Option<&NodeDef> {
        self.nodes.get(id).map(|e| &e.definition)
    }

    /// Get a node executor by ID
    pub fn get_executor(&self, id: &str) -> Option<Arc<dyn NodeExecutor>> {
        self.nodes.get(id).map(|e| Arc::clone(&e.executor))
    }

    /// Get all registered node IDs
    pub fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.nodes.keys().map(|s| s.as_str())
    }

    /// Get all node definitions
    pub fn definitions(&self) -> impl Iterator<Item = &NodeDef> {
        self.nodes.values().map(|e| &e.definition)
    }

    /// Get nodes by category
    pub fn nodes_in_category(&self, category: &str) -> Vec<&NodeDef> {
        self.nodes
            .values()
            .filter(|e| e.definition.category == category)
            .map(|e| &e.definition)
            .collect()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<_> = self
            .nodes
            .values()
            .map(|e| e.definition.category.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Built-in Nodes
// ─────────────────────────────────────────────────────────────────────────────

/// Register all built-in nodes
fn register_builtin_nodes(registry: &mut NodeRegistry) {
    // Flow Control
    register_branch(registry);
    register_sequence(registry);

    // Logic
    register_compare(registry);
    register_logic_gates(registry);

    // Math
    register_math_nodes(registry);

    // Utilities
    register_log(registry);
    register_variable_nodes(registry);

    // Latent (async) nodes
    register_latent_nodes(registry);
}

fn register_branch(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/Branch".to_string(),
        name: "Branch".to_string(),
        category: "Flow Control".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("condition", PinType::Boolean),
            PinDef::exec_out("true"),
            PinDef::exec_out("false"),
        ],
        description: Some("Branch execution based on a boolean condition".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let condition = ctx.get_input_bool("condition").unwrap_or(false);
        let exec_pin = if condition { "true" } else { "false" };
        NodeOutput::continue_to(exec_pin, HashMap::new())
    });
}

fn register_sequence(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/Sequence".to_string(),
        name: "Sequence".to_string(),
        category: "Flow Control".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::exec_out("then_0"),
            PinDef::exec_out("then_1"),
            PinDef::exec_out("then_2"),
            PinDef::exec_out("then_3"),
        ],
        description: Some("Execute multiple branches in sequence".to_string()),
    };

    // Note: Sequence is special - the executor handles running all branches
    // For now, we just continue to the first one (executor will handle the rest)
    registry.register_fn(def, |_ctx| {
        NodeOutput::continue_to("then_0", HashMap::new())
    });
}

fn register_compare(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/Compare".to_string(),
        name: "Compare".to_string(),
        category: "Logic".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("a", PinType::Any),
            PinDef::data_in("b", PinType::Any),
            PinDef::data_out("result", PinType::Boolean),
        ],
        description: Some("Compare two values. Configure operator via config.operator".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let a = ctx.get_input("a");
        let b = ctx.get_input("b");
        let operator = ctx.get_config_string("operator").unwrap_or("==");

        let result = match (a, b) {
            (Some(a), Some(b)) => {
                // Try numeric comparison first
                if let (Some(a_num), Some(b_num)) = (a.as_f64(), b.as_f64()) {
                    match operator {
                        "==" => a_num == b_num,
                        "!=" => a_num != b_num,
                        "<" => a_num < b_num,
                        "<=" => a_num <= b_num,
                        ">" => a_num > b_num,
                        ">=" => a_num >= b_num,
                        _ => false,
                    }
                } else {
                    // Fall back to JSON equality
                    match operator {
                        "==" => a == b,
                        "!=" => a != b,
                        _ => false,
                    }
                }
            }
            _ => false,
        };

        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::Bool(result));
        NodeOutput::pure(values)
    });
}

fn register_logic_gates(registry: &mut NodeRegistry) {
    // AND
    let def = NodeDef {
        id: "neo/And".to_string(),
        name: "And".to_string(),
        category: "Logic".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("a", PinType::Boolean),
            PinDef::data_in("b", PinType::Boolean),
            PinDef::data_out("result", PinType::Boolean),
        ],
        description: Some("Logical AND of two boolean values".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let a = ctx.get_input_bool("a").unwrap_or(false);
        let b = ctx.get_input_bool("b").unwrap_or(false);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::Bool(a && b));
        NodeOutput::pure(values)
    });

    // OR
    let def = NodeDef {
        id: "neo/Or".to_string(),
        name: "Or".to_string(),
        category: "Logic".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("a", PinType::Boolean),
            PinDef::data_in("b", PinType::Boolean),
            PinDef::data_out("result", PinType::Boolean),
        ],
        description: Some("Logical OR of two boolean values".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let a = ctx.get_input_bool("a").unwrap_or(false);
        let b = ctx.get_input_bool("b").unwrap_or(false);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::Bool(a || b));
        NodeOutput::pure(values)
    });

    // NOT
    let def = NodeDef {
        id: "neo/Not".to_string(),
        name: "Not".to_string(),
        category: "Logic".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("value", PinType::Boolean),
            PinDef::data_out("result", PinType::Boolean),
        ],
        description: Some("Logical NOT of a boolean value".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let value = ctx.get_input_bool("value").unwrap_or(false);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::Bool(!value));
        NodeOutput::pure(values)
    });
}

fn register_math_nodes(registry: &mut NodeRegistry) {
    // Add
    let def = NodeDef {
        id: "neo/Add".to_string(),
        name: "Add".to_string(),
        category: "Math".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("a", PinType::Real),
            PinDef::data_in("b", PinType::Real),
            PinDef::data_out("result", PinType::Real),
        ],
        description: Some("Add two numbers".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let a = ctx.get_input_real("a").unwrap_or(0.0);
        let b = ctx.get_input_real("b").unwrap_or(0.0);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::from(a + b));
        NodeOutput::pure(values)
    });

    // Subtract
    let def = NodeDef {
        id: "neo/Subtract".to_string(),
        name: "Subtract".to_string(),
        category: "Math".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("a", PinType::Real),
            PinDef::data_in("b", PinType::Real),
            PinDef::data_out("result", PinType::Real),
        ],
        description: Some("Subtract b from a".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let a = ctx.get_input_real("a").unwrap_or(0.0);
        let b = ctx.get_input_real("b").unwrap_or(0.0);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::from(a - b));
        NodeOutput::pure(values)
    });

    // Multiply
    let def = NodeDef {
        id: "neo/Multiply".to_string(),
        name: "Multiply".to_string(),
        category: "Math".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("a", PinType::Real),
            PinDef::data_in("b", PinType::Real),
            PinDef::data_out("result", PinType::Real),
        ],
        description: Some("Multiply two numbers".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let a = ctx.get_input_real("a").unwrap_or(0.0);
        let b = ctx.get_input_real("b").unwrap_or(0.0);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::from(a * b));
        NodeOutput::pure(values)
    });

    // Divide
    let def = NodeDef {
        id: "neo/Divide".to_string(),
        name: "Divide".to_string(),
        category: "Math".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("a", PinType::Real),
            PinDef::data_in("b", PinType::Real),
            PinDef::data_out("result", PinType::Real),
        ],
        description: Some("Divide a by b".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let a = ctx.get_input_real("a").unwrap_or(0.0);
        let b = ctx.get_input_real("b").unwrap_or(1.0);
        let result = if b != 0.0 { a / b } else { 0.0 };
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::from(result));
        NodeOutput::pure(values)
    });

    // Clamp
    let def = NodeDef {
        id: "neo/Clamp".to_string(),
        name: "Clamp".to_string(),
        category: "Math".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("value", PinType::Real),
            PinDef::data_in_with_default("min", PinType::Real, Value::from(0.0)),
            PinDef::data_in_with_default("max", PinType::Real, Value::from(1.0)),
            PinDef::data_out("result", PinType::Real),
        ],
        description: Some("Clamp a value between min and max".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let value = ctx.get_input_real("value").unwrap_or(0.0);
        let min = ctx.get_input_real("min").unwrap_or(0.0);
        let max = ctx.get_input_real("max").unwrap_or(1.0);
        let result = value.clamp(min, max);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::from(result));
        NodeOutput::pure(values)
    });
}

fn register_log(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/Log".to_string(),
        name: "Log".to_string(),
        category: "Utilities".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("message", PinType::String),
            PinDef::exec_out("exec"),
        ],
        description: Some("Log a message to the console".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let message = ctx.get_input_string("message").unwrap_or("");
        let level = ctx.get_config_string("level").unwrap_or("info");

        match level {
            "error" => tracing::error!(target: "blueprint", "{}", message),
            "warn" => tracing::warn!(target: "blueprint", "{}", message),
            "debug" => tracing::debug!(target: "blueprint", "{}", message),
            _ => tracing::info!(target: "blueprint", "{}", message),
        }

        NodeOutput::continue_default(HashMap::new())
    });
}

fn register_variable_nodes(registry: &mut NodeRegistry) {
    // Get Variable
    let def = NodeDef {
        id: "neo/GetVariable".to_string(),
        name: "Get Variable".to_string(),
        category: "Variables".to_string(),
        pure: true,
        latent: false,
        pins: vec![PinDef::data_out("value", PinType::Any)],
        description: Some("Get the value of a blueprint variable. Set variable name in config.variable".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let var_name = ctx.get_config_string("variable").unwrap_or("");
        let value = ctx.get_variable(var_name).cloned().unwrap_or(Value::Null);
        let mut values = HashMap::new();
        values.insert("value".to_string(), value);
        NodeOutput::pure(values)
    });

    // Set Variable
    let def = NodeDef {
        id: "neo/SetVariable".to_string(),
        name: "Set Variable".to_string(),
        category: "Variables".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("value", PinType::Any),
            PinDef::exec_out("exec"),
            PinDef::data_out("value", PinType::Any),
        ],
        description: Some("Set the value of a blueprint variable. Set variable name in config.variable".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let var_name = ctx.get_config_string("variable").unwrap_or("").to_string();
        let value = ctx.get_input("value").cloned().unwrap_or(Value::Null);
        ctx.set_variable(&var_name, value.clone());
        let mut values = HashMap::new();
        values.insert("value".to_string(), value);
        NodeOutput::continue_default(values)
    });
}

fn register_latent_nodes(registry: &mut NodeRegistry) {
    // Delay - pauses execution for a specified duration
    let def = NodeDef {
        id: "neo/Delay".to_string(),
        name: "Delay".to_string(),
        category: "Flow Control".to_string(),
        pure: false,
        latent: true,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in_with_default("duration_ms", PinType::Integer, Value::from(1000)),
            PinDef::exec_out("completed"),
        ],
        description: Some("Pause execution for a duration (in milliseconds), then continue".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let duration_ms = ctx.get_input_integer("duration_ms").unwrap_or(1000) as u64;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let wake_at = now_ms + duration_ms;

        NodeOutput {
            values: HashMap::new(),
            result: NodeResult::Latent(LatentState {
                node_id: ctx.node_id.clone(),
                resume_pin: "completed".to_string(),
                wake_condition: WakeCondition::Delay { until_ms: wake_at },
            }),
        }
    });

    // WaitForEvent - pauses until a specific event type occurs
    let def = NodeDef {
        id: "neo/WaitForEvent".to_string(),
        name: "Wait For Event".to_string(),
        category: "Flow Control".to_string(),
        pure: false,
        latent: true,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("event_type", PinType::String),
            PinDef::exec_out("received"),
            PinDef::data_out("event_data", PinType::Any),
        ],
        description: Some("Pause execution until a specific event type is received".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let event_type = ctx
            .get_input_string("event_type")
            .unwrap_or("unknown")
            .to_string();

        NodeOutput {
            values: HashMap::new(),
            result: NodeResult::Latent(LatentState {
                node_id: ctx.node_id.clone(),
                resume_pin: "received".to_string(),
                wake_condition: WakeCondition::Event {
                    event_type,
                    filter: None,
                },
            }),
        }
    });

    // WaitForPointChange - pauses until a point value changes
    let def = NodeDef {
        id: "neo/WaitForPointChange".to_string(),
        name: "Wait For Point Change".to_string(),
        category: "Flow Control".to_string(),
        pure: false,
        latent: true,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("point_path", PinType::String),
            PinDef::exec_out("changed"),
            PinDef::data_out("new_value", PinType::PointValue),
        ],
        description: Some("Pause execution until a specific point value changes".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let point_path = ctx
            .get_input_string("point_path")
            .unwrap_or("")
            .to_string();

        NodeOutput {
            values: HashMap::new(),
            result: NodeResult::Latent(LatentState {
                node_id: ctx.node_id.clone(),
                resume_pin: "changed".to_string(),
                wake_condition: WakeCondition::PointChanged {
                    point_path,
                    condition: None,
                },
            }),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_builtins() {
        let registry = NodeRegistry::with_builtins();

        // Check that built-in nodes are registered
        assert!(registry.get_definition("neo/Branch").is_some());
        assert!(registry.get_definition("neo/Compare").is_some());
        assert!(registry.get_definition("neo/Add").is_some());
        assert!(registry.get_definition("neo/Log").is_some());
    }

    #[test]
    fn test_categories() {
        let registry = NodeRegistry::with_builtins();
        let categories = registry.categories();

        assert!(categories.contains(&"Flow Control".to_string()));
        assert!(categories.contains(&"Logic".to_string()));
        assert!(categories.contains(&"Math".to_string()));
    }

    #[tokio::test]
    async fn test_compare_node() {
        let registry = NodeRegistry::with_builtins();
        let executor = registry.get_executor("neo/Compare").unwrap();

        let mut ctx = NodeContext {
            node_id: "test".to_string(),
            config: serde_json::json!({"operator": ">"}),
            inputs: {
                let mut m = HashMap::new();
                m.insert("a".to_string(), Value::from(10.0));
                m.insert("b".to_string(), Value::from(5.0));
                m
            },
            variables: HashMap::new(),
        };

        let output = executor.execute(&mut ctx).await;
        assert_eq!(output.values.get("result"), Some(&Value::Bool(true)));
    }

    #[tokio::test]
    async fn test_branch_node() {
        let registry = NodeRegistry::with_builtins();
        let executor = registry.get_executor("neo/Branch").unwrap();

        // Test true branch
        let mut ctx = NodeContext {
            node_id: "test".to_string(),
            config: Value::Null,
            inputs: {
                let mut m = HashMap::new();
                m.insert("condition".to_string(), Value::Bool(true));
                m
            },
            variables: HashMap::new(),
        };

        let output = executor.execute(&mut ctx).await;
        assert!(matches!(output.result, NodeResult::Continue(pin) if pin == "true"));

        // Test false branch
        ctx.inputs
            .insert("condition".to_string(), Value::Bool(false));
        let output = executor.execute(&mut ctx).await;
        assert!(matches!(output.result, NodeResult::Continue(pin) if pin == "false"));
    }

    #[tokio::test]
    async fn test_math_nodes() {
        let registry = NodeRegistry::with_builtins();

        let mut ctx = NodeContext {
            node_id: "test".to_string(),
            config: Value::Null,
            inputs: {
                let mut m = HashMap::new();
                m.insert("a".to_string(), Value::from(10.0));
                m.insert("b".to_string(), Value::from(3.0));
                m
            },
            variables: HashMap::new(),
        };

        // Add
        let executor = registry.get_executor("neo/Add").unwrap();
        let output = executor.execute(&mut ctx).await;
        assert_eq!(output.values.get("result"), Some(&Value::from(13.0)));

        // Subtract
        let executor = registry.get_executor("neo/Subtract").unwrap();
        let output = executor.execute(&mut ctx).await;
        assert_eq!(output.values.get("result"), Some(&Value::from(7.0)));

        // Multiply
        let executor = registry.get_executor("neo/Multiply").unwrap();
        let output = executor.execute(&mut ctx).await;
        assert_eq!(output.values.get("result"), Some(&Value::from(30.0)));
    }
}

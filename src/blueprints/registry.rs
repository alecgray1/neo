// Node Registry - Stores node definitions and their executors
//
// The registry holds all available node types (built-in and plugin-provided).
// Each node type has a definition (pins, category, etc.) and an executor function.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use rand::Rng;
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

/// Async function-based node executor (reserved for future use)
#[allow(dead_code)]
struct AsyncFnNodeExecutor<F, Fut>
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
    #[allow(dead_code)]
    fn new(func: F) -> Self {
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
    register_random_string(registry);

    // Struct operations
    register_struct_nodes(registry);

    // Function nodes
    register_function_nodes(registry);

    // Latent (async) nodes
    register_latent_nodes(registry);
    register_set_timer(registry);

    // Service integration nodes
    register_service_nodes(registry);
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

    // Abs (absolute value)
    let def = NodeDef {
        id: "neo/Abs".to_string(),
        name: "Absolute Value".to_string(),
        category: "Math".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("value", PinType::Real),
            PinDef::data_out("result", PinType::Real),
        ],
        description: Some("Return the absolute value of a number".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let value = ctx.get_input_real("value").unwrap_or(0.0);
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::from(value.abs()));
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

fn register_random_string(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/RandomString".to_string(),
        name: "Random String".to_string(),
        category: "Utilities".to_string(),
        pure: false, // Not pure - RNG produces different outputs each call
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in_with_default("length", PinType::Integer, Value::from(8)),
            PinDef::exec_out("exec"),
            PinDef::data_out("result", PinType::String),
        ],
        description: Some(
            "Generate a random string. Configure charset via config.charset (uppercase, lowercase, letters, alphanumeric, numeric, hex)".to_string()
        ),
    };

    registry.register_fn(def, |ctx| {
        let length = ctx.get_input_integer("length").unwrap_or(8) as usize;
        let charset = ctx.get_config_string("charset").unwrap_or("alphanumeric");

        let chars: Vec<char> = match charset {
            "uppercase" => ('A'..='Z').collect(),
            "lowercase" => ('a'..='z').collect(),
            "letters" => ('A'..='Z').chain('a'..='z').collect(),
            "alphanumeric" => ('A'..='Z').chain('a'..='z').chain('0'..='9').collect(),
            "numeric" => ('0'..='9').collect(),
            "hex" => ('0'..='9').chain('a'..='f').collect(),
            custom => custom.chars().collect(),
        };

        let result: String = if chars.is_empty() {
            String::new()
        } else {
            let mut rng = rand::thread_rng();
            (0..length)
                .map(|_| chars[rng.gen_range(0..chars.len())])
                .collect()
        };

        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::String(result));
        NodeOutput::continue_default(values)
    });
}

fn register_struct_nodes(registry: &mut NodeRegistry) {
    // CreateStruct - create a struct instance from field values
    let def = NodeDef {
        id: "neo/CreateStruct".to_string(),
        name: "Create Struct".to_string(),
        category: "Structs".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            // Dynamic pins based on struct_id config - for now just generic inputs
            PinDef::data_in("fields", PinType::Any),
            PinDef::data_out("struct", PinType::Any),
        ],
        description: Some(
            "Create a struct instance. Configure struct type via config.struct_id. Pass fields as JSON object.".to_string()
        ),
    };

    registry.register_fn(def, |ctx| {
        let fields = ctx.get_input("fields").cloned().unwrap_or(Value::Object(serde_json::Map::new()));

        // The struct is just a JSON object with a type marker
        let struct_id = ctx.get_config_string("struct_id").unwrap_or("unknown");
        let mut obj = match fields {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        obj.insert("__struct_type__".to_string(), Value::String(struct_id.to_string()));

        let mut values = HashMap::new();
        values.insert("struct".to_string(), Value::Object(obj));
        NodeOutput::pure(values)
    });

    // GetField - get a field value from a struct
    let def = NodeDef {
        id: "neo/GetField".to_string(),
        name: "Get Field".to_string(),
        category: "Structs".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("struct", PinType::Any),
            PinDef::data_out("value", PinType::Any),
        ],
        description: Some(
            "Get a field value from a struct. Configure field name via config.field".to_string()
        ),
    };

    registry.register_fn(def, |ctx| {
        let struct_val = ctx.get_input("struct");
        let field_name = ctx.get_config_string("field").unwrap_or("");

        let value = struct_val
            .and_then(|v| v.get(field_name))
            .cloned()
            .unwrap_or(Value::Null);

        let mut values = HashMap::new();
        values.insert("value".to_string(), value);
        NodeOutput::pure(values)
    });

    // SetField - create a new struct with a field value changed (immutable)
    let def = NodeDef {
        id: "neo/SetField".to_string(),
        name: "Set Field".to_string(),
        category: "Structs".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("struct", PinType::Any),
            PinDef::data_in("value", PinType::Any),
            PinDef::data_out("result", PinType::Any),
        ],
        description: Some(
            "Create a new struct with a field value changed. Configure field name via config.field".to_string()
        ),
    };

    registry.register_fn(def, |ctx| {
        let struct_val = ctx.get_input("struct").cloned().unwrap_or(Value::Null);
        let new_value = ctx.get_input("value").cloned().unwrap_or(Value::Null);
        let field_name = ctx.get_config_string("field").unwrap_or("");

        // Clone the struct and update the field
        let result = match struct_val {
            Value::Object(mut map) => {
                map.insert(field_name.to_string(), new_value);
                Value::Object(map)
            }
            _ => Value::Null,
        };

        let mut values = HashMap::new();
        values.insert("result".to_string(), result);
        NodeOutput::pure(values)
    });
}

fn register_function_nodes(registry: &mut NodeRegistry) {
    // FunctionEntry - entry point for a function, exposes input parameters as outputs
    let def = NodeDef {
        id: "neo/FunctionEntry".to_string(),
        name: "Function Entry".to_string(),
        category: "Functions".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            // Dynamic output pins based on function inputs - using Any for generic case
            PinDef::exec_out("exec"),
        ],
        description: Some(
            "Entry point for a function. Input parameters appear as output pins. This node's ID must be '__entry__'.".to_string()
        ),
    };

    // FunctionEntry just passes through - the executor sets up the input values
    registry.register_fn(def, |_ctx| {
        NodeOutput::continue_default(HashMap::new())
    });

    // FunctionExit - exit point for a function, collects output values
    let def = NodeDef {
        id: "neo/FunctionExit".to_string(),
        name: "Function Exit".to_string(),
        category: "Functions".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            // Dynamic input pins based on function outputs - using Any for generic case
        ],
        description: Some(
            "Exit point for a function. Output parameters appear as input pins. This node's ID must be '__exit__'.".to_string()
        ),
    };

    // FunctionExit ends execution - the executor collects the values
    registry.register_fn(def, |_ctx| {
        NodeOutput::end(HashMap::new())
    });

    // CallFunction - call a function defined in this blueprint
    let def = NodeDef {
        id: "neo/CallFunction".to_string(),
        name: "Call Function".to_string(),
        category: "Functions".to_string(),
        pure: false, // Can be pure if the function is pure
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::exec_out("exec"),
            // Dynamic pins based on function signature
        ],
        description: Some(
            "Call a function defined in this blueprint. Set function name via config.function".to_string()
        ),
    };

    // CallFunction execution is handled specially by the executor
    registry.register_fn(def, |ctx| {
        let function_name = ctx.get_config_string("function").unwrap_or("").to_string();

        let mut values = HashMap::new();
        values.insert("_call_function".to_string(), Value::String(function_name));

        // Collect all inputs to pass to the function
        for (key, value) in &ctx.inputs {
            if key != "exec" {
                values.insert(format!("_func_input_{}", key), value.clone());
            }
        }

        NodeOutput::continue_default(values)
    });

    // CallExternal - call a function from an imported blueprint
    let def = NodeDef {
        id: "neo/CallExternal".to_string(),
        name: "Call External".to_string(),
        category: "Functions".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::exec_out("exec"),
            // Dynamic pins based on function signature
        ],
        description: Some(
            "Call a function from an imported blueprint. Set blueprint ID via config.blueprint and function name via config.function".to_string()
        ),
    };

    // CallExternal execution is handled specially by the executor
    registry.register_fn(def, |ctx| {
        let blueprint_id = ctx.get_config_string("blueprint").unwrap_or("").to_string();
        let function_name = ctx.get_config_string("function").unwrap_or("").to_string();

        let mut values = HashMap::new();
        values.insert("_call_external_blueprint".to_string(), Value::String(blueprint_id));
        values.insert("_call_external_function".to_string(), Value::String(function_name));

        // Collect all inputs to pass to the function
        for (key, value) in &ctx.inputs {
            if key != "exec" {
                values.insert(format!("_func_input_{}", key), value.clone());
            }
        }

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

fn register_set_timer(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/SetTimer".to_string(),
        name: "Set Timer".to_string(),
        category: "Flow Control".to_string(),
        pure: false,
        latent: true,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in_with_default("interval_ms", PinType::Integer, Value::from(1000)),
            PinDef::exec_out("started"),
            PinDef::exec_out("tick"),
            PinDef::data_out("tick_count", PinType::Integer),
        ],
        description: Some(
            "Start a repeating timer that fires 'tick' every N milliseconds. Timer stops when service stops.".to_string()
        ),
    };

    registry.register_fn(def, |ctx| {
        let interval_ms = ctx.get_input_integer("interval_ms").unwrap_or(1000) as u64;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let first_tick = now_ms + interval_ms;
        let timer_id = format!("timer-{}-{}", ctx.node_id, now_ms);

        let mut values = HashMap::new();
        values.insert("tick_count".to_string(), Value::from(0));

        // Return latent state - the timer will be managed by tick_latent
        // First, continue to "started" to signal timer is set up
        // The executor will handle scheduling the interval ticks
        NodeOutput {
            values,
            result: NodeResult::Latent(LatentState {
                node_id: ctx.node_id.clone(),
                resume_pin: "tick".to_string(),
                wake_condition: WakeCondition::Interval {
                    interval_ms,
                    next_tick_ms: first_tick,
                    timer_id,
                    tick_count: 0,
                },
            }),
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Integration Nodes
// ─────────────────────────────────────────────────────────────────────────────

fn register_service_nodes(registry: &mut NodeRegistry) {
    // Event nodes (triggers)
    register_on_service_state_changed(registry);
    register_on_service_start(registry);
    register_on_service_stop(registry);
    register_on_service_request(registry);

    // Action nodes
    register_publish_event(registry);
    register_respond_to_request(registry);
    register_start_service(registry);
    register_stop_service(registry);
    register_get_service_status(registry);
}

/// OnServiceStateChanged - triggers when any service's state changes
fn register_on_service_state_changed(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/OnServiceStateChanged".to_string(),
        name: "On Service State Changed".to_string(),
        category: "Service Events".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_out("exec"),
            PinDef::data_out("service_name", PinType::String),
            PinDef::data_out("state", PinType::String),
        ],
        description: Some(
            "Triggered when a service state changes. Filter by service name via config.filter_service"
                .to_string(),
        ),
    };

    // This is an event node - outputs are set by the executor based on trigger data
    registry.register_fn(def, |_ctx| NodeOutput::continue_default(HashMap::new()));
}

/// OnServiceStart - triggers when this blueprint-service starts
fn register_on_service_start(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/OnServiceStart".to_string(),
        name: "On Service Start".to_string(),
        category: "Service Events".to_string(),
        pure: false,
        latent: false,
        pins: vec![PinDef::exec_out("exec")],
        description: Some(
            "Triggered when this blueprint service starts. Use to initialize resources.".to_string(),
        ),
    };

    registry.register_fn(def, |_ctx| NodeOutput::continue_default(HashMap::new()));
}

/// OnServiceStop - triggers when this blueprint-service stops
fn register_on_service_stop(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/OnServiceStop".to_string(),
        name: "On Service Stop".to_string(),
        category: "Service Events".to_string(),
        pure: false,
        latent: false,
        pins: vec![PinDef::exec_out("exec")],
        description: Some(
            "Triggered when this blueprint service stops. Use to cleanup resources.".to_string(),
        ),
    };

    registry.register_fn(def, |_ctx| NodeOutput::continue_default(HashMap::new()));
}

/// OnServiceRequest - triggers when a request is received
fn register_on_service_request(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/OnServiceRequest".to_string(),
        name: "On Service Request".to_string(),
        category: "Service Events".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_out("exec"),
            PinDef::data_out("request_id", PinType::String),
            PinDef::data_out("action", PinType::String),
            PinDef::data_out("payload", PinType::Any),
        ],
        description: Some(
            "Triggered when a service request is received. Use RespondToRequest to send a response."
                .to_string(),
        ),
    };

    registry.register_fn(def, |_ctx| NodeOutput::continue_default(HashMap::new()));
}

/// PublishEvent - emit a custom event to the PubSub system
fn register_publish_event(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/PublishEvent".to_string(),
        name: "Publish Event".to_string(),
        category: "Service Actions".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("event_type", PinType::String),
            PinDef::data_in_with_default("source", PinType::String, Value::String("blueprint".to_string())),
            PinDef::data_in_with_default("data", PinType::Any, Value::Object(serde_json::Map::new())),
            PinDef::exec_out("exec"),
        ],
        description: Some(
            "Publish a custom event to the system event bus. Other services can subscribe to it."
                .to_string(),
        ),
    };

    // The actual publishing is handled by the executor which has access to PubSub
    // This node just sets up the data - the executor reads it and publishes
    registry.register_fn(def, |ctx| {
        let event_type = ctx.get_input_string("event_type").unwrap_or("custom").to_string();
        let source = ctx.get_input_string("source").unwrap_or("blueprint").to_string();
        let data = ctx.get_input("data").cloned().unwrap_or(Value::Null);

        let mut values = HashMap::new();
        values.insert("_publish_event_type".to_string(), Value::String(event_type));
        values.insert("_publish_source".to_string(), Value::String(source));
        values.insert("_publish_data".to_string(), data);

        NodeOutput::continue_default(values)
    });
}

/// RespondToRequest - send a response for a service request
fn register_respond_to_request(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/RespondToRequest".to_string(),
        name: "Respond To Request".to_string(),
        category: "Service Actions".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("request_id", PinType::String),
            PinDef::data_in("response", PinType::Any),
            PinDef::data_in_with_default("success", PinType::Boolean, Value::Bool(true)),
            PinDef::exec_out("exec"),
        ],
        description: Some(
            "Send a response for a service request received via OnServiceRequest.".to_string(),
        ),
    };

    registry.register_fn(def, |ctx| {
        let request_id = ctx.get_input_string("request_id").unwrap_or("").to_string();
        let response = ctx.get_input("response").cloned().unwrap_or(Value::Null);
        let success = ctx.get_input_bool("success").unwrap_or(true);

        let mut values = HashMap::new();
        values.insert("_respond_request_id".to_string(), Value::String(request_id));
        values.insert("_respond_data".to_string(), response);
        values.insert("_respond_success".to_string(), Value::Bool(success));

        NodeOutput::continue_default(values)
    });
}

/// StartService - start another service by name
fn register_start_service(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/StartService".to_string(),
        name: "Start Service".to_string(),
        category: "Service Actions".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("service_id", PinType::String),
            PinDef::exec_out("success"),
            PinDef::exec_out("failed"),
            PinDef::data_out("error", PinType::String),
        ],
        description: Some("Start another service by its ID.".to_string()),
    };

    // The actual service control is handled by the executor
    registry.register_fn(def, |ctx| {
        let service_id = ctx.get_input_string("service_id").unwrap_or("").to_string();

        let mut values = HashMap::new();
        values.insert("_start_service_id".to_string(), Value::String(service_id));

        // Default to success - executor will modify if needed
        NodeOutput::continue_to("success", values)
    });
}

/// StopService - stop another service by name
fn register_stop_service(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/StopService".to_string(),
        name: "Stop Service".to_string(),
        category: "Service Actions".to_string(),
        pure: false,
        latent: false,
        pins: vec![
            PinDef::exec_in(),
            PinDef::data_in("service_id", PinType::String),
            PinDef::exec_out("success"),
            PinDef::exec_out("failed"),
            PinDef::data_out("error", PinType::String),
        ],
        description: Some("Stop another service by its ID.".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let service_id = ctx.get_input_string("service_id").unwrap_or("").to_string();

        let mut values = HashMap::new();
        values.insert("_stop_service_id".to_string(), Value::String(service_id));

        NodeOutput::continue_to("success", values)
    });
}

/// GetServiceStatus - query the status of a service
fn register_get_service_status(registry: &mut NodeRegistry) {
    let def = NodeDef {
        id: "neo/GetServiceStatus".to_string(),
        name: "Get Service Status".to_string(),
        category: "Service Actions".to_string(),
        pure: true,
        latent: false,
        pins: vec![
            PinDef::data_in("service_id", PinType::String),
            PinDef::data_out("state", PinType::String),
            PinDef::data_out("is_running", PinType::Boolean),
            PinDef::data_out("uptime_secs", PinType::Integer),
        ],
        description: Some("Get the current status of a service.".to_string()),
    };

    registry.register_fn(def, |ctx| {
        let service_id = ctx.get_input_string("service_id").unwrap_or("").to_string();

        let mut values = HashMap::new();
        values.insert("_get_status_service_id".to_string(), Value::String(service_id));
        // Default values - executor will populate actual values
        values.insert("state".to_string(), Value::String("unknown".to_string()));
        values.insert("is_running".to_string(), Value::Bool(false));
        values.insert("uptime_secs".to_string(), Value::from(0));

        NodeOutput::pure(values)
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

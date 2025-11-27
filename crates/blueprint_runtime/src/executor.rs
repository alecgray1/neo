// Executor - Node execution context and output types
//
// Provides the context passed to node executors and the output structure.

use std::collections::HashMap;

use serde_json::Value;

use blueprint_types::{LatentState, NodeResult};

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
    /// Create a new node context
    pub fn new(
        node_id: String,
        config: Value,
        inputs: HashMap<String, Value>,
        variables: HashMap<String, Value>,
    ) -> Self {
        Self {
            node_id,
            config,
            inputs,
            variables,
        }
    }

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

    /// Get input as array
    pub fn get_input_array(&self, name: &str) -> Option<&Vec<Value>> {
        self.inputs.get(name).and_then(|v| v.as_array())
    }

    /// Get input as object
    pub fn get_input_object(&self, name: &str) -> Option<&serde_json::Map<String, Value>> {
        self.inputs.get(name).and_then(|v| v.as_object())
    }

    /// Get a config value
    pub fn get_config(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }

    /// Get config as string
    pub fn get_config_string(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(|v| v.as_str())
    }

    /// Get config as bool
    pub fn get_config_bool(&self, key: &str) -> Option<bool> {
        self.config.get(key).and_then(|v| v.as_bool())
    }

    /// Get config as i64
    pub fn get_config_integer(&self, key: &str) -> Option<i64> {
        self.config.get(key).and_then(|v| v.as_i64())
    }

    /// Get a variable value
    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Set a variable value
    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Check if a variable exists
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node Output
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

    /// Create a latent output (execution suspended)
    pub fn latent(state: LatentState) -> Self {
        Self {
            values: HashMap::new(),
            result: NodeResult::Latent(state),
        }
    }

    /// Create a latent output with values
    pub fn latent_with_values(state: LatentState, values: HashMap<String, Value>) -> Self {
        Self {
            values,
            result: NodeResult::Latent(state),
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            values: HashMap::new(),
            result: NodeResult::Error(message.into()),
        }
    }

    /// Check if this output continues execution
    pub fn is_continue(&self) -> bool {
        matches!(self.result, NodeResult::Continue(_))
    }

    /// Check if this output ends execution
    pub fn is_end(&self) -> bool {
        matches!(self.result, NodeResult::End)
    }

    /// Check if this output is latent (suspended)
    pub fn is_latent(&self) -> bool {
        matches!(self.result, NodeResult::Latent(_))
    }

    /// Check if this output is an error
    pub fn is_error(&self) -> bool {
        matches!(self.result, NodeResult::Error(_))
    }

    /// Get the next exec pin if continuing
    pub fn next_exec_pin(&self) -> Option<&str> {
        match &self.result {
            NodeResult::Continue(pin) => Some(pin),
            _ => None,
        }
    }

    /// Get the error message if this is an error
    pub fn error_message(&self) -> Option<&str> {
        match &self.result {
            NodeResult::Error(msg) => Some(msg),
            _ => None,
        }
    }

    /// Get the latent state if suspended
    pub fn latent_state(&self) -> Option<&LatentState> {
        match &self.result {
            NodeResult::Latent(state) => Some(state),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_context_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert("a".to_string(), Value::from(42.0));
        inputs.insert("b".to_string(), Value::from("hello"));
        inputs.insert("c".to_string(), Value::from(true));

        let ctx = NodeContext::new(
            "test_node".to_string(),
            Value::Null,
            inputs,
            HashMap::new(),
        );

        assert_eq!(ctx.get_input_real("a"), Some(42.0));
        assert_eq!(ctx.get_input_string("b"), Some("hello"));
        assert_eq!(ctx.get_input_bool("c"), Some(true));
        assert_eq!(ctx.get_input_real("missing"), None);
    }

    #[test]
    fn test_node_context_config() {
        let config = serde_json::json!({
            "operator": "==",
            "enabled": true,
            "count": 5
        });

        let ctx = NodeContext::new(
            "test_node".to_string(),
            config,
            HashMap::new(),
            HashMap::new(),
        );

        assert_eq!(ctx.get_config_string("operator"), Some("=="));
        assert_eq!(ctx.get_config_bool("enabled"), Some(true));
        assert_eq!(ctx.get_config_integer("count"), Some(5));
    }

    #[test]
    fn test_node_context_variables() {
        let mut ctx = NodeContext::new(
            "test_node".to_string(),
            Value::Null,
            HashMap::new(),
            HashMap::new(),
        );

        assert!(!ctx.has_variable("counter"));

        ctx.set_variable("counter", Value::from(1));
        assert!(ctx.has_variable("counter"));
        assert_eq!(ctx.get_variable("counter"), Some(&Value::from(1)));
    }

    #[test]
    fn test_node_output_continue() {
        let output = NodeOutput::continue_default(HashMap::new());
        assert!(output.is_continue());
        assert_eq!(output.next_exec_pin(), Some("exec"));
    }

    #[test]
    fn test_node_output_continue_to() {
        let output = NodeOutput::continue_to("true", HashMap::new());
        assert!(output.is_continue());
        assert_eq!(output.next_exec_pin(), Some("true"));
    }

    #[test]
    fn test_node_output_error() {
        let output = NodeOutput::error("Something went wrong");
        assert!(output.is_error());
        assert_eq!(output.error_message(), Some("Something went wrong"));
    }

    #[test]
    fn test_node_output_pure() {
        let mut values = HashMap::new();
        values.insert("result".to_string(), Value::from(42));

        let output = NodeOutput::pure(values);
        assert!(output.is_end());
        assert_eq!(output.values.get("result"), Some(&Value::from(42)));
    }
}

//! Built-in Node Registration
//!
//! Registers all built-in blueprint nodes (math, logic, flow control, etc.)

use std::collections::HashMap;

use blueprint_runtime::{NodeOutput, NodeRegistry};
use blueprint_types::{NodeDef, PinDef, PinType};

/// Register all built-in nodes
pub fn register_builtin_nodes(registry: &mut NodeRegistry) {
    // Math nodes
    register_math_nodes(registry);

    // Logic nodes
    register_logic_nodes(registry);

    // Flow control nodes
    register_flow_nodes(registry);

    // Comparison nodes
    register_comparison_nodes(registry);

    // Utility nodes
    register_utility_nodes(registry);

    tracing::info!("Registered {} built-in nodes", registry.len());
}

// ─────────────────────────────────────────────────────────────────────────────
// Math Nodes
// ─────────────────────────────────────────────────────────────────────────────

fn register_math_nodes(registry: &mut NodeRegistry) {
    // Add
    registry.register_fn(
        NodeDef {
            id: "math/Add".to_string(),
            name: "Add".to_string(),
            category: "Math".to_string(),
            pure: true,
            latent: false,
            description: Some("Add two numbers".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Real),
                PinDef::data_in("b", PinType::Real),
                PinDef::data_out("result", PinType::Real),
            ],
        },
        |ctx| {
            let a = ctx.get_input_real("a").unwrap_or(0.0);
            let b = ctx.get_input_real("b").unwrap_or(0.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a + b));
            NodeOutput::pure(values)
        },
    );

    // Subtract
    registry.register_fn(
        NodeDef {
            id: "math/Subtract".to_string(),
            name: "Subtract".to_string(),
            category: "Math".to_string(),
            pure: true,
            latent: false,
            description: Some("Subtract two numbers".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Real),
                PinDef::data_in("b", PinType::Real),
                PinDef::data_out("result", PinType::Real),
            ],
        },
        |ctx| {
            let a = ctx.get_input_real("a").unwrap_or(0.0);
            let b = ctx.get_input_real("b").unwrap_or(0.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a - b));
            NodeOutput::pure(values)
        },
    );

    // Multiply
    registry.register_fn(
        NodeDef {
            id: "math/Multiply".to_string(),
            name: "Multiply".to_string(),
            category: "Math".to_string(),
            pure: true,
            latent: false,
            description: Some("Multiply two numbers".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Real),
                PinDef::data_in("b", PinType::Real),
                PinDef::data_out("result", PinType::Real),
            ],
        },
        |ctx| {
            let a = ctx.get_input_real("a").unwrap_or(0.0);
            let b = ctx.get_input_real("b").unwrap_or(0.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a * b));
            NodeOutput::pure(values)
        },
    );

    // Divide
    registry.register_fn(
        NodeDef {
            id: "math/Divide".to_string(),
            name: "Divide".to_string(),
            category: "Math".to_string(),
            pure: true,
            latent: false,
            description: Some("Divide two numbers".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Real),
                PinDef::data_in("b", PinType::Real),
                PinDef::data_out("result", PinType::Real),
            ],
        },
        |ctx| {
            let a = ctx.get_input_real("a").unwrap_or(0.0);
            let b = ctx.get_input_real("b").unwrap_or(1.0);
            let result = if b != 0.0 { a / b } else { 0.0 };
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(result));
            NodeOutput::pure(values)
        },
    );

    // Clamp
    registry.register_fn(
        NodeDef {
            id: "math/Clamp".to_string(),
            name: "Clamp".to_string(),
            category: "Math".to_string(),
            pure: true,
            latent: false,
            description: Some("Clamp a value between min and max".to_string()),
            pins: vec![
                PinDef::data_in("value", PinType::Real),
                PinDef::data_in("min", PinType::Real),
                PinDef::data_in("max", PinType::Real),
                PinDef::data_out("result", PinType::Real),
            ],
        },
        |ctx| {
            let value = ctx.get_input_real("value").unwrap_or(0.0);
            let min = ctx.get_input_real("min").unwrap_or(0.0);
            let max = ctx.get_input_real("max").unwrap_or(1.0);
            let result = value.clamp(min, max);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(result));
            NodeOutput::pure(values)
        },
    );

    // Abs
    registry.register_fn(
        NodeDef {
            id: "math/Abs".to_string(),
            name: "Absolute Value".to_string(),
            category: "Math".to_string(),
            pure: true,
            latent: false,
            description: Some("Get absolute value of a number".to_string()),
            pins: vec![
                PinDef::data_in("value", PinType::Real),
                PinDef::data_out("result", PinType::Real),
            ],
        },
        |ctx| {
            let value = ctx.get_input_real("value").unwrap_or(0.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(value.abs()));
            NodeOutput::pure(values)
        },
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Logic Nodes
// ─────────────────────────────────────────────────────────────────────────────

fn register_logic_nodes(registry: &mut NodeRegistry) {
    // AND
    registry.register_fn(
        NodeDef {
            id: "logic/And".to_string(),
            name: "AND".to_string(),
            category: "Logic".to_string(),
            pure: true,
            latent: false,
            description: Some("Logical AND operation".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Boolean),
                PinDef::data_in("b", PinType::Boolean),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let a = ctx.get_input_bool("a").unwrap_or(false);
            let b = ctx.get_input_bool("b").unwrap_or(false);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a && b));
            NodeOutput::pure(values)
        },
    );

    // OR
    registry.register_fn(
        NodeDef {
            id: "logic/Or".to_string(),
            name: "OR".to_string(),
            category: "Logic".to_string(),
            pure: true,
            latent: false,
            description: Some("Logical OR operation".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Boolean),
                PinDef::data_in("b", PinType::Boolean),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let a = ctx.get_input_bool("a").unwrap_or(false);
            let b = ctx.get_input_bool("b").unwrap_or(false);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a || b));
            NodeOutput::pure(values)
        },
    );

    // NOT
    registry.register_fn(
        NodeDef {
            id: "logic/Not".to_string(),
            name: "NOT".to_string(),
            category: "Logic".to_string(),
            pure: true,
            latent: false,
            description: Some("Logical NOT operation".to_string()),
            pins: vec![
                PinDef::data_in("value", PinType::Boolean),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let value = ctx.get_input_bool("value").unwrap_or(false);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(!value));
            NodeOutput::pure(values)
        },
    );

    // XOR
    registry.register_fn(
        NodeDef {
            id: "logic/Xor".to_string(),
            name: "XOR".to_string(),
            category: "Logic".to_string(),
            pure: true,
            latent: false,
            description: Some("Logical XOR operation".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Boolean),
                PinDef::data_in("b", PinType::Boolean),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let a = ctx.get_input_bool("a").unwrap_or(false);
            let b = ctx.get_input_bool("b").unwrap_or(false);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a ^ b));
            NodeOutput::pure(values)
        },
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Flow Control Nodes
// ─────────────────────────────────────────────────────────────────────────────

fn register_flow_nodes(registry: &mut NodeRegistry) {
    // Branch (If/Then/Else)
    registry.register_fn(
        NodeDef {
            id: "flow/Branch".to_string(),
            name: "Branch".to_string(),
            category: "Flow Control".to_string(),
            pure: false,
            latent: false,
            description: Some("Branch execution based on a condition".to_string()),
            pins: vec![
                PinDef::exec_in(),
                PinDef::data_in("condition", PinType::Boolean),
                PinDef::exec_out("true"),
                PinDef::exec_out("false"),
            ],
        },
        |ctx| {
            let condition = ctx.get_input_bool("condition").unwrap_or(false);
            if condition {
                NodeOutput::continue_to("true", HashMap::new())
            } else {
                NodeOutput::continue_to("false", HashMap::new())
            }
        },
    );

    // Sequence
    registry.register_fn(
        NodeDef {
            id: "flow/Sequence".to_string(),
            name: "Sequence".to_string(),
            category: "Flow Control".to_string(),
            pure: false,
            latent: false,
            description: Some("Execute multiple outputs in sequence".to_string()),
            pins: vec![
                PinDef::exec_in(),
                PinDef::exec_out("then_0"),
                PinDef::exec_out("then_1"),
            ],
        },
        |_ctx| {
            // In a full implementation, this would trigger multiple exec pins
            // For now, just continue to first
            NodeOutput::continue_to("then_0", HashMap::new())
        },
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Comparison Nodes
// ─────────────────────────────────────────────────────────────────────────────

fn register_comparison_nodes(registry: &mut NodeRegistry) {
    // Equal
    registry.register_fn(
        NodeDef {
            id: "compare/Equal".to_string(),
            name: "Equal".to_string(),
            category: "Comparison".to_string(),
            pure: true,
            latent: false,
            description: Some("Check if two values are equal".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Real),
                PinDef::data_in("b", PinType::Real),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let a = ctx.get_input_real("a").unwrap_or(0.0);
            let b = ctx.get_input_real("b").unwrap_or(0.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!((a - b).abs() < f64::EPSILON));
            NodeOutput::pure(values)
        },
    );

    // Greater Than
    registry.register_fn(
        NodeDef {
            id: "compare/Greater".to_string(),
            name: "Greater Than".to_string(),
            category: "Comparison".to_string(),
            pure: true,
            latent: false,
            description: Some("Check if A is greater than B".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Real),
                PinDef::data_in("b", PinType::Real),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let a = ctx.get_input_real("a").unwrap_or(0.0);
            let b = ctx.get_input_real("b").unwrap_or(0.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a > b));
            NodeOutput::pure(values)
        },
    );

    // Less Than
    registry.register_fn(
        NodeDef {
            id: "compare/Less".to_string(),
            name: "Less Than".to_string(),
            category: "Comparison".to_string(),
            pure: true,
            latent: false,
            description: Some("Check if A is less than B".to_string()),
            pins: vec![
                PinDef::data_in("a", PinType::Real),
                PinDef::data_in("b", PinType::Real),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let a = ctx.get_input_real("a").unwrap_or(0.0);
            let b = ctx.get_input_real("b").unwrap_or(0.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(a < b));
            NodeOutput::pure(values)
        },
    );

    // In Range
    registry.register_fn(
        NodeDef {
            id: "compare/InRange".to_string(),
            name: "In Range".to_string(),
            category: "Comparison".to_string(),
            pure: true,
            latent: false,
            description: Some("Check if value is within a range".to_string()),
            pins: vec![
                PinDef::data_in("value", PinType::Real),
                PinDef::data_in("min", PinType::Real),
                PinDef::data_in("max", PinType::Real),
                PinDef::data_out("result", PinType::Boolean),
            ],
        },
        |ctx| {
            let value = ctx.get_input_real("value").unwrap_or(0.0);
            let min = ctx.get_input_real("min").unwrap_or(0.0);
            let max = ctx.get_input_real("max").unwrap_or(100.0);
            let mut values = HashMap::new();
            values.insert("result".to_string(), serde_json::json!(value >= min && value <= max));
            NodeOutput::pure(values)
        },
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility Nodes
// ─────────────────────────────────────────────────────────────────────────────

fn register_utility_nodes(registry: &mut NodeRegistry) {
    // Constant - outputs a configured value
    registry.register_fn(
        NodeDef {
            id: "utility/Constant".to_string(),
            name: "Constant".to_string(),
            category: "Utility".to_string(),
            pure: true,
            latent: false,
            description: Some("Output a constant value".to_string()),
            pins: vec![
                PinDef::data_out("value", PinType::Any),
            ],
        },
        |ctx| {
            let mut values = HashMap::new();
            // Get the value from config
            if let Some(value) = ctx.get_config("value") {
                values.insert("value".to_string(), value.clone());
            }
            NodeOutput::pure(values)
        },
    );

    // Print/Log
    registry.register_fn(
        NodeDef {
            id: "utility/Print".to_string(),
            name: "Print".to_string(),
            category: "Utility".to_string(),
            pure: false,
            latent: false,
            description: Some("Print a message to the log".to_string()),
            pins: vec![
                PinDef::exec_in(),
                PinDef::data_in("message", PinType::String),
                PinDef::exec_out("then"),
            ],
        },
        |ctx| {
            // Convert any input type to string for display
            let message = match ctx.inputs.get("message") {
                Some(v) if v.is_string() => v.as_str().unwrap().to_string(),
                Some(v) if v.is_number() => v.to_string(),
                Some(v) if v.is_boolean() => v.to_string(),
                Some(v) if v.is_null() => "null".to_string(),
                Some(v) => v.to_string(), // Arrays, objects -> JSON
                None => "(empty)".to_string(),
            };
            tracing::info!(node_id = %ctx.node_id, "Blueprint: {}", message);
            NodeOutput::continue_to("then", HashMap::new())
        },
    );

    // Set Variable
    registry.register_fn(
        NodeDef {
            id: "utility/SetVariable".to_string(),
            name: "Set Variable".to_string(),
            category: "Utility".to_string(),
            pure: false,
            latent: false,
            description: Some("Set a blueprint variable".to_string()),
            pins: vec![
                PinDef::exec_in(),
                PinDef::data_in("value", PinType::Any),
                PinDef::exec_out("then"),
            ],
        },
        |ctx| {
            // Variable name comes from config
            if let Some(var_name) = ctx.get_config_string("variable") {
                if let Some(value) = ctx.get_input("value") {
                    let mut variables = ctx.variables.clone();
                    variables.insert(var_name.to_string(), value.clone());
                    // Note: In a full implementation, we'd need to propagate variable changes
                }
            }
            NodeOutput::continue_to("then", HashMap::new())
        },
    );

    // Get Variable
    registry.register_fn(
        NodeDef {
            id: "utility/GetVariable".to_string(),
            name: "Get Variable".to_string(),
            category: "Utility".to_string(),
            pure: true,
            latent: false,
            description: Some("Get a blueprint variable".to_string()),
            pins: vec![
                PinDef::data_out("value", PinType::Any),
            ],
        },
        |ctx| {
            let mut values = HashMap::new();
            if let Some(var_name) = ctx.get_config_string("variable") {
                if let Some(value) = ctx.get_variable(var_name) {
                    values.insert("value".to_string(), value.clone());
                }
            }
            NodeOutput::pure(values)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use blueprint_runtime::NodeContext;

    #[test]
    fn test_register_builtin_nodes() {
        let mut registry = NodeRegistry::new();
        register_builtin_nodes(&mut registry);

        // Check some nodes exist
        assert!(registry.contains("math/Add"));
        assert!(registry.contains("logic/And"));
        assert!(registry.contains("flow/Branch"));
        assert!(registry.contains("compare/Greater"));
        assert!(registry.contains("utility/Print"));
    }

    #[test]
    fn test_add_node() {
        let mut registry = NodeRegistry::new();
        register_builtin_nodes(&mut registry);

        let executor = registry.get_executor("math/Add").unwrap();
        let mut ctx = NodeContext::new(
            "test".to_string(),
            serde_json::Value::Null,
            {
                let mut inputs = HashMap::new();
                inputs.insert("a".to_string(), serde_json::json!(5.0));
                inputs.insert("b".to_string(), serde_json::json!(3.0));
                inputs
            },
            HashMap::new(),
        );

        let output = futures::executor::block_on(executor.execute(&mut ctx));
        assert_eq!(output.values.get("result"), Some(&serde_json::json!(8.0)));
    }
}

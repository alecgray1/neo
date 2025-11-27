// Blueprint Functions - Reusable callable subgraphs
//
// Functions are mini-blueprints within a blueprint that can be called like nodes.
// They encapsulate reusable logic with defined inputs and outputs.

use super::types::{FunctionDef, FUNCTION_ENTRY_NODE, FUNCTION_EXIT_NODE};

// ─────────────────────────────────────────────────────────────────────────────
// Function Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Validation error for function definitions
#[derive(Debug, Clone)]
pub struct FunctionValidationError {
    pub function_name: String,
    pub errors: Vec<String>,
}

impl std::fmt::Display for FunctionValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Function '{}' validation failed: {}",
            self.function_name,
            self.errors.join(", ")
        )
    }
}

impl std::error::Error for FunctionValidationError {}

/// Validate a function definition
pub fn validate_function(name: &str, func: &FunctionDef) -> Result<(), FunctionValidationError> {
    let mut errors = Vec::new();

    // Check for entry node
    let has_entry = func.nodes.iter().any(|n| n.id == FUNCTION_ENTRY_NODE);
    if !has_entry {
        errors.push(format!(
            "Missing entry node '{}'. Add a node with id '{}'.",
            FUNCTION_ENTRY_NODE, FUNCTION_ENTRY_NODE
        ));
    }

    // Check for exit node
    let has_exit = func.nodes.iter().any(|n| n.id == FUNCTION_EXIT_NODE);
    if !has_exit {
        errors.push(format!(
            "Missing exit node '{}'. Add a node with id '{}'.",
            FUNCTION_EXIT_NODE, FUNCTION_EXIT_NODE
        ));
    }

    // Validate all connections reference valid nodes
    let node_ids: Vec<&str> = func.nodes.iter().map(|n| n.id.as_str()).collect();
    for conn in &func.connections {
        if let Some((from_node, _from_pin)) = conn.from_parts() {
            if !node_ids.contains(&from_node) {
                errors.push(format!(
                    "Connection from unknown node '{}' in '{}'",
                    from_node, conn.from
                ));
            }
        } else {
            errors.push(format!("Invalid connection 'from' format: {}", conn.from));
        }

        if let Some((to_node, _to_pin)) = conn.to_parts() {
            if !node_ids.contains(&to_node) {
                errors.push(format!(
                    "Connection to unknown node '{}' in '{}'",
                    to_node, conn.to
                ));
            }
        } else {
            errors.push(format!("Invalid connection 'to' format: {}", conn.to));
        }
    }

    // Check that pure functions don't have exec pins connections to entry/exit
    // (pure functions evaluate data flow only, no exec flow)
    if func.pure {
        for conn in &func.connections {
            if let Some((from_node, from_pin)) = conn.from_parts() {
                if from_node == FUNCTION_ENTRY_NODE && from_pin == "exec" {
                    errors.push(
                        "Pure functions should not have exec connections from entry node"
                            .to_string(),
                    );
                }
            }
            if let Some((to_node, to_pin)) = conn.to_parts() {
                if to_node == FUNCTION_EXIT_NODE && to_pin == "exec" {
                    errors.push(
                        "Pure functions should not have exec connections to exit node".to_string(),
                    );
                }
            }
        }
    }

    // Validate input parameters have unique names
    let input_names: Vec<&str> = func.inputs.iter().map(|p| p.name.as_str()).collect();
    let mut seen_inputs = std::collections::HashSet::new();
    for name in &input_names {
        if !seen_inputs.insert(*name) {
            errors.push(format!("Duplicate input parameter name: {}", name));
        }
    }

    // Validate output parameters have unique names
    let output_names: Vec<&str> = func.outputs.iter().map(|p| p.name.as_str()).collect();
    let mut seen_outputs = std::collections::HashSet::new();
    for name in &output_names {
        if !seen_outputs.insert(*name) {
            errors.push(format!("Duplicate output parameter name: {}", name));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(FunctionValidationError {
            function_name: name.to_string(),
            errors,
        })
    }
}

/// Validate all functions in a map
pub fn validate_all_functions(
    functions: &std::collections::HashMap<String, FunctionDef>,
) -> Result<(), Vec<FunctionValidationError>> {
    let mut all_errors = Vec::new();

    for (name, func) in functions {
        if let Err(e) = validate_function(name, func) {
            all_errors.push(e);
        }
    }

    if all_errors.is_empty() {
        Ok(())
    } else {
        Err(all_errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints::types::{BlueprintNode, Connection, FunctionParam, PinType, Position};

    fn make_basic_function() -> FunctionDef {
        FunctionDef {
            name: Some("test_func".to_string()),
            description: None,
            inputs: vec![FunctionParam {
                name: "a".to_string(),
                param_type: PinType::Real,
                default: None,
                description: None,
            }],
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
                    position: Position::default(),
                    config: serde_json::Value::Null,
                },
                BlueprintNode {
                    id: FUNCTION_EXIT_NODE.to_string(),
                    node_type: "neo/FunctionExit".to_string(),
                    position: Position::default(),
                    config: serde_json::Value::Null,
                },
            ],
            connections: vec![Connection {
                from: format!("{}.a", FUNCTION_ENTRY_NODE),
                to: format!("{}.result", FUNCTION_EXIT_NODE),
            }],
        }
    }

    #[test]
    fn test_valid_function() {
        let func = make_basic_function();
        assert!(validate_function("test", &func).is_ok());
    }

    #[test]
    fn test_missing_entry_node() {
        let mut func = make_basic_function();
        func.nodes.retain(|n| n.id != FUNCTION_ENTRY_NODE);

        let result = validate_function("test", &func);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("Missing entry node")));
    }

    #[test]
    fn test_missing_exit_node() {
        let mut func = make_basic_function();
        func.nodes.retain(|n| n.id != FUNCTION_EXIT_NODE);

        let result = validate_function("test", &func);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("Missing exit node")));
    }

    #[test]
    fn test_invalid_connection() {
        let mut func = make_basic_function();
        func.connections.push(Connection {
            from: "nonexistent.output".to_string(),
            to: format!("{}.input", FUNCTION_EXIT_NODE),
        });

        let result = validate_function("test", &func);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("unknown node")));
    }

    #[test]
    fn test_duplicate_input_names() {
        let mut func = make_basic_function();
        func.inputs.push(FunctionParam {
            name: "a".to_string(), // duplicate
            param_type: PinType::Integer,
            default: None,
            description: None,
        });

        let result = validate_function("test", &func);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.errors.iter().any(|e| e.contains("Duplicate input")));
    }
}

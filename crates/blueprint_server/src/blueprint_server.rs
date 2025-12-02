//! Blueprint Server - Actor-based service integration
//!
//! This crate provides the server-side components for blueprint execution,
//! including service adapters and integration with the neo actor system.
//!
//! The actual BlueprintService actor remains in neo since it's tightly

pub use blueprint_runtime;
pub use blueprint_types;

// Re-export commonly used types for convenience
pub use blueprint_runtime::{NodeContext, NodeExecutor, NodeOutput, NodeRegistry};
pub use blueprint_types::{
    Blueprint, BlueprintNode, Connection, ExecutionResult, ExecutionTrigger, FunctionDef,
    LatentState, NodeDef, NodeResult, PinDef, PinDirection, PinType, WakeCondition,
};

/// Version of the blueprint_server crate
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Information about a loaded blueprint (for API responses)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlueprintInfo {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Version string
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Number of nodes in the blueprint
    pub node_count: usize,
    /// Number of connections
    pub connection_count: usize,
    /// Path to the source file (if loaded from disk)
    pub file_path: Option<String>,
}

impl BlueprintInfo {
    /// Create info from a Blueprint
    pub fn from_blueprint(bp: &Blueprint, path: Option<&std::path::Path>) -> Self {
        Self {
            id: bp.id.clone(),
            name: bp.name.clone(),
            version: bp.version.clone(),
            description: bp.description.clone(),
            node_count: bp.nodes.len(),
            connection_count: bp.connections.len(),
            file_path: path.map(|p| p.display().to_string()),
        }
    }
}

/// Validation result for a blueprint
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the blueprint is valid
    pub valid: bool,
    /// List of errors found
    pub errors: Vec<String>,
    /// List of warnings
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn failed(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.valid = false;
        self.errors.push(error.into());
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Validate a blueprint's structure against a node registry
pub fn validate_blueprint(blueprint: &Blueprint, registry: &NodeRegistry) -> ValidationResult {
    let mut result = ValidationResult::ok();

    // Check that all node types exist in the registry
    for node in &blueprint.nodes {
        // Skip event nodes - they're handled specially
        if node.node_type.contains("/On") || node.node_type.ends_with("Event") {
            continue;
        }

        if registry.get_definition(&node.node_type).is_none() {
            result.add_error(format!(
                "Unknown node type '{}' in node '{}'",
                node.node_type, node.id
            ));
        }
    }

    // Check that all connections reference valid nodes
    for conn in &blueprint.connections {
        if let Some((from_node, _)) = conn.from_parts() {
            if blueprint.get_node(from_node).is_none() {
                result.add_error(format!(
                    "Connection references unknown source node '{}'",
                    from_node
                ));
            }
        }

        if let Some((to_node, _)) = conn.to_parts() {
            if blueprint.get_node(to_node).is_none() {
                result.add_error(format!(
                    "Connection references unknown target node '{}'",
                    to_node
                ));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_blueprint_info() {
        let bp = Blueprint::new("test-id", "Test Blueprint");
        let info = BlueprintInfo::from_blueprint(&bp, None);

        assert_eq!(info.id, "test-id");
        assert_eq!(info.name, "Test Blueprint");
        assert_eq!(info.node_count, 0);
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult::ok();
        assert!(result.valid);
        assert!(result.errors.is_empty());

        let mut result = ValidationResult::ok();
        result.add_error("Something went wrong");
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_validate_empty_blueprint() {
        let registry = NodeRegistry::new();
        let bp = Blueprint::new("test", "Test");

        let result = validate_blueprint(&bp, &registry);
        assert!(result.valid);
    }
}

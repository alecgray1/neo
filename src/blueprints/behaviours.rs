// Blueprint Behaviours - Contracts/Interfaces
//
// Behaviours define contracts that blueprints must implement.
// A blueprint declaring `implements: ["Controllable"]` must provide
// the functions defined in that behaviour.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::types::{Blueprint, FunctionDef, PinType};

// ─────────────────────────────────────────────────────────────────────────────
// Behaviour Definitions
// ─────────────────────────────────────────────────────────────────────────────

/// A parameter in a callback definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    #[serde(rename = "type")]
    pub param_type: PinType,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A callback definition (required function signature)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackDef {
    /// Callback function name
    pub name: String,
    /// Description of what this callback should do
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Required input parameters
    #[serde(default)]
    pub inputs: Vec<CallbackParam>,
    /// Required output parameters
    #[serde(default)]
    pub outputs: Vec<CallbackParam>,
}

/// A behaviour definition (interface/contract)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviourDef {
    /// Unique identifier (e.g., "controllable", "neo/schedulable")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Version string
    #[serde(default = "default_version")]
    pub version: String,
    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Required callbacks that implementers must provide
    #[serde(default)]
    pub callbacks: Vec<CallbackDef>,
    /// Optional callbacks (implementers may provide)
    #[serde(default)]
    pub optional_callbacks: Vec<CallbackDef>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation Errors
// ─────────────────────────────────────────────────────────────────────────────

/// A signature mismatch between callback and function
#[derive(Debug, Clone)]
pub struct SignatureMismatch {
    pub callback_name: String,
    pub expected: String,
    pub found: String,
}

impl std::fmt::Display for SignatureMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: expected {}, found {}",
            self.callback_name, self.expected, self.found
        )
    }
}

/// Validation error for behaviour compliance
#[derive(Debug, Clone)]
pub struct BehaviourViolation {
    pub behaviour_id: String,
    pub missing_callbacks: Vec<String>,
    pub signature_mismatches: Vec<SignatureMismatch>,
    pub error: Option<String>,
}

impl std::fmt::Display for BehaviourViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();

        if let Some(ref err) = self.error {
            parts.push(err.clone());
        }

        if !self.missing_callbacks.is_empty() {
            parts.push(format!("missing: [{}]", self.missing_callbacks.join(", ")));
        }

        if !self.signature_mismatches.is_empty() {
            let mismatches: Vec<String> =
                self.signature_mismatches.iter().map(|m| m.to_string()).collect();
            parts.push(format!("signature errors: [{}]", mismatches.join("; ")));
        }

        write!(f, "Behaviour '{}': {}", self.behaviour_id, parts.join(", "))
    }
}

impl std::error::Error for BehaviourViolation {}

// ─────────────────────────────────────────────────────────────────────────────
// Behaviour Registry
// ─────────────────────────────────────────────────────────────────────────────

/// Registry of loaded behaviour definitions
#[derive(Debug, Default)]
pub struct BehaviourRegistry {
    behaviours: HashMap<String, BehaviourDef>,
}

impl BehaviourRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            behaviours: HashMap::new(),
        }
    }

    /// Register a behaviour definition
    pub fn register(&mut self, def: BehaviourDef) {
        self.behaviours.insert(def.id.clone(), def);
    }

    /// Get a behaviour definition by ID
    pub fn get(&self, id: &str) -> Option<&BehaviourDef> {
        self.behaviours.get(id)
    }

    /// Check if a behaviour is registered
    pub fn contains(&self, id: &str) -> bool {
        self.behaviours.contains_key(id)
    }

    /// Get all behaviour IDs
    pub fn behaviour_ids(&self) -> impl Iterator<Item = &str> {
        self.behaviours.keys().map(|s| s.as_str())
    }

    /// Load a behaviour definition from a JSON file
    pub fn load_from_file(&mut self, path: &Path) -> Result<String, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read behaviour file: {}", e))?;

        let def: BehaviourDef = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse behaviour JSON: {}", e))?;

        let id = def.id.clone();
        self.register(def);
        Ok(id)
    }

    /// Load all behaviour definitions from a directory
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<Vec<String>, String> {
        let mut loaded = Vec::new();

        if !dir.exists() {
            return Ok(loaded);
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read behaviours directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            // Only process .behaviour.json files
            if path.extension().map_or(false, |ext| ext == "json") {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename.ends_with(".behaviour.json") {
                    match self.load_from_file(&path) {
                        Ok(id) => {
                            tracing::info!("Loaded behaviour: {}", id);
                            loaded.push(id);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load behaviour from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Validate that a blueprint implements all declared behaviours
    pub fn validate_blueprint(&self, blueprint: &Blueprint) -> Result<(), Vec<BehaviourViolation>> {
        let mut violations = Vec::new();

        for behaviour_id in &blueprint.implements {
            if let Some(behaviour) = self.get(behaviour_id) {
                if let Err(violation) = self.check_compliance(blueprint, behaviour) {
                    violations.push(violation);
                }
            } else {
                violations.push(BehaviourViolation {
                    behaviour_id: behaviour_id.clone(),
                    missing_callbacks: vec![],
                    signature_mismatches: vec![],
                    error: Some(format!("Behaviour '{}' not found", behaviour_id)),
                });
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    /// Check if a blueprint complies with a behaviour
    fn check_compliance(
        &self,
        blueprint: &Blueprint,
        behaviour: &BehaviourDef,
    ) -> Result<(), BehaviourViolation> {
        let mut missing = Vec::new();
        let mut mismatches = Vec::new();

        for callback in &behaviour.callbacks {
            // Check if blueprint has a function with this name
            if let Some(func) = blueprint.functions.get(&callback.name) {
                // Check if it's exported
                if !blueprint.exports.contains(&callback.name) {
                    missing.push(format!("{} (defined but not exported)", callback.name));
                    continue;
                }

                // Check signature match
                if let Some(mismatch) = self.check_signature(callback, func) {
                    mismatches.push(mismatch);
                }
            } else {
                missing.push(callback.name.clone());
            }
        }

        if missing.is_empty() && mismatches.is_empty() {
            Ok(())
        } else {
            Err(BehaviourViolation {
                behaviour_id: behaviour.id.clone(),
                missing_callbacks: missing,
                signature_mismatches: mismatches,
                error: None,
            })
        }
    }

    /// Check if a function signature matches a callback definition
    fn check_signature(&self, callback: &CallbackDef, func: &FunctionDef) -> Option<SignatureMismatch> {
        // Check input count
        if callback.inputs.len() != func.inputs.len() {
            return Some(SignatureMismatch {
                callback_name: callback.name.clone(),
                expected: format!("{} inputs", callback.inputs.len()),
                found: format!("{} inputs", func.inputs.len()),
            });
        }

        // Check input types
        for (expected, found) in callback.inputs.iter().zip(func.inputs.iter()) {
            if !expected.param_type.is_compatible_with(&found.param_type) {
                return Some(SignatureMismatch {
                    callback_name: callback.name.clone(),
                    expected: format!("input '{}': {:?}", expected.name, expected.param_type),
                    found: format!("input '{}': {:?}", found.name, found.param_type),
                });
            }
        }

        // Check output count
        if callback.outputs.len() != func.outputs.len() {
            return Some(SignatureMismatch {
                callback_name: callback.name.clone(),
                expected: format!("{} outputs", callback.outputs.len()),
                found: format!("{} outputs", func.outputs.len()),
            });
        }

        // Check output types
        for (expected, found) in callback.outputs.iter().zip(func.outputs.iter()) {
            if !expected.param_type.is_compatible_with(&found.param_type) {
                return Some(SignatureMismatch {
                    callback_name: callback.name.clone(),
                    expected: format!("output '{}': {:?}", expected.name, expected.param_type),
                    found: format!("output '{}': {:?}", found.name, found.param_type),
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blueprints::types::FunctionParam;

    fn make_test_behaviour() -> BehaviourDef {
        BehaviourDef {
            id: "controllable".to_string(),
            name: "Controllable".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Interface for controllable equipment".to_string()),
            callbacks: vec![
                CallbackDef {
                    name: "get_status".to_string(),
                    description: None,
                    inputs: vec![],
                    outputs: vec![
                        CallbackParam {
                            name: "mode".to_string(),
                            param_type: PinType::String,
                            description: None,
                        },
                        CallbackParam {
                            name: "is_enabled".to_string(),
                            param_type: PinType::Boolean,
                            description: None,
                        },
                    ],
                },
                CallbackDef {
                    name: "set_mode".to_string(),
                    description: None,
                    inputs: vec![CallbackParam {
                        name: "mode".to_string(),
                        param_type: PinType::String,
                        description: None,
                    }],
                    outputs: vec![CallbackParam {
                        name: "success".to_string(),
                        param_type: PinType::Boolean,
                        description: None,
                    }],
                },
            ],
            optional_callbacks: vec![],
        }
    }

    fn make_compliant_blueprint() -> Blueprint {
        let mut blueprint = Blueprint::new("test-controller", "Test Controller");
        blueprint.implements = vec!["controllable".to_string()];
        blueprint.exports = vec!["get_status".to_string(), "set_mode".to_string()];
        blueprint.functions.insert(
            "get_status".to_string(),
            FunctionDef {
                name: Some("get_status".to_string()),
                description: None,
                inputs: vec![],
                outputs: vec![
                    FunctionParam {
                        name: "mode".to_string(),
                        param_type: PinType::String,
                        default: None,
                        description: None,
                    },
                    FunctionParam {
                        name: "is_enabled".to_string(),
                        param_type: PinType::Boolean,
                        default: None,
                        description: None,
                    },
                ],
                pure: true,
                nodes: vec![],
                connections: vec![],
            },
        );
        blueprint.functions.insert(
            "set_mode".to_string(),
            FunctionDef {
                name: Some("set_mode".to_string()),
                description: None,
                inputs: vec![FunctionParam {
                    name: "mode".to_string(),
                    param_type: PinType::String,
                    default: None,
                    description: None,
                }],
                outputs: vec![FunctionParam {
                    name: "success".to_string(),
                    param_type: PinType::Boolean,
                    default: None,
                    description: None,
                }],
                pure: false,
                nodes: vec![],
                connections: vec![],
            },
        );
        blueprint
    }

    #[test]
    fn test_behaviour_parsing() {
        let json = r#"{
            "id": "controllable",
            "name": "Controllable",
            "callbacks": [
                {
                    "name": "get_status",
                    "inputs": [],
                    "outputs": [
                        { "name": "mode", "type": { "type": "String" } }
                    ]
                }
            ]
        }"#;

        let def: BehaviourDef = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "controllable");
        assert_eq!(def.callbacks.len(), 1);
        assert_eq!(def.callbacks[0].name, "get_status");
    }

    #[test]
    fn test_compliant_blueprint() {
        let mut registry = BehaviourRegistry::new();
        registry.register(make_test_behaviour());

        let blueprint = make_compliant_blueprint();
        let result = registry.validate_blueprint(&blueprint);

        assert!(result.is_ok(), "Expected compliant blueprint to pass validation");
    }

    #[test]
    fn test_missing_function() {
        let mut registry = BehaviourRegistry::new();
        registry.register(make_test_behaviour());

        let mut blueprint = make_compliant_blueprint();
        blueprint.functions.remove("get_status");
        blueprint.exports.retain(|e| e != "get_status");

        let result = registry.validate_blueprint(&blueprint);
        assert!(result.is_err());

        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 1);
        assert!(violations[0].missing_callbacks.contains(&"get_status".to_string()));
    }

    #[test]
    fn test_not_exported() {
        let mut registry = BehaviourRegistry::new();
        registry.register(make_test_behaviour());

        let mut blueprint = make_compliant_blueprint();
        blueprint.exports.retain(|e| e != "get_status");

        let result = registry.validate_blueprint(&blueprint);
        assert!(result.is_err());

        let violations = result.unwrap_err();
        assert!(violations[0]
            .missing_callbacks
            .iter()
            .any(|m| m.contains("not exported")));
    }

    #[test]
    fn test_signature_mismatch() {
        let mut registry = BehaviourRegistry::new();
        registry.register(make_test_behaviour());

        let mut blueprint = make_compliant_blueprint();
        // Change the output type
        if let Some(func) = blueprint.functions.get_mut("get_status") {
            func.outputs[0].param_type = PinType::Integer; // Should be String
        }

        let result = registry.validate_blueprint(&blueprint);
        assert!(result.is_err());

        let violations = result.unwrap_err();
        assert!(!violations[0].signature_mismatches.is_empty());
    }

    #[test]
    fn test_unknown_behaviour() {
        let registry = BehaviourRegistry::new(); // Empty registry

        let blueprint = make_compliant_blueprint();
        let result = registry.validate_blueprint(&blueprint);

        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations[0].error.is_some());
    }
}

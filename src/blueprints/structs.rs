// Blueprint Structs - User-defined data types
//
// Structs are typed data shapes that can be used in blueprints.
// They are defined in .struct.json files and loaded at runtime.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::PinType;

// ─────────────────────────────────────────────────────────────────────────────
// Struct Definitions
// ─────────────────────────────────────────────────────────────────────────────

/// A field within a struct definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructField {
    /// Field name (e.g., "zone_temp")
    pub name: String,
    /// Field type
    #[serde(rename = "type")]
    pub field_type: PinType,
    /// Default value for this field
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Units for display (e.g., "degF", "%")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
}

/// A struct definition - a named collection of typed fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructDef {
    /// Unique identifier (e.g., "vav-device", "neo/ahu-controller")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Version string
    #[serde(default = "default_version")]
    pub version: String,
    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The fields that make up this struct
    pub fields: Vec<StructField>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl StructDef {
    /// Get a field by name
    pub fn get_field(&self, name: &str) -> Option<&StructField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get field names
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }

    /// Create a default instance of this struct
    pub fn create_default_instance(&self) -> Value {
        let mut obj = serde_json::Map::new();
        for field in &self.fields {
            let value = field.default.clone().unwrap_or(Value::Null);
            obj.insert(field.name.clone(), value);
        }
        Value::Object(obj)
    }

    /// Validate a value against this struct definition
    pub fn validate_instance(&self, value: &Value) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        let obj = match value.as_object() {
            Some(obj) => obj,
            None => {
                errors.push("Value is not an object".to_string());
                return Err(errors);
            }
        };

        // Check all required fields are present
        for field in &self.fields {
            if !obj.contains_key(&field.name) && field.default.is_none() {
                errors.push(format!("Missing required field: {}", field.name));
            }
        }

        // Check field types (basic validation)
        for (key, val) in obj {
            if let Some(field) = self.get_field(key) {
                if let Err(e) = validate_value_type(val, &field.field_type) {
                    errors.push(format!("Field '{}': {}", key, e));
                }
            }
            // Unknown fields are allowed (forward compatibility)
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validate a value against an expected type
fn validate_value_type(value: &Value, expected: &PinType) -> Result<(), String> {
    match expected {
        PinType::Any => Ok(()),
        PinType::Real => {
            if value.is_f64() || value.is_i64() {
                Ok(())
            } else {
                Err("expected number".to_string())
            }
        }
        PinType::Integer => {
            if value.is_i64() {
                Ok(())
            } else {
                Err("expected integer".to_string())
            }
        }
        PinType::Boolean => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err("expected boolean".to_string())
            }
        }
        PinType::String => {
            if value.is_string() {
                Ok(())
            } else {
                Err("expected string".to_string())
            }
        }
        PinType::Array { element } => {
            if let Some(arr) = value.as_array() {
                for (i, item) in arr.iter().enumerate() {
                    if let Err(e) = validate_value_type(item, element) {
                        return Err(format!("element [{}]: {}", i, e));
                    }
                }
                Ok(())
            } else {
                Err("expected array".to_string())
            }
        }
        PinType::Struct { .. } => {
            // Struct validation requires the registry; skip deep validation here
            if value.is_object() {
                Ok(())
            } else {
                Err("expected object".to_string())
            }
        }
        PinType::PointValue => {
            // PointValue accepts various types
            if value.is_f64() || value.is_i64() || value.is_boolean() || value.is_string() {
                Ok(())
            } else {
                Err("expected point value (number, boolean, or string)".to_string())
            }
        }
        PinType::Exec => Err("exec type cannot have a value".to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Struct Registry
// ─────────────────────────────────────────────────────────────────────────────

/// Registry of loaded struct definitions
#[derive(Debug, Default)]
pub struct StructRegistry {
    /// Struct definitions by ID
    structs: HashMap<String, StructDef>,
}

impl StructRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            structs: HashMap::new(),
        }
    }

    /// Register a struct definition
    pub fn register(&mut self, def: StructDef) {
        self.structs.insert(def.id.clone(), def);
    }

    /// Get a struct definition by ID
    pub fn get(&self, id: &str) -> Option<&StructDef> {
        self.structs.get(id)
    }

    /// Check if a struct is registered
    pub fn contains(&self, id: &str) -> bool {
        self.structs.contains_key(id)
    }

    /// Get all struct IDs
    pub fn struct_ids(&self) -> impl Iterator<Item = &str> {
        self.structs.keys().map(|s| s.as_str())
    }

    /// Load a struct definition from a JSON file
    pub fn load_from_file(&mut self, path: &Path) -> Result<String, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read struct file: {}", e))?;

        let def: StructDef = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse struct JSON: {}", e))?;

        let id = def.id.clone();
        self.register(def);
        Ok(id)
    }

    /// Load all struct definitions from a directory
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<Vec<String>, String> {
        let mut loaded = Vec::new();

        if !dir.exists() {
            // Directory doesn't exist yet, that's okay
            return Ok(loaded);
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read structs directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            // Only process .struct.json files
            if path.extension().map_or(false, |ext| ext == "json") {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename.ends_with(".struct.json") {
                    match self.load_from_file(&path) {
                        Ok(id) => {
                            tracing::info!("Loaded struct: {}", id);
                            loaded.push(id);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load struct from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Validate a struct instance
    pub fn validate_instance(&self, struct_id: &str, value: &Value) -> Result<(), Vec<String>> {
        match self.get(struct_id) {
            Some(def) => def.validate_instance(value),
            None => Err(vec![format!("Unknown struct type: {}", struct_id)]),
        }
    }

    /// Create a default instance of a struct
    pub fn create_default_instance(&self, struct_id: &str) -> Option<Value> {
        self.get(struct_id).map(|def| def.create_default_instance())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct_def_parsing() {
        let json = r#"{
            "id": "vav-device",
            "name": "VAV Device",
            "fields": [
                { "name": "zone_temp", "type": { "type": "Real" }, "units": "degF" },
                { "name": "setpoint", "type": { "type": "Real" }, "default": 72.0 },
                { "name": "occupied", "type": { "type": "Boolean" }, "default": false }
            ]
        }"#;

        let def: StructDef = serde_json::from_str(json).unwrap();
        assert_eq!(def.id, "vav-device");
        assert_eq!(def.fields.len(), 3);
        assert_eq!(def.get_field("zone_temp").unwrap().units, Some("degF".to_string()));
    }

    #[test]
    fn test_struct_validation() {
        let def = StructDef {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            fields: vec![
                StructField {
                    name: "value".to_string(),
                    field_type: PinType::Real,
                    default: None,
                    description: None,
                    units: None,
                },
            ],
        };

        // Valid instance
        let valid = serde_json::json!({ "value": 42.0 });
        assert!(def.validate_instance(&valid).is_ok());

        // Missing required field
        let missing = serde_json::json!({});
        assert!(def.validate_instance(&missing).is_err());

        // Wrong type
        let wrong_type = serde_json::json!({ "value": "not a number" });
        assert!(def.validate_instance(&wrong_type).is_err());
    }

    #[test]
    fn test_default_instance() {
        let def = StructDef {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            fields: vec![
                StructField {
                    name: "temp".to_string(),
                    field_type: PinType::Real,
                    default: Some(serde_json::json!(72.0)),
                    description: None,
                    units: None,
                },
                StructField {
                    name: "enabled".to_string(),
                    field_type: PinType::Boolean,
                    default: Some(serde_json::json!(true)),
                    description: None,
                    units: None,
                },
            ],
        };

        let instance = def.create_default_instance();
        assert_eq!(instance.get("temp"), Some(&serde_json::json!(72.0)));
        assert_eq!(instance.get("enabled"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_struct_registry() {
        let mut registry = StructRegistry::new();

        let def = StructDef {
            id: "my-struct".to_string(),
            name: "My Struct".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            fields: vec![],
        };

        registry.register(def);
        assert!(registry.contains("my-struct"));
        assert!(!registry.contains("other-struct"));
        assert!(registry.get("my-struct").is_some());
    }
}

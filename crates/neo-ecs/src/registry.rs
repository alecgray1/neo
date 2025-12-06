//! Component Registry for dynamic component types.
//!
//! Allows runtime registration of user-defined components from TOML schema files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during component registration.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Component already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Invalid field type: {0}")]
    InvalidFieldType(String),

    #[error("Failed to parse schema: {0}")]
    ParseError(String),

    #[error("Flecs error: {0}")]
    FlecsError(String),
}

/// Supported field types for dynamic components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    F32,
    F64,
    I32,
    I64,
    U32,
    U64,
    Bool,
    String,
}

impl FieldType {
    /// Parse a field type from a string.
    pub fn from_str(s: &str) -> Result<Self, RegistryError> {
        match s.to_lowercase().as_str() {
            "f32" | "float" => Ok(Self::F32),
            "f64" | "double" | "number" => Ok(Self::F64),
            "i32" | "int" | "integer" => Ok(Self::I32),
            "i64" | "long" => Ok(Self::I64),
            "u32" | "uint" => Ok(Self::U32),
            "u64" | "ulong" => Ok(Self::U64),
            "bool" | "boolean" => Ok(Self::Bool),
            "string" | "str" => Ok(Self::String),
            _ => Err(RegistryError::InvalidFieldType(s.to_string())),
        }
    }

    /// Get the size of this field type in bytes.
    pub fn size(&self) -> usize {
        match self {
            FieldType::F32 => 4,
            FieldType::F64 => 8,
            FieldType::I32 => 4,
            FieldType::I64 => 8,
            FieldType::U32 => 4,
            FieldType::U64 => 8,
            FieldType::Bool => 1,
            FieldType::String => std::mem::size_of::<String>(),
        }
    }
}

/// Definition of a single field in a component schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type
    #[serde(rename = "type")]
    pub field_type: FieldType,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Optional unit (e.g., "°F", "CFM")
    #[serde(default)]
    pub unit: Option<String>,
    /// Optional minimum value
    #[serde(default)]
    pub min: Option<f64>,
    /// Optional maximum value
    #[serde(default)]
    pub max: Option<f64>,
    /// Optional default value
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

/// Schema definition for a dynamic component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// Component name
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Field definitions
    pub fields: Vec<FieldDef>,
}

impl ComponentSchema {
    /// Parse a component schema from TOML content.
    pub fn from_toml(content: &str) -> Result<Self, RegistryError> {
        // The TOML format is:
        // [component]
        // name = "..."
        // description = "..."
        //
        // [fields]
        // fieldName = { type = "f64", ... }

        #[derive(Deserialize)]
        struct TomlSchema {
            component: TomlComponent,
            fields: HashMap<String, TomlField>,
        }

        #[derive(Deserialize)]
        struct TomlComponent {
            name: String,
            #[serde(default)]
            description: Option<String>,
        }

        #[derive(Deserialize)]
        struct TomlField {
            #[serde(rename = "type")]
            field_type: String,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            unit: Option<String>,
            #[serde(default)]
            min: Option<f64>,
            #[serde(default)]
            max: Option<f64>,
            #[serde(default)]
            default: Option<serde_json::Value>,
        }

        let parsed: TomlSchema =
            toml::from_str(content).map_err(|e| RegistryError::ParseError(e.to_string()))?;

        let fields = parsed
            .fields
            .into_iter()
            .map(|(name, field)| {
                Ok(FieldDef {
                    name,
                    field_type: FieldType::from_str(&field.field_type)?,
                    description: field.description,
                    unit: field.unit,
                    min: field.min,
                    max: field.max,
                    default: field.default,
                })
            })
            .collect::<Result<Vec<_>, RegistryError>>()?;

        Ok(ComponentSchema {
            name: parsed.component.name,
            description: parsed.component.description,
            fields,
        })
    }
}

/// Registry tracking all registered component schemas.
///
/// This is used to:
/// 1. Track which dynamic components are registered
/// 2. Provide schema info for JS serialization
/// 3. Map component names to Flecs entity IDs
#[derive(Debug, Default)]
pub struct ComponentRegistry {
    /// Schema definitions by component name
    schemas: HashMap<String, ComponentSchema>,
    /// Flecs entity IDs for dynamic components (set by EcsWorld)
    flecs_ids: HashMap<String, u64>,
}

impl ComponentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a component schema.
    pub fn register(&mut self, schema: ComponentSchema) -> Result<(), RegistryError> {
        if self.schemas.contains_key(&schema.name) {
            return Err(RegistryError::AlreadyRegistered(schema.name));
        }
        self.schemas.insert(schema.name.clone(), schema);
        Ok(())
    }

    /// Set the Flecs entity ID for a registered component.
    pub fn set_flecs_id(&mut self, name: &str, id: u64) {
        self.flecs_ids.insert(name.to_string(), id);
    }

    /// Get the Flecs entity ID for a component.
    pub fn get_flecs_id(&self, name: &str) -> Option<u64> {
        self.flecs_ids.get(name).copied()
    }

    /// Check if a component is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.schemas.contains_key(name)
    }

    /// Get a component schema by name.
    pub fn get(&self, name: &str) -> Option<&ComponentSchema> {
        self.schemas.get(name)
    }

    /// Get all registered schemas.
    pub fn all_schemas(&self) -> Vec<ComponentSchema> {
        self.schemas.values().cloned().collect()
    }

    /// Get all registered component names.
    pub fn component_names(&self) -> Vec<&str> {
        self.schemas.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_schema_from_toml() {
        let toml = r#"
[component]
name = "ChillerStatus"
description = "Chiller operational status"

[fields]
supplyTemp = { type = "f64", unit = "°F" }
returnTemp = { type = "f64", unit = "°F" }
loadPercent = { type = "f64", min = 0.0, max = 100.0 }
running = { type = "bool", default = false }
"#;

        let schema = ComponentSchema::from_toml(toml).unwrap();
        assert_eq!(schema.name, "ChillerStatus");
        assert_eq!(schema.fields.len(), 4);

        let supply_temp = schema.fields.iter().find(|f| f.name == "supplyTemp").unwrap();
        assert_eq!(supply_temp.field_type, FieldType::F64);
        assert_eq!(supply_temp.unit, Some("°F".to_string()));
    }

    #[test]
    fn test_field_type_parsing() {
        assert_eq!(FieldType::from_str("f64").unwrap(), FieldType::F64);
        assert_eq!(FieldType::from_str("number").unwrap(), FieldType::F64);
        assert_eq!(FieldType::from_str("bool").unwrap(), FieldType::Bool);
        assert_eq!(FieldType::from_str("string").unwrap(), FieldType::String);
        assert!(FieldType::from_str("invalid").is_err());
    }
}

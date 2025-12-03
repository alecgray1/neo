//! Runtime Type Registry for user-defined types
//!
//! This module provides a reactive type registry that stores user-defined types
//! (events, objects, services) and broadcasts changes to subscribers. This enables
//! live updates to TypeScript definitions and Blueprint schemas.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::PinType;

// ─────────────────────────────────────────────────────────────────────────────
// Type Categories
// ─────────────────────────────────────────────────────────────────────────────

/// Categories of user-defined types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TypeCategory {
    /// Event types (emitted by services, consumed by blueprints)
    Event,
    /// Object/struct types (data containers)
    Object,
    /// Service types (define service interfaces)
    Service,
}

impl std::fmt::Display for TypeCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeCategory::Event => write!(f, "event"),
            TypeCategory::Object => write!(f, "object"),
            TypeCategory::Service => write!(f, "service"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Type Source
// ─────────────────────────────────────────────────────────────────────────────

/// Where a type definition came from
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum TypeSource {
    /// Built into the system (from Rust code via #[neo::expose])
    #[default]
    Builtin,
    /// Defined in a JSON/TOML file
    File {
        path: String,
    },
    /// Defined by a JavaScript plugin
    JavaScript {
        plugin_id: String,
    },
    /// Defined in a Blueprint
    Blueprint {
        blueprint_id: String,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Field Definition
// ─────────────────────────────────────────────────────────────────────────────

/// Definition of a field in a user-defined type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type (uses the same type system as pins)
    pub field_type: PinType,
    /// Whether this field is optional
    #[serde(default)]
    pub optional: bool,
    /// Default value for optional fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl FieldDef {
    /// Create a required field
    pub fn required(name: impl Into<String>, field_type: PinType) -> Self {
        Self {
            name: name.into(),
            field_type,
            optional: false,
            default: None,
            description: None,
        }
    }

    /// Create an optional field
    pub fn optional(name: impl Into<String>, field_type: PinType) -> Self {
        Self {
            name: name.into(),
            field_type,
            optional: true,
            default: None,
            description: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a default value
    pub fn with_default(mut self, default: serde_json::Value) -> Self {
        self.default = Some(default);
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Type Definition
// ─────────────────────────────────────────────────────────────────────────────

/// A complete user-defined type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    /// Unique type ID (e.g., "neo/ZoneAlert", "user/MyEvent")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type category
    pub category: TypeCategory,
    /// Version string
    #[serde(default = "default_version")]
    pub version: String,
    /// Fields for this type
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    /// Source of this type definition
    #[serde(default)]
    pub source: TypeSource,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl TypeDef {
    /// Create a new event type
    pub fn event(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: TypeCategory::Event,
            version: default_version(),
            fields: Vec::new(),
            source: TypeSource::default(),
            description: None,
        }
    }

    /// Create a new object type
    pub fn object(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: TypeCategory::Object,
            version: default_version(),
            fields: Vec::new(),
            source: TypeSource::default(),
            description: None,
        }
    }

    /// Create a new service type
    pub fn service(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            category: TypeCategory::Service,
            version: default_version(),
            fields: Vec::new(),
            source: TypeSource::default(),
            description: None,
        }
    }

    /// Add fields
    pub fn with_fields(mut self, fields: Vec<FieldDef>) -> Self {
        self.fields = fields;
        self
    }

    /// Add a single field
    pub fn add_field(mut self, field: FieldDef) -> Self {
        self.fields.push(field);
        self
    }

    /// Set the source
    pub fn with_source(mut self, source: TypeSource) -> Self {
        self.source = source;
        self
    }

    /// Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Get a field by name
    pub fn get_field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Type Registry Events
// ─────────────────────────────────────────────────────────────────────────────

/// Events broadcast when the type registry changes
#[derive(Debug, Clone)]
pub enum TypeChange {
    /// A new type was registered
    Added(TypeDef),
    /// A type was updated
    Updated(TypeDef),
    /// A type was removed
    Removed {
        type_id: String,
        category: TypeCategory,
    },
}

impl TypeChange {
    /// Get the type ID affected by this change
    pub fn type_id(&self) -> &str {
        match self {
            TypeChange::Added(def) => &def.id,
            TypeChange::Updated(def) => &def.id,
            TypeChange::Removed { type_id, .. } => type_id,
        }
    }

    /// Get the category affected by this change
    pub fn category(&self) -> TypeCategory {
        match self {
            TypeChange::Added(def) => def.category,
            TypeChange::Updated(def) => def.category,
            TypeChange::Removed { category, .. } => *category,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Type Registry Error
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur when working with the type registry
#[derive(Debug, Clone, thiserror::Error)]
pub enum TypeRegistryError {
    #[error("Type already exists: {0}")]
    TypeAlreadyExists(String),
    #[error("Type not found: {0}")]
    TypeNotFound(String),
    #[error("Invalid type definition: {0}")]
    InvalidDefinition(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Type Registry
// ─────────────────────────────────────────────────────────────────────────────

/// The central type registry for user-defined types
///
/// This registry stores all user-defined types (events, objects, services) and
/// broadcasts changes to subscribers. It is thread-safe and can be shared across
/// async tasks.
pub struct TypeRegistry {
    /// All registered types by ID
    types: DashMap<String, TypeDef>,
    /// Change notification channel
    change_tx: broadcast::Sender<TypeChange>,
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    /// Create a new empty type registry
    pub fn new() -> Self {
        let (change_tx, _) = broadcast::channel(256);
        Self {
            types: DashMap::new(),
            change_tx,
        }
    }

    /// Create a new type registry wrapped in an Arc
    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Subscribe to type registry changes
    ///
    /// Returns a receiver that will receive all type changes. New subscribers
    /// will only receive changes made after subscribing.
    pub fn subscribe(&self) -> broadcast::Receiver<TypeChange> {
        self.change_tx.subscribe()
    }

    /// Register a new type
    ///
    /// Returns an error if a type with the same ID already exists.
    /// Use `register_or_update` if you want to update existing types.
    pub async fn register(&self, def: TypeDef) -> Result<(), TypeRegistryError> {
        let type_id = def.id.clone();

        if self.types.contains_key(&type_id) {
            return Err(TypeRegistryError::TypeAlreadyExists(type_id));
        }
        self.types.insert(type_id, def.clone());

        // Notify subscribers (ignore send errors - means no subscribers)
        let _ = self.change_tx.send(TypeChange::Added(def));

        Ok(())
    }

    /// Register or update a type
    ///
    /// If the type already exists, it will be updated. Otherwise, it will be
    /// registered as a new type.
    pub async fn register_or_update(&self, def: TypeDef) {
        let type_id = def.id.clone();
        let is_update = self.types.contains_key(&type_id);
        self.types.insert(type_id, def.clone());

        let change = if is_update {
            TypeChange::Updated(def)
        } else {
            TypeChange::Added(def)
        };
        let _ = self.change_tx.send(change);
    }

    /// Update an existing type
    ///
    /// Returns an error if the type does not exist.
    pub async fn update(&self, def: TypeDef) -> Result<(), TypeRegistryError> {
        let type_id = def.id.clone();

        if !self.types.contains_key(&type_id) {
            return Err(TypeRegistryError::TypeNotFound(type_id));
        }
        self.types.insert(type_id, def.clone());

        let _ = self.change_tx.send(TypeChange::Updated(def));

        Ok(())
    }

    /// Remove a type
    ///
    /// Returns an error if the type does not exist.
    pub async fn remove(&self, type_id: &str) -> Result<TypeDef, TypeRegistryError> {
        let (_, removed) = self
            .types
            .remove(type_id)
            .ok_or_else(|| TypeRegistryError::TypeNotFound(type_id.to_string()))?;

        let _ = self.change_tx.send(TypeChange::Removed {
            type_id: type_id.to_string(),
            category: removed.category,
        });

        Ok(removed)
    }

    /// Get a type definition by ID
    pub async fn get(&self, type_id: &str) -> Option<TypeDef> {
        self.types.get(type_id).map(|r| r.clone())
    }

    /// Check if a type exists
    pub async fn contains(&self, type_id: &str) -> bool {
        self.types.contains_key(type_id)
    }

    /// Get all types
    pub async fn all(&self) -> Vec<TypeDef> {
        self.types.iter().map(|r| r.value().clone()).collect()
    }

    /// Get all types in a category
    pub async fn get_by_category(&self, category: TypeCategory) -> Vec<TypeDef> {
        self.types
            .iter()
            .filter(|r| r.value().category == category)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all event types
    pub async fn events(&self) -> Vec<TypeDef> {
        self.get_by_category(TypeCategory::Event).await
    }

    /// Get all object types
    pub async fn objects(&self) -> Vec<TypeDef> {
        self.get_by_category(TypeCategory::Object).await
    }

    /// Get all service types
    pub async fn services(&self) -> Vec<TypeDef> {
        self.get_by_category(TypeCategory::Service).await
    }

    /// Get the number of registered types
    pub async fn len(&self) -> usize {
        self.types.len()
    }

    /// Check if the registry is empty
    pub async fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Get types by source
    pub async fn get_by_source(&self, source_type: &str) -> Vec<TypeDef> {
        self.types
            .iter()
            .filter(|r| match (&r.value().source, source_type) {
                (TypeSource::Builtin, "builtin") => true,
                (TypeSource::File { .. }, "file") => true,
                (TypeSource::JavaScript { .. }, "javascript") => true,
                (TypeSource::Blueprint { .. }, "blueprint") => true,
                _ => false,
            })
            .map(|r| r.value().clone())
            .collect()
    }

    /// Export the registry as a snapshot (for serialization)
    pub async fn snapshot(&self) -> TypeRegistrySnapshot {
        TypeRegistrySnapshot {
            types: self.types.iter().map(|r| (r.key().clone(), r.value().clone())).collect(),
        }
    }

    /// Import types from a snapshot
    pub async fn import(&self, snapshot: TypeRegistrySnapshot) {
        for (id, def) in snapshot.types {
            let is_update = self.types.contains_key(&id);
            self.types.insert(id, def.clone());
            let change = if is_update {
                TypeChange::Updated(def)
            } else {
                TypeChange::Added(def)
            };
            let _ = self.change_tx.send(change);
        }
    }

    /// Clear all types from a specific source
    pub async fn clear_source(&self, source_type: &str) {
        let to_remove: Vec<_> = self.get_by_source(source_type).await
            .into_iter()
            .map(|t| (t.id.clone(), t.category))
            .collect();

        for (type_id, category) in to_remove {
            self.types.remove(&type_id);
            let _ = self.change_tx.send(TypeChange::Removed { type_id, category });
        }
    }
}

/// A serializable snapshot of the type registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeRegistrySnapshot {
    pub types: HashMap<String, TypeDef>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_type() {
        let registry = TypeRegistry::new();

        let event = TypeDef::event("test/MyEvent", "My Event")
            .add_field(FieldDef::required("value", PinType::Real));

        registry.register(event.clone()).await.unwrap();

        let retrieved = registry.get("test/MyEvent").await.unwrap();
        assert_eq!(retrieved.name, "My Event");
        assert_eq!(retrieved.fields.len(), 1);
    }

    #[tokio::test]
    async fn test_register_duplicate_fails() {
        let registry = TypeRegistry::new();

        let event = TypeDef::event("test/MyEvent", "My Event");
        registry.register(event.clone()).await.unwrap();

        let result = registry.register(event).await;
        assert!(matches!(result, Err(TypeRegistryError::TypeAlreadyExists(_))));
    }

    #[tokio::test]
    async fn test_subscribe_to_changes() {
        let registry = TypeRegistry::new();
        let mut rx = registry.subscribe();

        let event = TypeDef::event("test/MyEvent", "My Event");
        registry.register(event).await.unwrap();

        let change = rx.recv().await.unwrap();
        assert!(matches!(change, TypeChange::Added(_)));
        assert_eq!(change.type_id(), "test/MyEvent");
    }

    #[tokio::test]
    async fn test_get_by_category() {
        let registry = TypeRegistry::new();

        registry
            .register(TypeDef::event("test/Event1", "Event 1"))
            .await
            .unwrap();
        registry
            .register(TypeDef::event("test/Event2", "Event 2"))
            .await
            .unwrap();
        registry
            .register(TypeDef::object("test/Object1", "Object 1"))
            .await
            .unwrap();

        let events = registry.get_by_category(TypeCategory::Event).await;
        assert_eq!(events.len(), 2);

        let objects = registry.get_by_category(TypeCategory::Object).await;
        assert_eq!(objects.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_type() {
        let registry = TypeRegistry::new();
        let mut rx = registry.subscribe();

        let event = TypeDef::event("test/MyEvent", "My Event");
        registry.register(event).await.unwrap();
        let _ = rx.recv().await; // Consume Added event

        let removed = registry.remove("test/MyEvent").await.unwrap();
        assert_eq!(removed.id, "test/MyEvent");

        let change = rx.recv().await.unwrap();
        assert!(matches!(change, TypeChange::Removed { .. }));

        assert!(registry.get("test/MyEvent").await.is_none());
    }
}

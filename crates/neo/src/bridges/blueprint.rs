//! Blueprint Bridge
//!
//! Pushes blueprint schema updates via WebSocket when types change.
//! This enables real-time type checking in the visual editor.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use blueprint_types::{PinType, TypeCategory, TypeChange, TypeDef, TypeRegistry};

/// Message sent to blueprint clients when types change
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum BlueprintSchemaUpdate {
    /// A new type was added
    TypeAdded(TypeSchemaInfo),

    /// An existing type was updated
    TypeUpdated(TypeSchemaInfo),

    /// A type was removed
    TypeRemoved {
        type_id: String,
        category: String,
    },

    /// Full schema sync (sent on initial connection)
    FullSync(Vec<TypeSchemaInfo>),
}

/// Schema information for a type, sent to blueprint clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaInfo {
    /// Unique type identifier
    pub type_id: String,

    /// Display name
    pub name: String,

    /// Category: "event", "object", or "service"
    pub category: String,

    /// Optional description
    pub description: Option<String>,

    /// Field definitions
    pub fields: Vec<FieldSchemaInfo>,
}

/// Schema information for a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchemaInfo {
    /// Field name
    pub name: String,

    /// Field type (as blueprint pin type string)
    pub field_type: String,

    /// Whether this field is optional
    pub optional: bool,

    /// Human-readable description
    pub description: Option<String>,
}

impl From<&TypeDef> for TypeSchemaInfo {
    fn from(def: &TypeDef) -> Self {
        Self {
            type_id: def.id.clone(),
            name: def.name.clone(),
            category: def.category.to_string(),
            description: def.description.clone(),
            fields: def
                .fields
                .iter()
                .map(|field| FieldSchemaInfo {
                    name: field.name.clone(),
                    field_type: pin_type_to_string(&field.field_type),
                    optional: field.optional,
                    description: field.description.clone(),
                })
                .collect(),
        }
    }
}

/// Convert a PinType to a string representation
fn pin_type_to_string(pin_type: &PinType) -> String {
    match pin_type {
        PinType::Exec => "exec".to_string(),
        PinType::Boolean => "bool".to_string(),
        PinType::Integer => "int".to_string(),
        PinType::Real => "real".to_string(),
        PinType::String => "string".to_string(),
        PinType::Any => "any".to_string(),
        PinType::PointValue => "pointvalue".to_string(),
        PinType::Array { element } => format!("{}[]", pin_type_to_string(element)),
        PinType::Struct { struct_id } => format!("struct:{}", struct_id),
        PinType::Object { object_id } => format!("object:{}", object_id),
        PinType::Event { event_id } => format!("event:{}", event_id),
        PinType::Handle { target_type } => format!("handle:{}", target_type),
    }
}

/// Bridge that pushes type changes to blueprint clients
pub struct BlueprintBridge {
    /// Reference to the type registry
    type_registry: Arc<TypeRegistry>,

    /// Channel for sending updates to connected clients
    update_tx: broadcast::Sender<BlueprintSchemaUpdate>,

    /// Shutdown signal
    shutdown: broadcast::Sender<()>,
}

impl BlueprintBridge {
    /// Create a new blueprint bridge
    pub fn new(type_registry: Arc<TypeRegistry>) -> Self {
        let (update_tx, _) = broadcast::channel(64);
        let (shutdown, _) = broadcast::channel(1);

        Self {
            type_registry,
            update_tx,
            shutdown,
        }
    }

    /// Get a receiver for schema updates
    pub fn subscribe(&self) -> broadcast::Receiver<BlueprintSchemaUpdate> {
        self.update_tx.subscribe()
    }

    /// Get the current full schema (for initial sync)
    pub async fn get_full_schema(&self) -> BlueprintSchemaUpdate {
        let mut types = Vec::new();

        for def in self.type_registry.get_by_category(TypeCategory::Event).await {
            types.push(TypeSchemaInfo::from(&def));
        }

        for def in self.type_registry.get_by_category(TypeCategory::Object).await {
            types.push(TypeSchemaInfo::from(&def));
        }

        for def in self.type_registry.get_by_category(TypeCategory::Service).await {
            types.push(TypeSchemaInfo::from(&def));
        }

        BlueprintSchemaUpdate::FullSync(types)
    }

    /// Start the bridge, listening for type changes
    pub async fn start(&self) {
        let mut changes = self.type_registry.subscribe();
        let mut shutdown = self.shutdown.subscribe();

        loop {
            tokio::select! {
                _ = shutdown.recv() => {
                    tracing::info!("Blueprint bridge shutting down");
                    break;
                }
                result = changes.recv() => {
                    match result {
                        Ok(change) => {
                            let update = self.type_change_to_update(change);
                            if let Err(_) = self.update_tx.send(update) {
                                // No receivers, that's okay
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Blueprint bridge lagged {} messages", n);
                            // Send a full sync to catch up
                            let _ = self.update_tx.send(self.get_full_schema().await);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Type registry channel closed");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Stop the bridge
    pub fn stop(&self) {
        let _ = self.shutdown.send(());
    }

    /// Convert a TypeChange to a BlueprintSchemaUpdate
    fn type_change_to_update(&self, change: TypeChange) -> BlueprintSchemaUpdate {
        match change {
            TypeChange::Added(def) => BlueprintSchemaUpdate::TypeAdded(TypeSchemaInfo::from(&def)),
            TypeChange::Updated(def) => {
                BlueprintSchemaUpdate::TypeUpdated(TypeSchemaInfo::from(&def))
            }
            TypeChange::Removed { type_id, category } => BlueprintSchemaUpdate::TypeRemoved {
                type_id,
                category: category.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blueprint_types::FieldDef;

    #[test]
    fn test_type_schema_info_from_typedef() {
        let def = TypeDef::event("test/TestEvent", "TestEvent")
            .add_field(FieldDef::required("zone_id", PinType::String))
            .add_field(FieldDef::optional("value", PinType::Real))
            .with_description("A test event");

        let info = TypeSchemaInfo::from(&def);

        assert_eq!(info.type_id, "test/TestEvent");
        assert_eq!(info.name, "TestEvent");
        assert_eq!(info.category, "event");
        assert_eq!(info.description, Some("A test event".to_string()));
        assert_eq!(info.fields.len(), 2);

        // Check fields
        assert_eq!(info.fields[0].name, "zone_id");
        assert_eq!(info.fields[0].field_type, "string");
        assert!(!info.fields[0].optional);

        assert_eq!(info.fields[1].name, "value");
        assert_eq!(info.fields[1].field_type, "real");
        assert!(info.fields[1].optional);
    }

    #[test]
    fn test_blueprint_bridge_subscribe() {
        let registry = Arc::new(TypeRegistry::new());
        let bridge = BlueprintBridge::new(registry);

        // Should be able to subscribe multiple times
        let _rx1 = bridge.subscribe();
        let _rx2 = bridge.subscribe();
    }

    #[tokio::test]
    async fn test_full_schema_sync() {
        let registry = Arc::new(TypeRegistry::new());

        // Register a type
        registry
            .register(
                TypeDef::object("test/TestType", "TestType")
                    .add_field(FieldDef::required("data", PinType::String)),
            )
            .await
            .unwrap();

        let bridge = BlueprintBridge::new(registry);
        let sync = bridge.get_full_schema().await;

        if let BlueprintSchemaUpdate::FullSync(types) = sync {
            assert_eq!(types.len(), 1);
            assert_eq!(types[0].type_id, "test/TestType");
        } else {
            panic!("Expected FullSync");
        }
    }

    #[test]
    fn test_schema_update_serialization() {
        let update = BlueprintSchemaUpdate::TypeAdded(TypeSchemaInfo {
            type_id: "test/TestEvent".to_string(),
            name: "TestEvent".to_string(),
            category: "event".to_string(),
            description: Some("Test".to_string()),
            fields: vec![FieldSchemaInfo {
                name: "value".to_string(),
                field_type: "real".to_string(),
                optional: false,
                description: None,
            }],
        });

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("TypeAdded"));
        assert!(json.contains("TestEvent"));
    }

    #[test]
    fn test_pin_type_to_string() {
        assert_eq!(pin_type_to_string(&PinType::Boolean), "bool");
        assert_eq!(pin_type_to_string(&PinType::Integer), "int");
        assert_eq!(pin_type_to_string(&PinType::Real), "real");
        assert_eq!(pin_type_to_string(&PinType::String), "string");
        assert_eq!(
            pin_type_to_string(&PinType::Array {
                element: Box::new(PinType::Integer)
            }),
            "int[]"
        );
        assert_eq!(
            pin_type_to_string(&PinType::Object {
                object_id: "MyObject".to_string()
            }),
            "object:MyObject"
        );
    }
}

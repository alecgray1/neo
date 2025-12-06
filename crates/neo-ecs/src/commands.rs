//! ECS Command and Response types for async communication with the ECS worker thread.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::oneshot;

/// Unique identifier for an entity in the ECS world.
/// Wraps the Flecs entity ID (u64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

impl From<u64> for EntityId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

/// Relationship constraint for queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryRelationship {
    /// Entity must be a child of the specified parent
    ChildOf(EntityId),
    /// Entity must be a descendant (any depth) of the specified ancestor
    DescendantOf(EntityId),
}

/// Result from a query operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// The entity ID
    pub entity: EntityId,
    /// Entity name (if named)
    pub name: Option<String>,
    /// Component data keyed by component name
    pub components: std::collections::HashMap<String, Value>,
    /// Tags present on the entity
    pub tags: Vec<String>,
}

/// Commands sent to the ECS worker thread.
#[derive(Debug)]
pub enum EcsCommand {
    // === Entity Operations ===
    /// Create a new entity
    CreateEntity {
        /// Optional name for the entity
        name: Option<String>,
        /// Optional parent entity (creates ChildOf relationship)
        parent: Option<EntityId>,
        /// Initial components to set (component_name -> data)
        components: Vec<(String, Value)>,
        /// Tags to add
        tags: Vec<String>,
        /// Response channel
        response: oneshot::Sender<EcsResponse>,
    },

    /// Delete an entity and all its children
    DeleteEntity {
        entity: EntityId,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Look up an entity by name/path
    LookupEntity {
        name: String,
        response: oneshot::Sender<EcsResponse>,
    },

    // === Component Operations ===
    /// Get a component's data from an entity
    GetComponent {
        entity: EntityId,
        component: String,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Set a component's data on an entity
    SetComponent {
        entity: EntityId,
        component: String,
        data: Value,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Remove a component from an entity
    RemoveComponent {
        entity: EntityId,
        component: String,
        response: oneshot::Sender<EcsResponse>,
    },

    // === Tag Operations ===
    /// Add a tag to an entity
    AddTag {
        entity: EntityId,
        tag: String,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Remove a tag from an entity
    RemoveTag {
        entity: EntityId,
        tag: String,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Check if an entity has a tag
    HasTag {
        entity: EntityId,
        tag: String,
        response: oneshot::Sender<EcsResponse>,
    },

    // === Hierarchy Operations ===
    /// Get an entity's parent
    GetParent {
        entity: EntityId,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Get an entity's children
    GetChildren {
        entity: EntityId,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Set an entity's parent (move in hierarchy)
    SetParent {
        entity: EntityId,
        parent: Option<EntityId>,
        response: oneshot::Sender<EcsResponse>,
    },

    // === Query Operations ===
    /// Query entities by components and relationships
    Query {
        /// Components that must be present
        with_components: Vec<String>,
        /// Tags that must be present
        with_tags: Vec<String>,
        /// Optional relationship constraint
        relationship: Option<QueryRelationship>,
        /// Whether to include component data in results
        include_data: bool,
        response: oneshot::Sender<EcsResponse>,
    },

    // === Schema Operations ===
    /// Register a dynamic component type
    RegisterComponent {
        schema: crate::registry::ComponentSchema,
        response: oneshot::Sender<EcsResponse>,
    },

    /// Get all registered component schemas
    GetSchemas {
        response: oneshot::Sender<EcsResponse>,
    },

    // === Persistence Operations ===
    /// Save the world state to JSON
    SaveWorld {
        response: oneshot::Sender<EcsResponse>,
    },

    /// Load world state from JSON
    LoadWorld {
        json: String,
        response: oneshot::Sender<EcsResponse>,
    },

    // === Lifecycle ===
    /// Shutdown the ECS worker
    Shutdown,
}

/// Responses from the ECS worker thread.
#[derive(Debug)]
pub enum EcsResponse {
    /// Entity was created successfully
    EntityCreated(EntityId),

    /// Entity lookup result
    EntityFound(Option<EntityId>),

    /// Component data (None if component not present)
    ComponentData(Option<Value>),

    /// Query results
    QueryResults(Vec<QueryResult>),

    /// List of entity IDs (for GetChildren)
    EntityList(Vec<EntityId>),

    /// Parent entity (None if no parent)
    Parent(Option<EntityId>),

    /// Boolean result (for HasTag)
    Bool(bool),

    /// World state as JSON (for SaveWorld)
    WorldJson(String),

    /// All registered schemas
    Schemas(Vec<crate::registry::ComponentSchema>),

    /// Operation completed successfully
    Ok,

    /// Operation failed
    Error(String),
}

impl EcsResponse {
    /// Check if the response is an error
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Get error message if this is an error response
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

//! Runtime services accessible from JavaScript.
//!
//! These services provide capabilities to JavaScript code running in the runtime,
//! such as event publishing, point value access, and ECS operations.

use std::sync::Arc;

use crate::error::{PointError, RuntimeError};

/// Error type for ECS operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EcsError {
    #[error("Entity not found: {0}")]
    EntityNotFound(String),
    #[error("Component not found: {0}")]
    ComponentNotFound(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    #[error("ECS store not available")]
    NotAvailable,
}

/// Services that can be accessed from JavaScript.
#[derive(Clone, Default)]
pub struct RuntimeServices {
    /// Event publisher for emitting events from JS
    pub events: Option<EventPublisher>,
    /// Point store for reading/writing point values from JS
    pub points: Option<Arc<dyn PointStore>>,
    /// ECS store for entity/component operations from JS
    pub ecs: Option<Arc<dyn EcsStore>>,
}

/// Trait for point value storage.
#[async_trait::async_trait]
pub trait PointStore: Send + Sync + 'static {
    /// Read a point value by ID.
    async fn read(&self, point_id: &str) -> Result<Option<serde_json::Value>, PointError>;
    /// Write a point value by ID.
    async fn write(&self, point_id: &str, value: serde_json::Value) -> Result<(), PointError>;
}

/// Trait for ECS operations from JavaScript.
#[async_trait::async_trait]
pub trait EcsStore: Send + Sync + 'static {
    /// Create a new entity with optional name, parent, components, and tags.
    async fn create_entity(
        &self,
        name: Option<String>,
        parent: Option<u64>,
        components: Vec<(String, serde_json::Value)>,
        tags: Vec<String>,
    ) -> Result<u64, EcsError>;

    /// Delete an entity by ID.
    async fn delete_entity(&self, entity_id: u64) -> Result<(), EcsError>;

    /// Get a component from an entity.
    async fn get_component(
        &self,
        entity_id: u64,
        component: &str,
    ) -> Result<Option<serde_json::Value>, EcsError>;

    /// Set a component on an entity.
    async fn set_component(
        &self,
        entity_id: u64,
        component: &str,
        data: serde_json::Value,
    ) -> Result<(), EcsError>;

    /// Remove a component from an entity.
    async fn remove_component(&self, entity_id: u64, component: &str) -> Result<(), EcsError>;

    /// Add a tag to an entity.
    async fn add_tag(&self, entity_id: u64, tag: &str) -> Result<(), EcsError>;

    /// Remove a tag from an entity.
    async fn remove_tag(&self, entity_id: u64, tag: &str) -> Result<(), EcsError>;

    /// Check if an entity has a tag.
    async fn has_tag(&self, entity_id: u64, tag: &str) -> Result<bool, EcsError>;

    /// Look up an entity by name.
    async fn lookup(&self, name: &str) -> Result<Option<u64>, EcsError>;

    /// Get children of an entity.
    async fn get_children(&self, entity_id: u64) -> Result<Vec<u64>, EcsError>;

    /// Get parent of an entity.
    async fn get_parent(&self, entity_id: u64) -> Result<Option<u64>, EcsError>;

    /// Query entities with specific components.
    async fn query(&self, components: Vec<String>) -> Result<Vec<serde_json::Value>, EcsError>;
}

/// Event publisher handle for emitting events.
#[derive(Clone)]
pub struct EventPublisher {
    tx: tokio::sync::broadcast::Sender<Event>,
}

impl EventPublisher {
    /// Create a new event publisher with the given broadcast sender.
    pub fn new(tx: tokio::sync::broadcast::Sender<Event>) -> Self {
        Self { tx }
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: Event) -> Result<(), RuntimeError> {
        self.tx
            .send(event)
            .map(|_| ())
            .map_err(|_| RuntimeError::ChannelClosed)
    }
}

/// An event that can be published.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    /// Type of the event (e.g., "device/point/changed")
    pub event_type: String,
    /// Source of the event (e.g., service ID)
    pub source: String,
    /// Event payload data
    pub data: serde_json::Value,
    /// Timestamp in milliseconds since Unix epoch
    pub timestamp: u64,
}

impl Event {
    /// Create a new event with the current timestamp.
    pub fn new(
        event_type: impl Into<String>,
        source: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            source: source.into(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

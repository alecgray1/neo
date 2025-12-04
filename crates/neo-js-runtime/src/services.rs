//! Runtime services accessible from JavaScript.
//!
//! These services provide capabilities to JavaScript code running in the runtime,
//! such as event publishing and point value access.

use std::sync::Arc;

use crate::error::{PointError, RuntimeError};

/// Services that can be accessed from JavaScript.
#[derive(Clone, Default)]
pub struct RuntimeServices {
    /// Event publisher for emitting events from JS
    pub events: Option<EventPublisher>,
    /// Point store for reading/writing point values from JS
    pub points: Option<Arc<dyn PointStore>>,
}

/// Trait for point value storage.
#[async_trait::async_trait]
pub trait PointStore: Send + Sync + 'static {
    /// Read a point value by ID.
    async fn read(&self, point_id: &str) -> Result<Option<serde_json::Value>, PointError>;
    /// Write a point value by ID.
    async fn write(&self, point_id: &str, value: serde_json::Value) -> Result<(), PointError>;
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

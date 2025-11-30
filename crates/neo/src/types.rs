use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use ts_rs::TS;

// Re-export BACnet types from the library
pub use bacnet::{BacnetError, ObjectIdentifier, ObjectType, PropertyIdentifier, PropertyValue};

/// Point quality indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub enum PointQuality {
    Good,
    Bad,
    Uncertain,
    Stale,
}

/// BACnet point data structure (replaces point actor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BACnetPoint {
    pub object_id: ObjectIdentifier,
    pub present_value: PropertyValue,
    pub quality: PointQuality,
    #[serde(skip, default = "Instant::now")]
    pub last_update: Instant,
    pub last_update_utc: DateTime<Utc>,

    // Metadata (discovered from device)
    pub object_name: Option<String>,
    pub description: Option<String>,
    pub units: Option<String>,
    pub cov_increment: Option<f32>,
}

impl BACnetPoint {
    pub fn new(object_id: ObjectIdentifier, present_value: PropertyValue) -> Self {
        Self {
            object_id,
            present_value,
            quality: PointQuality::Uncertain,
            last_update: Instant::now(),
            last_update_utc: Utc::now(),
            object_name: None,
            description: None,
            units: None,
            cov_increment: None,
        }
    }
}

/// Device status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Online,
    Offline,
    Timeout,
    Error,
}

/// Alarm severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub enum AlarmSeverity {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Service lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

impl Default for ServiceState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Actor error: {0}")]
    Actor(String),

    #[error("Timeout")]
    Timeout,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Service error: {0}")]
    Service(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("BACnet error: {0}")]
    BACnet(#[from] BacnetError),

    #[error("Other: {0}")]
    Other(String),
}

impl From<kameo::error::SendError> for Error {
    fn from(err: kameo::error::SendError) -> Self {
        Error::Actor(err.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}

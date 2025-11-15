use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Instant;

/// BACnet object identifier
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObjectId {
    pub object_type: ObjectType,
    pub instance: u32,
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.object_type, self.instance)
    }
}

/// BACnet object types
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u16)]
pub enum ObjectType {
    AnalogInput = 0,
    AnalogOutput = 1,
    AnalogValue = 2,
    BinaryInput = 3,
    BinaryOutput = 4,
    BinaryValue = 5,
    Device = 8,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::AnalogInput => write!(f, "AI"),
            ObjectType::AnalogOutput => write!(f, "AO"),
            ObjectType::AnalogValue => write!(f, "AV"),
            ObjectType::BinaryInput => write!(f, "BI"),
            ObjectType::BinaryOutput => write!(f, "BO"),
            ObjectType::BinaryValue => write!(f, "BV"),
            ObjectType::Device => write!(f, "Device"),
        }
    }
}

/// Point value types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PointValue {
    Real(f32),
    Unsigned(u32),
    Boolean(bool),
    Enumerated(u32),
    Null,
}

impl fmt::Display for PointValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PointValue::Real(v) => write!(f, "{:.2}", v),
            PointValue::Unsigned(v) => write!(f, "{}", v),
            PointValue::Boolean(v) => write!(f, "{}", v),
            PointValue::Enumerated(v) => write!(f, "enum({})", v),
            PointValue::Null => write!(f, "null"),
        }
    }
}

/// Point quality indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointQuality {
    Good,
    Bad,
    Uncertain,
    Stale,
}

/// BACnet point data structure (replaces point actor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BACnetPoint {
    pub object_id: ObjectId,
    pub present_value: PointValue,
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
    pub fn new(object_id: ObjectId, present_value: PointValue) -> Self {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlarmSeverity {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Service lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
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
    BACnet(String),

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

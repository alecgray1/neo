use crate::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Events that can be published via pub-sub
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    PointValueChanged {
        point: String,
        value: PointValue,
        quality: PointQuality,
        #[serde(skip, default = "Instant::now")]
        timestamp: Instant,
        timestamp_utc: DateTime<Utc>,
    },

    AlarmRaised {
        source: String,
        message: String,
        severity: AlarmSeverity,
        #[serde(skip, default = "Instant::now")]
        timestamp: Instant,
        timestamp_utc: DateTime<Utc>,
    },

    AlarmCleared {
        source: String,
        #[serde(skip, default = "Instant::now")]
        timestamp: Instant,
        timestamp_utc: DateTime<Utc>,
    },

    DeviceStatusChanged {
        device: String,
        network: String,
        status: DeviceStatus,
        #[serde(skip, default = "Instant::now")]
        timestamp: Instant,
        timestamp_utc: DateTime<Utc>,
    },

    ServiceStateChanged {
        service: String,
        state: ServiceState,
        #[serde(skip, default = "Instant::now")]
        timestamp: Instant,
        timestamp_utc: DateTime<Utc>,
    },
}

/// Station-level messages
#[derive(Debug)]
pub enum StationMsg {
    GetStatus,
    GetStats,
    Save,
    Shutdown,
}

/// Point actor messages
#[derive(Debug, Clone)]
pub enum PointMsg {
    UpdateValue(PointValue),
    GetValue,
}

/// Device actor messages
#[derive(Debug)]
pub enum DeviceMsg {
    ReadProperty { object_id: ObjectId, property_id: u8 },
    WriteProperty { object_id: ObjectId, property_id: u8, value: PointValue },
    Poll,
    GetStatus,
    AddPoint { object_id: ObjectId, initial_value: PointValue },
    GetPoint { object_id: ObjectId },
}

/// Pub-sub broker messages
#[derive(Debug)]
pub enum PubSubMsg {
    Publish { topic: String, event: Event },
    Subscribe { topic_pattern: String, subscriber_id: String },
    Unsubscribe { topic_pattern: String, subscriber_id: String },
    GetStats,
}

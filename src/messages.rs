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

    DeviceDiscovered {
        network: String,
        device: String,
        instance: u32,
        address: std::net::SocketAddr,
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
    DiscoverPoints,
    ListPoints,
    GetPoint { object_id: ObjectId },
    Reconnect,
    SetNetworkActor(kameo::actor::ActorRef<crate::actors::bacnet::BACnetNetworkActor>),
}

/// Network actor messages
#[derive(Debug)]
pub enum NetworkMsg {
    GetStatus,
    ListDevices,
    PollAll,
    AddDevice {
        device_name: String,
        device_instance: u32,
        device_address: Option<std::net::SocketAddr>
    },
    GetDevice { device_name: String },
    DiscoverDevices,
    EnableAutoDiscovery,
    DisableAutoDiscovery,
    IsAutoDiscoveryEnabled,
    SetDiscoveryInterval(u64),
    GetDiscoveryInterval,

    // BACnet I/O operations (processed sequentially by network actor)
    ReadProperty {
        device_id: u32,
        object_type: ObjectType,
        object_instance: u32,
        property_id: u8,
        array_index: Option<u32>,  // None for BACNET_ARRAY_ALL
        raw: bool,  // If true, return RawBACnetValue instead of converting to PointValue
    },
    WriteProperty {
        device_id: u32,
        object_type: ObjectType,
        object_instance: u32,
        property_id: u8,
        value: PointValue,
    },
}

/// Pub-sub broker messages
#[derive(Debug)]
pub enum PubSubMsg {
    Publish { topic: String, event: Event },
    Subscribe { topic_pattern: String, subscriber_id: String },
    Unsubscribe { topic_pattern: String, subscriber_id: String },
    GetStats,
}

/// BACnet I/O Actor messages - handles all BACnet protocol I/O operations
#[derive(Debug, Clone)]
pub enum BACnetIOMsg {
    /// Read a property from a device
    ReadProperty {
        device_id: u32,
        object_type: ObjectType,
        object_instance: u32,
        property_id: u8,
        array_index: Option<u32>,
        timeout_ms: Option<u64>,
    },

    /// Read a property from a device, returning raw BACnetValue
    ReadPropertyRaw {
        device_id: u32,
        object_type: ObjectType,
        object_instance: u32,
        property_id: u8,
        array_index: Option<u32>,
        timeout_ms: Option<u64>,
    },

    /// Write a property to a device
    WriteProperty {
        device_id: u32,
        object_type: ObjectType,
        object_instance: u32,
        property_id: u8,
        value: PointValue,
    },

    /// Read multiple properties in one request (batch operation)
    ReadMultipleProperties {
        device_id: u32,
        requests: Vec<PropertyReadRequest>,
    },

    /// Connect to a new device
    ConnectDevice {
        device_id: u32,
        address: std::net::SocketAddr,
    },

    /// Disconnect from a device
    DisconnectDevice {
        device_id: u32,
    },

    /// Check if device is connected
    IsConnected {
        device_id: u32,
    },

    /// Get I/O statistics
    GetStatistics,

    /// Perform Who-Is discovery
    WhoIs {
        timeout_secs: u64,
        subnet: Option<String>,
    },
}

/// Property read request for batch operations
#[derive(Debug, Clone)]
pub struct PropertyReadRequest {
    pub object_type: ObjectType,
    pub object_instance: u32,
    pub property_id: u8,
    pub array_index: Option<u32>,
}

/// BACnet I/O Actor replies
#[derive(Debug, Clone, kameo::Reply)]
pub enum BACnetIOReply {
    PropertyValue(PointValue),
    RawValue(bacnet::value::BACnetValue),
    PropertyWritten,
    MultipleValues(Vec<std::result::Result<PointValue, String>>),
    Connected,
    Disconnected,
    IsConnected(bool),
    Statistics(BACnetIOStats),
    Devices(Vec<(String, u32, std::net::SocketAddr)>),
    IoError(String),
}

/// I/O statistics for monitoring and debugging
#[derive(Debug, Clone)]
pub struct BACnetIOStats {
    pub total_reads: u64,
    pub total_writes: u64,
    pub successful_reads: u64,
    pub successful_writes: u64,
    pub failed_reads: u64,
    pub failed_writes: u64,
    pub timeouts: u64,
    pub avg_read_time_ms: f64,
    pub avg_write_time_ms: f64,
    pub connected_devices: usize,
}

//! BACnet types for Neo integration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use ts_rs::TS;
use uuid::Uuid;

/// Discovered BACnet device from I-Am response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DiscoveredDevice {
    /// BACnet device instance number
    pub device_id: u32,
    /// IP address and port
    pub address: String,
    /// Maximum APDU length the device supports
    pub max_apdu: u16,
    /// BACnet vendor ID
    pub vendor_id: u16,
    /// Segmentation support
    pub segmentation: String,
}

/// Result of reading a BACnet property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointReadResult {
    /// Device instance that was read from
    pub device_id: u32,
    /// Object type (e.g., "analog-input", "binary-output")
    pub object_type: String,
    /// Object instance number
    pub instance: u32,
    /// Property that was read (e.g., "present-value")
    pub property: String,
    /// The value read
    pub value: serde_json::Value,
    /// Timestamp when the value was read (unix millis)
    pub timestamp: u64,
}

/// Address cache entry for discovered devices
#[derive(Debug, Clone)]
pub struct DeviceAddress {
    /// Device instance ID
    pub device_id: u32,
    /// Socket address for direct communication
    pub address: SocketAddr,
    /// Max APDU length
    pub max_apdu: u16,
}

/// Commands sent from async service to blocking worker
#[derive(Debug)]
pub enum WorkerCommand {
    /// Send Who-Is broadcast to discover devices (legacy, auto-stores results)
    Discover {
        low_limit: Option<u32>,
        high_limit: Option<u32>,
    },
    /// Start a discovery session that streams results back to a specific client
    DiscoverSession {
        /// Unique ID for this discovery session
        session_id: Uuid,
        /// WebSocket client ID to send results to
        client_id: Uuid,
        /// Request ID from the client (for response correlation)
        request_id: String,
        /// Optional lower bound for device instance range
        low_limit: Option<u32>,
        /// Optional upper bound for device instance range
        high_limit: Option<u32>,
        /// How long to run discovery (seconds)
        duration_secs: u64,
    },
    /// Stop an active discovery session
    StopDiscoverySession {
        session_id: Uuid,
    },
    /// Read a property from a device
    ReadProperty {
        device_id: u32,
        object_type: String,
        instance: u32,
        property: String,
    },
    /// Read the object list from a device
    ReadObjectList {
        device_id: u32,
    },
    /// Start polling values for a device
    StartPolling {
        device_id: u32,
        /// Objects to poll (object_type, instance)
        objects: Vec<(String, u32)>,
        /// Poll interval in milliseconds
        interval_ms: u64,
    },
    /// Stop polling for a device
    StopPolling {
        device_id: u32,
    },
    /// Shutdown the worker thread
    Shutdown,
}

/// A BACnet object reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacnetObject {
    /// Object type name (e.g., "analog-input", "device")
    pub object_type: String,
    /// Object instance number
    pub instance: u32,
}

/// Result of reading an object list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectListResult {
    /// Device instance that was read from
    pub device_id: u32,
    /// List of objects in the device
    pub objects: Vec<BacnetObject>,
}

/// Responses sent from blocking worker to async service
#[derive(Debug)]
pub enum WorkerResponse {
    /// A device was discovered via I-Am (legacy, for auto-store flow)
    DeviceDiscovered(DiscoveredDevice),
    /// A device was discovered during a specific discovery session
    SessionDeviceDiscovered {
        /// WebSocket client ID to send results to
        client_id: Uuid,
        /// Request ID from the client
        request_id: String,
        /// The discovered device
        device: DiscoveredDevice,
    },
    /// A discovery session has completed (timeout reached or stopped)
    SessionComplete {
        /// WebSocket client ID to notify
        client_id: Uuid,
        /// Request ID from the client
        request_id: String,
        /// Number of devices found during this session
        devices_found: u32,
    },
    /// A property was successfully read
    PropertyRead(PointReadResult),
    /// Object list was successfully read
    ObjectListRead(ObjectListResult),
    /// An error occurred
    Error(String),
}

//! BACnet types for Neo integration

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Discovered BACnet device from I-Am response
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Send Who-Is broadcast to discover devices
    Discover {
        low_limit: Option<u32>,
        high_limit: Option<u32>,
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
    /// A device was discovered via I-Am
    DeviceDiscovered(DiscoveredDevice),
    /// A property was successfully read
    PropertyRead(PointReadResult),
    /// Object list was successfully read
    ObjectListRead(ObjectListResult),
    /// An error occurred
    Error(String),
}

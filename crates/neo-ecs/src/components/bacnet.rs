//! BACnet-related components for device and object references.

use flecs_ecs::prelude::*;

/// BACnet device information discovered via I-Am.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct BacnetDevice {
    /// BACnet device instance number
    pub device_id: u32,
    /// IP address and port (e.g., "10.0.1.50:47808")
    pub address: String,
    /// BACnet vendor ID
    pub vendor_id: u16,
    /// Maximum APDU length supported
    pub max_apdu: u16,
    /// Segmentation support level
    pub segmentation: String,
}

/// Reference to a BACnet object within a device.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct BacnetObjectRef {
    /// Object type (e.g., "analog-input", "binary-output")
    pub object_type: String,
    /// Object instance number
    pub instance: u32,
}

/// Current present-value from a BACnet object.
/// This is a generic value holder for any BACnet point.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct BacnetValue {
    /// The current value (can be number, boolean, string, etc.)
    pub value: f64,
    /// Timestamp when the value was last updated (unix millis)
    pub timestamp: u64,
    /// Optional status flags
    pub status: String,
}

/// COV (Change of Value) subscription status.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct CovSubscription {
    /// Whether COV is active
    pub active: bool,
    /// Subscription lifetime remaining (seconds)
    pub lifetime: u32,
    /// COV increment for analog values
    pub increment: f64,
}

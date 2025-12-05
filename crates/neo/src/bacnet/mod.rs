//! BACnet Network Service
//!
//! Provides BACnet/IP network communication for device discovery and point reading.
//!
//! The service runs a blocking worker thread for UDP I/O and bridges to the async
//! service architecture via channels.

mod service;
mod types;
mod worker;

pub use service::{BacnetConfig, BacnetService};
pub use types::{BacnetObject, DiscoveredDevice, ObjectListResult, PointReadResult, WorkerCommand, WorkerResponse};

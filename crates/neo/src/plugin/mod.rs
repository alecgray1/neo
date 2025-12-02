//! Process-isolated plugin system using deno_core
//!
//! Each JS/TS plugin runs in a separate OS process for fault isolation.

mod ipc;
mod process_service;
mod supervisor;
pub mod v8_serde;

pub use ipc::{MessageType, PluginMessage};
pub use process_service::{ProcessService, ProcessServiceConfig};
pub use supervisor::{RestartPolicy, Supervisor};
pub use v8_serde::{deserialize_to_json, serialize_from_json, V8SerdeError};

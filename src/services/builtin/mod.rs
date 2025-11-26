// Built-in native Rust services
//
// This module contains service actor implementations:
// - HistoryActor: Time-series data storage and retrieval
// - AlarmActor: Alarm management and condition monitoring

pub mod alarm;
pub mod history;

// History service
pub use history::{HistoryActor, HistoryConfig, HistoryMsg, HistoryReply};

// Alarm service
pub use alarm::{AlarmActor, AlarmCondition, AlarmConfig, AlarmMsg, AlarmReply};

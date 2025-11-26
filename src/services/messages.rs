// Service request and response types with TypeScript generation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::types::{AlarmSeverity, PointQuality, PointValue, ServiceState};

// ─────────────────────────────────────────────────────────────────────────────
// Service Requests
// ─────────────────────────────────────────────────────────────────────────────

/// Request types that can be sent to any service
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
#[serde(tag = "type")]
pub enum ServiceRequest {
    // ─────────────────────────────────────────────────────────────────────
    // Common requests (all services should handle these)
    // ─────────────────────────────────────────────────────────────────────

    /// Get the current status of the service
    GetStatus,

    /// Get the current configuration
    GetConfig,

    /// Update the configuration
    SetConfig {
        config: serde_json::Value,
    },

    // ─────────────────────────────────────────────────────────────────────
    // History service requests
    // ─────────────────────────────────────────────────────────────────────

    /// Query historical data for a point
    QueryHistory {
        point: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        #[serde(default)]
        limit: Option<u32>,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Alarm service requests
    // ─────────────────────────────────────────────────────────────────────

    /// Get all currently active alarms
    GetActiveAlarms,

    /// Acknowledge an alarm
    AcknowledgeAlarm {
        alarm_id: Uuid,
        #[serde(default)]
        comment: Option<String>,
    },

    /// Get alarm history
    GetAlarmHistory {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        #[serde(default)]
        source_filter: Option<String>,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Scheduler service requests
    // ─────────────────────────────────────────────────────────────────────

    /// Get all schedules
    GetSchedules,

    /// Create a new schedule
    CreateSchedule {
        schedule: Schedule,
    },

    /// Update an existing schedule
    UpdateSchedule {
        id: Uuid,
        schedule: Schedule,
    },

    /// Delete a schedule
    DeleteSchedule {
        id: Uuid,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Custom requests (for plugins and extensibility)
    // ─────────────────────────────────────────────────────────────────────

    /// Custom action with arbitrary payload
    Custom {
        action: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Responses
// ─────────────────────────────────────────────────────────────────────────────

/// Response types from services
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
#[serde(tag = "type")]
pub enum ServiceResponse {
    // ─────────────────────────────────────────────────────────────────────
    // Common responses
    // ─────────────────────────────────────────────────────────────────────

    /// Service status information
    Status {
        id: String,
        name: String,
        state: ServiceState,
        uptime_seconds: u64,
        #[serde(default)]
        extra: Option<serde_json::Value>,
    },

    /// Configuration data
    Config {
        config: serde_json::Value,
    },

    /// Generic success response
    Ok,

    // ─────────────────────────────────────────────────────────────────────
    // History responses
    // ─────────────────────────────────────────────────────────────────────

    /// Historical data samples
    HistoryData {
        samples: Vec<HistorySample>,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Alarm responses
    // ─────────────────────────────────────────────────────────────────────

    /// List of active alarms
    ActiveAlarms {
        alarms: Vec<Alarm>,
    },

    /// Alarm history
    AlarmHistory {
        alarms: Vec<Alarm>,
    },

    /// Alarm was acknowledged
    AlarmAcknowledged {
        alarm_id: Uuid,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Scheduler responses
    // ─────────────────────────────────────────────────────────────────────

    /// List of schedules
    Schedules {
        schedules: Vec<Schedule>,
    },

    /// Schedule was created
    ScheduleCreated {
        id: Uuid,
    },

    /// Schedule was updated
    ScheduleUpdated {
        id: Uuid,
    },

    /// Schedule was deleted
    ScheduleDeleted {
        id: Uuid,
    },

    // ─────────────────────────────────────────────────────────────────────
    // Custom and error responses
    // ─────────────────────────────────────────────────────────────────────

    /// Custom response with arbitrary payload
    Custom {
        payload: serde_json::Value,
    },

    /// Error response
    Error {
        code: String,
        message: String,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// Supporting Types
// ─────────────────────────────────────────────────────────────────────────────

/// A single historical data sample
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct HistorySample {
    pub timestamp: DateTime<Utc>,
    pub value: PointValue,
    pub quality: PointQuality,
}

/// An alarm instance
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct Alarm {
    pub id: Uuid,
    pub source: String,
    pub message: String,
    pub severity: AlarmSeverity,
    pub state: AlarmState,
    pub triggered_at: DateTime<Utc>,
    #[serde(default)]
    pub acknowledged_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub acknowledged_by: Option<String>,
    #[serde(default)]
    pub cleared_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub value_at_trigger: Option<PointValue>,
}

/// Alarm lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub enum AlarmState {
    /// Alarm condition is active
    Active,
    /// Alarm has been acknowledged but not cleared
    Acknowledged,
    /// Alarm condition has cleared
    Cleared,
}

impl Default for AlarmState {
    fn default() -> Self {
        Self::Active
    }
}

/// A schedule definition
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct Schedule {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub name: String,
    pub target_point: String,
    pub entries: Vec<ScheduleEntry>,
    #[serde(default)]
    pub exceptions: Vec<ScheduleException>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// A single schedule entry (time + value)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct ScheduleEntry {
    pub days: Vec<Weekday>,
    /// Time in HH:MM:SS format
    pub time: String,
    pub value: PointValue,
}

/// An exception to the normal schedule (e.g., holidays)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct ScheduleException {
    /// Date in YYYY-MM-DD format
    pub date: String,
    #[serde(default)]
    pub name: Option<String>,
    pub entries: Vec<ScheduleEntry>,
}

/// Days of the week
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

// ─────────────────────────────────────────────────────────────────────────────
// TypeScript Export Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod type_export {
    use super::*;

    #[test]
    fn export_typescript_types() {
        // Run with: cargo test export_typescript_types -- --nocapture
        // Generates .ts files in bindings/

        ServiceRequest::export().expect("Failed to export ServiceRequest");
        ServiceResponse::export().expect("Failed to export ServiceResponse");
        HistorySample::export().expect("Failed to export HistorySample");
        Alarm::export().expect("Failed to export Alarm");
        AlarmState::export().expect("Failed to export AlarmState");
        Schedule::export().expect("Failed to export Schedule");
        ScheduleEntry::export().expect("Failed to export ScheduleEntry");
        ScheduleException::export().expect("Failed to export ScheduleException");
        Weekday::export().expect("Failed to export Weekday");

        println!("TypeScript types exported to bindings/");
    }
}

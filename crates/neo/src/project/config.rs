//! Project Configuration Types
//!
//! Defines the structure of project files on disk.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Project manifest (project.toml)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectManifest {
    pub project: ProjectInfo,
    #[serde(default)]
    pub runtime: RuntimeConfig,
}

/// Project information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    pub description: Option<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Runtime configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub auto_start: bool,
}

/// Device configuration (devices/*.device.toml)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeviceConfig {
    pub device: DeviceInfo,
    #[serde(default)]
    pub connection: ConnectionConfig,
    #[serde(default)]
    pub points: Vec<PointConfig>,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub description: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub location: LocationConfig,
}

/// Device location
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocationConfig {
    pub building: Option<String>,
    pub floor: Option<String>,
    pub zone: Option<String>,
    pub room: Option<String>,
}

/// Connection configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConnectionConfig {
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub address: Option<String>,
    pub device_id: Option<u32>,
    pub port: Option<u16>,
}

fn default_protocol() -> String {
    "bacnet".to_string()
}

/// Point configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PointConfig {
    pub id: String,
    pub name: String,
    pub object_type: String,
    pub instance: u32,
    pub unit: Option<String>,
    pub description: Option<String>,
}

/// Schedule configuration (schedules/*.schedule.toml)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScheduleConfig {
    pub schedule: ScheduleInfo,
    #[serde(default)]
    pub default_schedule: HashMap<String, DaySchedule>,
    #[serde(default)]
    pub zone_overrides: HashMap<String, ZoneOverride>,
}

/// Schedule information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScheduleInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "America/New_York".to_string()
}

/// Day schedule
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DaySchedule {
    #[serde(default)]
    pub occupied: Vec<TimeRange>,
}

/// Time range
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}

/// Zone override
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ZoneOverride {
    pub name: String,
    pub schedule: HashMap<String, DaySchedule>,
    pub notes: Option<String>,
}

/// Alarm rule configuration (alarms/rules.toml)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AlarmRulesConfig {
    #[serde(default)]
    pub rules: Vec<AlarmRule>,
}

/// Alarm rule
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AlarmRule {
    pub id: String,
    pub name: String,
    pub condition: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub message: Option<String>,
}

fn default_priority() -> String {
    "medium".to_string()
}

fn default_true() -> bool {
    true
}

/// Blueprint configuration (blueprints/*.bp.json)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BlueprintConfig {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub nodes: Vec<serde_json::Value>,
    #[serde(default)]
    pub connections: Vec<serde_json::Value>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Plugin manifest (plugins/*/neo-plugin.json)
/// New declarative format with services and nodes arrays.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PluginManifest {
    /// Unique plugin identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Services provided by this plugin
    #[serde(default)]
    pub services: Vec<ServiceEntry>,
    /// Blueprint nodes provided by this plugin
    #[serde(default)]
    pub nodes: Vec<NodeEntry>,
}

/// A service entry in the plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ServiceEntry {
    /// Full service ID (plugin-id/service-name)
    pub id: String,
    /// Path to the built chunk relative to dist
    pub entry: String,
    /// Tick interval in milliseconds
    #[serde(rename = "tickInterval")]
    pub tick_interval: Option<u64>,
    /// Event subscriptions
    #[serde(default)]
    pub subscriptions: Vec<String>,
}

/// A node entry in the plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeEntry {
    /// Full node ID (plugin-id/node-name)
    pub id: String,
    /// Path to the built chunk relative to dist
    pub entry: String,
}

/// A loaded plugin with resolved paths
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Absolute path to the plugin directory
    pub plugin_dir: std::path::PathBuf,
    /// Absolute path to the manifest file's directory (where entry paths are relative to)
    pub manifest_dir: std::path::PathBuf,
}

/// Loaded project with all configuration
#[derive(Debug, Clone)]
pub struct Project {
    /// Project root path
    pub path: std::path::PathBuf,
    /// Project manifest
    pub manifest: ProjectManifest,
    /// Loaded devices
    pub devices: HashMap<String, DeviceConfig>,
    /// Loaded schedules
    pub schedules: HashMap<String, ScheduleConfig>,
    /// Loaded blueprints
    pub blueprints: HashMap<String, BlueprintConfig>,
    /// Loaded plugins
    pub plugins: HashMap<String, LoadedPlugin>,
    /// Loaded alarm rules
    pub alarm_rules: Option<AlarmRulesConfig>,
}

impl Project {
    /// Get project ID
    pub fn id(&self) -> &str {
        &self.manifest.project.id
    }

    /// Get project name
    pub fn name(&self) -> &str {
        &self.manifest.project.name
    }

    /// Get a device by ID
    pub fn get_device(&self, id: &str) -> Option<&DeviceConfig> {
        self.devices.get(id)
    }

    /// Get a schedule by ID
    pub fn get_schedule(&self, id: &str) -> Option<&ScheduleConfig> {
        self.schedules.get(id)
    }
}

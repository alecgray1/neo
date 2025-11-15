// Configuration management
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationConfig {
    pub station_name: String,
    pub license_points: u32,
    pub license_devices: u32,
}

impl Default for StationConfig {
    fn default() -> Self {
        Self {
            station_name: "Neo_Station".to_string(),
            license_points: 500,
            license_devices: 25,
        }
    }
}

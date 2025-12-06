//! HVAC-related components for temperature, setpoints, dampers, etc.

use flecs_ecs::prelude::*;

/// Temperature reading (zone temp, supply temp, discharge temp, etc.)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Temperature {
    /// Temperature value
    pub value: f64,
    /// Unit (e.g., "F", "C")
    pub unit: String,
}

/// Temperature setpoint
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Setpoint {
    /// Setpoint value
    pub value: f64,
}

/// Heating setpoint (for dual-setpoint systems)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct HeatingSetpoint {
    pub value: f64,
}

/// Cooling setpoint (for dual-setpoint systems)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct CoolingSetpoint {
    pub value: f64,
}

/// Damper position (VAV damper, outside air damper, etc.)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct DamperPosition {
    /// Current position (0-100%)
    pub value: f64,
    /// Minimum position limit
    pub min: f64,
    /// Maximum position limit
    pub max: f64,
}

/// Valve position (hot water, chilled water, etc.)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct ValvePosition {
    /// Current position (0-100%)
    pub value: f64,
}

/// Fan speed or status
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct FanSpeed {
    /// Speed as percentage (0-100%) or RPM depending on context
    pub value: f64,
    /// Whether the fan is running
    pub running: bool,
}

/// Airflow measurement (CFM)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Airflow {
    /// Airflow in CFM
    pub value: f64,
    /// Setpoint (if applicable)
    pub setpoint: f64,
}

/// Pressure reading (duct static pressure, etc.)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Pressure {
    /// Pressure value
    pub value: f64,
    /// Unit (e.g., "inWC", "Pa")
    pub unit: String,
}

/// Humidity reading
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Humidity {
    /// Relative humidity (0-100%)
    pub value: f64,
}

/// CO2 level
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct CO2Level {
    /// CO2 concentration in PPM
    pub value: f64,
}

/// Occupancy status
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Occupancy {
    /// Whether space is occupied
    pub occupied: bool,
    /// Occupancy count (if available)
    pub count: u32,
}

/// Operating mode
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct OperatingMode {
    /// Current mode (e.g., "auto", "cool", "heat", "off")
    pub mode: String,
}

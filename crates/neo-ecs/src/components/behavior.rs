//! Behavior components and tags that enable systems.
//!
//! Adding a behavior component to an entity activates the corresponding
//! system/observer for that entity.

use flecs_ecs::prelude::*;

// =============================================================================
// Behavior Components (with data)
// =============================================================================

/// PID control parameters. When present, the PID system runs for this entity.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct PidControl {
    /// Proportional gain
    pub kp: f64,
    /// Integral gain
    pub ki: f64,
    /// Derivative gain
    pub kd: f64,
    /// Integral accumulator (internal state)
    pub integral: f64,
    /// Previous error (internal state)
    pub prev_error: f64,
    /// Output limits
    pub output_min: f64,
    pub output_max: f64,
}

impl Default for PidControl {
    fn default() -> Self {
        Self {
            kp: 1.0,
            ki: 0.1,
            kd: 0.01,
            integral: 0.0,
            prev_error: 0.0,
            output_min: 0.0,
            output_max: 100.0,
        }
    }
}

/// Alarm limits. When present, values are checked against limits.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct AlarmLimits {
    /// High alarm limit
    pub high_limit: f64,
    /// Low alarm limit
    pub low_limit: f64,
    /// Deadband to prevent alarm chatter
    pub deadband: f64,
    /// Delay before alarming (seconds)
    pub delay: u32,
}

/// Trend configuration. When present, values are recorded periodically.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Trended {
    /// Recording interval in seconds
    pub interval: u32,
    /// Last recording timestamp (unix millis)
    pub last_recorded: u64,
}

/// Schedule configuration. When present, entity follows a schedule.
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Scheduled {
    /// Schedule name/ID
    pub schedule: String,
    /// Current scheduled value
    pub scheduled_value: f64,
    /// Whether override is active
    pub override_active: bool,
}

/// Commandable point (can receive write commands)
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct Commandable {
    /// Current commanded value
    pub commanded_value: f64,
    /// Priority level (1-16, BACnet style)
    pub priority: u8,
    /// Whether command is active
    pub active: bool,
}

// =============================================================================
// Device Type Tags (zero-size markers)
// =============================================================================

/// Variable Air Volume box
#[derive(Debug, Clone, Component, Default)]
pub struct VavBox;

/// Air Handling Unit
#[derive(Debug, Clone, Component, Default)]
pub struct Ahu;

/// Fan Coil Unit
#[derive(Debug, Clone, Component, Default)]
pub struct Fcu;

/// Chiller
#[derive(Debug, Clone, Component, Default)]
pub struct Chiller;

/// Boiler
#[derive(Debug, Clone, Component, Default)]
pub struct Boiler;

/// Heat Pump
#[derive(Debug, Clone, Component, Default)]
pub struct HeatPump;

/// Rooftop Unit
#[derive(Debug, Clone, Component, Default)]
pub struct Rtu;

/// Energy Meter
#[derive(Debug, Clone, Component, Default)]
pub struct EnergyMeter;

// =============================================================================
// Status Tags (zero-size markers)
// =============================================================================

/// Entity is offline (communication lost)
#[derive(Debug, Clone, Component, Default)]
pub struct Offline;

/// Entity is in alarm condition
#[derive(Debug, Clone, Component, Default)]
pub struct InAlarm;

/// Entity needs service/maintenance
#[derive(Debug, Clone, Component, Default)]
pub struct NeedsService;

/// Entity is in commissioning mode
#[derive(Debug, Clone, Component, Default)]
pub struct Commissioning;

/// Entity is overridden from normal control
#[derive(Debug, Clone, Component, Default)]
pub struct Overridden;

/// Entity is in manual mode
#[derive(Debug, Clone, Component, Default)]
pub struct ManualMode;

/// Entity is enabled
#[derive(Debug, Clone, Component, Default)]
pub struct Enabled;

/// Entity is disabled
#[derive(Debug, Clone, Component, Default)]
pub struct Disabled;

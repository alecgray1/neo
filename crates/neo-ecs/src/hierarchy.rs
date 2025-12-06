//! Building hierarchy components and relationships.
//!
//! Represents the physical structure: Site → Building → Floor → Zone → Device → Point

use flecs_ecs::prelude::*;

// =============================================================================
// Hierarchy Level Tags
// =============================================================================

/// Site level (campus, portfolio)
#[derive(Debug, Clone, Component, Default)]
pub struct Site;

/// Building level
#[derive(Debug, Clone, Component, Default)]
pub struct Building;

/// Floor level
#[derive(Debug, Clone, Component, Default)]
pub struct Floor;

/// Zone level (thermal zone, lighting zone, etc.)
#[derive(Debug, Clone, Component, Default)]
pub struct Zone;

/// Device level (controller, equipment)
#[derive(Debug, Clone, Component, Default)]
pub struct Device;

/// Point level (sensor, actuator)
#[derive(Debug, Clone, Component, Default)]
pub struct Point;

// =============================================================================
// Hierarchy Metadata Components
// =============================================================================

/// Floor information
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct FloorInfo {
    /// Floor number (can be negative for basements)
    pub floor_number: i32,
    /// Floor area in square feet
    pub area_sqft: f64,
}

/// Zone information
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct ZoneInfo {
    /// Zone type (e.g., "office", "conference", "lobby")
    pub zone_type: String,
    /// Design occupancy
    pub design_occupancy: u32,
    /// Zone area in square feet
    pub area_sqft: f64,
}

/// Building information
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct BuildingInfo {
    /// Street address
    pub address: String,
    /// Total square footage
    pub area_sqft: f64,
    /// Number of floors
    pub floor_count: i32,
    /// Year built
    pub year_built: u32,
}

/// Site information
#[derive(Debug, Clone, Component)]
#[flecs(meta)]
pub struct SiteInfo {
    /// Site name
    pub name: String,
    /// Geographic location
    pub latitude: f64,
    pub longitude: f64,
    /// Timezone (e.g., "America/New_York")
    pub timezone: String,
}

// =============================================================================
// Helper functions for hierarchy operations
// =============================================================================

/// Hierarchy level enum for querying
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyLevel {
    Site,
    Building,
    Floor,
    Zone,
    Device,
    Point,
}

impl HierarchyLevel {
    /// Get the parent level (None for Site)
    pub fn parent(&self) -> Option<HierarchyLevel> {
        match self {
            HierarchyLevel::Site => None,
            HierarchyLevel::Building => Some(HierarchyLevel::Site),
            HierarchyLevel::Floor => Some(HierarchyLevel::Building),
            HierarchyLevel::Zone => Some(HierarchyLevel::Floor),
            HierarchyLevel::Device => Some(HierarchyLevel::Zone),
            HierarchyLevel::Point => Some(HierarchyLevel::Device),
        }
    }

    /// Get the child level (None for Point)
    pub fn child(&self) -> Option<HierarchyLevel> {
        match self {
            HierarchyLevel::Site => Some(HierarchyLevel::Building),
            HierarchyLevel::Building => Some(HierarchyLevel::Floor),
            HierarchyLevel::Floor => Some(HierarchyLevel::Zone),
            HierarchyLevel::Zone => Some(HierarchyLevel::Device),
            HierarchyLevel::Device => Some(HierarchyLevel::Point),
            HierarchyLevel::Point => None,
        }
    }
}

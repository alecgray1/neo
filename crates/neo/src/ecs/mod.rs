//! ECS Service for Neo
//!
//! Integrates Flecs ECS into Neo's service architecture, providing:
//! - Entity-based data model for BACnet devices and points
//! - Dynamic component registration from TOML schemas
//! - Persistence via JSON serialization
//! - Bridge from BACnet events to ECS entities

mod service;

pub use service::{EcsConfig, EcsService};
pub use neo_ecs::{EcsHandle, EcsWorld, ComponentSchema, EntityId};

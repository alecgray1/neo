//! Neo ECS - Entity Component System for Building Management
//!
//! Provides an ECS-based data model for BACnet devices and points using Flecs.
//!
//! ## Architecture
//!
//! The ECS world runs in a dedicated worker thread (Flecs World is not Send+Sync).
//! Communication happens via command/response channels.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Async Services                           │
//! │  BacnetService  │  BlueprintExecutor  │  JsServices         │
//! └────────┬────────┴──────────┬──────────┴──────────┬──────────┘
//!          │                   │                     │
//!          ▼                   ▼                     ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ECS Command Channel                       │
//! └─────────────────────────────────────────────────────────────┘
//!                               │
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ECS Worker Thread                         │
//! │  - Owns Flecs World                                         │
//! │  - Processes commands                                        │
//! │  - Runs observers                                            │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod commands;
pub mod components;
pub mod hierarchy;
pub mod registry;
pub mod world;

pub use commands::{EcsCommand, EcsResponse, EntityId, QueryResult};
pub use components::*;
pub use hierarchy::*;
pub use registry::{ComponentRegistry, ComponentSchema, FieldDef, FieldType, RegistryError};
pub use world::{EcsHandle, EcsWorld};

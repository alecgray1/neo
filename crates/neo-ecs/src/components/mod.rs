//! Static component definitions for BMS entities.
//!
//! Components are defined with `#[derive(Component)]` and `#[flecs(meta)]`
//! to enable runtime reflection for serialization and dynamic access.

pub mod bacnet;
pub mod behavior;
pub mod hvac;

pub use bacnet::*;
pub use behavior::*;
pub use hvac::*;

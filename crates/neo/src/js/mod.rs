//! JavaScript Runtime Integration
//!
//! This module provides QuickJS integration for Neo, allowing services and
//! plugins to be written in JavaScript.
//!
//! # Example
//!
//! ```javascript
//! // Service lifecycle hooks
//! function onStart(config) {
//!     console.log("Service started with:", config);
//! }
//!
//! function onEvent(event) {
//!     if (event.type === "PointValueChanged") {
//!         console.log("Point changed:", event.data);
//!     }
//! }
//!
//! function onTick() {
//!     // Called periodically if tick_interval is set
//! }
//!
//! function onStop() {
//!     console.log("Service stopped");
//! }
//! ```
//!
//! # Global Objects
//!
//! The following globals are available in JavaScript services:
//!
//! - `neo.log(msg)` - Log at info level
//! - `neo.debug(msg)` - Log at debug level
//! - `neo.warn(msg)` - Log at warn level
//! - `neo.error(msg)` - Log at error level
//! - `console.log(...)` - Print to stdout
//! - `console.info(...)` - Log at info level
//! - `console.warn(...)` - Log at warn level
//! - `console.error(...)` - Log at error level
//! - `console.debug(...)` - Log at debug level

mod globals;
mod runtime;
mod service;

pub use globals::register_neo_globals;
pub use runtime::{JsError, JsResult, JsRuntime};
pub use service::{JsService, JsServiceConfig};

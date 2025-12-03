//! JavaScript Services
//!
//! Services can be implemented in JavaScript using neo-js-runtime.
//! Each JsService runs in its own thread with its own V8 isolate.

mod js_service;

pub use js_service::{JsService, JsServiceConfig};

//! JavaScript plugin system using in-process thread-based runtime
//!
//! Plugins run in dedicated threads using neo-js-runtime. Each plugin
//! gets its own V8 isolate running in a thread with a LocalSet (no work-stealing).
//!
//! Following Deno's pattern, crash recovery is not handled at the runtime level.
//! If a plugin crashes, the error is propagated to the service manager which
//! decides how to handle it (log, restart, alert, etc.).

mod js_service;

pub use js_service::{JsPluginConfig, JsPluginService};

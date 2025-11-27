// Plugin System - JavaScript/TypeScript plugin support via Deno runtime
//
// Plugins are loaded from the ./plugins directory and run in a shared
// JsRuntimePool for efficient resource usage. Each plugin is represented
// as a PluginActor that communicates with the pool.
//
// Plugin Lifecycle:
// 1. Discovery: Scan plugins directory for neo-plugin.json manifests
// 2. Loading: Create PluginActor for each discovered plugin
// 3. Initialization: Pool assigns plugin to a worker thread
// 4. Runtime: Plugin receives events and can:
//    - Access configuration
//    - Handle service requests
//    - Read and write point values
//    - Publish events back to the system

pub mod actor;
pub mod loader;
pub mod ops;
pub mod pool;
pub mod service;

// Plugin actor
pub use actor::{PluginActor, PluginMsg, PluginReply};

// Plugin loader
pub use loader::{discover_plugins, load_plugins, DiscoveredPlugin};

// Runtime pool
pub use pool::{JsRuntimePoolActor, PoolMsg, PoolReply, PoolStatus, WorkerCommand};

// Plugin manifest and bridge
pub use ops::{PluginBridge, PointReadRequest, PointReadResponse, PointWriteRequest};
pub use service::PluginManifest;

// Deno Ops - Native functions exposed to JavaScript plugins
//
// These ops provide the bridge between JavaScript plugins and the Neo runtime.
// They are registered via the deno_core extension! macro and called via Deno.core.ops.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use chrono::Utc;
use deno_core::op2;
use deno_core::OpState;
use tokio::sync::mpsc;

use crate::blueprints::NodeRegistryExt;
use crate::messages::Event;
use crate::types::PointValue;

/// Bridge state passed to each plugin runtime
/// Contains references to Neo subsystems and plugin-specific data
#[derive(Clone)]
pub struct PluginBridge {
    /// Plugin identifier for logging
    pub plugin_id: String,
    /// Plugin configuration
    pub config: serde_json::Value,
    /// Channel to send events back to the main runtime
    pub event_tx: mpsc::UnboundedSender<Event>,
    /// Channel to request point reads
    pub point_read_tx: mpsc::UnboundedSender<PointReadRequest>,
    /// Channel to receive point read responses
    pub point_read_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<PointReadResponse>>>,
    /// Channel to request point writes
    pub point_write_tx: mpsc::UnboundedSender<PointWriteRequest>,
}

/// Request to read a point value
#[derive(Debug, Clone)]
pub struct PointReadRequest {
    pub path: String,
    pub request_id: u64,
}

/// Response from a point read
#[derive(Debug, Clone)]
pub struct PointReadResponse {
    pub request_id: u64,
    pub result: Result<PointValue, String>,
}

/// Request to write a point value
#[derive(Debug, Clone)]
pub struct PointWriteRequest {
    pub path: String,
    pub value: PointValue,
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Get the plugin's configuration
#[op2]
#[serde]
pub fn op_neo_get_config(state: &OpState) -> serde_json::Value {
    let bridge = state.borrow::<PluginBridge>();
    bridge.config.clone()
}

/// Get the plugin's ID
#[op2]
#[string]
pub fn op_neo_get_plugin_id(state: &OpState) -> String {
    let bridge = state.borrow::<PluginBridge>();
    bridge.plugin_id.clone()
}

// ─────────────────────────────────────────────────────────────────────────────
// Logging Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Log a message at the specified level
#[op2(fast)]
pub fn op_neo_log(state: &OpState, #[string] level: &str, #[string] message: &str) {
    let bridge = state.borrow::<PluginBridge>();
    let plugin_id = &bridge.plugin_id;

    match level {
        "trace" => tracing::trace!(plugin = %plugin_id, "{}", message),
        "debug" => tracing::debug!(plugin = %plugin_id, "{}", message),
        "info" => tracing::info!(plugin = %plugin_id, "{}", message),
        "warn" => tracing::warn!(plugin = %plugin_id, "{}", message),
        "error" => tracing::error!(plugin = %plugin_id, "{}", message),
        _ => tracing::info!(plugin = %plugin_id, "{}", message),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Event Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Publish an event to the Neo event bus
/// Accepts raw JSON and converts unknown event types to Event::Custom
#[op2]
pub fn op_neo_event_publish(
    state: &OpState,
    #[serde] event_json: serde_json::Value,
) -> Result<(), deno_core::error::AnyError> {
    let bridge = state.borrow::<PluginBridge>();

    // Try to deserialize as a known Event type first
    let event = match serde_json::from_value::<Event>(event_json.clone()) {
        Ok(event) => event,
        Err(_) => {
            // Unknown event type - convert to Custom event
            let event_type = event_json.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();

            let source = event_json.get("source")
                .and_then(|v| v.as_str())
                .unwrap_or(&bridge.plugin_id)
                .to_string();

            // Extract data field, or use the whole object minus type/source/timestamp
            let data = event_json.get("data")
                .cloned()
                .unwrap_or_else(|| {
                    let mut obj = event_json.clone();
                    if let Some(o) = obj.as_object_mut() {
                        o.remove("type");
                        o.remove("source");
                        o.remove("timestamp");
                    }
                    obj
                });

            Event::Custom {
                event_type,
                source,
                data,
                timestamp: std::time::Instant::now(),
                timestamp_utc: Utc::now(),
            }
        }
    };

    bridge
        .event_tx
        .send(event)
        .map_err(|e| deno_core::error::generic_error(format!("Failed to publish event: {}", e)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Point Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Read a point value (async)
/// Path format: "network/device/objectType:instance" or "point/path"
#[op2(async)]
#[serde]
pub async fn op_neo_point_read(
    state: Rc<RefCell<OpState>>,
    #[string] path: String,
) -> Result<PointValue, deno_core::error::AnyError> {
    // Get a unique request ID
    static REQUEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let request_id = REQUEST_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let (read_tx, read_rx) = {
        let state = state.borrow();
        let bridge = state.borrow::<PluginBridge>();
        (bridge.point_read_tx.clone(), bridge.point_read_rx.clone())
    };

    // Send read request
    read_tx
        .send(PointReadRequest {
            path: path.clone(),
            request_id,
        })
        .map_err(|e| deno_core::error::generic_error(format!("Failed to send read request: {}", e)))?;

    // Wait for response
    let mut rx = read_rx.lock().await;
    while let Some(response) = rx.recv().await {
        if response.request_id == request_id {
            return response
                .result
                .map_err(|e| deno_core::error::generic_error(e));
        }
    }

    Err(deno_core::error::generic_error("Read request channel closed"))
}

/// Write a point value
#[op2]
pub fn op_neo_point_write(
    state: &OpState,
    #[string] path: String,
    #[serde] value: PointValue,
) -> Result<(), deno_core::error::AnyError> {
    let bridge = state.borrow::<PluginBridge>();

    bridge
        .point_write_tx
        .send(PointWriteRequest { path, value })
        .map_err(|e| deno_core::error::generic_error(format!("Failed to send write request: {}", e)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Time Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Get the current timestamp in milliseconds
#[op2(fast)]
pub fn op_neo_now_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Blueprint node info for JS
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsNodeInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub pure: bool,
    pub latent: bool,
    pub description: Option<String>,
}

/// List all available blueprint nodes
#[op2]
#[serde]
pub fn op_neo_blueprint_list_nodes(_state: &OpState) -> Vec<JsNodeInfo> {
    use crate::blueprints::NodeRegistry;

    let registry = NodeRegistry::with_builtins();
    registry.definitions().map(|def| JsNodeInfo {
        id: def.id.clone(),
        name: def.name.clone(),
        category: def.category.clone(),
        pure: def.pure,
        latent: def.latent,
        description: def.description.clone(),
    }).collect()
}

/// Get categories of blueprint nodes
#[op2]
#[serde]
pub fn op_neo_blueprint_get_categories(_state: &OpState) -> Vec<String> {
    use crate::blueprints::NodeRegistry;

    let registry = NodeRegistry::with_builtins();
    registry.categories()
}

// ─────────────────────────────────────────────────────────────────────────────
// Extension Definition
// ─────────────────────────────────────────────────────────────────────────────

/// The Neo plugin runtime JavaScript code
pub const RUNTIME_JS: &str = include_str!("runtime.js");

deno_core::extension!(
    neo_plugin,
    ops = [
        op_neo_get_config,
        op_neo_get_plugin_id,
        op_neo_log,
        op_neo_event_publish,
        op_neo_point_read,
        op_neo_point_write,
        op_neo_now_ms,
        op_neo_blueprint_list_nodes,
        op_neo_blueprint_get_categories,
    ],
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_bridge_clone() {
        let (event_tx, _) = mpsc::unbounded_channel();
        let (read_tx, _) = mpsc::unbounded_channel();
        let (_, read_rx) = mpsc::unbounded_channel();
        let (write_tx, _) = mpsc::unbounded_channel();

        let bridge = PluginBridge {
            plugin_id: "test".to_string(),
            config: serde_json::json!({"key": "value"}),
            event_tx,
            point_read_tx: read_tx,
            point_read_rx: Arc::new(tokio::sync::Mutex::new(read_rx)),
            point_write_tx: write_tx,
        };

        let cloned = bridge.clone();
        assert_eq!(cloned.plugin_id, "test");
    }
}

//! Typed structures for Neo JS runtime communication.
//!
//! These types are serialized/deserialized via serde_v8, eliminating the need
//! for JSON string serialization when passing data between Rust and JavaScript.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint types for JS execution
// ─────────────────────────────────────────────────────────────────────────────

/// Simplified blueprint structure for JS execution.
/// Contains only what the JS executor needs to run the blueprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintJs {
    /// Blueprint ID
    pub id: String,
    /// Blueprint name
    pub name: String,
    /// Nodes in the blueprint
    pub nodes: Vec<BlueprintNodeJs>,
    /// Connections between nodes
    pub connections: Vec<ConnectionJs>,
    /// Variable definitions with default values
    pub variables: HashMap<String, serde_json::Value>,
}

/// A node instance for JS execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintNodeJs {
    /// Unique instance ID
    pub id: String,
    /// Node type (e.g., "math/Add", "flow/Branch")
    #[serde(rename = "type")]
    pub node_type: String,
    /// Node configuration
    pub config: serde_json::Value,
}

/// A connection between pins for JS execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionJs {
    /// Source: "node_id.pin_name"
    pub from: String,
    /// Destination: "node_id.pin_name"
    pub to: String,
}


// ─────────────────────────────────────────────────────────────────────────────
// Execution result types
// ─────────────────────────────────────────────────────────────────────────────

/// Result from executing a blueprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResultJs {
    /// Status of the execution
    pub status: String,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Output values if completed (node outputs keyed by node ID)
    #[serde(default)]
    pub outputs: HashMap<String, serde_json::Value>,
    /// Final variable values after execution
    #[serde(default)]
    pub variables: HashMap<String, serde_json::Value>,
}

// Blueprint Types - Core data structures for the visual scripting system
//
// These types define the structure of blueprints, nodes, pins, and connections.
// Blueprints are stored as JSON files and loaded at runtime.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Pin Types
// ─────────────────────────────────────────────────────────────────────────────

/// Direction of a pin on a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PinDirection {
    Input,
    Output,
}

/// Data types that can flow through pins
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum PinType {
    /// Execution flow (no data, just control flow)
    Exec,
    /// 32-bit floating point
    Real,
    /// 32-bit signed integer
    Integer,
    /// Boolean value
    Boolean,
    /// String value
    String,
    /// Neo PointValue (any point value type)
    PointValue,
    /// Array of a specific type
    Array { element: Box<PinType> },
    /// Dynamic type (serde_json::Value) - accepts anything
    Any,
}

impl PinType {
    /// Check if this type is compatible with another (for connection validation)
    pub fn is_compatible_with(&self, other: &PinType) -> bool {
        match (self, other) {
            // Exact match
            (a, b) if a == b => true,
            // Any accepts everything
            (PinType::Any, _) | (_, PinType::Any) => true,
            // PointValue can accept Real, Integer, Boolean (common point value types)
            (PinType::PointValue, PinType::Real)
            | (PinType::PointValue, PinType::Integer)
            | (PinType::PointValue, PinType::Boolean) => true,
            (PinType::Real, PinType::PointValue)
            | (PinType::Integer, PinType::PointValue)
            | (PinType::Boolean, PinType::PointValue) => true,
            // Integer can be implicitly converted to Real
            (PinType::Real, PinType::Integer) | (PinType::Integer, PinType::Real) => true,
            // Array compatibility
            (PinType::Array { element: a }, PinType::Array { element: b }) => {
                a.is_compatible_with(b)
            }
            _ => false,
        }
    }

    /// Check if this is an execution pin type
    pub fn is_exec(&self) -> bool {
        matches!(self, PinType::Exec)
    }

    /// Check if this is a data pin type
    pub fn is_data(&self) -> bool {
        !self.is_exec()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pin Definitions
// ─────────────────────────────────────────────────────────────────────────────

/// Definition of a pin on a node type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinDef {
    /// Pin name (used in connections)
    pub name: String,
    /// Pin direction (input or output)
    pub direction: PinDirection,
    /// Data type of the pin
    #[serde(rename = "type")]
    pub pin_type: PinType,
    /// Default value for input pins (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl PinDef {
    /// Create an execution input pin
    pub fn exec_in() -> Self {
        Self {
            name: "exec".to_string(),
            direction: PinDirection::Input,
            pin_type: PinType::Exec,
            default: None,
            description: None,
        }
    }

    /// Create an execution output pin with a custom name
    pub fn exec_out(name: &str) -> Self {
        Self {
            name: name.to_string(),
            direction: PinDirection::Output,
            pin_type: PinType::Exec,
            default: None,
            description: None,
        }
    }

    /// Create a data input pin
    pub fn data_in(name: &str, pin_type: PinType) -> Self {
        Self {
            name: name.to_string(),
            direction: PinDirection::Input,
            pin_type,
            default: None,
            description: None,
        }
    }

    /// Create a data input pin with a default value
    pub fn data_in_with_default(
        name: &str,
        pin_type: PinType,
        default: serde_json::Value,
    ) -> Self {
        Self {
            name: name.to_string(),
            direction: PinDirection::Input,
            pin_type,
            default: Some(default),
            description: None,
        }
    }

    /// Create a data output pin
    pub fn data_out(name: &str, pin_type: PinType) -> Self {
        Self {
            name: name.to_string(),
            direction: PinDirection::Output,
            pin_type,
            default: None,
            description: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node Definitions
// ─────────────────────────────────────────────────────────────────────────────

/// Definition of a node type (registered in the NodeRegistry)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    /// Unique identifier (e.g., "neo/Branch" or "my-plugin/CustomNode")
    pub id: String,
    /// Human-readable display name
    pub name: String,
    /// Category for organization (e.g., "Flow Control", "Math")
    pub category: String,
    /// Whether this is a pure node (no exec pins, evaluated on demand)
    #[serde(default)]
    pub pure: bool,
    /// Whether this node can suspend execution (latent node)
    #[serde(default)]
    pub latent: bool,
    /// Pin definitions for this node type
    pub pins: Vec<PinDef>,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl NodeDef {
    /// Get all input pins
    pub fn input_pins(&self) -> impl Iterator<Item = &PinDef> {
        self.pins
            .iter()
            .filter(|p| p.direction == PinDirection::Input)
    }

    /// Get all output pins
    pub fn output_pins(&self) -> impl Iterator<Item = &PinDef> {
        self.pins
            .iter()
            .filter(|p| p.direction == PinDirection::Output)
    }

    /// Get all execution input pins
    pub fn exec_inputs(&self) -> impl Iterator<Item = &PinDef> {
        self.input_pins().filter(|p| p.pin_type.is_exec())
    }

    /// Get all execution output pins
    pub fn exec_outputs(&self) -> impl Iterator<Item = &PinDef> {
        self.output_pins().filter(|p| p.pin_type.is_exec())
    }

    /// Get all data input pins
    pub fn data_inputs(&self) -> impl Iterator<Item = &PinDef> {
        self.input_pins().filter(|p| p.pin_type.is_data())
    }

    /// Get all data output pins
    pub fn data_outputs(&self) -> impl Iterator<Item = &PinDef> {
        self.output_pins().filter(|p| p.pin_type.is_data())
    }

    /// Get a pin by name
    pub fn get_pin(&self, name: &str) -> Option<&PinDef> {
        self.pins.iter().find(|p| p.name == name)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint Structure
// ─────────────────────────────────────────────────────────────────────────────

/// Variable definition within a blueprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDef {
    /// Data type of the variable
    #[serde(rename = "type")]
    pub var_type: PinType,
    /// Default value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Position in the visual editor (for UI purposes)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// A node instance within a blueprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintNode {
    /// Unique instance ID within this blueprint
    pub id: String,
    /// Node type (references NodeDef.id)
    #[serde(rename = "type")]
    pub node_type: String,
    /// Position in the visual editor
    #[serde(default)]
    pub position: Position,
    /// Node-specific configuration (e.g., operator for Compare node)
    #[serde(default)]
    pub config: serde_json::Value,
}

/// A connection between two pins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    /// Source: "node_id.pin_name"
    pub from: String,
    /// Destination: "node_id.pin_name"
    pub to: String,
}

impl Connection {
    /// Parse the "from" field into (node_id, pin_name)
    pub fn from_parts(&self) -> Option<(&str, &str)> {
        self.from.split_once('.')
    }

    /// Parse the "to" field into (node_id, pin_name)
    pub fn to_parts(&self) -> Option<(&str, &str)> {
        self.to.split_once('.')
    }

    /// Create a new connection
    pub fn new(from_node: &str, from_pin: &str, to_node: &str, to_pin: &str) -> Self {
        Self {
            from: format!("{}.{}", from_node, from_pin),
            to: format!("{}.{}", to_node, to_pin),
        }
    }
}

/// Complete blueprint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Version string
    #[serde(default = "default_version")]
    pub version: String,
    /// Description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Blueprint-level variables
    #[serde(default)]
    pub variables: HashMap<String, VariableDef>,
    /// Nodes in this blueprint
    #[serde(default)]
    pub nodes: Vec<BlueprintNode>,
    /// Connections between nodes
    #[serde(default)]
    pub connections: Vec<Connection>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl Blueprint {
    /// Create a new empty blueprint
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            version: default_version(),
            description: None,
            variables: HashMap::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
        }
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&BlueprintNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all connections from a specific node and pin
    pub fn connections_from(&self, node_id: &str, pin_name: &str) -> Vec<&Connection> {
        let prefix = format!("{}.{}", node_id, pin_name);
        self.connections
            .iter()
            .filter(|c| c.from == prefix)
            .collect()
    }

    /// Get all connections to a specific node and pin
    pub fn connections_to(&self, node_id: &str, pin_name: &str) -> Vec<&Connection> {
        let prefix = format!("{}.{}", node_id, pin_name);
        self.connections.iter().filter(|c| c.to == prefix).collect()
    }

    /// Get all event nodes (nodes with no exec input connections)
    pub fn event_nodes(&self) -> Vec<&BlueprintNode> {
        self.nodes
            .iter()
            .filter(|n| {
                // Event nodes typically start with "On" or have no exec inputs
                n.node_type.contains("/On") || n.node_type.ends_with("Event")
            })
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Execution Types
// ─────────────────────────────────────────────────────────────────────────────

/// What triggered a blueprint execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionTrigger {
    /// Triggered by an event
    Event {
        event_type: String,
        data: serde_json::Value,
    },
    /// Triggered by a schedule
    Schedule { schedule_id: String },
    /// Triggered by a manual request
    Request { inputs: serde_json::Value },
}

/// Result of executing a single node
#[derive(Debug, Clone)]
pub enum NodeResult {
    /// Continue execution from the specified output exec pin
    Continue(String),
    /// Node execution completed, no more execution from this node
    End,
    /// Node is latent (async), execution is suspended
    Latent(LatentState),
    /// Node produced an error
    Error(String),
}

/// State for a suspended latent node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatentState {
    /// Node that is suspended
    pub node_id: String,
    /// Execution pin to resume from
    pub resume_pin: String,
    /// Condition to wake up
    pub wake_condition: WakeCondition,
}

/// Condition that will resume a latent node
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WakeCondition {
    /// Wake after a delay
    Delay {
        /// Unix timestamp (ms) when to wake
        until_ms: u64,
    },
    /// Wake when a specific event occurs
    Event {
        event_type: String,
        #[serde(default)]
        filter: Option<serde_json::Value>,
    },
    /// Wake when a point value changes
    PointChanged {
        point_path: String,
        #[serde(default)]
        condition: Option<PointCondition>,
    },
}

/// Condition for point value matching
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PointCondition {
    /// Any change
    Changed,
    /// Value equals
    Equals { value: serde_json::Value },
    /// Value greater than
    GreaterThan { value: f64 },
    /// Value less than
    LessThan { value: f64 },
    /// Value in range
    InRange { min: f64, max: f64 },
}

/// Result of blueprint execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ExecutionResult {
    /// Execution completed successfully
    Completed {
        #[serde(default)]
        outputs: HashMap<String, serde_json::Value>,
    },
    /// Execution is suspended, waiting for a condition
    Suspended { state: LatentState },
    /// Execution failed with an error
    Failed { error: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_type_compatibility() {
        assert!(PinType::Real.is_compatible_with(&PinType::Real));
        assert!(PinType::Real.is_compatible_with(&PinType::Integer));
        assert!(PinType::Any.is_compatible_with(&PinType::String));
        assert!(!PinType::Boolean.is_compatible_with(&PinType::String));
    }

    #[test]
    fn test_connection_parsing() {
        let conn = Connection::new("node1", "output", "node2", "input");
        assert_eq!(conn.from_parts(), Some(("node1", "output")));
        assert_eq!(conn.to_parts(), Some(("node2", "input")));
    }

    #[test]
    fn test_blueprint_json_roundtrip() {
        let json = r#"{
            "id": "test-bp",
            "name": "Test Blueprint",
            "nodes": [
                {"id": "n1", "type": "neo/Branch", "config": {}}
            ],
            "connections": [
                {"from": "n1.true", "to": "n2.exec"}
            ]
        }"#;

        let bp: Blueprint = serde_json::from_str(json).unwrap();
        assert_eq!(bp.id, "test-bp");
        assert_eq!(bp.nodes.len(), 1);
        assert_eq!(bp.connections.len(), 1);

        // Roundtrip
        let json2 = serde_json::to_string(&bp).unwrap();
        let bp2: Blueprint = serde_json::from_str(&json2).unwrap();
        assert_eq!(bp.id, bp2.id);
    }
}

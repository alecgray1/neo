// Blueprint Types - Core data structures for the visual scripting system
//
// These types define the structure of blueprints, nodes, pins, and connections.
// Blueprints are stored as JSON files and loaded at runtime.

use std::any::Any;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Abstract Types for Neo Integration
// ─────────────────────────────────────────────────────────────────────────────

/// Type-erased event for cross-crate compatibility
/// Neo provides concrete Event type at runtime
pub type DynEvent = Box<dyn Any + Send + Sync>;

/// Type-erased service request for cross-crate compatibility
/// Neo provides concrete ServiceRequest type at runtime
pub type DynRequest = Box<dyn Any + Send + Sync>;

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
    /// User-defined struct type
    Struct { struct_id: String },
    /// User-defined event type (from TypeRegistry)
    Event { event_id: String },
    /// User-defined object type (from TypeRegistry)
    Object { object_id: String },
    /// Opaque handle to a Rust object
    Handle { target_type: String },
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
            // Struct compatibility - must be same struct type
            (PinType::Struct { struct_id: a }, PinType::Struct { struct_id: b }) => a == b,
            // Event compatibility - must be same event type
            (PinType::Event { event_id: a }, PinType::Event { event_id: b }) => a == b,
            // Object compatibility - must be same object type
            (PinType::Object { object_id: a }, PinType::Object { object_id: b }) => a == b,
            // Handle compatibility - must be same target type
            (PinType::Handle { target_type: a }, PinType::Handle { target_type: b }) => a == b,
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

    /// Get the user-defined type ID if this is a user-defined type
    pub fn user_type_id(&self) -> Option<&str> {
        match self {
            PinType::Struct { struct_id } => Some(struct_id),
            PinType::Event { event_id } => Some(event_id),
            PinType::Object { object_id } => Some(object_id),
            PinType::Handle { target_type } => Some(target_type),
            _ => None,
        }
    }

    /// Check if this is a user-defined type (struct, event, object, or handle)
    pub fn is_user_defined(&self) -> bool {
        self.user_type_id().is_some()
    }

    /// Create a new Event pin type
    pub fn event(event_id: impl Into<String>) -> Self {
        PinType::Event {
            event_id: event_id.into(),
        }
    }

    /// Create a new Object pin type
    pub fn object(object_id: impl Into<String>) -> Self {
        PinType::Object {
            object_id: object_id.into(),
        }
    }

    /// Create a new Handle pin type
    pub fn handle(target_type: impl Into<String>) -> Self {
        PinType::Handle {
            target_type: target_type.into(),
        }
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

// ─────────────────────────────────────────────────────────────────────────────
// Function Definitions
// ─────────────────────────────────────────────────────────────────────────────

/// Special node ID for function entry point
pub const FUNCTION_ENTRY_NODE: &str = "__entry__";
/// Special node ID for function exit point
pub const FUNCTION_EXIT_NODE: &str = "__exit__";

/// A parameter for a function (input or output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    #[serde(rename = "type")]
    pub param_type: PinType,
    /// Default value for input parameters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A function definition within a blueprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    /// Function name (for display)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input parameters (become output pins on entry node)
    #[serde(default)]
    pub inputs: Vec<FunctionParam>,
    /// Output values (become input pins on exit node)
    #[serde(default)]
    pub outputs: Vec<FunctionParam>,
    /// Whether this is a pure function (no side effects, no exec pins)
    #[serde(default)]
    pub pure: bool,
    /// Nodes within this function's graph
    #[serde(default)]
    pub nodes: Vec<BlueprintNode>,
    /// Connections within this function's graph
    #[serde(default)]
    pub connections: Vec<Connection>,
}

impl FunctionDef {
    /// Get a node by ID within this function
    pub fn get_node(&self, id: &str) -> Option<&BlueprintNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get connections from a specific node and pin within this function
    pub fn connections_from(&self, node_id: &str, pin_name: &str) -> Vec<&Connection> {
        let prefix = format!("{}.{}", node_id, pin_name);
        self.connections.iter().filter(|c| c.from == prefix).collect()
    }

    /// Get connections to a specific node and pin within this function
    pub fn connections_to(&self, node_id: &str, pin_name: &str) -> Vec<&Connection> {
        let prefix = format!("{}.{}", node_id, pin_name);
        self.connections.iter().filter(|c| c.to == prefix).collect()
    }
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
    /// Service configuration (if this blueprint acts as a service)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<ServiceConfig>,
    /// Blueprint-level variables
    #[serde(default)]
    pub variables: HashMap<String, VariableDef>,
    /// Nodes in this blueprint
    #[serde(default)]
    pub nodes: Vec<BlueprintNode>,
    /// Connections between nodes
    #[serde(default)]
    pub connections: Vec<Connection>,
    /// Functions defined in this blueprint (name -> definition)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub functions: HashMap<String, FunctionDef>,
    /// Imported blueprints (for calling their exported functions)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<String>,
    /// Exported functions (can be called from other blueprints)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<String>,
    /// Behaviours this blueprint implements
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub implements: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

fn default_singleton() -> bool {
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for blueprints that act as services
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceConfig {
    /// Whether this blueprint should be registered as a service
    #[serde(default)]
    pub enabled: bool,
    /// Event patterns to subscribe to (e.g., "PointValueChanged", "ServiceStateChanged/*")
    #[serde(default)]
    pub subscriptions: Vec<String>,
    /// Whether only one instance can run (default: true)
    #[serde(default = "default_singleton")]
    pub singleton: bool,
    /// Human-readable description for the service
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Blueprint {
    /// Create a new empty blueprint
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            version: default_version(),
            description: None,
            service: None,
            variables: HashMap::new(),
            nodes: Vec::new(),
            connections: Vec::new(),
            functions: HashMap::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            implements: Vec::new(),
        }
    }

    /// Get a function by name
    pub fn get_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions.get(name)
    }

    /// Check if a function is exported
    pub fn is_function_exported(&self, name: &str) -> bool {
        self.exports.contains(&name.to_string())
    }

    /// Check if this blueprint should be registered as a service
    pub fn is_service(&self) -> bool {
        self.service.as_ref().map(|s| s.enabled).unwrap_or(false)
    }

    /// Get service subscriptions (empty if not a service)
    pub fn service_subscriptions(&self) -> Vec<String> {
        self.service
            .as_ref()
            .map(|s| s.subscriptions.clone())
            .unwrap_or_default()
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
#[derive(Debug, Serialize, Deserialize)]
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
    /// Triggered when the blueprint-as-service starts
    ServiceStart,
    /// Triggered when the blueprint-as-service stops
    ServiceStop,
    /// Triggered by a service request (HandleRequest)
    /// The request field is populated at runtime by neo with the concrete type
    ServiceRequest {
        request_id: String,
        /// Runtime-only field: contains Box<dyn Any> downcastable to neo's ServiceRequest
        #[serde(skip)]
        request: Option<DynRequest>,
    },
    /// Triggered by a system event routed to this service
    /// The event field is populated at runtime by neo with the concrete type
    ServiceEvent {
        /// Runtime-only field: contains Box<dyn Any> downcastable to neo's Event
        #[serde(skip)]
        event: Option<DynEvent>,
    },
}

// Manual Clone implementation because DynEvent/DynRequest don't implement Clone
// Runtime-only fields are set to None when cloning (they're #[serde(skip)] anyway)
impl Clone for ExecutionTrigger {
    fn clone(&self) -> Self {
        match self {
            Self::Event { event_type, data } => Self::Event {
                event_type: event_type.clone(),
                data: data.clone(),
            },
            Self::Schedule { schedule_id } => Self::Schedule {
                schedule_id: schedule_id.clone(),
            },
            Self::Request { inputs } => Self::Request {
                inputs: inputs.clone(),
            },
            Self::ServiceStart => Self::ServiceStart,
            Self::ServiceStop => Self::ServiceStop,
            Self::ServiceRequest { request_id, .. } => Self::ServiceRequest {
                request_id: request_id.clone(),
                request: None, // Runtime-only, not cloned
            },
            Self::ServiceEvent { .. } => Self::ServiceEvent {
                event: None, // Runtime-only, not cloned
            },
        }
    }
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

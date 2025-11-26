# Neo Blueprints - Visual Scripting Engine

A server-side visual scripting execution engine inspired by [Unreal Engine Blueprints](https://dev.epicgames.com/documentation/en-us/unreal-engine/nodes-in-unreal-engine).

## Overview

Neo Blueprints allows building automation logic using a graph-based approach where:
- **Nodes** represent operations (read point, compare values, send alarm, etc.)
- **Pins** are inputs/outputs on nodes (execution flow + data)
- **Connections** wire pins together to form logic flows
- **Graphs** are saved as JSON and executed by the Blueprint runtime

## Core Concepts (from Unreal)

### Node Types

| Type | Exec Pins | Description |
|------|-----------|-------------|
| **Event** | Output only | Entry points that trigger execution (e.g., `OnPointChanged`) |
| **Pure** | None | Stateless functions, re-evaluated each use (e.g., `Add`, `Compare`) |
| **Impure** | Input + Output | Side effects, executed once (e.g., `WritePoint`, `SendAlarm`) |
| **Latent** | Multiple outputs | Async operations (e.g., `Delay`, `WaitForCondition`) |
| **Flow Control** | Multiple outputs | Branch, ForEach, Sequence, etc. |

### Pin Types

```
Execution Pins (control flow):
  ▶ Input  - receives execution
  ▷ Output - passes execution to next node

Data Pins (values):
  ● Boolean (red)
  ● Number (green)
  ● String (magenta)
  ● Point Value (cyan)
  ● Any/Wildcard (gray)
  ● Array (same color, with [])
```

### Execution Model

1. **Event fires** → starts at event node
2. **Follow exec wires** → left to right through impure nodes
3. **Evaluate data** → pure nodes evaluated on-demand when data is needed
4. **Latent nodes** → can pause execution, resume later (e.g., after delay)

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Blueprint Runtime                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ Node Registry│  │Graph Loader │  │   Execution Engine      │  │
│  │ (built-in +  │  │(JSON→Graph) │  │   - Event dispatch      │  │
│  │  JS plugins) │  │             │  │   - Exec flow traversal │  │
│  └─────────────┘  └─────────────┘  │   - Data evaluation     │  │
│                                     │   - Latent management   │  │
│                                     └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Neo Core                                  │
│   Points │ Alarms │ History │ Events │ BACnet │ Plugins         │
└─────────────────────────────────────────────────────────────────┘
```

## Graph JSON Schema

```json
{
  "id": "hvac-optimization",
  "name": "HVAC Optimization Logic",
  "description": "Adjusts setpoints based on occupancy",
  "version": "1.0.0",
  "nodes": [
    {
      "id": "node_1",
      "type": "event:OnPointChanged",
      "position": { "x": 0, "y": 0 },
      "config": {
        "point_pattern": "*/occupancy/*"
      }
    },
    {
      "id": "node_2",
      "type": "flow:Branch",
      "position": { "x": 200, "y": 0 }
    },
    {
      "id": "node_3",
      "type": "point:Write",
      "position": { "x": 400, "y": -50 },
      "config": {
        "point": "building/hvac/setpoint"
      }
    }
  ],
  "connections": [
    { "from": "node_1.exec", "to": "node_2.exec" },
    { "from": "node_1.value", "to": "node_2.condition" },
    { "from": "node_2.true", "to": "node_3.exec" },
    { "from": "node_1.point_id", "to": "node_3.point" }
  ],
  "variables": [
    { "name": "occupied_setpoint", "type": "number", "default": 72 },
    { "name": "unoccupied_setpoint", "type": "number", "default": 65 }
  ]
}
```

## Built-in Node Categories

### Events (Entry Points)
| Node | Description |
|------|-------------|
| `event:OnPointChanged` | Fires when a point value changes |
| `event:OnAlarmRaised` | Fires when an alarm is triggered |
| `event:OnAlarmCleared` | Fires when an alarm clears |
| `event:OnSchedule` | Fires on cron schedule |
| `event:OnStartup` | Fires when blueprint loads |
| `event:OnDeviceDiscovered` | Fires when new BACnet device found |
| `event:OnTimer` | Fires at interval |
| `event:Custom` | Triggered programmatically |

### Point Operations
| Node | Type | Description |
|------|------|-------------|
| `point:Read` | Pure | Read current point value |
| `point:Write` | Impure | Write value to point |
| `point:Subscribe` | Impure | Start watching a point |
| `point:GetMetadata` | Pure | Get point name, units, etc. |

### Flow Control
| Node | Description |
|------|-------------|
| `flow:Branch` | If/else based on boolean |
| `flow:Switch` | Multi-way branch on value |
| `flow:Sequence` | Execute multiple paths in order |
| `flow:ForEach` | Loop over array |
| `flow:WhileLoop` | Loop while condition true |
| `flow:Gate` | Open/close execution gate |
| `flow:DoOnce` | Execute only first time |
| `flow:Delay` | Latent: wait N seconds |
| `flow:Debounce` | Latent: rate limit execution |

### Math & Logic
| Node | Description |
|------|-------------|
| `math:Add`, `Sub`, `Mul`, `Div` | Arithmetic |
| `math:Min`, `Max`, `Clamp` | Range operations |
| `math:Abs`, `Round`, `Floor`, `Ceil` | Rounding |
| `math:Scale` | Linear interpolation |
| `logic:And`, `Or`, `Not`, `Xor` | Boolean logic |
| `logic:Compare` | ==, !=, <, >, <=, >= |
| `logic:InRange` | Check if value in range |

### Alarms
| Node | Type | Description |
|------|------|-------------|
| `alarm:Raise` | Impure | Raise an alarm |
| `alarm:Clear` | Impure | Clear an alarm |
| `alarm:GetActive` | Pure | Get active alarms |
| `alarm:Acknowledge` | Impure | Acknowledge alarm |

### Utilities
| Node | Description |
|------|-------------|
| `util:Log` | Log message |
| `util:Format` | String formatting |
| `util:Now` | Current timestamp |
| `util:Random` | Random number |
| `util:ArrayGet` | Get array element |
| `util:ArrayLength` | Array size |
| `util:MakeArray` | Create array from inputs |

### Variables
| Node | Description |
|------|-------------|
| `var:Get` | Read graph variable |
| `var:Set` | Write graph variable |

## Custom Nodes via JS Plugins

Plugins can register custom nodes:

```javascript
// In a Neo JS plugin

defineService({
    async onStart() {
        // Register a pure node (no exec pins)
        Neo.blueprints.registerNode({
            id: "myPlugin:TemperatureConvert",
            name: "Convert Temperature",
            category: "My Plugin",
            pure: true,  // No exec pins

            inputs: [
                { name: "value", type: "number", label: "Temperature" },
                { name: "from", type: "string", label: "From Unit", default: "F" },
                { name: "to", type: "string", label: "To Unit", default: "C" },
            ],

            outputs: [
                { name: "result", type: "number", label: "Converted" },
            ],

            execute: (inputs) => {
                const { value, from, to } = inputs;
                let celsius = from === "F" ? (value - 32) * 5/9 : value;
                let result = to === "F" ? celsius * 9/5 + 32 : celsius;
                return { result };
            },
        });

        // Register an impure node (has exec pins)
        Neo.blueprints.registerNode({
            id: "myPlugin:SendNotification",
            name: "Send Notification",
            category: "My Plugin",
            pure: false,  // Has exec pins

            inputs: [
                { name: "exec", type: "exec" },
                { name: "message", type: "string" },
                { name: "severity", type: "string", default: "info" },
            ],

            outputs: [
                { name: "exec", type: "exec" },
                { name: "success", type: "boolean" },
            ],

            execute: async (inputs, context) => {
                // Do something with side effects
                const success = await sendNotification(inputs.message);
                return { success };
            },
        });

        // Register a latent node (async with multiple exit points)
        Neo.blueprints.registerNode({
            id: "myPlugin:WaitForResponse",
            name: "Wait For Response",
            category: "My Plugin",
            latent: true,

            inputs: [
                { name: "exec", type: "exec" },
                { name: "timeout", type: "number", default: 30 },
            ],

            outputs: [
                { name: "received", type: "exec", label: "On Received" },
                { name: "timeout", type: "exec", label: "On Timeout" },
                { name: "response", type: "any" },
            ],

            execute: async (inputs, context) => {
                try {
                    const response = await waitWithTimeout(inputs.timeout);
                    return { exitPin: "received", response };
                } catch {
                    return { exitPin: "timeout", response: null };
                }
            },
        });
    },
});
```

## Rust Implementation Plan

### Phase 1: Core Types & Registry

```rust
// src/blueprints/mod.rs
mod types;      // Node, Pin, Connection, Graph
mod registry;   // NodeRegistry - stores node definitions
mod loader;     // Load graphs from JSON
mod engine;     // Execution engine

// Key types
pub struct NodeDefinition {
    pub id: String,           // "point:Read", "myPlugin:Convert"
    pub name: String,
    pub category: String,
    pub node_type: NodeType,  // Pure, Impure, Latent, Event
    pub inputs: Vec<PinDefinition>,
    pub outputs: Vec<PinDefinition>,
}

pub enum NodeType {
    Event,
    Pure,
    Impure,
    Latent,
}

pub struct PinDefinition {
    pub name: String,
    pub pin_type: PinType,
    pub label: Option<String>,
    pub default: Option<Value>,
}

pub enum PinType {
    Exec,
    Boolean,
    Number,
    String,
    PointValue,
    Any,
    Array(Box<PinType>),
}
```

### Phase 2: Graph Representation

```rust
pub struct BlueprintGraph {
    pub id: String,
    pub name: String,
    pub nodes: HashMap<NodeId, NodeInstance>,
    pub connections: Vec<Connection>,
    pub variables: HashMap<String, Variable>,
}

pub struct NodeInstance {
    pub id: NodeId,
    pub node_type: String,  // References NodeDefinition
    pub config: serde_json::Value,
}

pub struct Connection {
    pub from_node: NodeId,
    pub from_pin: String,
    pub to_node: NodeId,
    pub to_pin: String,
}
```

### Phase 3: Execution Engine

```rust
pub struct BlueprintExecutor {
    registry: Arc<NodeRegistry>,
    graphs: HashMap<String, BlueprintGraph>,
    // Running instances with state
    instances: HashMap<InstanceId, ExecutionState>,
}

impl BlueprintExecutor {
    /// Execute from an event node
    pub async fn trigger_event(&self, graph_id: &str, event: &str, data: Value);

    /// Follow execution flow from a node
    async fn execute_from(&self, state: &mut ExecutionState, node_id: NodeId);

    /// Evaluate a pure node (may be called multiple times)
    fn evaluate_pure(&self, state: &ExecutionState, node_id: NodeId) -> Value;
}
```

### Phase 4: Integration with Neo

- BlueprintService actor manages graphs
- Subscribes to Neo events, triggers blueprint events
- JS plugins can register custom nodes via `Neo.blueprints.registerNode()`
- Graphs stored in `./blueprints/*.json`

## File Structure

```
src/
  blueprints/
    mod.rs
    types.rs        # Core types
    registry.rs     # Node registry
    loader.rs       # JSON loading
    engine.rs       # Execution engine
    nodes/
      mod.rs
      events.rs     # Event nodes
      points.rs     # Point operations
      flow.rs       # Flow control
      math.rs       # Math nodes
      logic.rs      # Logic nodes
      alarms.rs     # Alarm nodes
      utils.rs      # Utility nodes
```

## Example Blueprint: Temperature Alert

```json
{
  "id": "temp-alert",
  "name": "Temperature Alert",
  "nodes": [
    {
      "id": "1",
      "type": "event:OnPointChanged",
      "config": { "point_pattern": "*/temperature" }
    },
    {
      "id": "2",
      "type": "logic:Compare",
      "config": { "operator": ">" }
    },
    {
      "id": "3",
      "type": "flow:Branch"
    },
    {
      "id": "4",
      "type": "alarm:Raise",
      "config": {
        "alarm_id": "high-temp",
        "severity": "warning"
      }
    },
    {
      "id": "5",
      "type": "var:Get",
      "config": { "variable": "threshold" }
    }
  ],
  "connections": [
    { "from": "1.exec", "to": "3.exec" },
    { "from": "1.value", "to": "2.a" },
    { "from": "5.value", "to": "2.b" },
    { "from": "2.result", "to": "3.condition" },
    { "from": "3.true", "to": "4.exec" },
    { "from": "1.point_id", "to": "4.source" }
  ],
  "variables": [
    { "name": "threshold", "type": "number", "default": 80 }
  ]
}
```

## Sources

- [Unreal Engine Nodes Documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/nodes-in-unreal-engine)
- [Blueprint Evaluation Internals](https://zomgmoz.tv/unreal/Blueprints/How-blueprint-evaluation-works)
- [Creating Custom Blueprint Nodes](https://dev.epicgames.com/community/learning/tutorials/Klde/unreal-engine-custom-blueprint-nodes-exposing-c-to-blueprint-with-ufunction)
- [Custom K2 Nodes](https://unrealcommunity.wiki/create-custom-k2-node-for-blueprint-zwuncdkq)

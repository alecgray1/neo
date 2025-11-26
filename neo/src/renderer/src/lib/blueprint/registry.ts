// Node registry - mirrors the backend node definitions
import type { NodeDef, PinDef } from './types'

// Built-in node definitions matching the Rust backend
const builtInNodes: NodeDef[] = [
  // === Events ===
  {
    id: 'neo/OnEvent',
    name: 'On Event',
    category: 'Events',
    description: 'Triggered when an event occurs',
    pins: [
      { name: 'exec', direction: 'output', type: 'Exec', description: 'Execution output' },
      { name: 'event_type', direction: 'output', type: 'String', description: 'Type of event' },
      { name: 'value', direction: 'output', type: 'Any', description: 'Event value' }
    ]
  },

  // === Flow Control ===
  {
    id: 'neo/Branch',
    name: 'Branch',
    category: 'Flow Control',
    description: 'Conditional branch based on boolean condition',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'condition', direction: 'input', type: 'Boolean' },
      { name: 'true', direction: 'output', type: 'Exec', description: 'Executes if condition is true' },
      { name: 'false', direction: 'output', type: 'Exec', description: 'Executes if condition is false' }
    ]
  },
  {
    id: 'neo/Sequence',
    name: 'Sequence',
    category: 'Flow Control',
    description: 'Execute multiple outputs in sequence',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'then_0', direction: 'output', type: 'Exec' },
      { name: 'then_1', direction: 'output', type: 'Exec' },
      { name: 'then_2', direction: 'output', type: 'Exec' }
    ]
  },
  {
    id: 'neo/Delay',
    name: 'Delay',
    category: 'Flow Control',
    latent: true,
    description: 'Wait for a specified duration',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'duration', direction: 'input', type: 'Real', default: 1.0, description: 'Delay in seconds' },
      { name: 'completed', direction: 'output', type: 'Exec' }
    ]
  },

  // === Logic ===
  {
    id: 'neo/Compare',
    name: 'Compare',
    category: 'Logic',
    description: 'Compare two values',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'a', direction: 'input', type: 'Any' },
      { name: 'b', direction: 'input', type: 'Any' },
      { name: 'exec', direction: 'output', type: 'Exec' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },
  {
    id: 'neo/And',
    name: 'AND',
    category: 'Logic',
    pure: true,
    description: 'Logical AND of two booleans',
    pins: [
      { name: 'a', direction: 'input', type: 'Boolean' },
      { name: 'b', direction: 'input', type: 'Boolean' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },
  {
    id: 'neo/Or',
    name: 'OR',
    category: 'Logic',
    pure: true,
    description: 'Logical OR of two booleans',
    pins: [
      { name: 'a', direction: 'input', type: 'Boolean' },
      { name: 'b', direction: 'input', type: 'Boolean' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },
  {
    id: 'neo/Not',
    name: 'NOT',
    category: 'Logic',
    pure: true,
    description: 'Logical NOT of a boolean',
    pins: [
      { name: 'value', direction: 'input', type: 'Boolean' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },

  // === Math ===
  {
    id: 'neo/Add',
    name: 'Add',
    category: 'Math',
    pure: true,
    description: 'Add two numbers',
    pins: [
      { name: 'a', direction: 'input', type: 'Real' },
      { name: 'b', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Real' }
    ]
  },
  {
    id: 'neo/Subtract',
    name: 'Subtract',
    category: 'Math',
    pure: true,
    description: 'Subtract two numbers',
    pins: [
      { name: 'a', direction: 'input', type: 'Real' },
      { name: 'b', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Real' }
    ]
  },
  {
    id: 'neo/Multiply',
    name: 'Multiply',
    category: 'Math',
    pure: true,
    description: 'Multiply two numbers',
    pins: [
      { name: 'a', direction: 'input', type: 'Real' },
      { name: 'b', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Real' }
    ]
  },
  {
    id: 'neo/Divide',
    name: 'Divide',
    category: 'Math',
    pure: true,
    description: 'Divide two numbers',
    pins: [
      { name: 'a', direction: 'input', type: 'Real' },
      { name: 'b', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Real' }
    ]
  },
  {
    id: 'neo/Clamp',
    name: 'Clamp',
    category: 'Math',
    pure: true,
    description: 'Clamp a value between min and max',
    pins: [
      { name: 'value', direction: 'input', type: 'Real' },
      { name: 'min', direction: 'input', type: 'Real' },
      { name: 'max', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Real' }
    ]
  },

  // === Variables ===
  {
    id: 'neo/GetVariable',
    name: 'Get Variable',
    category: 'Variables',
    pure: true,
    description: 'Get a blueprint variable value',
    pins: [{ name: 'value', direction: 'output', type: 'Any' }]
  },
  {
    id: 'neo/SetVariable',
    name: 'Set Variable',
    category: 'Variables',
    description: 'Set a blueprint variable value',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'value', direction: 'input', type: 'Any' },
      { name: 'exec', direction: 'output', type: 'Exec' },
      { name: 'value', direction: 'output', type: 'Any' }
    ]
  },

  // === Utilities ===
  {
    id: 'neo/Log',
    name: 'Log',
    category: 'Utilities',
    description: 'Log a message',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'message', direction: 'input', type: 'String' },
      { name: 'exec', direction: 'output', type: 'Exec' }
    ]
  },

  // === Latent/Async ===
  {
    id: 'neo/WaitForEvent',
    name: 'Wait For Event',
    category: 'Async',
    latent: true,
    description: 'Wait until a specific event occurs',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'event_type', direction: 'input', type: 'String' },
      { name: 'completed', direction: 'output', type: 'Exec' },
      { name: 'event_data', direction: 'output', type: 'Any' }
    ]
  },
  {
    id: 'neo/WaitForPointChange',
    name: 'Wait For Point Change',
    category: 'Async',
    latent: true,
    description: 'Wait until a point value changes',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'point_path', direction: 'input', type: 'String' },
      { name: 'completed', direction: 'output', type: 'Exec' },
      { name: 'new_value', direction: 'output', type: 'PointValue' }
    ]
  }
]

// Node registry singleton
class NodeRegistry {
  private nodes: Map<string, NodeDef> = new Map()
  private categories: Map<string, NodeDef[]> = new Map()

  constructor() {
    // Register built-in nodes
    for (const node of builtInNodes) {
      this.register(node)
    }
  }

  register(node: NodeDef) {
    this.nodes.set(node.id, node)

    // Add to category
    const category = this.categories.get(node.category) || []
    category.push(node)
    this.categories.set(node.category, category)
  }

  get(id: string): NodeDef | undefined {
    return this.nodes.get(id)
  }

  getAll(): NodeDef[] {
    return Array.from(this.nodes.values())
  }

  getCategories(): string[] {
    return Array.from(this.categories.keys())
  }

  getByCategory(category: string): NodeDef[] {
    return this.categories.get(category) || []
  }

  getCategorized(): Map<string, NodeDef[]> {
    return this.categories
  }

  // Get input pins for a node type
  getInputPins(nodeType: string): PinDef[] {
    const node = this.get(nodeType)
    if (!node) return []
    return node.pins.filter((p) => p.direction === 'input')
  }

  // Get output pins for a node type
  getOutputPins(nodeType: string): PinDef[] {
    const node = this.get(nodeType)
    if (!node) return []
    return node.pins.filter((p) => p.direction === 'output')
  }
}

// Export singleton instance
export const nodeRegistry = new NodeRegistry()

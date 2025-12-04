// Node registry - mirrors the backend node definitions
import type { NodeDef, PinDef } from './types'

// Built-in node definitions matching the Rust backend
const builtInNodes: NodeDef[] = [
  // === Events ===
  {
    id: 'event/OnStart',
    name: 'On Start',
    category: 'Events',
    description: 'Triggered when the blueprint starts',
    pins: [
      { name: 'exec', direction: 'output', type: 'Exec', description: 'Execution output' }
    ]
  },
  {
    id: 'event/OnEvent',
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
    id: 'flow/Branch',
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
    id: 'flow/Sequence',
    name: 'Sequence',
    category: 'Flow Control',
    description: 'Execute multiple outputs in sequence',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'then_0', direction: 'output', type: 'Exec' },
      { name: 'then_1', direction: 'output', type: 'Exec' }
    ]
  },

  // === Logic ===
  {
    id: 'logic/And',
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
    id: 'logic/Or',
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
    id: 'logic/Not',
    name: 'NOT',
    category: 'Logic',
    pure: true,
    description: 'Logical NOT of a boolean',
    pins: [
      { name: 'value', direction: 'input', type: 'Boolean' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },
  {
    id: 'logic/Xor',
    name: 'XOR',
    category: 'Logic',
    pure: true,
    description: 'Logical XOR of two booleans',
    pins: [
      { name: 'a', direction: 'input', type: 'Boolean' },
      { name: 'b', direction: 'input', type: 'Boolean' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },

  // === Comparison ===
  {
    id: 'compare/Equal',
    name: 'Equal',
    category: 'Comparison',
    pure: true,
    description: 'Check if two values are equal',
    pins: [
      { name: 'a', direction: 'input', type: 'Real' },
      { name: 'b', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },
  {
    id: 'compare/Greater',
    name: 'Greater Than',
    category: 'Comparison',
    pure: true,
    description: 'Check if A is greater than B',
    pins: [
      { name: 'a', direction: 'input', type: 'Real' },
      { name: 'b', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },
  {
    id: 'compare/Less',
    name: 'Less Than',
    category: 'Comparison',
    pure: true,
    description: 'Check if A is less than B',
    pins: [
      { name: 'a', direction: 'input', type: 'Real' },
      { name: 'b', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },
  {
    id: 'compare/InRange',
    name: 'In Range',
    category: 'Comparison',
    pure: true,
    description: 'Check if value is within a range',
    pins: [
      { name: 'value', direction: 'input', type: 'Real' },
      { name: 'min', direction: 'input', type: 'Real' },
      { name: 'max', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Boolean' }
    ]
  },

  // === Math ===
  {
    id: 'math/Add',
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
    id: 'math/Subtract',
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
    id: 'math/Multiply',
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
    id: 'math/Divide',
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
    id: 'math/Clamp',
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
  {
    id: 'math/Abs',
    name: 'Absolute Value',
    category: 'Math',
    pure: true,
    description: 'Get absolute value of a number',
    pins: [
      { name: 'value', direction: 'input', type: 'Real' },
      { name: 'result', direction: 'output', type: 'Real' }
    ]
  },

  // === Utility ===
  {
    id: 'utility/Print',
    name: 'Print',
    category: 'Utility',
    description: 'Print a message to the log',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'message', direction: 'input', type: 'String' },
      { name: 'then', direction: 'output', type: 'Exec' }
    ]
  },
  {
    id: 'utility/SetVariable',
    name: 'Set Variable',
    category: 'Utility',
    description: 'Set a blueprint variable value',
    pins: [
      { name: 'exec', direction: 'input', type: 'Exec' },
      { name: 'value', direction: 'input', type: 'Any' },
      { name: 'then', direction: 'output', type: 'Exec' }
    ]
  },
  {
    id: 'utility/GetVariable',
    name: 'Get Variable',
    category: 'Utility',
    pure: true,
    description: 'Get a blueprint variable value',
    pins: [{ name: 'value', direction: 'output', type: 'Any' }]
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

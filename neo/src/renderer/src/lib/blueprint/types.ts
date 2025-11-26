// Blueprint types matching the Rust backend at /home/alec/Work/neo/src/blueprints/types.rs

// Pin types for blueprint connections
export type PinType =
  | 'Exec'
  | 'Real'
  | 'Integer'
  | 'Boolean'
  | 'String'
  | 'PointValue'
  | 'Any'
  | { Array: PinType }

// Pin direction
export type PinDirection = 'input' | 'output'

// Pin definition for node definitions
export interface PinDef {
  name: string
  direction: PinDirection
  type: PinType
  default?: unknown
  description?: string
}

// Node definition in the registry
export interface NodeDef {
  id: string // e.g., "neo/Branch"
  name: string // Display name
  category: string // For palette grouping
  pure?: boolean // No exec pins, stateless
  latent?: boolean // Async/suspendable node
  pins: PinDef[]
  description?: string
}

// Variable definition in a blueprint
export interface VariableDef {
  type: PinType
  default?: unknown
  description?: string
}

// Node instance in a blueprint
export interface BlueprintNode {
  id: string
  type: string // Node type ID (e.g., "neo/Branch")
  position: { x: number; y: number }
  config?: Record<string, unknown>
}

// Connection between pins
export interface Connection {
  from: string // Format: "node_id.pin_name"
  to: string // Format: "node_id.pin_name"
}

// Complete blueprint structure
export interface Blueprint {
  id: string
  name: string
  version: string
  description?: string
  variables: Record<string, VariableDef>
  nodes: BlueprintNode[]
  connections: Connection[]
}

// Pin type colors using CSS variables for theming
export const PIN_COLORS: Record<string, string> = {
  Exec: 'var(--neo-blueprint-pin-exec)',
  Boolean: 'var(--neo-blueprint-pin-boolean)',
  Integer: 'var(--neo-blueprint-pin-integer)',
  Real: 'var(--neo-blueprint-pin-real)',
  String: 'var(--neo-blueprint-pin-string)',
  PointValue: 'var(--neo-blueprint-pin-pointValue)',
  Any: 'var(--neo-blueprint-pin-any)'
}

// Get color CSS variable for a pin type
export function getPinColor(type: PinType): string {
  if (typeof type === 'string') {
    return PIN_COLORS[type] || PIN_COLORS.Any
  }
  // Array type - use element type color
  if ('Array' in type) {
    return getPinColor(type.Array)
  }
  return PIN_COLORS.Any
}

// Check if two pin types are compatible for connection
export function areTypesCompatible(from: PinType, to: PinType): boolean {
  // Exec only connects to Exec
  if (from === 'Exec' || to === 'Exec') {
    return from === 'Exec' && to === 'Exec'
  }

  // Any accepts everything (except Exec)
  if (from === 'Any' || to === 'Any') {
    return true
  }

  // Same type always compatible
  if (from === to) {
    return true
  }

  // Integer <-> Real conversion allowed
  if ((from === 'Integer' && to === 'Real') || (from === 'Real' && to === 'Integer')) {
    return true
  }

  // Array types - check element compatibility
  if (typeof from === 'object' && 'Array' in from && typeof to === 'object' && 'Array' in to) {
    return areTypesCompatible(from.Array, to.Array)
  }

  return false
}

// Parse a connection endpoint (e.g., "node_id.pin_name")
export function parseConnectionEndpoint(endpoint: string): { nodeId: string; pinName: string } | null {
  const dotIndex = endpoint.indexOf('.')
  if (dotIndex === -1) return null
  return {
    nodeId: endpoint.substring(0, dotIndex),
    pinName: endpoint.substring(dotIndex + 1)
  }
}

// Create a connection endpoint string
export function createConnectionEndpoint(nodeId: string, pinName: string): string {
  return `${nodeId}.${pinName}`
}

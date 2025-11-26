// Conversion functions between Blueprint JSON and Svelte Flow format
import type { Node, Edge } from '@xyflow/svelte'
import type { Blueprint, BlueprintNode, Connection } from './types'
import { nodeRegistry } from './registry'
import { parseConnectionEndpoint, createConnectionEndpoint, getPinColor } from './types'

// Custom data stored in Svelte Flow nodes
export interface BlueprintNodeData {
  nodeType: string // The blueprint node type (e.g., "neo/Branch")
  label: string
  config?: Record<string, unknown>
  [key: string]: unknown
}

// Convert Blueprint JSON to Svelte Flow format
export function blueprintToFlow(blueprint: Blueprint): { nodes: Node<BlueprintNodeData>[]; edges: Edge[] } {
  const nodes: Node<BlueprintNodeData>[] = blueprint.nodes.map((bpNode) => {
    const nodeDef = nodeRegistry.get(bpNode.type)

    return {
      id: bpNode.id,
      type: 'blueprintNode', // Custom node type
      position: bpNode.position,
      data: {
        nodeType: bpNode.type,
        label: nodeDef?.name || bpNode.type.split('/').pop() || 'Unknown',
        config: bpNode.config
      }
    }
  })

  const edges: Edge[] = blueprint.connections.map((conn, index) => {
    const from = parseConnectionEndpoint(conn.from)
    const to = parseConnectionEndpoint(conn.to)

    if (!from || !to) {
      console.warn('Invalid connection:', conn)
      return null
    }

    // Determine edge color based on pin type
    const sourceNode = blueprint.nodes.find((n) => n.id === from.nodeId)
    const sourceNodeDef = sourceNode ? nodeRegistry.get(sourceNode.type) : null
    const sourcePin = sourceNodeDef?.pins.find((p) => p.name === from.pinName && p.direction === 'output')
    const edgeColor = sourcePin ? getPinColor(sourcePin.type) : '#888888'

    return {
      id: `edge-${index}`,
      source: from.nodeId,
      target: to.nodeId,
      sourceHandle: from.pinName,
      targetHandle: to.pinName,
      type: 'blueprintEdge',
      style: `stroke: ${edgeColor}; stroke-width: 2px;`,
      data: {
        pinType: sourcePin?.type || 'Any'
      }
    }
  }).filter((e): e is Edge => e !== null)

  return { nodes, edges }
}

// Convert Svelte Flow format back to Blueprint JSON
export function flowToBlueprint(
  nodes: Node<BlueprintNodeData>[],
  edges: Edge[],
  metadata: { id: string; name: string; version: string; description?: string; variables?: Blueprint['variables'] }
): Blueprint {
  const bpNodes: BlueprintNode[] = nodes.map((node) => ({
    id: node.id,
    type: node.data.nodeType,
    position: { x: node.position.x, y: node.position.y },
    config: node.data.config
  }))

  const connections: Connection[] = edges.map((edge) => ({
    from: createConnectionEndpoint(edge.source, edge.sourceHandle || ''),
    to: createConnectionEndpoint(edge.target, edge.targetHandle || '')
  }))

  return {
    id: metadata.id,
    name: metadata.name,
    version: metadata.version,
    description: metadata.description,
    variables: metadata.variables || {},
    nodes: bpNodes,
    connections
  }
}

// Generate a unique node ID
let nodeIdCounter = 0
export function generateNodeId(): string {
  return `node_${Date.now()}_${nodeIdCounter++}`
}

// Create a new node at a given position
export function createNode(nodeType: string, position: { x: number; y: number }): Node<BlueprintNodeData> {
  const nodeDef = nodeRegistry.get(nodeType)

  return {
    id: generateNodeId(),
    type: 'blueprintNode',
    position,
    data: {
      nodeType,
      label: nodeDef?.name || nodeType.split('/').pop() || 'Unknown',
      config: {}
    }
  }
}

// Check if a connection is valid
export function isValidConnection(
  sourceNodeType: string,
  sourcePin: string,
  targetNodeType: string,
  targetPin: string
): boolean {
  const sourceDef = nodeRegistry.get(sourceNodeType)
  const targetDef = nodeRegistry.get(targetNodeType)

  if (!sourceDef || !targetDef) return false

  const sourcePinDef = sourceDef.pins.find((p) => p.name === sourcePin && p.direction === 'output')
  const targetPinDef = targetDef.pins.find((p) => p.name === targetPin && p.direction === 'input')

  if (!sourcePinDef || !targetPinDef) return false

  // Use type compatibility check from types
  const { areTypesCompatible } = require('./types')
  return areTypesCompatible(sourcePinDef.type, targetPinDef.type)
}

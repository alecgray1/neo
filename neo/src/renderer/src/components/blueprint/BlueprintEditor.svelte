<script lang="ts">
  import { SvelteFlow, Background, Controls, MiniMap, type Node, type Edge, type Connection } from '@xyflow/svelte'
  import '@xyflow/svelte/dist/style.css'

  import BlueprintNode from './BlueprintNode.svelte'
  import NodeContextMenu from './NodeContextMenu.svelte'
  import { blueprintToFlow, flowToBlueprint, createNode, type BlueprintNodeData } from '$lib/blueprint/conversion'
  import { areTypesCompatible, parseConnectionEndpoint, getPinColor } from '$lib/blueprint/types'
  import { nodeRegistry } from '$lib/blueprint/registry'
  import type { Blueprint } from '$lib/blueprint/types'

  interface Props {
    content: string
    externalUpdateCounter?: number
    onchange?: (newContent: string) => void
  }

  let { content, externalUpdateCounter = 0, onchange }: Props = $props()

  // Parse blueprint from content
  let blueprint: Blueprint | null = $derived.by(() => {
    try {
      return JSON.parse(content) as Blueprint
    } catch {
      return null
    }
  })

  // Convert to Svelte Flow format
  let initialData = $derived.by(() => {
    if (!blueprint) return { nodes: [], edges: [] }
    return blueprintToFlow(blueprint)
  })

  // Reactive state for nodes and edges
  let nodes = $state<Node<BlueprintNodeData>[]>([])
  let edges = $state<Edge[]>([])

  // Track which blueprint we've loaded and the last external update we processed
  let loadedBlueprintId = $state<string | null>(null)
  let lastProcessedExternalUpdate = $state(0)

  // Reload when a DIFFERENT blueprint is loaded OR when external update occurs
  $effect(() => {
    const currentId = blueprint?.id ?? null
    const needsReload = currentId !== loadedBlueprintId || externalUpdateCounter > lastProcessedExternalUpdate

    if (needsReload) {
      console.log('Reloading blueprint:', currentId, 'externalUpdateCounter:', externalUpdateCounter, 'lastProcessed:', lastProcessedExternalUpdate)
      nodes = initialData.nodes
      edges = initialData.edges
      loadedBlueprintId = currentId
      lastProcessedExternalUpdate = externalUpdateCounter
    }
  })

  // Custom node types
  const nodeTypes = {
    blueprintNode: BlueprintNode
  }

  // Context menu state
  let contextMenu = $state<{ x: number; y: number; flowPosition: { x: number; y: number } } | null>(null)

  // Handle connection validation
  function isValidConnection(connection: Connection): boolean {
    const sourceNode = nodes.find((n) => n.id === connection.source)
    const targetNode = nodes.find((n) => n.id === connection.target)

    if (!sourceNode || !targetNode) return false

    const sourceDef = nodeRegistry.get(sourceNode.data.nodeType)
    const targetDef = nodeRegistry.get(targetNode.data.nodeType)

    if (!sourceDef || !targetDef) return false

    const sourcePin = sourceDef.pins.find(
      (p) => p.name === connection.sourceHandle && p.direction === 'output'
    )
    const targetPin = targetDef.pins.find(
      (p) => p.name === connection.targetHandle && p.direction === 'input'
    )

    if (!sourcePin || !targetPin) return false

    return areTypesCompatible(sourcePin.type, targetPin.type)
  }

  // Handle new connections
  function handleConnect(connection: Connection) {
    if (!isValidConnection(connection)) {
      console.warn('Invalid connection:', connection)
      return
    }

    // Get edge color from source pin type
    const sourceNode = nodes.find((n) => n.id === connection.source)
    const sourceDef = sourceNode ? nodeRegistry.get(sourceNode.data.nodeType) : null
    const sourcePin = sourceDef?.pins.find(
      (p) => p.name === connection.sourceHandle && p.direction === 'output'
    )
    const edgeColor = sourcePin ? getPinColor(sourcePin.type) : '#888888'

    const newEdge: Edge = {
      id: `edge-${Date.now()}`,
      source: connection.source,
      target: connection.target,
      sourceHandle: connection.sourceHandle,
      targetHandle: connection.targetHandle,
      type: 'default',
      style: `stroke: ${edgeColor}; stroke-width: 2px;`
    }

    edges = [...edges, newEdge]
    notifyChange()
  }

  // Handle node drag end - sync to server when drag completes
  function handleNodeDragStop() {
    console.log('Drag stop - nodes positions:', JSON.stringify(nodes.map(n => ({ id: n.id, x: n.position.x, y: n.position.y }))))
    notifyChange()
  }


  // Notify parent of changes
  function notifyChange() {
    if (!onchange || !blueprint) {
      console.log('notifyChange: skipping - onchange:', !!onchange, 'blueprint:', !!blueprint)
      return
    }

    console.log('notifyChange: current nodes:', nodes.map(n => ({ id: n.id, x: n.position.x, y: n.position.y })))

    const updatedBlueprint = flowToBlueprint(nodes, edges, {
      id: blueprint.id,
      name: blueprint.name,
      version: blueprint.version,
      description: blueprint.description,
      variables: blueprint.variables
    })

    console.log('notifyChange: updatedBlueprint nodes:', updatedBlueprint.nodes.map(n => ({ id: n.id, x: n.position.x, y: n.position.y })))

    onchange(JSON.stringify(updatedBlueprint, null, 2))
  }

  // Handle right-click for context menu
  function handleContextMenu(event: MouseEvent) {
    event.preventDefault()

    // Get flow position from screen coordinates
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect()
    const x = event.clientX
    const y = event.clientY

    // Approximate flow position (would need viewport transform for accuracy)
    const flowX = event.clientX - bounds.left
    const flowY = event.clientY - bounds.top

    contextMenu = { x, y, flowPosition: { x: flowX, y: flowY } }
  }

  // Add a node from context menu
  function handleAddNode(nodeType: string) {
    if (!contextMenu) return

    const newNode = createNode(nodeType, contextMenu.flowPosition)
    nodes = [...nodes, newNode]
    contextMenu = null
    notifyChange()
  }

  // Close context menu
  function handleCloseContextMenu() {
    contextMenu = null
  }

  // Handle click on canvas to close context menu
  function handlePaneClick() {
    contextMenu = null
  }
</script>

<div class="blueprint-editor" oncontextmenu={handleContextMenu}>
  {#if blueprint}
    <SvelteFlow
      bind:nodes
      bind:edges
      {nodeTypes}
      onconnect={handleConnect}
      onnodedragstop={handleNodeDragStop}
      isValidConnection={isValidConnection}
      onpaneclick={handlePaneClick}
      fitView
      snapToGrid
      snapGrid={[20, 20]}
      defaultEdgeOptions={{ type: 'default' }}
    >
      <Background gap={20} />
      <Controls />
      <MiniMap />
    </SvelteFlow>

    {#if contextMenu}
      <NodeContextMenu
        x={contextMenu.x}
        y={contextMenu.y}
        onselect={handleAddNode}
        onclose={handleCloseContextMenu}
      />
    {/if}
  {:else}
    <div class="error-state">
      <p>Failed to parse blueprint JSON</p>
      <pre>{content}</pre>
    </div>
  {/if}
</div>

<style>
  .blueprint-editor {
    width: 100%;
    height: 100%;
    background: var(--neo-blueprint-background);
  }

  .blueprint-editor :global(.svelte-flow) {
    background: var(--neo-blueprint-background);
  }

  .blueprint-editor :global(.svelte-flow__background) {
    background: var(--neo-blueprint-background);
  }

  .blueprint-editor :global(.svelte-flow__background pattern circle) {
    fill: var(--neo-blueprint-grid);
  }

  .blueprint-editor :global(.svelte-flow__controls) {
    background: var(--neo-blueprint-node-background);
    border: 1px solid var(--neo-blueprint-node-border);
  }

  .blueprint-editor :global(.svelte-flow__controls-button) {
    background: var(--neo-blueprint-node-background);
    border-color: var(--neo-blueprint-node-border);
    color: var(--neo-foreground);
  }

  .blueprint-editor :global(.svelte-flow__controls-button:hover) {
    background: var(--neo-list-hoverBackground);
  }

  .blueprint-editor :global(.svelte-flow__minimap) {
    background: var(--neo-blueprint-node-background);
    border: 1px solid var(--neo-blueprint-node-border);
  }

  .blueprint-editor :global(.svelte-flow__edge-path) {
    stroke-width: 2px;
  }

  .error-state {
    padding: 20px;
    color: var(--neo-error);
  }

  .error-state pre {
    background: var(--neo-blueprint-node-background);
    padding: 10px;
    border-radius: 4px;
    overflow: auto;
    max-height: 200px;
    font-size: 12px;
    color: var(--neo-foreground);
  }
</style>

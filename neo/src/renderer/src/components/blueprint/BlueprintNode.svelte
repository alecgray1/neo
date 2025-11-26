<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte'
  import { nodeRegistry } from '$lib/blueprint/registry'

  interface Props {
    id: string
    data: {
      nodeType: string
      label: string
      config?: Record<string, unknown>
    }
  }

  let { id, data }: Props = $props()

  // Get node definition
  let nodeDef = $derived(nodeRegistry.get(data.nodeType))
  let inputPins = $derived(nodeDef?.pins.filter((p) => p.direction === 'input') || [])
  let outputPins = $derived(nodeDef?.pins.filter((p) => p.direction === 'output') || [])

  // Get category CSS variable name
  function getCategoryVar(category: string | undefined): string {
    switch (category) {
      case 'Events':
        return 'var(--neo-blueprint-category-events)'
      case 'Flow Control':
        return 'var(--neo-blueprint-category-flowControl)'
      case 'Logic':
        return 'var(--neo-blueprint-category-logic)'
      case 'Math':
        return 'var(--neo-blueprint-category-math)'
      case 'Variables':
        return 'var(--neo-blueprint-category-variables)'
      case 'Utilities':
        return 'var(--neo-blueprint-category-utilities)'
      case 'Async':
        return 'var(--neo-blueprint-category-async)'
      default:
        return 'var(--neo-blueprint-category-default)'
    }
  }

  // Get pin color CSS variable
  function getPinColorVar(type: string): string {
    switch (type) {
      case 'Exec':
        return 'var(--neo-blueprint-pin-exec)'
      case 'Boolean':
        return 'var(--neo-blueprint-pin-boolean)'
      case 'Integer':
        return 'var(--neo-blueprint-pin-integer)'
      case 'Real':
        return 'var(--neo-blueprint-pin-real)'
      case 'String':
        return 'var(--neo-blueprint-pin-string)'
      case 'PointValue':
        return 'var(--neo-blueprint-pin-pointValue)'
      default:
        return 'var(--neo-blueprint-pin-any)'
    }
  }
</script>

<div class="blueprint-node" class:pure={nodeDef?.pure} class:latent={nodeDef?.latent}>
  <!-- Header -->
  <div class="node-header" style="background: {getCategoryVar(nodeDef?.category)}">
    <span class="node-title">{data.label}</span>
    {#if nodeDef?.latent}
      <span class="latent-indicator" title="Async/Latent Node">⏳</span>
    {/if}
  </div>

  <!-- Body with all pins -->
  <div class="node-body">
    <!-- Input pins (left side) -->
    <div class="pins-left">
      {#each inputPins as pin}
        <div class="pin" class:exec-pin={pin.type === 'Exec'} class:data-pin={pin.type !== 'Exec'}>
          {#if pin.type === 'Exec'}
            <div class="exec-slot input">
              <Handle
                type="target"
                position={Position.Left}
                id={pin.name}
                class="exec-handle"
              />
              <svg class="exec-icon" viewBox="0 0 16 16" width="12" height="12">
                <path d="M2 2 L14 8 L2 14 Z" fill="var(--neo-blueprint-execPin-fill)" stroke="var(--neo-blueprint-execPin-stroke)" stroke-width="1"/>
              </svg>
            </div>
          {:else}
            <Handle
              type="target"
              position={Position.Left}
              id={pin.name}
              style="background: {getPinColorVar(pin.type)};"
            />
          {/if}
          <span class="pin-label" style="color: {pin.type === 'Exec' ? 'var(--neo-blueprint-execPin-stroke)' : getPinColorVar(pin.type)}">{pin.name}</span>
        </div>
      {/each}
    </div>

    <!-- Output pins (right side) -->
    <div class="pins-right">
      {#each outputPins as pin}
        <div class="pin" class:exec-pin={pin.type === 'Exec'} class:data-pin={pin.type !== 'Exec'}>
          <span class="pin-label" style="color: {pin.type === 'Exec' ? 'var(--neo-blueprint-execPin-stroke)' : getPinColorVar(pin.type)}">{pin.name}</span>
          {#if pin.type === 'Exec'}
            <div class="exec-slot output">
              <svg class="exec-icon" viewBox="0 0 16 16" width="12" height="12">
                <path d="M2 2 L14 8 L2 14 Z" fill="var(--neo-blueprint-execPin-fill)" stroke="var(--neo-blueprint-execPin-stroke)" stroke-width="1"/>
              </svg>
              <Handle
                type="source"
                position={Position.Right}
                id={pin.name}
                class="exec-handle"
              />
            </div>
          {:else}
            <Handle
              type="source"
              position={Position.Right}
              id={pin.name}
              style="background: {getPinColorVar(pin.type)};"
            />
          {/if}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .blueprint-node {
    background: var(--neo-blueprint-node-background);
    border: 2px solid var(--neo-blueprint-node-border);
    border-radius: 4px;
    min-width: 150px;
    font-family: system-ui, -apple-system, sans-serif;
    font-size: 12px;
    box-shadow: 0 4px 6px var(--neo-blueprint-node-shadow);
  }

  .blueprint-node.pure {
    border-color: var(--neo-blueprint-node-borderPure);
  }

  .blueprint-node.latent {
    border-color: var(--neo-blueprint-node-borderLatent);
    border-style: dashed;
  }

  .node-header {
    padding: 6px 10px;
    border-radius: 2px 2px 0 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .node-title {
    color: var(--neo-blueprint-node-title);
    font-weight: 600;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .latent-indicator {
    font-size: 10px;
  }

  .node-body {
    padding: 8px 0;
    display: flex;
    justify-content: space-between;
    gap: 20px;
    min-height: 30px;
  }

  .pins-left,
  .pins-right {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .pins-left {
    align-items: flex-start;
    padding-left: 8px;
  }

  .pins-right {
    align-items: flex-end;
    padding-right: 8px;
  }

  .pin {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    position: relative;
    height: 18px;
  }

  .pin-label {
    font-size: 11px;
  }

  /* Exec pin triangle slot */
  .exec-slot {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
  }

  .exec-icon {
    pointer-events: none;
  }

  /* Hide the actual handle visually but keep it functional */
  .exec-slot :global(.exec-handle) {
    width: 6px !important;
    height: 6px !important;
    background: transparent !important;
    border: none !important;
    opacity: 0;
    position: absolute;
    top: 7px;
  }

  /* Input exec: line connects to the left base of triangle */
  .exec-slot.input :global(.exec-handle) {
    left: -3px;
  }

  /* Output exec: line connects from the right tip of triangle */
  .exec-slot.output :global(.exec-handle) {
    right: -3px;
    left: auto;
  }

  /* Data pin handle styling - circles */
  .data-pin :global(.svelte-flow__handle) {
    width: 10px;
    height: 10px;
    border: 2px solid var(--neo-blueprint-background);
    border-radius: 50%;
  }

  .data-pin :global(.svelte-flow__handle-left) {
    left: -6px;
  }

  .data-pin :global(.svelte-flow__handle-right) {
    right: -6px;
  }
</style>

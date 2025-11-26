<script lang="ts">
  import { nodeRegistry } from '$lib/blueprint/registry'
  import { ChevronRight } from '@lucide/svelte'

  interface Props {
    x: number
    y: number
    onselect: (nodeType: string) => void
    onclose: () => void
  }

  let { x, y, onselect, onclose }: Props = $props()

  // Get categorized nodes
  let categories = $derived(nodeRegistry.getCategories())
  let expandedCategory = $state<string | null>(null)

  // Search state
  let searchQuery = $state('')
  let searchInput: HTMLInputElement | undefined = $state()

  // Filter nodes by search
  let filteredNodes = $derived.by(() => {
    if (!searchQuery) return null
    const query = searchQuery.toLowerCase()
    return nodeRegistry.getAll().filter(
      (node) =>
        node.name.toLowerCase().includes(query) ||
        node.id.toLowerCase().includes(query) ||
        node.category.toLowerCase().includes(query)
    )
  })

  // Focus search on mount
  $effect(() => {
    searchInput?.focus()
  })

  function handleSelect(nodeType: string) {
    onselect(nodeType)
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose()
    }
  }

  function toggleCategory(category: string) {
    expandedCategory = expandedCategory === category ? null : category
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="context-menu-backdrop" onclick={onclose}></div>

<div class="context-menu" style="left: {x}px; top: {y}px;">
  <!-- Search input -->
  <div class="search-container">
    <input
      bind:this={searchInput}
      bind:value={searchQuery}
      type="text"
      placeholder="Search nodes..."
      class="search-input"
    />
  </div>

  <div class="menu-content">
    {#if filteredNodes && filteredNodes.length > 0}
      <!-- Search results -->
      <div class="search-results">
        {#each filteredNodes as node}
          <button class="menu-item" onclick={() => handleSelect(node.id)}>
            <span class="node-name">{node.name}</span>
            <span class="node-category">{node.category}</span>
          </button>
        {/each}
      </div>
    {:else if filteredNodes && filteredNodes.length === 0}
      <div class="no-results">No nodes found</div>
    {:else}
      <!-- Category view -->
      {#each categories as category}
        <div class="category">
          <button class="category-header" onclick={() => toggleCategory(category)}>
            <ChevronRight
              class="chevron"
              style="transform: rotate({expandedCategory === category ? '90deg' : '0deg'})"
            />
            <span>{category}</span>
          </button>

          {#if expandedCategory === category}
            <div class="category-items">
              {#each nodeRegistry.getByCategory(category) as node}
                <button class="menu-item" onclick={() => handleSelect(node.id)}>
                  <span class="node-name">{node.name}</span>
                  {#if node.pure}
                    <span class="badge pure">Pure</span>
                  {/if}
                  {#if node.latent}
                    <span class="badge latent">Async</span>
                  {/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .context-menu-backdrop {
    position: fixed;
    inset: 0;
    z-index: 999;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--neo-blueprint-menu-background);
    border: 1px solid var(--neo-blueprint-menu-border);
    border-radius: 4px;
    min-width: 220px;
    max-width: 300px;
    max-height: 400px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    box-shadow: 0 4px 12px var(--neo-blueprint-node-shadow);
  }

  .search-container {
    padding: 8px;
    border-bottom: 1px solid var(--neo-blueprint-menu-border);
  }

  .search-input {
    width: 100%;
    padding: 6px 10px;
    background: var(--neo-blueprint-menu-searchBackground);
    border: 1px solid var(--neo-blueprint-menu-border);
    border-radius: 4px;
    color: var(--neo-foreground);
    font-size: 12px;
    outline: none;
  }

  .search-input:focus {
    border-color: var(--neo-focusBorder);
  }

  .search-input::placeholder {
    color: var(--neo-input-placeholderForeground);
  }

  .menu-content {
    overflow-y: auto;
    max-height: 350px;
  }

  .category {
    border-bottom: 1px solid var(--neo-blueprint-menu-border);
  }

  .category:last-child {
    border-bottom: none;
  }

  .category-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    background: transparent;
    border: none;
    color: var(--neo-foreground);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    text-align: left;
  }

  .category-header:hover {
    background: var(--neo-list-hoverBackground);
  }

  .category-header :global(.chevron) {
    width: 14px;
    height: 14px;
    transition: transform 0.15s ease;
  }

  .category-items {
    background: var(--neo-blueprint-menu-categoryBackground);
  }

  .menu-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px 6px 30px;
    background: transparent;
    border: none;
    color: var(--neo-foreground);
    font-size: 12px;
    cursor: pointer;
    text-align: left;
  }

  .menu-item:hover {
    background: var(--neo-blueprint-menu-itemHover);
    color: var(--neo-primaryForeground);
  }

  .node-name {
    flex: 1;
  }

  .node-category {
    color: var(--neo-blueprint-node-pinLabel);
    font-size: 10px;
  }

  .badge {
    font-size: 9px;
    padding: 1px 4px;
    border-radius: 2px;
    text-transform: uppercase;
  }

  .badge.pure {
    background: var(--neo-blueprint-node-borderPure);
    color: white;
  }

  .badge.latent {
    background: var(--neo-blueprint-node-borderLatent);
    color: white;
  }

  .search-results {
    padding: 4px 0;
  }

  .search-results .menu-item {
    padding-left: 10px;
  }

  .no-results {
    padding: 20px;
    text-align: center;
    color: var(--neo-blueprint-node-pinLabel);
    font-size: 12px;
  }
</style>

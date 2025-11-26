<script lang="ts">
  import { ChevronRight, ChevronDown } from '@lucide/svelte'
  import type { ISettingCategory } from '$lib/settings/types'

  interface Props {
    categories: ISettingCategory[]
    selectedCategory: string | null
    expandedCategories: Set<string>
    onselect: (id: string | null) => void
    ontoggle: (id: string) => void
  }

  let { categories, selectedCategory, expandedCategories, onselect, ontoggle }: Props = $props()

  function isExpanded(id: string): boolean {
    return expandedCategories.has(id)
  }
</script>

<div class="toc">
  <!-- All Settings option -->
  <button
    class="toc-item"
    class:selected={selectedCategory === null}
    onclick={() => onselect(null)}
  >
    <span class="toc-label">Commonly Used</span>
  </button>

  <!-- Category tree -->
  {#each categories as category (category.id)}
    {@const expanded = isExpanded(category.id)}
    {@const hasChildren = category.children.length > 0}

    <div class="toc-category">
      <button
        class="toc-item"
        class:selected={selectedCategory === category.id}
        onclick={() => {
          if (hasChildren) {
            ontoggle(category.id)
          }
          onselect(category.id)
        }}
      >
        {#if hasChildren}
          <span class="expand-icon">
            {#if expanded}
              <ChevronDown class="w-3.5 h-3.5" />
            {:else}
              <ChevronRight class="w-3.5 h-3.5" />
            {/if}
          </span>
        {:else}
          <span class="expand-placeholder"></span>
        {/if}
        <span class="toc-label">{category.label}</span>
      </button>

      {#if hasChildren && expanded}
        <div class="toc-children">
          {#each category.children as child (child.id)}
            <button
              class="toc-item child"
              class:selected={selectedCategory === child.id}
              onclick={() => onselect(child.id)}
            >
              <span class="toc-label">{child.label}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .toc {
    padding: 0;
  }

  .toc-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 4px 16px 4px 8px;
    background: none;
    border: none;
    color: var(--neo-foreground);
    opacity: 0.9;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    line-height: 22px;
    transition: opacity 0.1s;
  }

  .toc-item:hover {
    opacity: 1;
    background: var(--neo-list-hoverBackground, rgba(255, 255, 255, 0.05));
  }

  .toc-item.selected {
    opacity: 1;
    font-weight: 600;
    background: var(--neo-list-activeSelectionBackground, rgba(255, 255, 255, 0.1));
  }

  .toc-item.child {
    padding-left: 28px;
  }

  .expand-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    margin-right: 4px;
    color: inherit;
    flex-shrink: 0;
    opacity: 0.7;
  }

  .expand-placeholder {
    width: 20px;
    flex-shrink: 0;
  }

  .toc-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toc-children {
    display: flex;
    flex-direction: column;
  }

  .toc-category {
    display: flex;
    flex-direction: column;
  }
</style>

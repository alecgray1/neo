<script lang="ts">
  import * as Command from '$lib/components/ui/command'
  import { quickAccessStore } from './store.svelte'
  import type { IHighlight } from '$lib/quickaccess/types'

  // Reactive state from store
  let visible = $derived(quickAccessStore.visible)
  let items = $derived(quickAccessStore.items)
  let selectedIndex = $derived(quickAccessStore.selectedIndex)
  let loading = $derived(quickAccessStore.loading)
  let placeholder = $derived(quickAccessStore.placeholder)

  // Local input value
  let inputValue = $state('')

  // Sync input value only when palette becomes visible
  let wasVisible = $state(false)
  $effect(() => {
    if (visible && !wasVisible) {
      // Just became visible - sync from store
      inputValue = quickAccessStore.value
    }
    wasVisible = visible
  })

  // Update store when input changes
  function onInput(e: Event) {
    const newValue = (e.target as HTMLInputElement).value
    inputValue = newValue
    quickAccessStore.setValue(newValue)
  }

  let dropdownRef: HTMLDivElement | undefined = $state()

  // Focus input when visible
  $effect(() => {
    if (visible && dropdownRef) {
      // Find and focus the input element
      requestAnimationFrame(() => {
        const input = dropdownRef?.querySelector('.quick-access-input') as HTMLInputElement
        if (input) {
          input.focus()
          // Move cursor to end (don't select, so prefix isn't replaced)
          const len = input.value.length
          input.setSelectionRange(len, len)
        }
      })
    }
  })

  // Scroll selected item into view when selection changes
  $effect(() => {
    // Depend on selectedIndex to trigger on change
    const idx = selectedIndex
    if (visible && dropdownRef && items.length > 0 && idx >= 0) {
      requestAnimationFrame(() => {
        const selectedEl = dropdownRef?.querySelector('.quick-access-item.selected')
        if (selectedEl) {
          selectedEl.scrollIntoView({ block: 'nearest' })
        }
      })
    }
  })

  // Handle backdrop click
  function handleBackdropClick() {
    quickAccessStore.hide()
  }

  // Handle item selection
  function handleSelect(itemId: string) {
    const item = items.find((i) => i.id === itemId)
    if (item) {
      const itemIndex = items.indexOf(item)
      quickAccessStore.selectIndex(itemIndex)
      quickAccessStore.acceptSelected()
    }
  }

  // Handle keyboard navigation at window level to capture before bits-ui
  function handleWindowKeydown(event: KeyboardEvent) {
    if (!visible) return

    if (event.key === 'ArrowDown') {
      event.preventDefault()
      event.stopPropagation()
      quickAccessStore.selectNext()
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      event.stopPropagation()
      quickAccessStore.selectPrevious()
    } else if (event.key === 'Enter') {
      event.preventDefault()
      event.stopPropagation()
      quickAccessStore.acceptSelected()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      quickAccessStore.hide()
    }
  }

  // Render text with highlights
  function renderHighlightedText(text: string, highlights?: IHighlight[]): string {
    if (!highlights || highlights.length === 0) {
      return escapeHtml(text)
    }

    let result = ''
    let lastIndex = 0

    for (const hl of highlights) {
      if (hl.start > lastIndex) {
        result += escapeHtml(text.slice(lastIndex, hl.start))
      }
      result += `<mark>${escapeHtml(text.slice(hl.start, hl.end))}</mark>`
      lastIndex = hl.end
    }

    if (lastIndex < text.length) {
      result += escapeHtml(text.slice(lastIndex))
    }

    return result
  }

  function escapeHtml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
  }

  // Group items by category
  let groupedItems = $derived.by(() => {
    const groups = new Map<string, typeof items>()

    for (const item of items) {
      const group = item.group || ''
      if (!groups.has(group)) {
        groups.set(group, [])
      }
      groups.get(group)!.push(item)
    }

    return Array.from(groups.entries())
  })
</script>

<!-- Global keyboard handler -->
<svelte:window onkeydowncapture={handleWindowKeydown} />

{#if visible}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="quick-access-backdrop"
    onclick={handleBackdropClick}
  ></div>

  <!-- Dropdown container -->
  <div
    bind:this={dropdownRef}
    class="quick-access-dropdown"
    role="dialog"
    aria-label="Quick Access"
  >
    <Command.Root
      class="quick-access-command"
      shouldFilter={false}
    >
      <div class="quick-access-input-wrapper">
        <input
          type="text"
          {placeholder}
          bind:value={inputValue}
          oninput={onInput}
          class="quick-access-input"
        />
      </div>
      <Command.List class="quick-access-list">
        {#if loading}
          <Command.Loading>
            <div class="py-4 text-center text-xs text-muted-foreground">
              Loading...
            </div>
          </Command.Loading>
        {:else if items.length === 0}
          <Command.Empty>
            <div class="py-4 text-center text-xs text-muted-foreground">
              No results found.
            </div>
          </Command.Empty>
        {:else}
          {#each groupedItems as [group, groupItems], groupIndex}
            <Command.Group heading={group || undefined}>
              {#each groupItems as item, itemIndex}
                {@const globalIndex = groupIndex === 0 ? itemIndex : items.indexOf(item)}
                <Command.Item
                  value={item.id}
                  onSelect={() => handleSelect(item.id)}
                  class="quick-access-item {globalIndex === selectedIndex ? 'selected' : ''}"
                >
                  <span class="item-label">
                    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                    {@html renderHighlightedText(item.label, item.labelHighlights)}
                  </span>
                  {#if item.keybinding}
                    <span class="item-keybinding">{item.keybinding}</span>
                  {/if}
                </Command.Item>
              {/each}
            </Command.Group>
          {/each}
        {/if}
      </Command.List>
    </Command.Root>
  </div>
{/if}

<style>
  .quick-access-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: transparent;
  }

  .quick-access-dropdown {
    position: fixed;
    top: 30px; /* Below titlebar */
    left: 50%;
    transform: translateX(-50%);
    z-index: 51;
    width: min(600px, calc(100vw - 40px));
    background: var(--neo-editor-background, #1e1e1e);
    border: 1px solid var(--neo-widget-border, #454545);
    border-radius: 6px;
    box-shadow:
      0 8px 32px rgb(0 0 0 / 0.4),
      0 4px 16px rgb(0 0 0 / 0.3);
    overflow: hidden;
  }

  .quick-access-dropdown :global(.quick-access-command) {
    background: transparent;
    border: none;
  }

  .quick-access-input-wrapper {
    border-bottom: 1px solid var(--neo-widget-border, #454545);
    background: var(--neo-input-background, #3c3c3c);
  }

  .quick-access-input {
    width: 100%;
    background: transparent;
    border: none;
    padding: 8px 12px;
    font-size: 13px;
    color: var(--neo-input-foreground, #cccccc);
  }

  .quick-access-input::placeholder {
    color: var(--neo-input-placeholderForeground, #808080);
  }

  .quick-access-input:focus {
    outline: none;
    box-shadow: none;
  }

  .quick-access-dropdown :global(.quick-access-list) {
    max-height: 322px;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 6px 0;
  }

  .quick-access-dropdown :global(.quick-access-item) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 22px;
    padding: 0 10px;
    cursor: pointer;
    font-size: 13px;
    line-height: 22px;
  }

  .quick-access-dropdown :global(.quick-access-item .item-label) {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-access-dropdown :global(.quick-access-item .item-keybinding) {
    margin-left: 12px;
    font-size: 11px;
    opacity: 0.7;
    flex-shrink: 0;
  }

  .quick-access-dropdown :global(.quick-access-item.selected) {
    background: var(--neo-list-activeSelectionBackground, #04395e);
    color: var(--neo-list-activeSelectionForeground, #ffffff);
  }

  .quick-access-dropdown :global(.quick-access-item.selected .item-keybinding) {
    opacity: 1;
  }

  .quick-access-dropdown :global(.quick-access-item:hover:not(.selected)) {
    background: var(--neo-list-hoverBackground, #2a2d2e);
  }

  .quick-access-dropdown :global([data-command-group-heading]) {
    display: none;
  }

  /* Highlight styling */
  .quick-access-dropdown :global(mark) {
    background: transparent;
    color: var(--neo-editorSuggestWidget-highlightForeground, #18a3ff);
    font-weight: 600;
  }
</style>

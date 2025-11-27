<script lang="ts">
  import { editorStore, type EditorTab as EditorTabType } from '$lib/stores/editor.svelte'
  import EditorTab from './EditorTab.svelte'
  import { Columns2 } from '@lucide/svelte'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { getMenuActions, type IMenuActionGroup } from '$lib/menus/service'
  import { MenuId } from '$lib/menus/menuId'
  import { getContextKeyService } from '$lib/services/context'

  interface Props {
    groupId: string
  }

  let { groupId }: Props = $props()

  // Get the group state directly (it's its own $state class)
  let group = $derived(editorStore.getGroup(groupId))
  // Derive tabs directly so the {#each} block properly tracks the $state array
  let tabs = $derived(group?.tabs ?? [])

  let isDragOver = $state(false)

  // Context menu state
  let contextMenuOpen = $state(false)
  let contextMenuPosition = $state({ x: 0, y: 0 })
  let contextMenuTab = $state<EditorTabType | null>(null)
  let contextMenuGroups = $state<IMenuActionGroup[]>([])

  // Create a scoped context for tab context menu
  const tabContext = getContextKeyService().createScoped()

  function handleTabContextMenu(e: MouseEvent, tab: EditorTabType) {
    e.preventDefault()

    // Update context for "when" clause evaluation
    const tabIndex = tabs.findIndex((t) => t.id === tab.id)
    const hasTabsToRight = tabIndex >= 0 && tabIndex < tabs.length - 1

    tabContext.set('tabId', tab.id)
    tabContext.set('tabGroupId', group.id)
    tabContext.set('tabIsPinned', tab.isPinned)
    tabContext.set('tabIsDirty', tab.dirty)
    tabContext.set('tabIsPreview', tab.isPreview)
    tabContext.set('tabCount', tabs.length)
    tabContext.set('hasTabsToRight', hasTabsToRight)

    // Build menu actions with tab context
    const tabArg = { tabId: tab.id, groupId: group.id }
    contextMenuGroups = getMenuActions(MenuId.EditorTabContext, tabContext, { arg: tabArg })

    // Store position and tab for the menu
    contextMenuPosition = { x: e.clientX, y: e.clientY }
    contextMenuTab = tab
    contextMenuOpen = true
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move'
    }
    isDragOver = true
  }

  function handleDragLeave() {
    isDragOver = false
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault()
    isDragOver = false

    if (!e.dataTransfer) return

    try {
      const data = JSON.parse(e.dataTransfer.getData('text/plain'))
      const { tabId, sourceGroupId } = data

      if (tabId && sourceGroupId !== group.id) {
        editorStore.moveTab(tabId, group.id)
      }
    } catch {
      // Invalid drop data
    }
  }

  function handleSplitRight() {
    if (group.activeTabId) {
      const newGroupId = editorStore.splitGroup('horizontal', group.id)
      editorStore.moveTab(group.activeTabId, newGroupId)
    }
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div
  class="editor-tab-bar flex items-end h-[35px] overflow-x-auto"
  class:drag-over={isDragOver}
  style="background: var(--neo-editorGroupHeader-tabsBackground); border-bottom: 1px solid var(--neo-editorGroupHeader-tabsBorder);"
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  role="tablist"
>
  {#key tabs}
    {#each tabs as tab (tab.id)}
      <EditorTab {tab} {groupId} oncontextmenu={handleTabContextMenu} />
    {/each}
  {/key}

  <!-- Empty space for drop target -->
  <div class="flex-1 h-full" ondragover={handleDragOver} ondrop={handleDrop} role="presentation"></div>

  <!-- Actions -->
  <div class="tab-actions flex items-center px-1 h-full shrink-0">
    <button
      class="split-btn p-1 rounded hover:bg-[var(--neo-toolbar-hoverBackground)] opacity-60 hover:opacity-100 transition-opacity"
      onclick={handleSplitRight}
      title="Split Editor Right"
      disabled={!group.activeTabId}
    >
      <Columns2 class="w-4 h-4" />
    </button>
  </div>
</div>

<!-- Tab Context Menu (positioned at click location) -->
{#if contextMenuOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <!-- Backdrop to close menu on click outside -->
  <div
    class="fixed inset-0 z-50"
    onclick={() => (contextMenuOpen = false)}
    oncontextmenu={(e) => { e.preventDefault(); contextMenuOpen = false }}
    onkeydown={(e) => e.key === 'Escape' && (contextMenuOpen = false)}
  ></div>
  <!-- Menu -->
  <div
    class="fixed z-50 min-w-[180px] rounded-md border bg-popover p-1 text-popover-foreground shadow-md"
    style="left: {contextMenuPosition.x}px; top: {contextMenuPosition.y}px;"
  >
    {#if contextMenuGroups.length === 0}
      <div class="px-2 py-1.5 text-sm text-muted-foreground">No actions available</div>
    {:else}
      {#each contextMenuGroups as menuGroup, groupIndex}
        {#if groupIndex > 0}
          <div class="my-1 h-px bg-border"></div>
        {/if}
        {#each menuGroup.actions as action}
          <button
            class="relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground disabled:pointer-events-none disabled:opacity-50"
            onclick={() => { action.run(); contextMenuOpen = false }}
            disabled={!action.enabled}
          >
            {action.label}
          </button>
        {/each}
      {/each}
    {/if}
  </div>
{/if}

<style>
  .editor-tab-bar {
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar) transparent;
  }

  .editor-tab-bar::-webkit-scrollbar {
    height: 4px;
  }

  .editor-tab-bar::-webkit-scrollbar-thumb {
    background: var(--scrollbar);
    border-radius: 2px;
  }

  .editor-tab-bar::-webkit-scrollbar-thumb:hover {
    background: var(--scrollbar-hover);
  }

  .editor-tab-bar.drag-over {
    outline: 2px solid var(--neo-tab-dragAndDropBorder);
    outline-offset: -2px;
  }

  .split-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
</style>

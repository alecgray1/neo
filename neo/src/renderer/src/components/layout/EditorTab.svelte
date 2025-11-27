<script lang="ts">
  import { editorStore, type EditorTab } from '$lib/stores/editor.svelte'
  import { X, Circle, Pin } from '@lucide/svelte'

  interface Props {
    tab: EditorTab
    groupId: string
    oncontextmenu?: (e: MouseEvent, tab: EditorTab) => void
  }

  let { tab, groupId, oncontextmenu: onContextMenuProp }: Props = $props()

  // Compute isActive by reading directly from group state
  let group = $derived(editorStore.getGroup(groupId))
  let isActive = $derived(group?.activeTabId === tab.id)

  function handleDragStart(e: DragEvent) {
    if (e.dataTransfer) {
      e.dataTransfer.setData('text/plain', JSON.stringify({ tabId: tab.id, sourceGroupId: groupId }))
      e.dataTransfer.effectAllowed = 'move'
    }
  }

  function handleClose(e: MouseEvent) {
    e.stopPropagation()
    editorStore.closeTab(tab.id, groupId)
  }

  function handleClick() {
    editorStore.setActiveTab(tab.id, groupId)
  }

  function handleDblClick() {
    // Double-click promotes from preview or pins the tab
    if (tab.isPreview) {
      editorStore.promoteFromPreview(tab.id, groupId)
    }
  }

  function handleMiddleClick(e: MouseEvent) {
    if (e.button === 1) {
      e.preventDefault()
      editorStore.closeTab(tab.id, groupId)
    }
  }

  function handleContextMenu(e: MouseEvent) {
    onContextMenuProp?.(e, tab)
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="editor-tab flex items-center gap-1.5 h-[35px] text-sm border-r transition-colors cursor-pointer"
  class:active={isActive}
  class:preview={tab.isPreview}
  class:pinned={tab.isPinned}
  draggable="true"
  ondragstart={handleDragStart}
  onclick={handleClick}
  ondblclick={handleDblClick}
  onauxclick={handleMiddleClick}
  oncontextmenu={handleContextMenu}
  onkeydown={(e) => e.key === 'Enter' && handleClick()}
  role="tab"
  tabindex="0"
  aria-selected={isActive}
  style="padding-left: {tab.isPinned ? '0.5rem' : '0.75rem'}; padding-right: {tab.isPinned ? '0.5rem' : '0.75rem'};"
>
  <!-- Pinned indicator -->
  {#if tab.isPinned}
    <Pin class="w-3 h-3 shrink-0 opacity-70" />
  {/if}

  <!-- Dirty indicator -->
  {#if tab.dirty}
    <Circle class="w-2 h-2 fill-current shrink-0" />
  {/if}

  <!-- Tab title (italic for preview) -->
  <span class="truncate" class:italic={tab.isPreview} style="max-width: {tab.isPinned ? '60px' : '120px'};">
    {tab.title}
  </span>

  <!-- Close button (hidden for pinned tabs unless hovered) -->
  {#if !tab.isPinned}
    <button
      class="close-btn p-0.5 rounded hover:bg-[var(--neo-list-hoverBackground)] opacity-0"
      onclick={handleClose}
      tabindex={-1}
    >
      <X class="w-3.5 h-3.5" />
    </button>
  {/if}
</div>

<style>
  .editor-tab {
    background: var(--neo-tab-inactiveBackground);
    color: var(--neo-tab-inactiveForeground);
    border-color: var(--neo-tab-border);
  }

  .editor-tab:hover {
    background: var(--neo-tab-hoverBackground);
  }

  .editor-tab.active {
    background: var(--neo-tab-activeBackground);
    color: var(--neo-tab-activeForeground);
    border-top: 2px solid var(--neo-tab-activeBorderTop);
    margin-top: -2px;
  }

  .editor-tab:hover .close-btn {
    opacity: 1;
  }
</style>

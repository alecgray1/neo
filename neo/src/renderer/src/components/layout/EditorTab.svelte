<script lang="ts">
  import { editorStore, type EditorTab } from '$lib/stores/editor.svelte'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { X, Circle } from '@lucide/svelte'

  interface Props {
    tab: EditorTab
    groupId: string
    isActive: boolean
  }

  let { tab, groupId, isActive }: Props = $props()

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

  function closeOtherTabs() {
    const group = editorStore.layout.groups.get(groupId)
    if (!group) return
    const otherTabs = group.tabs.filter((t) => t.id !== tab.id)
    otherTabs.forEach((t) => editorStore.closeTab(t.id, groupId))
  }

  function closeTabsToRight() {
    const group = editorStore.layout.groups.get(groupId)
    if (!group) return
    const tabIndex = group.tabs.findIndex((t) => t.id === tab.id)
    const tabsToClose = group.tabs.slice(tabIndex + 1)
    tabsToClose.forEach((t) => editorStore.closeTab(t.id, groupId))
  }

  function splitRight() {
    const newGroupId = editorStore.splitGroup('horizontal', groupId)
    editorStore.moveTab(tab.id, newGroupId)
  }

  function splitDown() {
    const newGroupId = editorStore.splitGroup('vertical', groupId)
    editorStore.moveTab(tab.id, newGroupId)
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger asChild>
    {#snippet child({ props })}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      {...props}
      class="editor-tab flex items-center gap-1.5 px-3 h-[35px] text-sm border-r transition-colors cursor-pointer"
      class:active={isActive}
      draggable="true"
      ondragstart={handleDragStart}
      onclick={handleClick}
      onkeydown={(e) => e.key === 'Enter' && handleClick()}
      role="tab"
      tabindex="0"
      aria-selected={isActive}
    >
      <!-- Dirty indicator or icon -->
      {#if tab.dirty}
        <Circle class="w-2 h-2 fill-current" />
      {/if}

      <!-- Tab title -->
      <span class="truncate max-w-[120px]">{tab.title}</span>

      <!-- Close button -->
      <button
        class="close-btn p-0.5 rounded hover:bg-[var(--neo-list-hoverBackground)] opacity-0"
        onclick={handleClose}
        tabindex={-1}
      >
        <X class="w-3.5 h-3.5" />
      </button>
    </div>
    {/snippet}
  </ContextMenu.Trigger>

  <ContextMenu.Content>
    <ContextMenu.Item onclick={handleClose}>Close</ContextMenu.Item>
    <ContextMenu.Item onclick={closeOtherTabs}>Close Others</ContextMenu.Item>
    <ContextMenu.Item onclick={closeTabsToRight}>Close to the Right</ContextMenu.Item>
    <ContextMenu.Separator />
    <ContextMenu.Item onclick={splitRight}>Split Right</ContextMenu.Item>
    <ContextMenu.Item onclick={splitDown}>Split Down</ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu.Root>

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

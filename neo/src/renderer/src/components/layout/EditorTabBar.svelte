<script lang="ts">
  import { editorStore, type EditorGroup } from '$lib/stores/editor.svelte'
  import EditorTab from './EditorTab.svelte'

  interface Props {
    group: EditorGroup
  }

  let { group }: Props = $props()

  let isDragOver = $state(false)

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
  {#each group.tabs as tab (tab.id)}
    <EditorTab {tab} groupId={group.id} isActive={group.activeTabId === tab.id} />
  {/each}

  <!-- Empty space for drop target -->
  <div class="flex-1 h-full" ondragover={handleDragOver} ondrop={handleDrop} role="presentation"></div>
</div>

<style>
  .editor-tab-bar {
    scrollbar-width: thin;
  }

  .editor-tab-bar::-webkit-scrollbar {
    height: 4px;
  }

  .editor-tab-bar::-webkit-scrollbar-thumb {
    background: var(--neo-scrollbar-thumb);
    border-radius: 2px;
  }

  .editor-tab-bar.drag-over {
    outline: 2px solid var(--neo-tab-dragAndDropBorder);
    outline-offset: -2px;
  }
</style>

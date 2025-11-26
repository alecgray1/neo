<script lang="ts">
  import { editorStore, type EditorGroup } from '$lib/stores/editor.svelte'
  import EditorTabBar from './EditorTabBar.svelte'
  import { FileText, Plus } from '@lucide/svelte'

  interface Props {
    groupId: string
  }

  let { groupId }: Props = $props()

  let group = $derived(editorStore.layout.groups.get(groupId))
  let activeTab = $derived(group?.tabs.find((t) => t.id === group?.activeTabId))
  let isActiveGroup = $derived(editorStore.activeGroupId === groupId)

  function handleClick() {
    editorStore.setActiveGroup(groupId)
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move'
    }
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault()
    if (!e.dataTransfer) return

    try {
      const data = JSON.parse(e.dataTransfer.getData('text/plain'))
      const { tabId, sourceGroupId } = data

      if (tabId && sourceGroupId !== groupId) {
        editorStore.moveTab(tabId, groupId)
      }
    } catch {
      // Invalid drop data
    }
  }

  function openNewTab() {
    editorStore.openTab(
      {
        title: 'Untitled',
        content: null
      },
      groupId
    )
  }
</script>

{#if group}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions -->
  <div
    class="editor-group h-full flex flex-col"
    class:active={isActiveGroup}
    onclick={handleClick}
    ondragover={handleDragOver}
    ondrop={handleDrop}
  >
    <!-- Tab Bar -->
    <EditorTabBar {group} />

    <!-- Content Area -->
    <div
      class="editor-content flex-1 overflow-auto"
      style="background: var(--neo-background);"
    >
      {#if activeTab}
        <!-- Active tab content -->
        <div class="p-4">
          <div class="text-sm opacity-60">Content for: {activeTab.title}</div>
          {#if activeTab.content}
            <pre class="mt-4 text-xs">{JSON.stringify(activeTab.content, null, 2)}</pre>
          {/if}
        </div>
      {:else}
        <!-- Empty state -->
        <div class="h-full flex flex-col items-center justify-center gap-4 opacity-60">
          <FileText class="w-16 h-16 opacity-30" />
          <div class="text-sm">No editor is open</div>
          <button
            class="flex items-center gap-2 px-3 py-1.5 text-sm rounded border hover:bg-[var(--neo-list-hoverBackground)]"
            style="border-color: var(--neo-border);"
            onclick={openNewTab}
          >
            <Plus class="w-4 h-4" />
            New File
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .editor-group {
    border: 1px solid transparent;
  }

  .editor-group.active {
    /* Could add subtle highlight for active group */
  }
</style>

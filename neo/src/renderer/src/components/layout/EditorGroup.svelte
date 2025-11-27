<script lang="ts">
  import { editorStore } from '$lib/stores/editor.svelte'
  import { documentStore } from '$lib/stores/documents.svelte'
  import EditorTabBar from './EditorTabBar.svelte'
  import EditorContent from './editor/EditorContent.svelte'
  import EditorDropOverlay, { type DropZone } from './EditorDropOverlay.svelte'
  import { FileText } from '@lucide/svelte'

  interface Props {
    groupId: string
  }

  let { groupId }: Props = $props()

  let group = $derived(editorStore.getGroup(groupId))
  let activeTab = $derived(group?.tabs.find((t) => t.id === group?.activeTabId))
  let isActiveGroup = $derived(editorStore.activeGroupId === groupId)

  // Drop overlay state
  let showDropOverlay = $state(false)
  let dropZone = $state<DropZone>(null)
  let contentAreaEl: HTMLDivElement | undefined = $state()

  function handleClick() {
    editorStore.setActiveGroup(groupId)
  }

  // Calculate drop zone based on mouse position (VS Code uses 1/3 thresholds)
  function calculateDropZone(e: DragEvent): DropZone {
    if (!contentAreaEl) return 'center'

    const rect = contentAreaEl.getBoundingClientRect()
    const x = e.clientX - rect.left
    const y = e.clientY - rect.top
    const width = rect.width
    const height = rect.height

    // Use 1/3 thresholds like VS Code
    const leftThreshold = width / 3
    const rightThreshold = (width * 2) / 3
    const topThreshold = height / 3
    const bottomThreshold = (height * 2) / 3

    // Determine zone based on position
    if (x < leftThreshold) return 'left'
    if (x > rightThreshold) return 'right'
    if (y < topThreshold) return 'top'
    if (y > bottomThreshold) return 'bottom'
    return 'center'
  }

  function handleDragEnter(e: DragEvent) {
    e.preventDefault()
    showDropOverlay = true
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault()
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = 'move'
    }
    dropZone = calculateDropZone(e)
  }

  function handleDragLeave(e: DragEvent) {
    // Only hide if leaving the content area entirely
    const relatedTarget = e.relatedTarget as Node | null
    if (contentAreaEl && relatedTarget && contentAreaEl.contains(relatedTarget)) {
      return
    }
    showDropOverlay = false
    dropZone = null
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault()
    showDropOverlay = false
    const currentZone = dropZone
    dropZone = null

    if (!e.dataTransfer) return

    try {
      const data = JSON.parse(e.dataTransfer.getData('text/plain'))
      const { tabId, sourceGroupId, uri, title } = data

      // Handle file drop from file tree
      if (uri && !tabId) {
        const doc = await documentStore.open(uri)
        if (doc) {
          if (currentZone === 'center') {
            // Open in this group
            editorStore.openTab({ title: title || doc.name, uri: doc.uri, isPreview: false }, groupId)
          } else {
            // Create split and open in new group
            const direction = currentZone === 'left' || currentZone === 'right' ? 'horizontal' : 'vertical'
            const newGroupId = editorStore.splitGroup(direction, groupId)

            // If dropping on left/top, we need to swap the groups
            if (currentZone === 'left' || currentZone === 'top') {
              // Move existing tabs to new group, open file in original
              const existingTabs = [...(group?.tabs || [])]
              for (const tab of existingTabs) {
                editorStore.moveTab(tab.id, newGroupId)
              }
              editorStore.openTab({ title: title || doc.name, uri: doc.uri, isPreview: false }, groupId)
            } else {
              editorStore.openTab({ title: title || doc.name, uri: doc.uri, isPreview: false }, newGroupId)
            }
          }
        }
        return
      }

      // Handle tab drop
      if (tabId) {
        if (currentZone === 'center') {
          // Move to this group (merge)
          if (sourceGroupId !== groupId) {
            editorStore.moveTab(tabId, groupId)
          }
        } else {
          // Create split
          const direction = currentZone === 'left' || currentZone === 'right' ? 'horizontal' : 'vertical'
          const newGroupId = editorStore.splitGroup(direction, groupId)

          if (currentZone === 'left' || currentZone === 'top') {
            // Move existing tabs to new group, move dragged tab to original
            const existingTabs = [...(group?.tabs || [])]
            for (const tab of existingTabs) {
              editorStore.moveTab(tab.id, newGroupId)
            }
            editorStore.moveTab(tabId, groupId)
          } else {
            editorStore.moveTab(tabId, newGroupId)
          }
        }
      }
    } catch {
      // Invalid drop data
    }
  }
</script>

{#if group}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_no_noninteractive_element_interactions -->
  <div
    class="editor-group h-full flex flex-col"
    class:active={isActiveGroup}
    onclick={handleClick}
  >
    <!-- Tab Bar -->
    <EditorTabBar {groupId} />

    <!-- Content Area with Drop Target -->
    <div
      bind:this={contentAreaEl}
      class="editor-content-area flex-1 overflow-hidden relative"
      style="background: var(--neo-editor-background);"
      ondragenter={handleDragEnter}
      ondragover={handleDragOver}
      ondragleave={handleDragLeave}
      ondrop={handleDrop}
    >
      {#if activeTab && activeTab.uri}
        <!-- Active tab content -->
        <EditorContent uri={activeTab.uri} />
      {:else}
        <!-- Empty state -->
        <div class="h-full flex flex-col items-center justify-center gap-2 opacity-60">
          <FileText class="w-12 h-12 opacity-30" />
          <div class="text-sm">No file open</div>
          <div class="text-xs opacity-70">Select a file from the Explorer or drag a tab here</div>
        </div>
      {/if}

      <!-- Drop Overlay -->
      <EditorDropOverlay visible={showDropOverlay} zone={dropZone} />
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

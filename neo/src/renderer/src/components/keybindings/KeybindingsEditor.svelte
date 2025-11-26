<script lang="ts">
  import { getKeybindingsEditorStore } from './store.svelte'
  import KeybindingsSearch from './KeybindingsSearch.svelte'
  import KeybindingsTable from './KeybindingsTable.svelte'
  import DefineKeybindingDialog from './DefineKeybindingDialog.svelte'
  import { onMount } from 'svelte'
  import { getUserKeybindingsService } from '$lib/keybindings/userKeybindings'
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'

  const store = getKeybindingsEditorStore()

  // Initialize user keybindings service on mount
  onMount(async () => {
    const userService = getUserKeybindingsService()
    await userService.initialize()
    store.refresh()
  })

  // Reactive state from store
  let entries = $derived(store.entries)
  let searchQuery = $derived(store.searchQuery)
  let sortMode = $derived(store.sortMode)
  let isRecordMode = $derived(store.isRecordMode)
  let isDefineDialogOpen = $derived(store.isDefineDialogOpen)
</script>

<div class="keybindings-editor h-full flex flex-col" style="background: var(--neo-editor-background);">
  <!-- Header -->
  <div
    class="editor-header flex items-center gap-4 px-4 h-[38px] shrink-0"
    style="
      background: var(--neo-editorWidget-background);
      border-bottom: 1px solid var(--neo-editorGroupHeader-tabsBorder);
    "
  >
    <span class="text-sm font-medium" style="color: var(--neo-foreground);">
      Keyboard Shortcuts
    </span>
    <div class="flex-1"></div>
    <button
      class="text-xs px-2 py-1 rounded"
      style="
        background: var(--neo-button-secondaryBackground);
        color: var(--neo-button-secondaryForeground);
      "
      onclick={() => store.setSortMode(sortMode === 'command' ? 'precedence' : 'command')}
    >
      Sort: {sortMode === 'command' ? 'A-Z' : 'Precedence'}
    </button>
  </div>

  <!-- Search -->
  <KeybindingsSearch
    value={searchQuery}
    {isRecordMode}
    onchange={(value) => store.setSearchQuery(value)}
    ontogglerecord={() => store.toggleRecordMode()}
    onrecord={(parsed) => store.recordKeybinding(parsed)}
    onexitrecord={() => store.exitRecordMode()}
  />

  <!-- Table -->
  <div class="flex-1 overflow-hidden">
    <ScrollArea class="h-full">
      <KeybindingsTable
        {entries}
        selectedId={store.selectedId}
        onselect={(id) => store.selectEntry(id)}
        onedit={(commandId, key) => store.openDefineDialog(commandId, key)}
        onadd={(commandId) => store.openDefineDialog(commandId)}
        onremove={(commandId, key, when) => store.removeKeybinding(commandId, key, when)}
        onreset={(commandId, key) => store.resetKeybinding(commandId, key)}
      />
    </ScrollArea>
  </div>

  <!-- Status bar -->
  <div
    class="status-bar flex items-center gap-4 px-4 h-[22px] text-xs shrink-0"
    style="
      background: var(--neo-editorWidget-background);
      border-top: 1px solid var(--neo-editorGroupHeader-tabsBorder);
      color: var(--neo-descriptionForeground);
    "
  >
    <span>{entries.length} keybindings</span>
    {#if isRecordMode}
      <span class="text-yellow-400">Recording keys... (Escape to cancel)</span>
    {/if}
  </div>

  <!-- Define Keybinding Dialog -->
  {#if isDefineDialogOpen}
    <DefineKeybindingDialog
      commandId={store.defineDialogCommandId}
      existingKey={store.defineDialogExistingKey}
      onconfirm={(key, when) => {
        if (store.defineDialogCommandId) {
          store.saveKeybinding(store.defineDialogCommandId, key, when)
        }
      }}
      oncancel={() => store.closeDefineDialog()}
      findConflicts={(key) => store.findConflicts(key, store.defineDialogCommandId ?? undefined)}
    />
  {/if}
</div>

<style>
  .keybindings-editor {
    font-family: var(--neo-font-family);
  }
</style>

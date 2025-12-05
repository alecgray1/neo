<script lang="ts">
  import { documentStore } from '$lib/stores/documents.svelte'
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'
  import JsonViewer from './JsonViewer.svelte'
  import BlueprintEditor from '../../blueprint/BlueprintEditor.svelte'
  import KeybindingsEditor from '../../keybindings/KeybindingsEditor.svelte'
  import SettingsEditor from '../../settings/SettingsEditor.svelte'
  import BacnetDeviceEditor from '../../bacnet/BacnetDeviceEditor.svelte'
  import { FileText, ChevronRight } from '@lucide/svelte'
  import NeoContextMenu from '../../contextmenu/NeoContextMenu.svelte'
  import { MenuId } from '$lib/menus/menuId'
  import { getContextKeyService } from '$lib/services/context'

  interface Props {
    uri: string
  }

  let { uri }: Props = $props()

  let document = $derived(documentStore.get(uri))

  // Create editor context with relevant keys
  const editorContext = getContextKeyService().createScoped()

  // Update editor context when document changes (for "when" clause evaluation)
  $effect(() => {
    const hasSelection = window.getSelection()?.toString().length ?? 0 > 0
    editorContext.set('editorFocus', true)
    editorContext.set('editorReadOnly', true) // Our current editors are read-only
    editorContext.set('hasSelection', hasSelection)
    editorContext.set('languageId', document?.language ?? 'plaintext')
    editorContext.set('editorUri', uri)
  })

  // Editor arg for commands (VS Code pattern)
  let editorArg = $derived({
    uri: uri,
    languageId: document?.language ?? 'plaintext'
  })

  // Parse path for breadcrumb
  let pathParts = $derived(() => {
    if (!uri) return []
    const path = uri.replace('mock://', '')
    return path.split('/')
  })

  // Handle content changes from editors
  function handleContentChange(newContent: string) {
    if (document) {
      documentStore.updateContent(uri, newContent)
    }
  }
</script>

<div class="editor-content h-full flex flex-col" style="background: var(--neo-editor-background);">
  {#if document}
    <!-- Breadcrumb -->
    <div
      class="breadcrumb flex items-center gap-1 px-4 h-[22px] text-xs shrink-0"
      style="background: var(--neo-breadcrumb-background); border-bottom: 1px solid var(--neo-editorGroupHeader-tabsBorder);"
    >
      {#each pathParts() as part, index}
        {#if index > 0}
          <ChevronRight class="w-3 h-3 opacity-50" />
        {/if}
        <span
          class="breadcrumb-item"
          class:active={index === pathParts().length - 1}
          style="color: var(--neo-breadcrumb-foreground);"
        >
          {part}
        </span>
      {/each}
    </div>

    <!-- Content -->
    {#if document.language === 'settings'}
      <!-- Settings editor -->
      <div class="flex-1 overflow-hidden">
        <SettingsEditor />
      </div>
    {:else if document.language === 'keybindings'}
      <!-- Keybindings editor -->
      <div class="flex-1 overflow-hidden">
        <KeybindingsEditor />
      </div>
    {:else if document.language === 'blueprint'}
      <!-- Blueprint editor takes full area (handles its own panning) -->
      <div class="flex-1 overflow-hidden">
        <BlueprintEditor
          content={document.content}
          externalUpdateCounter={document.externalUpdateCounter}
          onchange={handleContentChange}
        />
      </div>
    {:else if document.language === 'bacnet-device'}
      <!-- BACnet device viewer -->
      <div class="flex-1 overflow-hidden">
        <BacnetDeviceEditor content={document.content} />
      </div>
    {:else}
      <ScrollArea class="flex-1">
        <NeoContextMenu menuId={MenuId.EditorContext} contextKeyService={editorContext} arg={editorArg}>
          {#snippet children()}
            <div class="content-wrapper" style="color: var(--neo-editor-foreground);">
              {#if document.language === 'json'}
                <JsonViewer content={document.content} />
              {:else}
                <!-- Plain text fallback -->
                <pre class="p-4 text-sm font-mono whitespace-pre-wrap">{document.content}</pre>
              {/if}
            </div>
          {/snippet}
        </NeoContextMenu>
      </ScrollArea>
    {/if}

    <!-- Status info (hide for special editors) -->
    {#if document.language !== 'settings' && document.language !== 'keybindings'}
      <div
        class="status-info flex items-center gap-4 px-4 h-[22px] text-xs shrink-0"
        style="background: var(--neo-editorWidget-background); border-top: 1px solid var(--neo-editorGroupHeader-tabsBorder);"
      >
        <span style="color: var(--neo-descriptionForeground);">
          {document.language.toUpperCase()}
        </span>
        <span style="color: var(--neo-descriptionForeground);">
          {document.metadata.lineCount} lines
        </span>
        {#if document.metadata.isLargeFile}
          <span class="text-yellow-500">Large file</span>
        {/if}
      </div>
    {/if}
  {:else}
    <!-- Loading or not found state -->
    <div class="h-full flex items-center justify-center">
      <div class="text-center opacity-60">
        <FileText class="w-12 h-12 mx-auto mb-2 opacity-30" />
        <div class="text-sm">Loading document...</div>
      </div>
    </div>
  {/if}
</div>

<style>
  .breadcrumb-item {
    opacity: 0.7;
  }

  .breadcrumb-item.active {
    opacity: 1;
    font-weight: 500;
  }

  .breadcrumb-item:hover:not(.active) {
    opacity: 1;
    cursor: pointer;
  }
</style>

<script lang="ts">
  import { documentStore } from '$lib/stores/documents.svelte'
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'
  import JsonViewer from './JsonViewer.svelte'
  import { FileText, ChevronRight } from '@lucide/svelte'

  interface Props {
    uri: string
  }

  let { uri }: Props = $props()

  let document = $derived(documentStore.get(uri))

  // Parse path for breadcrumb
  let pathParts = $derived(() => {
    if (!uri) return []
    const path = uri.replace('mock://', '')
    return path.split('/')
  })
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
    <ScrollArea class="flex-1">
      <div class="content-wrapper" style="color: var(--neo-editor-foreground);">
        {#if document.language === 'json'}
          <JsonViewer content={document.content} />
        {:else}
          <!-- Plain text fallback -->
          <pre class="p-4 text-sm font-mono whitespace-pre-wrap">{document.content}</pre>
        {/if}
      </div>
    </ScrollArea>

    <!-- Status info -->
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

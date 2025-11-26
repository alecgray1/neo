<script lang="ts">
  import { Search, Keyboard } from '@lucide/svelte'
  import type { IParsedKeybinding } from '$lib/keybindings/types'
  import { keyboardEventToKeybinding, formatKeybindingForDisplay, normalizeKeybinding } from '$lib/keybindings/parser'

  interface Props {
    value: string
    isRecordMode: boolean
    onchange: (value: string) => void
    ontogglerecord: () => void
    onrecord: (parsed: IParsedKeybinding) => void
    onexitrecord: () => void
  }

  let { value, isRecordMode, onchange, ontogglerecord, onrecord, onexitrecord }: Props = $props()

  let inputRef: HTMLInputElement | undefined = $state()
  let recordedKeys = $state<string>('')

  // Handle keyboard events in record mode
  function handleKeydown(event: KeyboardEvent) {
    if (!isRecordMode) return

    event.preventDefault()
    event.stopPropagation()

    // Escape exits record mode
    if (event.key === 'Escape') {
      recordedKeys = ''
      onexitrecord()
      return
    }

    // Ignore lone modifier keys
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) {
      return
    }

    const parsed = keyboardEventToKeybinding(event)
    const formatted = formatKeybindingForDisplay(normalizeKeybinding(parsed))
    recordedKeys = formatted
    onrecord(parsed)
  }

  // Focus input when record mode changes
  $effect(() => {
    if (isRecordMode && inputRef) {
      inputRef.focus()
    }
  })
</script>

<div
  class="keybindings-search flex items-center gap-2 px-4 py-2"
  style="
    background: var(--neo-input-background);
    border-bottom: 1px solid var(--neo-editorGroupHeader-tabsBorder);
  "
>
  <!-- Search icon -->
  <Search class="w-4 h-4 opacity-50" style="color: var(--neo-input-foreground);" />

  <!-- Input -->
  {#if isRecordMode}
    <!-- Record mode input -->
    <input
      bind:this={inputRef}
      type="text"
      readonly
      value={recordedKeys || 'Press a key combination...'}
      class="flex-1 bg-transparent border-none text-sm focus:outline-none"
      style="color: var(--neo-input-foreground);"
      onkeydown={handleKeydown}
    />
  {:else}
    <!-- Regular search input -->
    <input
      bind:this={inputRef}
      type="text"
      placeholder="Search keybindings (e.g., save, @source:user, @keybinding:ctrl+s)"
      {value}
      oninput={(e) => onchange(e.currentTarget.value)}
      class="flex-1 bg-transparent border-none text-sm focus:outline-none"
      style="color: var(--neo-input-foreground);"
    />
  {/if}

  <!-- Record button -->
  <button
    class="flex items-center gap-1 px-2 py-1 rounded text-xs"
    style="
      background: {isRecordMode ? 'var(--neo-button-background)' : 'var(--neo-button-secondaryBackground)'};
      color: {isRecordMode ? 'var(--neo-button-foreground)' : 'var(--neo-button-secondaryForeground)'};
    "
    onclick={ontogglerecord}
    title="Record keyboard shortcut to search"
  >
    <Keyboard class="w-3 h-3" />
    <span>{isRecordMode ? 'Recording...' : 'Record Keys'}</span>
  </button>
</div>

<style>
  .keybindings-search input::placeholder {
    color: var(--neo-input-placeholderForeground);
    opacity: 0.6;
  }
</style>

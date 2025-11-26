<script lang="ts">
  import { onMount } from 'svelte'
  import { AlertTriangle } from '@lucide/svelte'
  import type { IKeybindingItemEntry } from '$lib/keybindings/editorModel'
  import { keyboardEventToKeybinding, formatKeybindingForDisplay, normalizeKeybinding } from '$lib/keybindings/parser'
  import { getCommandRegistry } from '$lib/commands/registry'
  import { formatCommandTitle } from '$lib/commands/types'

  interface Props {
    commandId: string | null
    existingKey: string | null
    onconfirm: (key: string, when?: string) => void
    oncancel: () => void
    findConflicts: (key: string) => IKeybindingItemEntry[]
  }

  let { commandId, existingKey, onconfirm, oncancel, findConflicts }: Props = $props()

  // State
  let capturedKey = $state<string>('')
  let capturedKeyDisplay = $state<string>('')
  let whenExpression = $state<string>('')
  let conflicts = $state<IKeybindingItemEntry[]>([])
  let inputRef: HTMLInputElement | undefined = $state()

  // Get command label
  let commandLabel = $derived(() => {
    if (!commandId) return ''
    const registry = getCommandRegistry()
    const meta = registry.getAvailableMeta().find((m) => m.id === commandId)
    return meta ? formatCommandTitle(meta) : commandId
  })

  // Handle keyboard capture
  function handleKeydown(event: KeyboardEvent) {
    event.preventDefault()
    event.stopPropagation()

    // Escape cancels
    if (event.key === 'Escape') {
      oncancel()
      return
    }

    // Enter confirms (if we have a key)
    if (event.key === 'Enter' && capturedKey) {
      onconfirm(capturedKey, whenExpression || undefined)
      return
    }

    // Ignore lone modifier keys
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) {
      return
    }

    const parsed = keyboardEventToKeybinding(event)
    const normalized = normalizeKeybinding(parsed)
    const display = formatKeybindingForDisplay(normalized)

    capturedKey = normalized
    capturedKeyDisplay = display

    // Find conflicts
    conflicts = findConflicts(normalized)
  }

  // Focus input on mount
  onMount(() => {
    if (inputRef) {
      inputRef.focus()
    }

    // If editing existing, show the current keybinding
    if (existingKey) {
      capturedKey = existingKey
      capturedKeyDisplay = formatKeybindingForDisplay(existingKey)
    }
  })

  // Handle backdrop click
  function handleBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      oncancel()
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div
  class="dialog-backdrop"
  onclick={handleBackdropClick}
>
  <div
    class="dialog-content"
    style="
      background: var(--neo-editor-background);
      border: 1px solid var(--neo-widget-border);
    "
    role="dialog"
    aria-modal="true"
    aria-label="Define Keybinding"
  >
    <!-- Header -->
    <div
      class="dialog-header"
      style="
        border-bottom: 1px solid var(--neo-widget-border);
        background: var(--neo-editorWidget-background);
      "
    >
      <h2 class="text-sm font-medium" style="color: var(--neo-foreground);">
        {existingKey ? 'Change' : 'Define'} Keybinding for "{commandLabel()}"
      </h2>
    </div>

    <!-- Body -->
    <div class="dialog-body">
      <p class="text-xs mb-4" style="color: var(--neo-descriptionForeground);">
        Press the desired key combination and then press Enter to confirm.
      </p>

      <!-- Key capture input -->
      <div
        class="key-capture"
        style="
          background: var(--neo-input-background);
          border: 2px solid var(--neo-focusBorder);
        "
      >
        <input
          bind:this={inputRef}
          type="text"
          readonly
          value={capturedKeyDisplay || 'Press a key combination...'}
          class="w-full bg-transparent border-none text-center text-lg font-mono focus:outline-none"
          style="color: var(--neo-input-foreground);"
          onkeydown={handleKeydown}
        />
      </div>

      <!-- Conflicts warning -->
      {#if conflicts.length > 0}
        <div
          class="conflicts-warning mt-4 p-3 rounded"
          style="
            background: var(--neo-inputValidation-warningBackground, #352a05);
            border: 1px solid var(--neo-inputValidation-warningBorder, #b89500);
          "
        >
          <div class="flex items-center gap-2 mb-2">
            <AlertTriangle class="w-4 h-4 text-yellow-500" />
            <span class="text-sm font-medium text-yellow-400">
              {conflicts.length} existing {conflicts.length === 1 ? 'binding' : 'bindings'}
            </span>
          </div>
          <ul class="text-xs space-y-1" style="color: var(--neo-descriptionForeground);">
            {#each conflicts.slice(0, 5) as conflict}
              <li class="truncate">
                {conflict.commandLabel}
                {#if conflict.when}
                  <span class="opacity-50">(when: {conflict.when})</span>
                {/if}
              </li>
            {/each}
            {#if conflicts.length > 5}
              <li class="opacity-50">...and {conflicts.length - 5} more</li>
            {/if}
          </ul>
        </div>
      {/if}

      <!-- When expression (optional) -->
      <div class="mt-4">
        <label class="block text-xs mb-1" style="color: var(--neo-descriptionForeground);">
          When Expression (optional)
        </label>
        <input
          type="text"
          placeholder="e.g., editorTextFocus && !editorReadonly"
          bind:value={whenExpression}
          class="w-full px-3 py-2 rounded text-sm"
          style="
            background: var(--neo-input-background);
            border: 1px solid var(--neo-input-border, #3c3c3c);
            color: var(--neo-input-foreground);
          "
        />
      </div>
    </div>

    <!-- Footer -->
    <div
      class="dialog-footer"
      style="
        border-top: 1px solid var(--neo-widget-border);
        background: var(--neo-editorWidget-background);
      "
    >
      <button
        class="btn-secondary"
        onclick={oncancel}
      >
        Cancel
      </button>
      <button
        class="btn-primary"
        disabled={!capturedKey}
        onclick={() => {
          if (capturedKey) {
            onconfirm(capturedKey, whenExpression || undefined)
          }
        }}
      >
        Confirm
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 100px;
    background: rgba(0, 0, 0, 0.5);
  }

  .dialog-content {
    width: 480px;
    max-width: calc(100vw - 40px);
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
    overflow: hidden;
  }

  .dialog-header {
    padding: 12px 16px;
  }

  .dialog-body {
    padding: 16px;
  }

  .dialog-footer {
    padding: 12px 16px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .key-capture {
    padding: 16px;
    border-radius: 6px;
    text-align: center;
  }

  .btn-primary {
    padding: 6px 14px;
    border-radius: 4px;
    font-size: 13px;
    background: var(--neo-button-background);
    color: var(--neo-button-foreground);
    border: none;
    cursor: pointer;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--neo-button-hoverBackground);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    padding: 6px 14px;
    border-radius: 4px;
    font-size: 13px;
    background: var(--neo-button-secondaryBackground);
    color: var(--neo-button-secondaryForeground);
    border: none;
    cursor: pointer;
  }

  .btn-secondary:hover {
    background: var(--neo-button-secondaryHoverBackground);
  }
</style>

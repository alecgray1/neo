<script lang="ts">
  import { X, Plus } from '@lucide/svelte'
  import { Input } from '$lib/components/ui/input'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import type { ISettingSchema } from '$lib/settings/types'

  interface Props {
    value: Record<string, unknown>
    schema: ISettingSchema
    onupdate: (value: Record<string, unknown>) => void
  }

  let { value, schema, onupdate }: Props = $props()

  let newKey = $state('')
  let newValue = $state('')

  // Check if this is a boolean pattern object (like files.exclude)
  function isBooleanPatternObject(): boolean {
    if (typeof schema.additionalProperties === 'object') {
      return schema.additionalProperties.type === 'boolean'
    }
    return false
  }

  const isPatternMode = isBooleanPatternObject()

  function handleTogglePattern(key: string, enabled: boolean) {
    const newObj = { ...value, [key]: enabled }
    onupdate(newObj)
  }

  function handleRemoveKey(key: string) {
    const newObj = { ...value }
    delete newObj[key]
    onupdate(newObj)
  }

  function handleAddPattern() {
    if (!newKey.trim()) return
    const newObj = { ...value, [newKey.trim()]: true }
    onupdate(newObj)
    newKey = ''
  }

  function handleAddKeyValue() {
    if (!newKey.trim()) return

    let parsedValue: unknown = newValue

    // Try to parse as JSON for complex values
    if (newValue.startsWith('{') || newValue.startsWith('[') || newValue === 'true' || newValue === 'false' || !isNaN(Number(newValue))) {
      try {
        parsedValue = JSON.parse(newValue)
      } catch {
        // Keep as string
      }
    }

    const newObj = { ...value, [newKey.trim()]: parsedValue }
    onupdate(newObj)
    newKey = ''
    newValue = ''
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault()
      if (isPatternMode) {
        handleAddPattern()
      } else {
        handleAddKeyValue()
      }
    }
  }

  // Get sorted entries
  function getEntries(): [string, unknown][] {
    return Object.entries(value).sort((a, b) => a[0].localeCompare(b[0]))
  }
</script>

<div class="object-control">
  {#if isPatternMode}
    <!-- Pattern mode (like files.exclude) -->
    <div class="patterns-list">
      {#each getEntries() as [pattern, enabled] (pattern)}
        <div class="pattern-row">
          <Checkbox
            checked={Boolean(enabled)}
            onCheckedChange={(checked) => handleTogglePattern(pattern, Boolean(checked))}
          />
          <span class="pattern-text" class:disabled={!enabled}>{pattern}</span>
          <button
            class="remove-btn"
            onclick={() => handleRemoveKey(pattern)}
            title="Remove pattern"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      {/each}
    </div>

    <div class="add-row">
      <Input
        type="text"
        placeholder="Add pattern (e.g., **/*.log)"
        bind:value={newKey}
        onkeydown={handleKeyDown}
        class="add-input"
      />
      <Button
        variant="secondary"
        size="sm"
        onclick={handleAddPattern}
        disabled={!newKey.trim()}
      >
        <Plus class="w-4 h-4 mr-1" />
        Add
      </Button>
    </div>
  {:else}
    <!-- Key-value mode -->
    <div class="kv-list">
      {#each getEntries() as [key, val] (key)}
        <div class="kv-row">
          <span class="kv-key">{key}</span>
          <span class="kv-sep">:</span>
          <span class="kv-value">{JSON.stringify(val)}</span>
          <button
            class="remove-btn"
            onclick={() => handleRemoveKey(key)}
            title="Remove entry"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      {/each}
    </div>

    <div class="add-row kv-add">
      <Input
        type="text"
        placeholder="Key"
        bind:value={newKey}
        class="add-input key-input"
      />
      <Input
        type="text"
        placeholder="Value"
        bind:value={newValue}
        onkeydown={handleKeyDown}
        class="add-input value-input"
      />
      <Button
        variant="secondary"
        size="sm"
        onclick={handleAddKeyValue}
        disabled={!newKey.trim()}
      >
        <Plus class="w-4 h-4" />
      </Button>
    </div>
  {/if}
</div>

<style>
  .object-control {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 500px;
  }

  .patterns-list,
  .kv-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .pattern-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--neo-foreground) 5%, transparent);
    border-radius: 4px;
  }

  .pattern-text {
    flex: 1;
    font-size: 13px;
    font-family: monospace;
    color: var(--neo-foreground);
  }

  .pattern-text.disabled {
    text-decoration: line-through;
    opacity: 0.5;
  }

  .kv-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--neo-foreground) 5%, transparent);
    border-radius: 4px;
  }

  .kv-key {
    font-size: 13px;
    font-family: monospace;
    color: var(--neo-primary);
    font-weight: 500;
  }

  .kv-sep {
    color: var(--neo-foreground);
    opacity: 0.5;
  }

  .kv-value {
    flex: 1;
    font-size: 13px;
    font-family: monospace;
    color: var(--neo-foreground);
  }

  .remove-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    background: none;
    border: none;
    color: var(--neo-foreground);
    opacity: 0.5;
    cursor: pointer;
    border-radius: 4px;
  }

  .remove-btn:hover {
    opacity: 1;
    background: color-mix(in srgb, var(--neo-error) 20%, transparent);
    color: var(--neo-error);
  }

  .add-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .add-row.kv-add {
    gap: 4px;
  }

  .object-control :global(.add-input) {
    flex: 1;
    background: var(--neo-input-background);
    border: 1px solid var(--neo-input-border);
    color: var(--neo-foreground);
  }

  .object-control :global(.key-input) {
    max-width: 150px;
  }
</style>

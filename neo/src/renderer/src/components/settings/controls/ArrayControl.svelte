<script lang="ts">
  import { X, Plus } from '@lucide/svelte'
  import { Input } from '$lib/components/ui/input'
  import { Button } from '$lib/components/ui/button'
  import type { ISettingSchema } from '$lib/settings/types'

  interface Props {
    value: unknown[]
    itemSchema?: ISettingSchema
    onupdate: (value: unknown[]) => void
  }

  let { value, itemSchema, onupdate }: Props = $props()

  let newItemValue = $state('')

  function handleRemove(index: number) {
    const newValue = [...value]
    newValue.splice(index, 1)
    onupdate(newValue)
  }

  function handleAdd() {
    if (!newItemValue.trim()) return

    let itemValue: unknown = newItemValue.trim()

    // Convert to appropriate type based on schema
    if (itemSchema) {
      const itemType = Array.isArray(itemSchema.type) ? itemSchema.type[0] : itemSchema.type
      if (itemType === 'number' || itemType === 'integer') {
        itemValue = parseFloat(newItemValue)
        if (isNaN(itemValue as number)) return
      } else if (itemType === 'boolean') {
        itemValue = newItemValue.toLowerCase() === 'true'
      }
    }

    onupdate([...value, itemValue])
    newItemValue = ''
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault()
      handleAdd()
    }
  }
</script>

<div class="array-control">
  <!-- Existing items -->
  <div class="items-list">
    {#each value as item, index (index)}
      <div class="item-row">
        <span class="item-value">{String(item)}</span>
        <button
          class="remove-btn"
          onclick={() => handleRemove(index)}
          title="Remove item"
        >
          <X class="w-3.5 h-3.5" />
        </button>
      </div>
    {/each}
  </div>

  <!-- Add new item -->
  <div class="add-row">
    <Input
      type="text"
      placeholder="Add item..."
      bind:value={newItemValue}
      onkeydown={handleKeyDown}
      class="add-input"
    />
    <Button
      variant="secondary"
      size="sm"
      onclick={handleAdd}
      disabled={!newItemValue.trim()}
    >
      <Plus class="w-4 h-4 mr-1" />
      Add
    </Button>
  </div>
</div>

<style>
  .array-control {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 400px;
  }

  .items-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .item-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--neo-foreground) 5%, transparent);
    border-radius: 4px;
  }

  .item-value {
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

  .array-control :global(.add-input) {
    flex: 1;
    background: var(--neo-input-background);
    border: 1px solid var(--neo-input-border);
    color: var(--neo-foreground);
  }
</style>

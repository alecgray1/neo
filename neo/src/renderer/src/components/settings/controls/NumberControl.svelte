<script lang="ts">
  import { Input } from '$lib/components/ui/input'

  interface Props {
    value: number
    minimum?: number
    maximum?: number
    isInteger?: boolean
    onupdate: (value: number) => void
  }

  let { value, minimum, maximum, isInteger = false, onupdate }: Props = $props()

  let error = $state<string | null>(null)
  let localValue = $state(String(value ?? 0))

  // Sync with prop changes
  $effect(() => {
    localValue = String(value ?? 0)
  })

  function validate(val: string): number | null {
    const num = isInteger ? parseInt(val, 10) : parseFloat(val)

    if (isNaN(num)) {
      error = 'Please enter a valid number'
      return null
    }

    if (minimum !== undefined && num < minimum) {
      error = `Value must be at least ${minimum}`
      return null
    }

    if (maximum !== undefined && num > maximum) {
      error = `Value must be at most ${maximum}`
      return null
    }

    error = null
    return num
  }

  function handleChange(e: Event) {
    const target = e.target as HTMLInputElement
    localValue = target.value
    const num = validate(localValue)
    if (num !== null) {
      onupdate(num)
    }
  }

  function handleBlur() {
    const num = validate(localValue)
    if (num !== null) {
      localValue = String(num)
      onupdate(num)
    }
  }
</script>

<div class="number-control">
  <div class="input-row">
    <Input
      type="number"
      value={localValue}
      min={minimum}
      max={maximum}
      step={isInteger ? 1 : 'any'}
      oninput={handleChange}
      onblur={handleBlur}
      class="number-input"
    />
    {#if minimum !== undefined || maximum !== undefined}
      <span class="range-hint">
        {#if minimum !== undefined && maximum !== undefined}
          ({minimum} - {maximum})
        {:else if minimum !== undefined}
          (min: {minimum})
        {:else if maximum !== undefined}
          (max: {maximum})
        {/if}
      </span>
    {/if}
  </div>
  {#if error}
    <span class="error-message">{error}</span>
  {/if}
</div>

<style>
  .number-control {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .input-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .number-control :global(.number-input) {
    width: 100px;
    height: 26px;
    font-size: 13px;
    background: var(--neo-input-background);
    border: 1px solid var(--neo-input-border, transparent);
    color: var(--neo-foreground);
  }

  .number-control :global(.number-input:focus) {
    border-color: var(--neo-focusBorder, #0078d4);
    outline: none;
  }

  .range-hint {
    font-size: 12px;
    color: var(--neo-foreground);
    opacity: 0.6;
  }

  .error-message {
    font-size: 12px;
    color: var(--neo-editorError-foreground, #f14c4c);
  }
</style>

<script lang="ts">
  import { Input } from '$lib/components/ui/input'
  import { Textarea } from '$lib/components/ui/textarea'

  interface Props {
    value: string
    multiline?: boolean
    pattern?: string
    onupdate: (value: string) => void
  }

  let { value, multiline = false, pattern, onupdate }: Props = $props()

  let error = $state<string | null>(null)
  let localValue = $state(value ?? '')

  // Sync with prop changes
  $effect(() => {
    localValue = value ?? ''
  })

  function validate(val: string): boolean {
    if (pattern) {
      try {
        const regex = new RegExp(pattern)
        if (!regex.test(val)) {
          error = 'Value does not match required pattern'
          return false
        }
      } catch {
        // Invalid pattern, skip validation
      }
    }
    error = null
    return true
  }

  function handleChange(e: Event) {
    const target = e.target as HTMLInputElement | HTMLTextAreaElement
    localValue = target.value
    if (validate(localValue)) {
      onupdate(localValue)
    }
  }

  function handleBlur() {
    if (validate(localValue)) {
      onupdate(localValue)
    }
  }
</script>

<div class="string-control">
  {#if multiline}
    <Textarea
      value={localValue}
      oninput={handleChange}
      onblur={handleBlur}
      class="string-textarea"
      rows={4}
    />
  {:else}
    <Input
      type="text"
      value={localValue}
      oninput={handleChange}
      onblur={handleBlur}
      class="string-input"
    />
  {/if}
  {#if error}
    <span class="error-message">{error}</span>
  {/if}
</div>

<style>
  .string-control {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .string-control :global(.string-input) {
    width: 300px;
    height: 26px;
    font-size: 13px;
    background: var(--neo-input-background);
    border: 1px solid var(--neo-input-border, transparent);
    color: var(--neo-foreground);
  }

  .string-control :global(.string-textarea) {
    width: 100%;
    max-width: 500px;
    font-size: 13px;
    background: var(--neo-input-background);
    border: 1px solid var(--neo-input-border, transparent);
    color: var(--neo-foreground);
    font-family: var(--neo-font-family-monospace, monospace);
  }

  .string-control :global(.string-input:focus),
  .string-control :global(.string-textarea:focus) {
    border-color: var(--neo-focusBorder, #0078d4);
    outline: none;
  }

  .error-message {
    font-size: 12px;
    color: var(--neo-editorError-foreground, #f14c4c);
  }
</style>

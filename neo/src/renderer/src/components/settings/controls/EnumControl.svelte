<script lang="ts">
  import * as Select from '$lib/components/ui/select'

  interface Props {
    value: string
    options: unknown[]
    descriptions?: string[]
    labels?: string[]
    onupdate: (value: string) => void
  }

  let { value, options, descriptions, labels, onupdate }: Props = $props()

  function getLabel(option: unknown, index: number): string {
    if (labels && labels[index]) {
      return labels[index]
    }
    return String(option)
  }

  function getDescription(index: number): string | undefined {
    if (descriptions && descriptions[index]) {
      return descriptions[index]
    }
    return undefined
  }

  // Get current selected index for description
  $effect(() => {
    const idx = options.findIndex((o) => String(o) === String(value))
    if (idx >= 0) {
      currentDescription = getDescription(idx)
    } else {
      currentDescription = undefined
    }
  })

  let currentDescription = $state<string | undefined>(undefined)
</script>

<div class="enum-control">
  <Select.Root
    type="single"
    value={{ value: String(value), label: getLabel(value, options.findIndex((o) => String(o) === String(value))) }}
    onValueChange={(v) => {
      if (v) {
        onupdate(v.value)
      }
    }}
  >
    <Select.Trigger class="enum-trigger">
      {getLabel(value, options.findIndex((o) => String(o) === String(value)))}
    </Select.Trigger>
    <Select.Content>
      {#each options as option, index (String(option))}
        <Select.Item value={String(option)} label={getLabel(option, index)}>
          {getLabel(option, index)}
        </Select.Item>
      {/each}
    </Select.Content>
  </Select.Root>
  {#if currentDescription}
    <span class="option-description">{currentDescription}</span>
  {/if}
</div>

<style>
  .enum-control {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .enum-control :global(.enum-trigger) {
    width: 250px;
    height: 26px;
    font-size: 13px;
    background: var(--neo-dropdown-background, var(--neo-input-background));
    border: 1px solid var(--neo-dropdown-border, var(--neo-input-border, transparent));
    color: var(--neo-foreground);
  }

  .enum-control :global(.enum-trigger:focus) {
    border-color: var(--neo-focusBorder, #0078d4);
    outline: none;
  }

  .option-description {
    font-size: 12px;
    color: var(--neo-foreground);
    opacity: 0.7;
  }
</style>

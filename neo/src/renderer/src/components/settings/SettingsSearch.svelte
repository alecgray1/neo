<script lang="ts">
  import { Search } from '@lucide/svelte'
  import { Input } from '$lib/components/ui/input'

  interface Props {
    value: string
    onchange: (value: string) => void
  }

  let { value, onchange }: Props = $props()
</script>

<div class="settings-search">
  <Search class="search-icon" />
  <Input
    type="text"
    placeholder="Search settings"
    {value}
    oninput={(e) => onchange(e.currentTarget.value)}
    class="search-input"
  />
</div>

<style>
  .settings-search {
    position: relative;
    display: flex;
    align-items: center;
  }

  .settings-search :global(.search-icon) {
    position: absolute;
    left: 8px;
    width: 16px;
    height: 16px;
    color: var(--neo-foreground);
    opacity: 0.5;
    pointer-events: none;
    z-index: 1;
  }

  .settings-search :global(.search-input) {
    padding-left: 32px;
    height: 26px;
    font-size: 13px;
    background: var(--neo-input-background);
    border: 1px solid var(--neo-input-border, transparent);
    color: var(--neo-foreground);
  }

  .settings-search :global(.search-input:focus) {
    border-color: var(--neo-focusBorder, #0078d4);
    outline: none;
  }

  .settings-search :global(.search-input::placeholder) {
    color: var(--neo-input-placeholderForeground, var(--neo-foreground));
    opacity: 0.5;
  }
</style>

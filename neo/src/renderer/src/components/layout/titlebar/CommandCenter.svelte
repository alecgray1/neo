<script lang="ts">
  import { Search } from '@lucide/svelte'

  function openCommandPalette() {
    // TODO: Implement command palette
    console.log('Open command palette')
  }
</script>

<button
  class="command-center flex items-center gap-2 px-4 h-[22px] rounded-md text-xs transition-colors"
  style="
    background: var(--neo-commandCenter-background);
    border: 1px solid var(--neo-commandCenter-border);
    color: var(--neo-commandCenter-foreground);
    width: clamp(200px, 38vw, 600px);
    -webkit-app-region: no-drag;
  "
  onclick={openCommandPalette}
>
  <Search class="w-3.5 h-3.5 opacity-60" />
  <span class="opacity-60 truncate">Search or type a command...</span>
  <span class="ml-auto opacity-40 text-[10px] hidden sm:inline">Ctrl+P</span>
</button>

<style>
  .command-center:hover {
    background: var(--neo-commandCenter-activeBackground);
    border-color: var(--neo-commandCenter-activeBorder);
  }

  .command-center:focus {
    outline: none;
    border-color: var(--neo-commandCenter-activeBorder);
  }
</style>

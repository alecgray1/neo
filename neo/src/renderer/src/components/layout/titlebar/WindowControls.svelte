<script lang="ts">
  import { Minus, Square, X, Copy } from '@lucide/svelte'
  import { onMount } from 'svelte'

  let isMaximized = $state(false)

  onMount(async () => {
    isMaximized = await window.windowAPI.isMaximized()
    window.windowAPI.onMaximizedChange((maximized) => {
      isMaximized = maximized
    })
  })

  function minimize() {
    window.windowAPI.minimize()
  }

  function maximize() {
    window.windowAPI.maximize()
  }

  function close() {
    window.windowAPI.close()
  }
</script>

<div class="window-controls flex h-full" style="-webkit-app-region: no-drag;">
  <!-- Minimize -->
  <button
    class="window-control w-[46px] h-full flex items-center justify-center hover:bg-white/10 transition-colors"
    onclick={minimize}
    aria-label="Minimize"
  >
    <Minus class="w-4 h-4" />
  </button>

  <!-- Maximize/Restore -->
  <button
    class="window-control w-[46px] h-full flex items-center justify-center hover:bg-white/10 transition-colors"
    onclick={maximize}
    aria-label={isMaximized ? 'Restore' : 'Maximize'}
  >
    {#if isMaximized}
      <Copy class="w-3.5 h-3.5 rotate-180" />
    {:else}
      <Square class="w-3 h-3" />
    {/if}
  </button>

  <!-- Close -->
  <button
    class="window-control window-close w-[46px] h-full flex items-center justify-center hover:bg-red-600 transition-colors"
    onclick={close}
    aria-label="Close"
  >
    <X class="w-4 h-4" />
  </button>
</div>

<style>
  .window-control {
    color: var(--neo-titleBar-activeForeground);
  }
</style>

<script lang="ts">
  import { layoutStore } from '$lib/stores/layout.svelte'
  import { PanelLeft, PanelRight, PanelBottom } from '@lucide/svelte'
  import * as Tooltip from '$lib/components/ui/tooltip'
</script>

<div class="flex items-center gap-0.5 px-2" style="-webkit-app-region: no-drag;">
  <!-- Toggle Primary Sidebar -->
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          class="p-1.5 rounded transition-colors"
          class:opacity-100={layoutStore.state.primarySidebarVisible}
          class:opacity-50={!layoutStore.state.primarySidebarVisible}
          style="color: var(--neo-titleBar-activeForeground);"
          onclick={() => layoutStore.togglePrimarySidebar()}
        >
          <PanelLeft class="w-4 h-4" />
        </button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="bottom">Toggle Primary Sidebar (Ctrl+B)</Tooltip.Content>
  </Tooltip.Root>

  <!-- Toggle Panel -->
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          class="p-1.5 rounded transition-colors"
          class:opacity-100={layoutStore.state.panelVisible}
          class:opacity-50={!layoutStore.state.panelVisible}
          style="color: var(--neo-titleBar-activeForeground);"
          onclick={() => layoutStore.togglePanel()}
        >
          <PanelBottom class="w-4 h-4" />
        </button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="bottom">Toggle Panel (Ctrl+J)</Tooltip.Content>
  </Tooltip.Root>

  <!-- Toggle Secondary Sidebar -->
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <button
          {...props}
          class="p-1.5 rounded transition-colors"
          class:opacity-100={layoutStore.state.auxiliaryBarVisible}
          class:opacity-50={!layoutStore.state.auxiliaryBarVisible}
          style="color: var(--neo-titleBar-activeForeground);"
          onclick={() => layoutStore.toggleAuxiliaryBar()}
        >
          <PanelRight class="w-4 h-4" />
        </button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="bottom">Toggle Secondary Sidebar (Ctrl+Alt+B)</Tooltip.Content>
  </Tooltip.Root>
</div>

<style>
  button:hover {
    background: var(--neo-toolbar-hoverBackground);
  }
</style>

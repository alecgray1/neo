<script lang="ts">
  import { layoutStore, type PanelPosition } from '$lib/stores/layout.svelte'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
  import {
    Terminal,
    AlertCircle,
    FileText,
    Bug,
    Ellipsis,
    X,
    Maximize2,
    ArrowDown,
    ArrowUp,
    ArrowLeft,
    ArrowRight
  } from '@lucide/svelte'

  const tabs = [
    { id: 'problems', label: 'PROBLEMS', icon: AlertCircle, badge: 0 },
    { id: 'output', label: 'OUTPUT', icon: FileText },
    { id: 'debug', label: 'DEBUG CONSOLE', icon: Bug },
    { id: 'terminal', label: 'TERMINAL', icon: Terminal }
  ]

  const positions: { value: PanelPosition; label: string; icon: typeof ArrowDown }[] = [
    { value: 'bottom', label: 'Move Panel to Bottom', icon: ArrowDown },
    { value: 'top', label: 'Move Panel to Top', icon: ArrowUp },
    { value: 'left', label: 'Move Panel to Left', icon: ArrowLeft },
    { value: 'right', label: 'Move Panel to Right', icon: ArrowRight }
  ]
</script>

<div
  class="panel h-full flex flex-col"
  style="background: var(--neo-panel-background); border-color: var(--neo-panel-border);"
>
  <!-- Panel Header -->
  <div
    class="panel-header flex items-center justify-between h-[35px] px-2 border-b"
    style="border-color: var(--neo-panel-border);"
  >
    <!-- Tabs -->
    <div class="tabs flex items-center gap-1">
      {#each tabs as tab}
        <button
          class="tab-trigger flex items-center gap-1.5 px-2 py-1 text-[11px] font-medium uppercase tracking-wide transition-colors"
          class:active={layoutStore.state.activePanelTab === tab.id}
          onclick={() => layoutStore.setActivePanelTab(tab.id)}
        >
          <span>{tab.label}</span>
          {#if 'badge' in tab && tab.badge !== undefined && tab.badge > 0}
            <span
              class="badge px-1.5 py-0.5 text-[10px] rounded-full"
              style="background: var(--neo-primary); color: var(--neo-primaryForeground);"
            >
              {tab.badge}
            </span>
          {/if}
        </button>
      {/each}
    </div>

    <!-- Actions -->
    <div class="actions flex items-center gap-1">
      <button
        class="action-btn p-1 rounded hover:bg-[var(--neo-list-hoverBackground)]"
        onclick={() => layoutStore.togglePanel()}
        title="Maximize Panel"
      >
        <Maximize2 class="w-4 h-4" style="color: var(--neo-foreground); opacity: 0.7;" />
      </button>

      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <button
            class="action-btn p-1 rounded hover:bg-[var(--neo-list-hoverBackground)]"
            title="Panel Options"
          >
            <Ellipsis class="w-4 h-4" style="color: var(--neo-foreground); opacity: 0.7;" />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end">
          <DropdownMenu.Label>Panel Position</DropdownMenu.Label>
          <DropdownMenu.Separator />
          {#each positions as pos}
            <DropdownMenu.Item
              onclick={() => layoutStore.setPanelPosition(pos.value)}
              class={layoutStore.state.panelPosition === pos.value ? 'bg-accent' : ''}
            >
              <svelte:component this={pos.icon} class="w-4 h-4 mr-2" />
              {pos.label}
            </DropdownMenu.Item>
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Root>

      <button
        class="action-btn p-1 rounded hover:bg-[var(--neo-list-hoverBackground)]"
        onclick={() => layoutStore.togglePanel()}
        title="Close Panel"
      >
        <X class="w-4 h-4" style="color: var(--neo-foreground); opacity: 0.7;" />
      </button>
    </div>
  </div>

  <!-- Panel Content -->
    <ScrollArea class="flex-1 h-full">
      <div class="panel-content p-2 text-sm" style="color: var(--neo-foreground);">
        {#if layoutStore.state.activePanelTab === 'terminal'}
          <div class="terminal font-mono text-xs">
            <div class="opacity-60">Welcome to Neo Terminal</div>
            <div class="flex items-center gap-2 mt-2">
              <span class="text-green-500">$</span>
              <span class="opacity-70">_</span>
            </div>
          </div>
        {:else if layoutStore.state.activePanelTab === 'problems'}
          <div class="opacity-60">No problems detected in workspace.</div>
        {:else if layoutStore.state.activePanelTab === 'output'}
          <div class="opacity-60">No output to display.</div>
        {:else if layoutStore.state.activePanelTab === 'debug'}
          <div class="opacity-60">Debug console ready.</div>
        {/if}
      </div>
  </ScrollArea>
</div>

<style>
  .tab-trigger {
    color: var(--neo-panelTitle-inactiveForeground);
    border-bottom: 1px solid transparent;
  }

  .tab-trigger:hover {
    color: var(--neo-panelTitle-activeForeground);
  }

  .tab-trigger.active {
    color: var(--neo-panelTitle-activeForeground);
    border-bottom-color: var(--neo-panelTitle-activeBorder);
  }
</style>

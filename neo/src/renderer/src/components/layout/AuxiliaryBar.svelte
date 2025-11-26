<script lang="ts">
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'
  import { FileCode, History, GitBranch } from '@lucide/svelte'

  let activeView = $state<'outline' | 'timeline'>('outline')
</script>

<div
  class="auxiliary-bar h-full flex flex-col"
  style="background: var(--neo-sideBar-background); color: var(--neo-sideBar-foreground);"
>
  <!-- Header with view tabs -->
  <div
    class="header flex items-center h-[35px] px-2 border-b shrink-0"
    style="border-color: var(--neo-sideBarSectionHeader-border); background: var(--neo-sideBarSectionHeader-background);"
  >
    <button
      class="tab-button px-2 py-1 text-xs rounded-sm transition-colors"
      class:active={activeView === 'outline'}
      onclick={() => (activeView = 'outline')}
    >
      <span class="flex items-center gap-1.5">
        <FileCode class="w-3.5 h-3.5" />
        Outline
      </span>
    </button>
    <button
      class="tab-button px-2 py-1 text-xs rounded-sm transition-colors"
      class:active={activeView === 'timeline'}
      onclick={() => (activeView = 'timeline')}
    >
      <span class="flex items-center gap-1.5">
        <History class="w-3.5 h-3.5" />
        Timeline
      </span>
    </button>
  </div>

  <!-- Content -->
  <ScrollArea class="flex-1 h-full">
    {#if activeView === 'outline'}
      <div class="p-4 text-sm opacity-60">No symbols found in editor.</div>
    {:else if activeView === 'timeline'}
      <div class="p-4 text-sm opacity-60">
        <div class="flex items-center gap-2 mb-2">
          <GitBranch class="w-4 h-4" />
          <span>Timeline</span>
        </div>
        <p class="text-xs opacity-80">No timeline entries available.</p>
      </div>
    {/if}
  </ScrollArea>
</div>

<style>
  .tab-button {
    color: var(--neo-sideBar-foreground);
    opacity: 0.7;
  }

  .tab-button:hover {
    opacity: 1;
    background: var(--neo-list-hoverBackground);
  }

  .tab-button.active {
    opacity: 1;
    background: var(--neo-list-activeSelectionBackground);
    color: var(--neo-list-activeSelectionForeground);
  }
</style>

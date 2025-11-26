<script lang="ts">
  import { layoutStore } from '$lib/stores/layout.svelte'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { Files, Search, GitBranch, Package, Settings, Bug } from '@lucide/svelte'

  const activities = [
    { id: 'explorer', icon: Files, label: 'Explorer' },
    { id: 'search', icon: Search, label: 'Search' },
    { id: 'git', icon: GitBranch, label: 'Source Control' },
    { id: 'debug', icon: Bug, label: 'Run and Debug' },
    { id: 'extensions', icon: Package, label: 'Extensions' }
  ]
</script>

<div
  class="activity-bar flex flex-col items-center w-12 py-1"
  style="background: var(--neo-activityBar-background);"
>
  {#each activities as activity}
    <Tooltip.Root>
      <Tooltip.Trigger>
        <button
          class="activity-item w-12 h-12 flex items-center justify-center relative transition-colors"
          class:active={layoutStore.state.activeActivityItem === activity.id}
          onclick={() => layoutStore.setActiveActivityItem(activity.id)}
        >
          <svelte:component this={activity.icon} class="w-6 h-6" />
          {#if layoutStore.state.activeActivityItem === activity.id && layoutStore.state.primarySidebarVisible}
            <div
              class="absolute left-0 top-1 bottom-1 w-0.5"
              style="background: var(--neo-activityBar-foreground);"
            ></div>
          {/if}
        </button>
      </Tooltip.Trigger>
      <Tooltip.Content side="right">
        <p>{activity.label}</p>
      </Tooltip.Content>
    </Tooltip.Root>
  {/each}

  <div class="flex-1"></div>

  <Tooltip.Root>
    <Tooltip.Trigger>
      <button class="activity-item w-12 h-12 flex items-center justify-center transition-colors">
        <Settings class="w-6 h-6" />
      </button>
    </Tooltip.Trigger>
    <Tooltip.Content side="right">
      <p>Settings</p>
    </Tooltip.Content>
  </Tooltip.Root>
</div>

<style>
  .activity-item {
    color: var(--neo-activityBar-inactiveForeground);
  }

  .activity-item:hover {
    color: var(--neo-activityBar-foreground);
  }

  .activity-item.active {
    color: var(--neo-activityBar-foreground);
  }
</style>

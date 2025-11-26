<script lang="ts">
  import { layoutStore } from '$lib/stores/layout.svelte'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { Files, Search, GitBranch, Package, Settings, Bug, Keyboard } from '@lucide/svelte'
  import { documentStore } from '$lib/stores/documents.svelte'
  import { editorStore } from '$lib/stores/editor.svelte'

  const activities = [
    { id: 'explorer', icon: Files, label: 'Explorer' },
    { id: 'search', icon: Search, label: 'Search' },
    { id: 'git', icon: GitBranch, label: 'Source Control' },
    { id: 'debug', icon: Bug, label: 'Run and Debug' },
    { id: 'extensions', icon: Package, label: 'Extensions' }
  ]

  async function openKeybindings() {
    const uri = 'keybindings://shortcuts'
    const doc = await documentStore.open(uri)
    if (doc) {
      editorStore.openTab({
        title: 'Keyboard Shortcuts',
        uri: doc.uri,
        isPreview: false
      })
    }
  }

  async function openSettings() {
    const uri = 'settings://preferences'
    const doc = await documentStore.open(uri)
    if (doc) {
      editorStore.openTab({
        title: 'Settings',
        uri: doc.uri,
        isPreview: false
      })
    }
  }
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

  <DropdownMenu.Root>
    <DropdownMenu.Trigger>
      <button class="activity-item w-12 h-12 flex items-center justify-center transition-colors">
        <Settings class="w-6 h-6" />
      </button>
    </DropdownMenu.Trigger>
    <DropdownMenu.Content side="right" align="end" class="w-56">
      <DropdownMenu.Item onclick={openKeybindings}>
        <Keyboard class="w-4 h-4 mr-2" />
        Keyboard Shortcuts
        <DropdownMenu.Shortcut>Ctrl+Shift+K</DropdownMenu.Shortcut>
      </DropdownMenu.Item>
      <DropdownMenu.Separator />
      <DropdownMenu.Item onclick={openSettings}>
        <Settings class="w-4 h-4 mr-2" />
        Settings
        <DropdownMenu.Shortcut>Ctrl+,</DropdownMenu.Shortcut>
      </DropdownMenu.Item>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
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

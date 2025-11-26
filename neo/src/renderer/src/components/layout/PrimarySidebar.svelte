<script lang="ts">
  import { layoutStore } from '$lib/stores/layout.svelte'
    import { ScrollArea } from "$lib/components/ui/scroll-area/index.js";
  import { ChevronRight, ChevronDown, File, Folder, FolderOpen } from '@lucide/svelte'

  // Demo file tree structure
  const fileTree = [
    {
      name: 'src',
      type: 'folder' as const,
      expanded: true,
      children: [
        {
          name: 'components',
          type: 'folder' as const,
          expanded: true,
          children: [
            { name: 'App.svelte', type: 'file' as const },
            { name: 'Button.svelte', type: 'file' as const }
          ]
        },
        {
          name: 'lib',
          type: 'folder' as const,
          expanded: false,
          children: [{ name: 'utils.ts', type: 'file' as const }]
        },
        { name: 'main.ts', type: 'file' as const }
      ]
    },
    { name: 'package.json', type: 'file' as const },
    { name: 'README.md', type: 'file' as const }
  ]

  function getSidebarTitle(): string {
    switch (layoutStore.state.activeActivityItem) {
      case 'explorer':
        return 'EXPLORER'
      case 'search':
        return 'SEARCH'
      case 'git':
        return 'SOURCE CONTROL'
      case 'debug':
        return 'RUN AND DEBUG'
      case 'extensions':
        return 'EXTENSIONS'
      default:
        return 'EXPLORER'
    }
  }
</script>

<div
  class="primary-sidebar h-full flex flex-col"
  style="background: var(--neo-sideBar-background); color: var(--neo-sideBar-foreground);"
>
  <!-- Sidebar Header -->
  <div
    class="sidebar-header px-4 py-2 text-[11px] font-medium tracking-wide"
    style="color: var(--neo-sideBarSectionHeader-foreground);"
  >
    {getSidebarTitle()}
  </div>

  <!-- Sidebar Content -->
    <ScrollArea class="flex-1 h-full">
      {#if layoutStore.state.activeActivityItem === 'explorer'}
        <!-- Section Header -->
        <button
          class="section-header w-full flex items-center gap-1 px-2 py-1 text-[11px] font-semibold"
          style="background: var(--neo-sideBarSectionHeader-background);"
        >
          <ChevronDown class="w-4 h-4" />
          <span>NEO</span>
        </button>

        <!-- File Tree -->
        <div class="file-tree px-2 py-1">
          {#each fileTree as item}
            {@render treeItem(item, 0)}
          {/each}
        </div>
      {:else if layoutStore.state.activeActivityItem === 'search'}
        <div class="p-4 text-sm opacity-60">Search functionality coming soon...</div>
      {:else if layoutStore.state.activeActivityItem === 'git'}
        <div class="p-4 text-sm opacity-60">Source control coming soon...</div>
      {:else if layoutStore.state.activeActivityItem === 'debug'}
        <div class="p-4 text-sm opacity-60">Debug functionality coming soon...</div>
      {:else if layoutStore.state.activeActivityItem === 'extensions'}
        <div class="p-4 text-sm opacity-60">Extensions coming soon...</div>
      {/if}
    </ScrollArea>
</div>

{#snippet treeItem(item: (typeof fileTree)[0], depth: number)}
  <div class="tree-item">
    <button
      class="tree-row w-full flex items-center gap-1 py-0.5 text-sm hover:bg-[var(--neo-list-hoverBackground)] rounded-sm"
      style="padding-left: {depth * 12 + 4}px;"
    >
      {#if item.type === 'folder'}
        {#if 'expanded' in item && item.expanded}
          <ChevronDown class="w-4 h-4 shrink-0" />
          <FolderOpen class="w-4 h-4 shrink-0 text-yellow-500" />
        {:else}
          <ChevronRight class="w-4 h-4 shrink-0" />
          <Folder class="w-4 h-4 shrink-0 text-yellow-500" />
        {/if}
      {:else}
        <span class="w-4"></span>
        <File class="w-4 h-4 shrink-0 opacity-70" />
      {/if}
      <span class="truncate">{item.name}</span>
    </button>

    {#if item.type === 'folder' && 'expanded' in item && item.expanded && 'children' in item}
      {#each item.children as child}
        {@render treeItem(child, depth + 1)}
      {/each}
    {/if}
  </div>
{/snippet}

<style>
  .tree-row:hover {
    background: var(--neo-list-hoverBackground);
  }
</style>

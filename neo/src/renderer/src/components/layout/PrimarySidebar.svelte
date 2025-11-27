<script lang="ts">
  import { layoutStore } from '$lib/stores/layout.svelte'
  import { editorStore } from '$lib/stores/editor.svelte'
  import { documentStore } from '$lib/stores/documents.svelte'
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'
  import { ChevronRight, ChevronDown, FileJson, Folder, FolderOpen } from '@lucide/svelte'
  import { mockFiles, type MockFile } from '../../mock-data'
  import NeoContextMenu from '../contextmenu/NeoContextMenu.svelte'
  import { MenuId } from '$lib/menus/menuId'
  import { getContextKeyService } from '$lib/services/context'

  // Build file tree from mock files
  interface TreeNode {
    name: string
    type: 'folder' | 'file'
    uri?: string
    expanded?: boolean
    children?: TreeNode[]
  }

  function buildFileTree(files: MockFile[]): TreeNode[] {
    const root: TreeNode[] = []
    const folders = new Map<string, TreeNode>()

    // Create folder structure
    for (const file of files) {
      const parts = file.path.split('/')
      const fileName = parts.pop()!
      let currentPath = ''
      let currentLevel = root

      for (const part of parts) {
        currentPath = currentPath ? `${currentPath}/${part}` : part

        if (!folders.has(currentPath)) {
          const folder: TreeNode = {
            name: part,
            type: 'folder',
            expanded: true,
            children: []
          }
          folders.set(currentPath, folder)
          currentLevel.push(folder)
        }

        currentLevel = folders.get(currentPath)!.children!
      }

      // Add file
      currentLevel.push({
        name: fileName,
        type: 'file',
        uri: file.uri
      })
    }

    return root
  }

  let fileTree = $state(buildFileTree(mockFiles))

  function toggleFolder(node: TreeNode) {
    if (node.type === 'folder') {
      node.expanded = !node.expanded
    }
  }

  async function openFile(uri: string, isPreview: boolean = true) {
    const doc = await documentStore.open(uri)
    if (doc) {
      editorStore.openTab({
        title: doc.name,
        uri: doc.uri,
        isPreview
      })
    }
  }

  function handleFileClick(node: TreeNode, e: MouseEvent) {
    if (node.type === 'file' && node.uri) {
      // Double-click opens as non-preview, single-click as preview
      openFile(node.uri, true)
    } else if (node.type === 'folder') {
      toggleFolder(node)
    }
  }

  function handleFileDblClick(node: TreeNode) {
    if (node.type === 'file' && node.uri) {
      // Double-click promotes from preview
      openFile(node.uri, false)
    }
  }

  function handleDragStart(e: DragEvent, node: TreeNode) {
    if (node.type === 'file' && node.uri && e.dataTransfer) {
      e.dataTransfer.setData('text/plain', JSON.stringify({
        uri: node.uri,
        title: node.name
      }))
      e.dataTransfer.effectAllowed = 'copyMove'
    }
  }

  // Create a scoped context for a tree node (for "when" clause evaluation)
  function createNodeContext(node: TreeNode) {
    const ctx = getContextKeyService().createScoped()
    ctx.set('explorerResourceIsFile', node.type === 'file')
    ctx.set('explorerResourceIsFolder', node.type === 'folder')
    ctx.set('explorerResourcePath', node.uri ?? node.name)
    ctx.set('explorerResourceName', node.name)
    ctx.set('explorerFocus', true)
    return ctx
  }

  // Create arg for explorer commands (VS Code pattern)
  function createNodeArg(node: TreeNode) {
    return {
      resourcePath: node.uri ?? node.name,
      resourceName: node.name,
      isFile: node.type === 'file',
      isFolder: node.type === 'folder'
    }
  }

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

{#snippet treeItem(item: TreeNode, depth: number)}
  {@const nodeContext = createNodeContext(item)}
  {@const nodeArg = createNodeArg(item)}
  <div class="tree-item">
    <NeoContextMenu menuId={MenuId.ExplorerContext} contextKeyService={nodeContext} arg={nodeArg}>
      {#snippet children()}
        <button
          class="tree-row w-full flex items-center gap-1 py-0.5 text-sm hover:bg-[var(--neo-list-hoverBackground)] rounded-sm"
          style="padding-left: {depth * 12 + 4}px;"
          onclick={(e) => handleFileClick(item, e)}
          ondblclick={() => handleFileDblClick(item)}
          draggable={item.type === 'file'}
          ondragstart={(e) => handleDragStart(e, item)}
        >
          {#if item.type === 'folder'}
            {#if item.expanded}
              <ChevronDown class="w-4 h-4 shrink-0" />
              <FolderOpen class="w-4 h-4 shrink-0 text-yellow-500" />
            {:else}
              <ChevronRight class="w-4 h-4 shrink-0" />
              <Folder class="w-4 h-4 shrink-0 text-yellow-500" />
            {/if}
          {:else}
            <span class="w-4"></span>
            <FileJson class="w-4 h-4 shrink-0 text-yellow-400" />
          {/if}
          <span class="truncate">{item.name}</span>
        </button>
      {/snippet}
    </NeoContextMenu>

    {#if item.type === 'folder' && item.expanded && item.children}
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

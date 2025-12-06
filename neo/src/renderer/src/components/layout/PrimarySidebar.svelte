<script lang="ts">
  import { layoutStore } from '$lib/stores/layout.svelte'
  import { editorStore } from '$lib/stores/editor.svelte'
  import { documentStore } from '$lib/stores/documents.svelte'
  import { serverStore } from '$lib/stores/server.svelte'
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import {
    ChevronRight,
    ChevronDown,
    FileJson,
    Folder,
    FolderOpen,
    Server,
    Cpu,
    Calendar,
    Workflow,
    Radio
  } from '@lucide/svelte'
  import NeoContextMenu from '../contextmenu/NeoContextMenu.svelte'
  import ConnectionDialog from './ConnectionDialog.svelte'
  import BacnetDiscoveryView from '../bacnet/BacnetDiscoveryView.svelte'
  import { MenuId } from '$lib/menus/menuId'
  import { getContextKeyService } from '$lib/services/context'

  // Tree node structure
  interface TreeNode {
    name: string
    type: 'folder' | 'file' | 'server'
    uri?: string
    children?: TreeNode[]
    icon?: 'device' | 'schedule' | 'blueprint' | 'server' | 'bacnet'
  }

  let showConnectionDialog = $state(false)

  // Build file tree from server data
  function buildFileTree(): TreeNode[] {
    if (serverStore.connection.state !== 'connected') {
      return []
    }

    const serverChildren: TreeNode[] = []

    // Devices folder
    if (serverStore.devices.length > 0) {
      const devicesFolder: TreeNode = {
        name: 'devices',
        type: 'folder',
        children: serverStore.devices.map((device) => ({
          name: `${device.id}.device.toml`,
          type: 'file' as const,
          uri: `neo://devices/${device.id}`,
          icon: 'device' as const
        }))
      }
      serverChildren.push(devicesFolder)
    }

    // Blueprints folder
    if (serverStore.blueprints.length > 0) {
      const blueprintsFolder: TreeNode = {
        name: 'blueprints',
        type: 'folder',
        children: serverStore.blueprints.map((blueprint) => ({
          name: `${blueprint.id}.bp.json`,
          type: 'file' as const,
          uri: `neo://blueprints/${blueprint.id}`,
          icon: 'blueprint' as const
        }))
      }
      serverChildren.push(blueprintsFolder)
    }

    // Schedules folder
    if (serverStore.schedules.length > 0) {
      const schedulesFolder: TreeNode = {
        name: 'schedules',
        type: 'folder',
        children: serverStore.schedules.map((schedule) => ({
          name: `${schedule.id}.schedule.toml`,
          type: 'file' as const,
          uri: `neo://schedules/${schedule.id}`,
          icon: 'schedule' as const
        }))
      }
      serverChildren.push(schedulesFolder)
    }

    // BACnet devices folder
    if (serverStore.bacnetDevices.length > 0) {
      const bacnetFolder: TreeNode = {
        name: 'bacnet',
        type: 'folder',
        children: serverStore.bacnetDevices.map((device) => ({
          name: `Device ${device.device_id}`,
          type: 'file' as const,
          uri: `neo://bacnet/devices/${device.device_id}`,
          icon: 'bacnet' as const
        }))
      }
      serverChildren.push(bacnetFolder)
    }

    // Server root node
    const serverNode: TreeNode = {
      name: `${serverStore.config.host}:${serverStore.config.port}`,
      type: 'server',
      icon: 'server',
      children: serverChildren
    }

    return [serverNode]
  }

  // Reactive file tree that updates when server data changes
  const fileTree = $derived(buildFileTree())
  const isConnected = $derived(serverStore.connection.state === 'connected')

  // Track expanded state separately since derived can't be mutated
  // Server root and subfolders are expanded by default
  let expandedFolders = $state<Set<string>>(new Set())

  // Expand server and subfolders when connected
  $effect(() => {
    if (serverStore.connection.state === 'connected') {
      const serverKey = `${serverStore.config.host}:${serverStore.config.port}`
      if (!expandedFolders.has(serverKey)) {
        expandedFolders.add(serverKey)
        expandedFolders.add('devices')
        expandedFolders.add('blueprints')
        expandedFolders.add('schedules')
        expandedFolders.add('bacnet')
        expandedFolders = new Set(expandedFolders)
      }
    }
  })

  function toggleFolder(node: TreeNode) {
    if (node.type === 'folder' || node.type === 'server') {
      if (expandedFolders.has(node.name)) {
        expandedFolders.delete(node.name)
      } else {
        expandedFolders.add(node.name)
      }
      expandedFolders = new Set(expandedFolders) // Trigger reactivity
    }
  }

  function isFolderExpanded(node: TreeNode): boolean {
    return (node.type === 'folder' || node.type === 'server') && expandedFolders.has(node.name)
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
      openFile(node.uri, true)
    } else if (node.type === 'folder' || node.type === 'server') {
      toggleFolder(node)
    }
  }

  function handleFileDblClick(node: TreeNode) {
    if (node.type === 'file' && node.uri) {
      openFile(node.uri, false)
    }
  }

  function handleDragStart(e: DragEvent, node: TreeNode) {
    if (node.type === 'file' && node.uri && e.dataTransfer) {
      e.dataTransfer.setData(
        'text/plain',
        JSON.stringify({
          uri: node.uri,
          title: node.name
        })
      )
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
    // BACnet device detection
    const isBacnetDevice = node.icon === 'bacnet' && node.uri?.startsWith('neo://bacnet/devices/')
    ctx.set('explorerResourceIsBacnetDevice', isBacnetDevice)
    return ctx
  }

  // Create arg for explorer commands (VS Code pattern)
  function createNodeArg(node: TreeNode) {
    // Extract device ID from BACnet device URI (neo://bacnet/devices/{id})
    let bacnetDeviceId: number | undefined
    if (node.icon === 'bacnet' && node.uri?.startsWith('neo://bacnet/devices/')) {
      const match = node.uri.match(/\/bacnet\/devices\/(\d+)/)
      if (match) {
        bacnetDeviceId = parseInt(match[1], 10)
      }
    }

    return {
      resourcePath: node.uri ?? node.name,
      resourceName: node.name,
      isFile: node.type === 'file',
      isFolder: node.type === 'folder',
      bacnetDeviceId
    }
  }

  function getSidebarTitle(): string {
    switch (layoutStore.state.activeActivityItem) {
      case 'explorer':
        return 'EXPLORER'
      case 'search':
        return 'SEARCH'
      case 'bacnet-discovery':
        return 'BACNET DISCOVERY'
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

  function getFileIcon(node: TreeNode) {
    switch (node.icon) {
      case 'server':
        return Server
      case 'device':
        return Cpu
      case 'schedule':
        return Calendar
      case 'blueprint':
        return Workflow
      case 'bacnet':
        return Radio
      default:
        return FileJson
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
      {#if !isConnected}
        <!-- Not Connected State -->
        <div class="flex flex-col items-center justify-center h-full p-6 text-center">
          <Server class="w-12 h-12 mb-4 opacity-30" />
          <p class="text-sm opacity-60 mb-4">Connect to a Neo station to browse devices, blueprints, and schedules.</p>
          <Button variant="outline" size="sm" onclick={() => (showConnectionDialog = true)}>
            <Server class="w-4 h-4 mr-2" />
            Connect to Station
          </Button>
        </div>
      {:else}
        <!-- File Tree with Server Root -->
        <div class="file-tree px-2 py-1">
          {#each fileTree as item}
            {@render treeItem(item, 0)}
          {/each}
        </div>
      {/if}
    {:else if layoutStore.state.activeActivityItem === 'search'}
      <div class="p-4 text-sm opacity-60">Search functionality coming soon...</div>
    {:else if layoutStore.state.activeActivityItem === 'bacnet-discovery'}
      <BacnetDiscoveryView />
    {:else if layoutStore.state.activeActivityItem === 'git'}
      <div class="p-4 text-sm opacity-60">Source control coming soon...</div>
    {:else if layoutStore.state.activeActivityItem === 'debug'}
      <div class="p-4 text-sm opacity-60">Debug functionality coming soon...</div>
    {:else if layoutStore.state.activeActivityItem === 'extensions'}
      <div class="p-4 text-sm opacity-60">Extensions coming soon...</div>
    {/if}
  </ScrollArea>
</div>

<ConnectionDialog bind:open={showConnectionDialog} />

{#snippet treeItem(item: TreeNode, depth: number)}
  {@const nodeContext = createNodeContext(item)}
  {@const nodeArg = createNodeArg(item)}
  {@const expanded = isFolderExpanded(item)}
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
          {#if item.type === 'server'}
            {#if expanded}
              <ChevronDown class="w-4 h-4 shrink-0" />
            {:else}
              <ChevronRight class="w-4 h-4 shrink-0" />
            {/if}
            <Server class="w-4 h-4 shrink-0 text-green-500" />
          {:else if item.type === 'folder'}
            {#if expanded}
              <ChevronDown class="w-4 h-4 shrink-0" />
              <FolderOpen class="w-4 h-4 shrink-0 text-yellow-500" />
            {:else}
              <ChevronRight class="w-4 h-4 shrink-0" />
              <Folder class="w-4 h-4 shrink-0 text-yellow-500" />
            {/if}
          {:else}
            <span class="w-4"></span>
            <svelte:component this={getFileIcon(item)} class="w-4 h-4 shrink-0 text-blue-400" />
          {/if}
          <span class="truncate">{item.name}</span>
        </button>
      {/snippet}
    </NeoContextMenu>

    {#if (item.type === 'folder' || item.type === 'server') && expanded && item.children}
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

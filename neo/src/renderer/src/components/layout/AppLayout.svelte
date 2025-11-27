<script lang="ts">
  import { onMount } from 'svelte'
  import { PaneGroup, Pane, type PaneAPI } from 'paneforge'
  import { layoutStore, type LayoutState } from '$lib/stores/layout.svelte'
  import * as Tooltip from '$lib/components/ui/tooltip'

  import TitleBar from './TitleBar.svelte'
  import ActivityBar from './ActivityBar.svelte'
  import PrimarySidebar from './PrimarySidebar.svelte'
  import EditorArea from './EditorArea.svelte'
  import Panel from './Panel.svelte'
  import AuxiliaryBar from './AuxiliaryBar.svelte'
  import StatusBar from './StatusBar.svelte'
  import Resizer from './Resizer.svelte'

  // Pane API refs for programmatic control
  let primarySidebarPane: PaneAPI | undefined = $state()
  let auxiliaryBarPane: PaneAPI | undefined = $state()
  let panelPane: PaneAPI | undefined = $state()

  // Expose pane APIs globally for toggle commands
  $effect(() => {
    layoutStore.setPaneAPIs({
      primarySidebar: primarySidebarPane,
      auxiliaryBar: auxiliaryBarPane,
      panel: panelPane
    })
  })

  // Layout persistence
  let initialized = $state(false)

  onMount(async () => {
    try {
      const savedLayout = (await window.layoutAPI.getLayout()) as Partial<LayoutState>
      if (savedLayout) {
        layoutStore.setLayout(savedLayout)
      }
    } catch (error) {
      console.error('Failed to load layout:', error)
    }
    initialized = true

    // Sync initial collapsed state from store to paneforge
    // (paneforge starts expanded by default, but our store may say collapsed)
    requestAnimationFrame(() => {
      if (!layoutStore.state.primarySidebarVisible && primarySidebarPane) {
        primarySidebarPane.collapse()
      }
      if (!layoutStore.state.auxiliaryBarVisible && auxiliaryBarPane) {
        auxiliaryBarPane.collapse()
      }
      if (!layoutStore.state.panelVisible && panelPane) {
        panelPane.collapse()
      }
    })
  })

  // Debounced save
  let saveTimeout: ReturnType<typeof setTimeout> | null = null

  function saveLayout() {
    if (saveTimeout) clearTimeout(saveTimeout)
    saveTimeout = setTimeout(() => {
      window.layoutAPI.setLayout({
        primarySidebarVisible: layoutStore.state.primarySidebarVisible,
        auxiliaryBarVisible: layoutStore.state.auxiliaryBarVisible,
        panelVisible: layoutStore.state.panelVisible,
        panelPosition: layoutStore.state.panelPosition,
        activeActivityItem: layoutStore.state.activeActivityItem,
        activePanelTab: layoutStore.state.activePanelTab
      })
    }, 500)
  }

  $effect(() => {
    if (initialized) {
      const _ = layoutStore.state
      saveLayout()
    }
  })

  // Sync paneforge collapse/expand events to our store
  function onPrimarySidebarCollapse() {
    layoutStore.state.primarySidebarVisible = false
  }
  function onPrimarySidebarExpand() {
    layoutStore.state.primarySidebarVisible = true
  }
  function onAuxiliaryBarCollapse() {
    layoutStore.state.auxiliaryBarVisible = false
  }
  function onAuxiliaryBarExpand() {
    layoutStore.state.auxiliaryBarVisible = true
  }
  function onPanelCollapse() {
    layoutStore.state.panelVisible = false
  }
  function onPanelExpand() {
    layoutStore.state.panelVisible = true
  }

  // Derived state for layout
  let isPanelHorizontal = $derived(
    layoutStore.state.panelPosition === 'left' || layoutStore.state.panelPosition === 'right'
  )
</script>

<Tooltip.Provider>
<div
  class="app-layout h-screen flex flex-col overflow-hidden select-none"
  style="background: var(--neo-background); color: var(--neo-foreground);"
>
  <!-- Title Bar -->
  <TitleBar />

  <!-- Main Content Area -->
  <div class="flex-1 flex overflow-hidden">
    <!-- Activity Bar (fixed width) -->
    <ActivityBar />

    <!-- Main horizontal layout with paneforge -->
    <PaneGroup direction="horizontal" class="flex-1">
      <!-- Panel on left (if horizontal position) -->
      {#if isPanelHorizontal && layoutStore.state.panelPosition === 'left'}
        <Pane
          bind:this={panelPane}
          defaultSize={20}
          minSize={10}
          collapsible
          collapsedSize={0}
          onCollapse={onPanelCollapse}
          onExpand={onPanelExpand}
        >
          <Panel />
        </Pane>
        <Resizer />
      {/if}

      <!-- Primary Sidebar -->
      <Pane
        bind:this={primarySidebarPane}
        defaultSize={20}
        minSize={10}
        collapsible
        collapsedSize={0}
        order={1}
        onCollapse={onPrimarySidebarCollapse}
        onExpand={onPrimarySidebarExpand}
      >
        <PrimarySidebar />
      </Pane>
      <Resizer />

      <!-- Center: Editor + Panel (vertical) -->
      <Pane defaultSize={60} minSize={30} order={2}>
        {#if isPanelHorizontal}
          <!-- No vertical panel, just editor -->
          <EditorArea />
        {:else}
          <!-- Vertical panel layout -->
          <PaneGroup direction="vertical">
            <!-- Panel on top -->
            {#if layoutStore.state.panelPosition === 'top'}
              <Pane
                bind:this={panelPane}
                defaultSize={30}
                minSize={10}
                collapsible
                collapsedSize={0}
                onCollapse={onPanelCollapse}
                onExpand={onPanelExpand}
              >
                <Panel />
              </Pane>
              <Resizer />
            {/if}

            <!-- Editor Area -->
            <Pane defaultSize={70} minSize={30}>
              <EditorArea />
            </Pane>

            <!-- Panel on bottom -->
            {#if layoutStore.state.panelPosition === 'bottom'}
              <Resizer />
              <Pane
                bind:this={panelPane}
                defaultSize={30}
                minSize={10}
                collapsible
                collapsedSize={0}
                onCollapse={onPanelCollapse}
                onExpand={onPanelExpand}
              >
                <Panel />
              </Pane>
            {/if}
          </PaneGroup>
        {/if}
      </Pane>

      <!-- Auxiliary Bar -->
      <Resizer />
      <Pane
        bind:this={auxiliaryBarPane}
        defaultSize={20}
        minSize={10}
        collapsible
        collapsedSize={0}
        order={3}
        onCollapse={onAuxiliaryBarCollapse}
        onExpand={onAuxiliaryBarExpand}
      >
        <AuxiliaryBar />
      </Pane>

      <!-- Panel on right (if horizontal position) -->
      {#if isPanelHorizontal && layoutStore.state.panelPosition === 'right'}
        <Resizer />
        <Pane
          bind:this={panelPane}
          defaultSize={20}
          minSize={10}
          collapsible
          collapsedSize={0}
          onCollapse={onPanelCollapse}
          onExpand={onPanelExpand}
        >
          <Panel />
        </Pane>
      {/if}
    </PaneGroup>
  </div>

  <!-- Status Bar -->
  <StatusBar />
</div>
</Tooltip.Provider>

<style>
  .app-layout {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell,
      sans-serif;
  }
</style>

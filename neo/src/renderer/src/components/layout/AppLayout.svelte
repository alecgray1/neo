<script lang="ts">
  import { PaneGroup, Pane, PaneResizer } from 'paneforge'
  import { onMount } from 'svelte'
  import { layoutStore, type LayoutState } from '$lib/stores/layout.svelte'
  import * as Tooltip from '$lib/components/ui/tooltip'

  import TitleBar from './TitleBar.svelte'
  import ActivityBar from './ActivityBar.svelte'
  import PrimarySidebar from './PrimarySidebar.svelte'
  import EditorArea from './EditorArea.svelte'
  import Panel from './Panel.svelte'
  import AuxiliaryBar from './AuxiliaryBar.svelte'
  import StatusBar from './StatusBar.svelte'

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
      // Track all state changes
      const _ = layoutStore.state
      saveLayout()
    }
  })

  // Derived state for layout structure
  let isPanelHorizontal = $derived(
    layoutStore.state.panelPosition === 'left' || layoutStore.state.panelPosition === 'right'
  )
  // Include all visibility states in the key to force complete re-render when any changes
  let layoutKey = $derived(
    `layout-${layoutStore.state.panelPosition}-ps${layoutStore.state.primarySidebarVisible}-aux${layoutStore.state.auxiliaryBarVisible}-panel${layoutStore.state.panelVisible}`
  )

  // Resizer classes
  const hResizerClass =
    'w-1 bg-transparent hover:bg-[var(--neo-focusBorder)] active:bg-[var(--neo-focusBorder)] transition-colors cursor-col-resize'
  const vResizerClass =
    'h-1 bg-transparent hover:bg-[var(--neo-focusBorder)] active:bg-[var(--neo-focusBorder)] transition-colors cursor-row-resize'
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

    <!-- Resizable Panes -->
    {#key layoutKey}
      {#if isPanelHorizontal}
        <!-- Panel is left or right: flat horizontal layout -->
        <PaneGroup direction="horizontal" autoSaveId={layoutKey} class="flex-1">
          <!-- Panel on left -->
          {#if layoutStore.state.panelPosition === 'left' && layoutStore.state.panelVisible}
            <Pane defaultSize={20} minSize={10} collapsible>
              <Panel />
            </Pane>
            <PaneResizer class={hResizerClass} />
          {/if}

          <!-- Primary Sidebar -->
          {#if layoutStore.state.primarySidebarVisible}
            <Pane defaultSize={17} minSize={10} collapsible>
              <PrimarySidebar />
            </Pane>
            <PaneResizer class={hResizerClass} />
          {/if}

          <!-- Editor Area -->
          <Pane defaultSize={50} minSize={20}>
            <EditorArea />
          </Pane>

          <!-- Auxiliary Bar -->
          {#if layoutStore.state.auxiliaryBarVisible}
            <PaneResizer class={hResizerClass} />
            <Pane defaultSize={17} minSize={10} collapsible>
              <AuxiliaryBar />
            </Pane>
          {/if}

          <!-- Panel on right -->
          {#if layoutStore.state.panelPosition === 'right' && layoutStore.state.panelVisible}
            <PaneResizer class={hResizerClass} />
            <Pane defaultSize={20} minSize={10} collapsible>
              <Panel />
            </Pane>
          {/if}
        </PaneGroup>
      {:else}
        <!-- Panel is top or bottom: nested vertical layout -->
        <PaneGroup direction="horizontal" autoSaveId={layoutKey} class="flex-1">
          <!-- Primary Sidebar -->
          {#if layoutStore.state.primarySidebarVisible}
            <Pane defaultSize={17} minSize={10} collapsible>
              <PrimarySidebar />
            </Pane>
            <PaneResizer class={hResizerClass} />
          {/if}

          <!-- Editor + Panel vertical stack -->
          <Pane defaultSize={66} minSize={20}>
            <PaneGroup direction="vertical" autoSaveId="{layoutKey}-vertical">
              <!-- Panel on top -->
              {#if layoutStore.state.panelPosition === 'top' && layoutStore.state.panelVisible}
                <Pane defaultSize={30} minSize={10} collapsible>
                  <Panel />
                </Pane>
                <PaneResizer class={vResizerClass} />
              {/if}

              <!-- Editor Area -->
              <Pane defaultSize={70}>
                <EditorArea />
              </Pane>

              <!-- Panel on bottom -->
              {#if layoutStore.state.panelPosition === 'bottom' && layoutStore.state.panelVisible}
                <PaneResizer class={vResizerClass} />
                <Pane defaultSize={30} minSize={10} collapsible>
                  <Panel />
                </Pane>
              {/if}
            </PaneGroup>
          </Pane>

          <!-- Auxiliary Bar -->
          {#if layoutStore.state.auxiliaryBarVisible}
            <PaneResizer class={hResizerClass} />
            <Pane defaultSize={17} minSize={10} collapsible>
              <AuxiliaryBar />
            </Pane>
          {/if}
        </PaneGroup>
      {/if}
    {/key}
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

<script lang="ts">
  import * as Menubar from '$lib/components/ui/menubar'
  import { layoutStore } from '$lib/stores/layout.svelte'

  function handleNewFile() {
    console.log('New file')
  }

  function handleOpen() {
    console.log('Open')
  }

  function handleSave() {
    console.log('Save')
  }

  function handleUndo() {
    console.log('Undo')
  }

  function handleRedo() {
    console.log('Redo')
  }

  function handleCut() {
    document.execCommand('cut')
  }

  function handleCopy() {
    document.execCommand('copy')
  }

  function handlePaste() {
    document.execCommand('paste')
  }

  function handleFind() {
    console.log('Find')
  }

  function handleReplace() {
    console.log('Replace')
  }

  function toggleSidebar() {
    layoutStore.togglePrimarySidebar()
  }

  function togglePanel() {
    layoutStore.togglePanel()
  }

  function toggleSecondarySidebar() {
    layoutStore.toggleAuxiliaryBar()
  }

  function handleZoomIn() {
    console.log('Zoom in')
  }

  function handleZoomOut() {
    console.log('Zoom out')
  }

  function handleGoToFile() {
    console.log('Go to file')
  }

  function handleGoToLine() {
    console.log('Go to line')
  }

  function handleGoToSymbol() {
    console.log('Go to symbol')
  }

  function handleDocs() {
    console.log('Open docs')
  }

  function handleAbout() {
    console.log('About')
  }
</script>

<div style="-webkit-app-region: no-drag;">
  <Menubar.Root class="border-none bg-transparent h-full rounded-none px-0">
    <!-- File Menu -->
    <Menubar.Menu>
      <Menubar.Trigger class="text-xs px-2.5 py-1 h-full data-[state=open]:bg-[var(--neo-menubar-selectionBackground)] hover:bg-[var(--neo-menubar-selectionBackground)] rounded-sm">
        File
      </Menubar.Trigger>
      <Menubar.Content class="min-w-[180px]" style="background: var(--neo-menu-background); border-color: var(--neo-menu-border);">
        <Menubar.Item onclick={handleNewFile}>
          New File
          <Menubar.Shortcut>Ctrl+N</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handleOpen}>
          Open...
          <Menubar.Shortcut>Ctrl+O</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Separator />
        <Menubar.Item onclick={handleSave}>
          Save
          <Menubar.Shortcut>Ctrl+S</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item>
          Save As...
          <Menubar.Shortcut>Ctrl+Shift+S</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Separator />
        <Menubar.Item>Close Editor</Menubar.Item>
        <Menubar.Separator />
        <Menubar.Item onclick={() => window.windowAPI.close()}>
          Exit
        </Menubar.Item>
      </Menubar.Content>
    </Menubar.Menu>

    <!-- Edit Menu -->
    <Menubar.Menu>
      <Menubar.Trigger class="text-xs px-2.5 py-1 h-full data-[state=open]:bg-[var(--neo-menubar-selectionBackground)] hover:bg-[var(--neo-menubar-selectionBackground)] rounded-sm">
        Edit
      </Menubar.Trigger>
      <Menubar.Content class="min-w-[180px]" style="background: var(--neo-menu-background); border-color: var(--neo-menu-border);">
        <Menubar.Item onclick={handleUndo}>
          Undo
          <Menubar.Shortcut>Ctrl+Z</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handleRedo}>
          Redo
          <Menubar.Shortcut>Ctrl+Y</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Separator />
        <Menubar.Item onclick={handleCut}>
          Cut
          <Menubar.Shortcut>Ctrl+X</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handleCopy}>
          Copy
          <Menubar.Shortcut>Ctrl+C</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handlePaste}>
          Paste
          <Menubar.Shortcut>Ctrl+V</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Separator />
        <Menubar.Item onclick={handleFind}>
          Find
          <Menubar.Shortcut>Ctrl+F</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handleReplace}>
          Replace
          <Menubar.Shortcut>Ctrl+H</Menubar.Shortcut>
        </Menubar.Item>
      </Menubar.Content>
    </Menubar.Menu>

    <!-- View Menu -->
    <Menubar.Menu>
      <Menubar.Trigger class="text-xs px-2.5 py-1 h-full data-[state=open]:bg-[var(--neo-menubar-selectionBackground)] hover:bg-[var(--neo-menubar-selectionBackground)] rounded-sm">
        View
      </Menubar.Trigger>
      <Menubar.Content class="min-w-[200px]" style="background: var(--neo-menu-background); border-color: var(--neo-menu-border);">
        <Menubar.CheckboxItem checked={layoutStore.state.primarySidebarVisible} onclick={toggleSidebar}>
          Primary Sidebar
          <Menubar.Shortcut>Ctrl+B</Menubar.Shortcut>
        </Menubar.CheckboxItem>
        <Menubar.CheckboxItem checked={layoutStore.state.panelVisible} onclick={togglePanel}>
          Panel
          <Menubar.Shortcut>Ctrl+J</Menubar.Shortcut>
        </Menubar.CheckboxItem>
        <Menubar.CheckboxItem checked={layoutStore.state.auxiliaryBarVisible} onclick={toggleSecondarySidebar}>
          Secondary Sidebar
          <Menubar.Shortcut>Ctrl+Alt+B</Menubar.Shortcut>
        </Menubar.CheckboxItem>
        <Menubar.Separator />
        <Menubar.Item onclick={handleZoomIn}>
          Zoom In
          <Menubar.Shortcut>Ctrl+=</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handleZoomOut}>
          Zoom Out
          <Menubar.Shortcut>Ctrl+-</Menubar.Shortcut>
        </Menubar.Item>
      </Menubar.Content>
    </Menubar.Menu>

    <!-- Go Menu -->
    <Menubar.Menu>
      <Menubar.Trigger class="text-xs px-2.5 py-1 h-full data-[state=open]:bg-[var(--neo-menubar-selectionBackground)] hover:bg-[var(--neo-menubar-selectionBackground)] rounded-sm">
        Go
      </Menubar.Trigger>
      <Menubar.Content class="min-w-[180px]" style="background: var(--neo-menu-background); border-color: var(--neo-menu-border);">
        <Menubar.Item onclick={handleGoToFile}>
          Go to File...
          <Menubar.Shortcut>Ctrl+P</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handleGoToLine}>
          Go to Line...
          <Menubar.Shortcut>Ctrl+G</Menubar.Shortcut>
        </Menubar.Item>
        <Menubar.Item onclick={handleGoToSymbol}>
          Go to Symbol...
          <Menubar.Shortcut>Ctrl+Shift+O</Menubar.Shortcut>
        </Menubar.Item>
      </Menubar.Content>
    </Menubar.Menu>

    <!-- Help Menu -->
    <Menubar.Menu>
      <Menubar.Trigger class="text-xs px-2.5 py-1 h-full data-[state=open]:bg-[var(--neo-menubar-selectionBackground)] hover:bg-[var(--neo-menubar-selectionBackground)] rounded-sm">
        Help
      </Menubar.Trigger>
      <Menubar.Content class="min-w-[160px]" style="background: var(--neo-menu-background); border-color: var(--neo-menu-border);">
        <Menubar.Item onclick={handleDocs}>
          Documentation
        </Menubar.Item>
        <Menubar.Separator />
        <Menubar.Item onclick={handleAbout}>
          About
        </Menubar.Item>
      </Menubar.Content>
    </Menubar.Menu>
  </Menubar.Root>
</div>

<style>
  :global([data-menubar-content]) {
    color: var(--neo-menu-foreground);
  }

  :global([data-menubar-item]:hover),
  :global([data-menubar-item][data-highlighted]) {
    background: var(--neo-menu-selectionBackground) !important;
    color: var(--neo-menu-selectionForeground) !important;
  }

  :global([data-menubar-separator]) {
    background: var(--neo-menu-separatorBackground);
  }
</style>

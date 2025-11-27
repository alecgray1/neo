<script lang="ts">
  import { onMount } from 'svelte'
  import { Code2 } from '@lucide/svelte'
  import MenuBar from './titlebar/MenuBar.svelte'
  import CommandCenter from './titlebar/CommandCenter.svelte'
  import TitleBarActions from './titlebar/TitleBarActions.svelte'
  import WindowControls from './titlebar/WindowControls.svelte'

  let isMac = $state(false)

  onMount(async () => {
    isMac = await window.windowAPI.isMac()
  })
</script>

<div
  class="titlebar h-9 flex items-stretch justify-between select-none shrink-0 relative"
  style="
    background: var(--neo-titleBar-activeBackground);
    color: var(--neo-titleBar-activeForeground);
    -webkit-app-region: drag;
  "
>
  <!-- Left: App Icon + Menu Bar -->
  <div
    class="titlebar-left flex items-center min-w-0"
    class:pl-[70px]={isMac}
  >
    <div class="app-icon px-3 flex items-center" style="-webkit-app-region: no-drag;">
      <Code2 class="w-4 h-4" />
    </div>
    <MenuBar />
  </div>

  <!-- Center: Command Center (absolutely positioned for true center) -->
  <div
    class="titlebar-center absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2"
  >
    <CommandCenter />
  </div>

  <!-- Right: Toggle Actions + Window Controls -->
  <div
    class="titlebar-right flex items-stretch justify-end"
  >
    <TitleBarActions />
    {#if !isMac}
      <WindowControls />
    {/if}
  </div>
</div>

<style>
  .titlebar {
    border-bottom: 1px solid var(--neo-titleBar-border);
  }
</style>

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
  class="titlebar h-9 flex items-stretch select-none shrink-0"
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
    style="flex: 2 2 20%;"
  >
    <div class="app-icon px-3 flex items-center" style="-webkit-app-region: no-drag;">
      <Code2 class="w-4 h-4" />
    </div>
    <MenuBar />
  </div>

  <!-- Center: Command Center -->
  <div
    class="titlebar-center flex justify-center mx-2.5"
    style="flex: 1 1 60%; max-width: fit-content;"
  >
    <CommandCenter />
  </div>

  <!-- Right: Toggle Actions + Window Controls -->
  <div
    class="titlebar-right flex items-stretch justify-end"
    style="flex: 2 2 20%;"
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

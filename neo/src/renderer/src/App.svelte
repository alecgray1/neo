<script lang="ts">
  import Versions from './components/Versions.svelte'
  import electronLogo from './assets/electron.svg'
  import { toggleTheme, currentTheme } from './lib/theme'
  import { Button } from "$lib/components/ui/button/index.js";

  const ipcHandle = (): void => window.electron.ipcRenderer.send('ping')
</script>

<img alt="logo" class="logo" src={electronLogo} />
<div class="creator">Powered by electron-vite</div>
<div class="text">
  Build an Electron app with
  <span class="svelte">Svelte</span>
  and
  <span class="ts">TypeScript</span>
</div>
<p class="tip">
  Current theme: <code>{$currentTheme?.name ?? 'Loading...'}</code>
</p>
<Button>Hello</Button>
<div class="actions">
  <div class="action">
    <a href="https://electron-vite.org/" target="_blank" rel="noreferrer">Documentation</a>
  </div>
  <div class="action">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <Button onclick={toggleTheme}>Toggle Theme</Button>
  </div>
  <div class="action">
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <a on:click={ipcHandle}>Send IPC</a>
  </div>
</div>
<Versions />

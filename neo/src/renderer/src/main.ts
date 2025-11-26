import { mount } from 'svelte'

import './assets/main.css'

import App from './App.svelte'
import { initTheme } from './lib/theme'

// Initialize theme before mounting app to prevent flash of unstyled content (FOUC)
initTheme().then(() => {
  const app = mount(App, {
    target: document.getElementById('app')!
  })

  // Export for HMR
  // @ts-ignore
  if (import.meta.hot) {
    // @ts-ignore
    import.meta.hot.accept()
  }

  return app
})

// Developer commands - devtools, extension reload, etc.
// These commands are only visible when developer mode is enabled

import type { ICommand } from '../types'

declare global {
  interface Window {
    developerAPI?: {
      toggleDevTools: () => void
      setDevMode: (enabled: boolean) => void
      reloadExtensions: () => Promise<void>
      reloadExtension: (extensionId: string) => Promise<void>
    }
  }
}

export const developerCommands: ICommand[] = [
  {
    id: 'neo.developer.toggleDevTools',
    title: 'Toggle Developer Tools',
    category: 'Developer',
    when: 'isDeveloperMode',
    keybinding: {
      key: 'ctrl+shift+i',
      mac: 'cmd+alt+i'
    },
    handler: () => {
      if (window.developerAPI?.toggleDevTools) {
        window.developerAPI.toggleDevTools()
      } else {
        console.warn('DevTools toggle not available (not running in Electron)')
      }
    }
  },
  {
    id: 'neo.developer.reloadExtensions',
    title: 'Reload All Extensions',
    category: 'Developer',
    when: 'isDeveloperMode',
    keybinding: {
      key: 'ctrl+shift+e',
      mac: 'cmd+shift+e'
    },
    handler: async () => {
      if (window.developerAPI?.reloadExtensions) {
        await window.developerAPI.reloadExtensions()
        console.log('Extensions reloaded')
      } else {
        console.warn('Extension reload not available (not running in Electron)')
      }
    }
  }
]

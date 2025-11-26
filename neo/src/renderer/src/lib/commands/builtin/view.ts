// View commands - toggle sidebars, panels, etc.

import type { ICommand } from '../types'
import { layoutStore } from '$lib/stores/layout.svelte'
import { quickAccessStore } from '../../../components/quickaccess/store.svelte'
import { documentStore } from '$lib/stores/documents.svelte'
import { editorStore } from '$lib/stores/editor.svelte'

export const viewCommands: ICommand[] = [
  {
    id: 'neo.view.togglePrimarySidebar',
    title: 'Toggle Primary Sidebar',
    category: 'View',
    keybinding: {
      key: 'ctrl+b',
      mac: 'cmd+b'
    },
    handler: () => {
      layoutStore.togglePrimarySidebar()
    }
  },
  {
    id: 'neo.view.toggleSecondarySidebar',
    title: 'Toggle Secondary Sidebar',
    category: 'View',
    keybinding: {
      key: 'ctrl+shift+b',
      mac: 'cmd+shift+b'
    },
    handler: () => {
      layoutStore.toggleAuxiliaryBar()
    }
  },
  {
    id: 'neo.view.togglePanel',
    title: 'Toggle Panel',
    category: 'View',
    keybinding: {
      key: 'ctrl+j',
      mac: 'cmd+j'
    },
    handler: () => {
      layoutStore.togglePanel()
    }
  },
  {
    id: 'neo.view.focusExplorer',
    title: 'Focus on Explorer',
    category: 'View',
    handler: () => {
      layoutStore.setActiveActivityItem('explorer')
    }
  },
  {
    id: 'neo.view.focusSearch',
    title: 'Focus on Search',
    category: 'View',
    handler: () => {
      layoutStore.setActiveActivityItem('search')
    }
  },
  {
    id: 'neo.view.focusTerminal',
    title: 'Focus on Terminal',
    category: 'View',
    keybinding: {
      key: 'ctrl+`',
      mac: 'cmd+`'
    },
    handler: () => {
      layoutStore.setActivePanelTab('terminal')
    }
  },
  {
    id: 'neo.view.focusProblems',
    title: 'Focus on Problems',
    category: 'View',
    keybinding: {
      key: 'ctrl+shift+m',
      mac: 'cmd+shift+m'
    },
    handler: () => {
      layoutStore.setActivePanelTab('problems')
    }
  },
  {
    id: 'neo.view.focusOutput',
    title: 'Focus on Output',
    category: 'View',
    handler: () => {
      layoutStore.setActivePanelTab('output')
    }
  },
  {
    id: 'neo.quickAccess.show',
    title: 'Go to File',
    category: 'View',
    keybinding: {
      key: 'ctrl+p',
      mac: 'cmd+p'
    },
    handler: () => {
      quickAccessStore.show('')
    }
  },
  {
    id: 'neo.quickAccess.showCommands',
    title: 'Show All Commands',
    category: 'View',
    keybinding: {
      key: 'ctrl+shift+p',
      mac: 'cmd+shift+p'
    },
    handler: () => {
      quickAccessStore.show('>')
    }
  },
  {
    id: 'neo.quickAccess.showHelp',
    title: 'Show Quick Access Help',
    category: 'View',
    handler: () => {
      quickAccessStore.show('?')
    }
  },
  {
    id: 'neo.preferences.openKeybindings',
    title: 'Open Keyboard Shortcuts',
    category: 'Preferences',
    keybinding: {
      key: 'ctrl+shift+k',
      mac: 'cmd+shift+k'
    },
    handler: async () => {
      const uri = 'keybindings://shortcuts'
      const doc = await documentStore.open(uri)
      if (doc) {
        editorStore.openTab({
          title: 'Keyboard Shortcuts',
          uri: doc.uri,
          isPreview: false
        })
      }
    }
  },
  {
    id: 'neo.preferences.openSettings',
    title: 'Open Settings',
    category: 'Preferences',
    keybinding: {
      key: 'ctrl+,',
      mac: 'cmd+,'
    },
    handler: async () => {
      const uri = 'settings://preferences'
      const doc = await documentStore.open(uri)
      if (doc) {
        editorStore.openTab({
          title: 'Settings',
          uri: doc.uri,
          isPreview: false
        })
      }
    }
  }
]

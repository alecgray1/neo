// Edit commands - undo, redo, copy, paste, etc.

import type { ICommand } from '../types'

export const editCommands: ICommand[] = [
  {
    id: 'neo.edit.undo',
    title: 'Undo',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+z',
      mac: 'cmd+z'
    },
    when: 'editorFocus',
    handler: () => {
      document.execCommand('undo')
    }
  },
  {
    id: 'neo.edit.redo',
    title: 'Redo',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+y',
      mac: 'cmd+shift+z'
    },
    when: 'editorFocus',
    handler: () => {
      document.execCommand('redo')
    }
  },
  {
    id: 'neo.edit.cut',
    title: 'Cut',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+x',
      mac: 'cmd+x'
    },
    when: 'editorFocus',
    handler: () => {
      document.execCommand('cut')
    }
  },
  {
    id: 'neo.edit.copy',
    title: 'Copy',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+c',
      mac: 'cmd+c'
    },
    when: 'editorFocus',
    handler: () => {
      document.execCommand('copy')
    }
  },
  {
    id: 'neo.edit.paste',
    title: 'Paste',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+v',
      mac: 'cmd+v'
    },
    when: 'editorFocus',
    handler: () => {
      document.execCommand('paste')
    }
  },
  {
    id: 'neo.edit.selectAll',
    title: 'Select All',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+a',
      mac: 'cmd+a'
    },
    when: 'editorFocus',
    handler: () => {
      document.execCommand('selectAll')
    }
  },
  {
    id: 'neo.edit.find',
    title: 'Find',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+f',
      mac: 'cmd+f'
    },
    when: 'editorFocus',
    handler: () => {
      // TODO: Implement find
      console.log('Find')
    }
  },
  {
    id: 'neo.edit.replace',
    title: 'Replace',
    category: 'Edit',
    keybinding: {
      key: 'ctrl+h',
      mac: 'cmd+alt+f'
    },
    when: 'editorFocus',
    handler: () => {
      // TODO: Implement replace
      console.log('Replace')
    }
  }
]

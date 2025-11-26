// File commands - new, open, save, close

import type { ICommand } from '../types'

export const fileCommands: ICommand[] = [
  {
    id: 'neo.file.new',
    title: 'New File',
    category: 'File',
    keybinding: {
      key: 'ctrl+n',
      mac: 'cmd+n'
    },
    handler: () => {
      // TODO: Implement new file creation
      console.log('New file')
    }
  },
  {
    id: 'neo.file.open',
    title: 'Open File',
    category: 'File',
    keybinding: {
      key: 'ctrl+o',
      mac: 'cmd+o'
    },
    handler: () => {
      // TODO: Implement file open dialog
      console.log('Open file')
    }
  },
  {
    id: 'neo.file.save',
    title: 'Save',
    category: 'File',
    keybinding: {
      key: 'ctrl+s',
      mac: 'cmd+s'
    },
    when: 'editorFocus',
    handler: () => {
      // TODO: Implement save
      console.log('Save file')
    }
  },
  {
    id: 'neo.file.saveAs',
    title: 'Save As...',
    category: 'File',
    keybinding: {
      key: 'ctrl+shift+s',
      mac: 'cmd+shift+s'
    },
    when: 'editorFocus',
    handler: () => {
      // TODO: Implement save as
      console.log('Save file as')
    }
  },
  {
    id: 'neo.file.saveAll',
    title: 'Save All',
    category: 'File',
    keybinding: {
      key: 'ctrl+k s',
      mac: 'cmd+alt+s'
    },
    handler: () => {
      // TODO: Implement save all
      console.log('Save all files')
    }
  },
  {
    id: 'neo.file.close',
    title: 'Close Editor',
    category: 'File',
    keybinding: {
      key: 'ctrl+w',
      mac: 'cmd+w'
    },
    when: 'editorFocus',
    handler: () => {
      // TODO: Implement close
      console.log('Close editor')
    }
  },
  {
    id: 'neo.file.closeAll',
    title: 'Close All Editors',
    category: 'File',
    keybinding: {
      key: 'ctrl+k ctrl+w',
      mac: 'cmd+k cmd+w'
    },
    handler: () => {
      // TODO: Implement close all
      console.log('Close all editors')
    }
  }
]

// File explorer commands

import type { ICommand } from '../types'

/** Context arg passed from explorer context menu */
interface ExplorerContextArg {
  resourcePath: string
  resourceName: string
  isFile: boolean
  isFolder: boolean
}

function isExplorerContextArg(arg: unknown): arg is ExplorerContextArg {
  return (
    typeof arg === 'object' &&
    arg !== null &&
    'resourcePath' in arg &&
    typeof (arg as ExplorerContextArg).resourcePath === 'string'
  )
}

/**
 * Explorer commands for context menu actions
 * These receive context via the arg parameter (VS Code pattern)
 */
export const explorerCommands: ICommand[] = [
  {
    id: 'neo.file.new',
    title: 'New File',
    category: 'File',
    handler: (_accessor, arg) => {
      // TODO: Implement new file creation
      console.log('New file requested', arg)
    }
  },
  {
    id: 'neo.folder.new',
    title: 'New Folder',
    category: 'File',
    handler: (_accessor, arg) => {
      // TODO: Implement new folder creation
      console.log('New folder requested', arg)
    }
  },
  {
    id: 'neo.file.cut',
    title: 'Cut',
    category: 'File',
    handler: (_accessor, arg) => {
      if (!isExplorerContextArg(arg)) return
      // TODO: Implement cut
      console.log('Cut:', arg.resourcePath)
    }
  },
  {
    id: 'neo.file.copy',
    title: 'Copy',
    category: 'File',
    handler: (_accessor, arg) => {
      if (!isExplorerContextArg(arg)) return
      // TODO: Implement copy
      console.log('Copy:', arg.resourcePath)
    }
  },
  {
    id: 'neo.file.paste',
    title: 'Paste',
    category: 'File',
    handler: (_accessor, arg) => {
      // TODO: Implement paste
      console.log('Paste requested', arg)
    }
  },
  {
    id: 'neo.file.rename',
    title: 'Rename',
    category: 'File',
    handler: (_accessor, arg) => {
      if (!isExplorerContextArg(arg)) return
      // TODO: Implement rename
      console.log('Rename:', arg.resourcePath)
    }
  },
  {
    id: 'neo.file.delete',
    title: 'Delete',
    category: 'File',
    handler: (_accessor, arg) => {
      if (!isExplorerContextArg(arg)) return
      // TODO: Implement delete
      console.log('Delete:', arg.resourcePath)
    }
  },
  {
    id: 'neo.file.revealInExplorer',
    title: 'Reveal in File Explorer',
    category: 'File',
    handler: (_accessor, arg) => {
      if (!isExplorerContextArg(arg)) return
      // TODO: Use shell.showItemInFolder via IPC
      console.log('Reveal in explorer:', arg.resourcePath)
    }
  },
  {
    id: 'neo.file.copyPath',
    title: 'Copy Path',
    category: 'File',
    handler: async (_accessor, arg) => {
      if (!isExplorerContextArg(arg)) return
      try {
        await navigator.clipboard.writeText(arg.resourcePath)
        console.log('Path copied:', arg.resourcePath)
      } catch (e) {
        console.error('Failed to copy path:', e)
      }
    }
  }
]

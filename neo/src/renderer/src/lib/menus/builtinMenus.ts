// Built-in menu definitions
// This file registers all default menu items for neo

import { MenuId } from './menuId'
import { menuRegistry } from './registry'

/**
 * Register all built-in menu items
 * Should be called during app initialization
 */
export function registerBuiltinMenus(): void {
  // Clear existing menus first (handles HMR reloads)
  menuRegistry.clearMenu(MenuId.EditorTabContext)
  menuRegistry.clearMenu(MenuId.ExplorerContext)
  menuRegistry.clearMenu(MenuId.EditorContext)
  // ========================================
  // Editor Tab Context Menu
  // ========================================

  // Close actions group
  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.close', title: 'Close' },
    group: '1_close',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.closeOthers', title: 'Close Others' },
    when: 'tabCount > 1',
    group: '1_close',
    order: 2
  })

  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.closeToRight', title: 'Close to the Right' },
    when: 'hasTabsToRight',
    group: '1_close',
    order: 3
  })

  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.closeAll', title: 'Close All' },
    when: 'tabCount > 1',
    group: '1_close',
    order: 4
  })

  // Pin actions group
  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.pin', title: 'Pin' },
    when: '!tabIsPinned',
    group: '2_pin',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.unpin', title: 'Unpin' },
    when: 'tabIsPinned',
    group: '2_pin',
    order: 1
  })

  // Split actions group
  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.splitRight', title: 'Split Right' },
    group: '3_split',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.EditorTabContext, {
    command: { id: 'neo.tab.splitDown', title: 'Split Down' },
    group: '3_split',
    order: 2
  })

  // ========================================
  // File Explorer Context Menu
  // ========================================

  // New file/folder group
  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.new', title: 'New File...' },
    group: '1_new',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.folder.new', title: 'New Folder...' },
    group: '1_new',
    order: 2
  })

  // Cut/Copy/Paste group
  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.cut', title: 'Cut' },
    when: 'explorerResourceIsFile || explorerResourceIsFolder',
    group: '5_cutcopypaste',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.copy', title: 'Copy' },
    when: 'explorerResourceIsFile || explorerResourceIsFolder',
    group: '5_cutcopypaste',
    order: 2
  })

  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.paste', title: 'Paste' },
    group: '5_cutcopypaste',
    order: 3
  })

  // Rename/Delete group
  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.rename', title: 'Rename' },
    when: 'explorerResourceIsFile || explorerResourceIsFolder',
    group: '7_modification',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.delete', title: 'Delete' },
    when: 'explorerResourceIsFile || explorerResourceIsFolder',
    group: '7_modification',
    order: 2
  })

  // Reveal group
  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.revealInExplorer', title: 'Reveal in File Explorer' },
    when: 'explorerResourceIsFile || explorerResourceIsFolder',
    group: '9_reveal',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.ExplorerContext, {
    command: { id: 'neo.file.copyPath', title: 'Copy Path' },
    when: 'explorerResourceIsFile || explorerResourceIsFolder',
    group: '9_reveal',
    order: 2
  })

  // ========================================
  // Editor Context Menu
  // ========================================

  // Cut/Copy/Paste group
  menuRegistry.appendMenuItem(MenuId.EditorContext, {
    command: { id: 'neo.edit.cut', title: 'Cut' },
    when: 'editorFocus && !editorReadOnly && hasSelection',
    group: '9_cutcopypaste',
    order: 1
  })

  menuRegistry.appendMenuItem(MenuId.EditorContext, {
    command: { id: 'neo.edit.copy', title: 'Copy' },
    when: 'editorFocus && hasSelection',
    group: '9_cutcopypaste',
    order: 2
  })

  menuRegistry.appendMenuItem(MenuId.EditorContext, {
    command: { id: 'neo.edit.paste', title: 'Paste' },
    when: 'editorFocus && !editorReadOnly',
    group: '9_cutcopypaste',
    order: 3
  })

  // Selection group
  menuRegistry.appendMenuItem(MenuId.EditorContext, {
    command: { id: 'neo.edit.selectAll', title: 'Select All' },
    when: 'editorFocus',
    group: '10_selection',
    order: 1
  })

  // Command palette
  menuRegistry.appendMenuItem(MenuId.EditorContext, {
    command: { id: 'neo.quickAccess.showCommands', title: 'Command Palette...' },
    group: '99_commandPalette',
    order: 1
  })
}

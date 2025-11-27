// Tab-related commands

import type { ICommand } from '../types'
import { editorStore } from '$lib/stores/editor.svelte'

/** Context arg passed from tab context menu */
interface TabContextArg {
  tabId: string
  groupId: string
}

function isTabContextArg(arg: unknown): arg is TabContextArg {
  return (
    typeof arg === 'object' &&
    arg !== null &&
    'tabId' in arg &&
    'groupId' in arg &&
    typeof (arg as TabContextArg).tabId === 'string' &&
    typeof (arg as TabContextArg).groupId === 'string'
  )
}

/**
 * Tab commands for context menu actions
 * These commands receive context via the arg parameter (VS Code pattern)
 */
export const tabCommands: ICommand[] = [
  {
    id: 'neo.tab.close',
    title: 'Close Tab',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return
      editorStore.closeTab(arg.tabId, arg.groupId)
    }
  },
  {
    id: 'neo.tab.closeOthers',
    title: 'Close Other Tabs',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return

      const group = editorStore.getGroup(arg.groupId)
      if (!group) return

      const otherTabs = group.tabs.filter((t) => t.id !== arg.tabId)
      otherTabs.forEach((t) => editorStore.closeTab(t.id, arg.groupId))
    }
  },
  {
    id: 'neo.tab.closeToRight',
    title: 'Close Tabs to the Right',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return

      const group = editorStore.getGroup(arg.groupId)
      if (!group) return

      const tabIndex = group.tabs.findIndex((t) => t.id === arg.tabId)
      if (tabIndex < 0) return

      const tabsToClose = group.tabs.slice(tabIndex + 1)
      tabsToClose.forEach((t) => editorStore.closeTab(t.id, arg.groupId))
    }
  },
  {
    id: 'neo.tab.closeAll',
    title: 'Close All Tabs',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return

      const group = editorStore.getGroup(arg.groupId)
      if (!group) return

      // Close all tabs in this group
      const allTabs = [...group.tabs]
      allTabs.forEach((t) => editorStore.closeTab(t.id, arg.groupId))
    }
  },
  {
    id: 'neo.tab.pin',
    title: 'Pin Tab',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return
      editorStore.pinTab(arg.tabId, arg.groupId)
    }
  },
  {
    id: 'neo.tab.unpin',
    title: 'Unpin Tab',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return
      editorStore.unpinTab(arg.tabId, arg.groupId)
    }
  },
  {
    id: 'neo.tab.splitRight',
    title: 'Split Right',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return
      const group = editorStore.getGroup(arg.groupId)
      if (!group) return

      const tab = group.tabs.find((t) => t.id === arg.tabId)
      if (!tab) return

      const newGroupId = editorStore.splitGroup('horizontal', arg.groupId)

      if (group.tabs.length === 1) {
        // Only 1 tab - duplicate it to show same file in both groups
        editorStore.openTab(
          { title: tab.title, uri: tab.uri, isPreview: false },
          newGroupId
        )
      } else {
        // Multiple tabs - move this tab to new group
        editorStore.moveTab(arg.tabId, newGroupId)
      }
    }
  },
  {
    id: 'neo.tab.splitDown',
    title: 'Split Down',
    category: 'Tab',
    handler: (_accessor, arg) => {
      if (!isTabContextArg(arg)) return
      const group = editorStore.getGroup(arg.groupId)
      if (!group) return

      const tab = group.tabs.find((t) => t.id === arg.tabId)
      if (!tab) return

      const newGroupId = editorStore.splitGroup('vertical', arg.groupId)

      if (group.tabs.length === 1) {
        // Only 1 tab - duplicate it to show same file in both groups
        editorStore.openTab(
          { title: tab.title, uri: tab.uri, isPreview: false },
          newGroupId
        )
      } else {
        // Multiple tabs - move this tab to new group
        editorStore.moveTab(arg.tabId, newGroupId)
      }
    }
  }
]

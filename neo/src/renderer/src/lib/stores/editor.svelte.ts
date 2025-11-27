import { SvelteMap } from 'svelte/reactivity'

export interface EditorTab {
  id: string
  title: string
  uri: string // Reference to document in documentStore
  icon?: string
  dirty?: boolean
  isPreview: boolean // Preview mode (italicized, replaceable)
  isPinned: boolean // Pinned (won't auto-close, stays left)
  lastAccessed: number // For MRU tracking
}

export interface EditorGroup {
  id: string
  tabs: EditorTab[]
  activeTabId: string | null
}

export type EditorLayoutNode =
  | { type: 'group'; groupId: string }
  | { type: 'split'; direction: 'horizontal' | 'vertical'; children: EditorLayoutNode[]; sizes: number[] }

export interface EditorLayout {
  root: EditorLayoutNode
}

let nextGroupId = 1
let nextTabId = 1

function generateGroupId(): string {
  return `group-${nextGroupId++}`
}

function generateTabId(): string {
  return `tab-${nextTabId++}`
}

// Each group is its own $state for proper reactivity
class EditorGroupState {
  id: string
  tabs = $state<EditorTab[]>([])
  activeTabId = $state<string | null>(null)

  constructor(id: string) {
    this.id = id
  }
}

function createEditorStore() {
  const initialGroupId = generateGroupId()

  // Each group is its own reactive state - using SvelteMap for reactive lookups
  const groupStates = new SvelteMap<string, EditorGroupState>()
  groupStates.set(initialGroupId, new EditorGroupState(initialGroupId))

  // Layout and active group state
  const layoutState = $state({
    root: { type: 'group', groupId: initialGroupId } as EditorLayoutNode,
    activeGroupId: initialGroupId
  })

  // Track group IDs for reactivity (Map isn't reactive)
  let groupIds = $state<string[]>([initialGroupId])

  function getGroup(groupId: string): EditorGroupState | undefined {
    return groupStates.get(groupId)
  }

  return {
    get layout() {
      return layoutState
    },

    get groupIds() {
      return groupIds
    },

    getGroup,

    get activeGroupId() {
      return layoutState.activeGroupId
    },

    get activeGroup(): EditorGroupState | undefined {
      return groupStates.get(layoutState.activeGroupId)
    },

    setActiveGroup(groupId: string) {
      if (groupStates.has(groupId)) {
        layoutState.activeGroupId = groupId
      }
    },

    // Open a new tab in the active group
    openTab(
      tab: Omit<EditorTab, 'id' | 'isPreview' | 'isPinned' | 'lastAccessed'> & {
        isPreview?: boolean
        isPinned?: boolean
      },
      groupId?: string
    ): string {
      const targetGroupId = groupId ?? layoutState.activeGroupId
      const group = groupStates.get(targetGroupId)
      if (!group) return ''

      // Check if this URI is already open in this group
      const existingTab = group.tabs.find((t) => t.uri === tab.uri)
      if (existingTab) {
        group.activeTabId = existingTab.id
        existingTab.lastAccessed = Date.now()
        layoutState.activeGroupId = targetGroupId
        if (!tab.isPreview && existingTab.isPreview) {
          existingTab.isPreview = false
        }
        return existingTab.id
      }

      const isPreview = tab.isPreview ?? false

      // If opening a preview tab, replace existing preview tab if any
      if (isPreview) {
        const existingPreviewIndex = group.tabs.findIndex((t) => t.isPreview && !t.isPinned)
        if (existingPreviewIndex !== -1) {
          const newTab: EditorTab = {
            ...tab,
            id: generateTabId(),
            isPreview: true,
            isPinned: false,
            lastAccessed: Date.now()
          }
          group.tabs[existingPreviewIndex] = newTab
          group.activeTabId = newTab.id
          layoutState.activeGroupId = targetGroupId
          return newTab.id
        }
      }

      const newTab: EditorTab = {
        ...tab,
        id: generateTabId(),
        isPreview,
        isPinned: tab.isPinned ?? false,
        lastAccessed: Date.now()
      }

      // Find insertion position: right after the currently active tab
      const lastPinnedIndex = group.tabs.reduce(
        (acc, t, i) => (t.isPinned ? i + 1 : acc),
        0
      )

      let insertIndex = lastPinnedIndex

      if (group.activeTabId) {
        const activeTabIndex = group.tabs.findIndex((t) => t.id === group.activeTabId)
        if (activeTabIndex >= 0) {
          insertIndex = Math.max(lastPinnedIndex, activeTabIndex + 1)
        }
      }

      // Create new array and assign (triggers reactivity on the $state array)
      group.tabs = [
        ...group.tabs.slice(0, insertIndex),
        newTab,
        ...group.tabs.slice(insertIndex)
      ]
      group.activeTabId = newTab.id
      layoutState.activeGroupId = targetGroupId

      return newTab.id
    },

    // Find tab by URI
    findTabByUri(uri: string, groupId?: string): { groupId: string; tab: EditorTab } | null {
      if (groupId) {
        const group = groupStates.get(groupId)
        if (group) {
          const tab = group.tabs.find((t) => t.uri === uri)
          if (tab) return { groupId, tab }
        }
        return null
      }
      for (const [gid, group] of groupStates) {
        const tab = group.tabs.find((t) => t.uri === uri)
        if (tab) return { groupId: gid, tab }
      }
      return null
    },

    // Pin a tab
    pinTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = groupStates.get(targetGroupId)
      if (!group) return

      const tabIndex = group.tabs.findIndex((t) => t.id === tabId)
      if (tabIndex === -1) return

      const tab = group.tabs[tabIndex]
      if (tab.isPinned) return

      const updatedTab = { ...tab, isPinned: true, isPreview: false }

      const lastPinnedIndex = group.tabs.reduce(
        (acc, t, i) => (t.isPinned && i !== tabIndex ? i + 1 : acc),
        0
      )

      const newTabs = group.tabs.filter((_, i) => i !== tabIndex)
      newTabs.splice(lastPinnedIndex, 0, updatedTab)
      group.tabs = newTabs
    },

    // Unpin a tab
    unpinTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = groupStates.get(targetGroupId)
      if (!group) return

      group.tabs = group.tabs.map((t) =>
        t.id === tabId ? { ...t, isPinned: false } : t
      )
    },

    // Promote preview tab to regular tab
    promoteFromPreview(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = groupStates.get(targetGroupId)
      if (!group) return

      group.tabs = group.tabs.map((t) =>
        t.id === tabId ? { ...t, isPreview: false } : t
      )
    },

    // Close a tab
    closeTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = groupStates.get(targetGroupId)
      if (!group) return

      const tabIndex = group.tabs.findIndex((t) => t.id === tabId)
      if (tabIndex === -1) return

      const newTabs = group.tabs.filter((t) => t.id !== tabId)

      let newActiveTabId = group.activeTabId
      if (group.activeTabId === tabId) {
        if (newTabs.length > 0) {
          const newIndex = Math.min(tabIndex, newTabs.length - 1)
          newActiveTabId = newTabs[newIndex].id
        } else {
          newActiveTabId = null
        }
      }

      group.tabs = newTabs
      group.activeTabId = newActiveTabId

      if (newTabs.length === 0 && groupStates.size > 1) {
        this.removeGroup(targetGroupId)
      }
    },

    // Set active tab in a group
    setActiveTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = groupStates.get(targetGroupId)
      if (!group) return

      if (group.tabs.some((t) => t.id === tabId)) {
        group.activeTabId = tabId
        layoutState.activeGroupId = targetGroupId
      }
    },

    // Find which group contains a tab
    findGroupContainingTab(tabId: string): string | null {
      for (const [gid, group] of groupStates) {
        if (group.tabs.some((t) => t.id === tabId)) {
          return gid
        }
      }
      return null
    },

    // Move a tab to another group
    moveTab(tabId: string, targetGroupId: string, index?: number) {
      const sourceGroupId = this.findGroupContainingTab(tabId)
      if (!sourceGroupId) return

      const sourceGroup = groupStates.get(sourceGroupId)
      const targetGroup = groupStates.get(targetGroupId)
      if (!sourceGroup || !targetGroup) return

      const tabIndex = sourceGroup.tabs.findIndex((t) => t.id === tabId)
      if (tabIndex === -1) return

      const tab = sourceGroup.tabs[tabIndex]

      // Remove from source
      const newSourceTabs = sourceGroup.tabs.filter((t) => t.id !== tabId)
      let newSourceActiveTabId = sourceGroup.activeTabId
      if (sourceGroup.activeTabId === tabId && newSourceTabs.length > 0) {
        const newActiveIndex = Math.min(tabIndex, newSourceTabs.length - 1)
        newSourceActiveTabId = newSourceTabs[newActiveIndex].id
      } else if (newSourceTabs.length === 0) {
        newSourceActiveTabId = null
      }
      sourceGroup.tabs = newSourceTabs
      sourceGroup.activeTabId = newSourceActiveTabId

      // Add to target
      const newTargetTabs = [...targetGroup.tabs]
      if (index !== undefined && index >= 0) {
        newTargetTabs.splice(index, 0, tab)
      } else {
        newTargetTabs.push(tab)
      }
      targetGroup.tabs = newTargetTabs
      targetGroup.activeTabId = tabId
      layoutState.activeGroupId = targetGroupId

      // Clean up empty source group
      if (newSourceTabs.length === 0 && groupStates.size > 1) {
        this.removeGroup(sourceGroupId)
      }
    },

    // Split current group
    splitGroup(direction: 'horizontal' | 'vertical', groupId?: string): string {
      const targetGroupId = groupId ?? layoutState.activeGroupId
      const newGroupId = generateGroupId()

      // Create new group state
      groupStates.set(newGroupId, new EditorGroupState(newGroupId))
      groupIds = [...groupIds, newGroupId]

      // Update layout tree
      const updateNode = (node: EditorLayoutNode): EditorLayoutNode => {
        if (node.type === 'group' && node.groupId === targetGroupId) {
          return {
            type: 'split',
            direction,
            children: [{ type: 'group', groupId: targetGroupId }, { type: 'group', groupId: newGroupId }],
            sizes: [50, 50]
          }
        }
        if (node.type === 'split') {
          return {
            ...node,
            children: node.children.map(updateNode)
          }
        }
        return node
      }

      layoutState.root = updateNode(layoutState.root)
      layoutState.activeGroupId = newGroupId

      return newGroupId
    },

    // Remove a group from the layout
    removeGroup(groupId: string) {
      if (!groupStates.has(groupId) || groupStates.size <= 1) return

      groupStates.delete(groupId)
      groupIds = groupIds.filter((id) => id !== groupId)

      // Update layout tree
      const removeFromNode = (node: EditorLayoutNode): EditorLayoutNode | null => {
        if (node.type === 'group') {
          return node.groupId === groupId ? null : node
        }

        const newChildren = node.children.map(removeFromNode).filter((n): n is EditorLayoutNode => n !== null)

        if (newChildren.length === 0) return null
        if (newChildren.length === 1) return newChildren[0]

        const totalSize = node.sizes.reduce((a, b) => a + b, 0)
        const remainingIndices = node.children
          .map((child, i) => (child.type === 'group' && child.groupId === groupId ? -1 : i))
          .filter((i) => i >= 0)

        const newSizes = remainingIndices.map((i) => (node.sizes[i] / totalSize) * 100)

        return { ...node, children: newChildren, sizes: newSizes }
      }

      const newRoot = removeFromNode(layoutState.root)
      if (newRoot) {
        layoutState.root = newRoot
      }

      if (layoutState.activeGroupId === groupId) {
        layoutState.activeGroupId = groupIds[0] ?? ''
      }
    },

    // Update pane sizes in a split
    updateSplitSizes(path: number[], sizes: number[]) {
      const updateNode = (node: EditorLayoutNode, currentPath: number[]): EditorLayoutNode => {
        if (currentPath.length === 0 && node.type === 'split') {
          return { ...node, sizes }
        }
        if (node.type === 'split' && currentPath.length > 0) {
          const [head, ...rest] = currentPath
          return {
            ...node,
            children: node.children.map((child, i) => (i === head ? updateNode(child, rest) : child))
          }
        }
        return node
      }

      layoutState.root = updateNode(layoutState.root, path)
    },

    // Mark a tab as dirty (unsaved)
    setTabDirty(tabId: string, dirty: boolean) {
      const groupId = this.findGroupContainingTab(tabId)
      if (!groupId) return

      const group = groupStates.get(groupId)
      if (!group) return

      group.tabs = group.tabs.map((t) =>
        t.id === tabId ? { ...t, dirty } : t
      )
    }
  }
}

export const editorStore = createEditorStore()

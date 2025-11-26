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
  groups: Record<string, EditorGroup>
}

let nextGroupId = 1
let nextTabId = 1

function generateGroupId(): string {
  return `group-${nextGroupId++}`
}

function generateTabId(): string {
  return `tab-${nextTabId++}`
}

function createEditorStore() {
  // Initialize with a single empty group
  const initialGroupId = generateGroupId()

  // Expose state directly for proper reactivity - using plain objects for deep tracking
  const state = $state({
    layout: {
      root: { type: 'group', groupId: initialGroupId } as EditorLayoutNode,
      groups: {
        [initialGroupId]: {
          id: initialGroupId,
          tabs: [],
          activeTabId: null
        }
      } as Record<string, EditorGroup>
    },
    activeGroupId: initialGroupId
  })

  return {
    // Direct state access for reactivity
    state,

    get activeGroup(): EditorGroup | undefined {
      return state.layout.groups[state.activeGroupId]
    },

    setActiveGroup(groupId: string) {
      if (state.layout.groups[groupId]) {
        state.activeGroupId = groupId
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
      const targetGroupId = groupId ?? state.activeGroupId
      const group = state.layout.groups[targetGroupId]
      if (!group) return ''

      // Check if this URI is already open in this group
      const existingTab = group.tabs.find((t) => t.uri === tab.uri)
      if (existingTab) {
        // If found, just activate it
        group.activeTabId = existingTab.id
        existingTab.lastAccessed = Date.now()
        state.activeGroupId = targetGroupId
        // If opening non-preview and existing is preview, promote it
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
          state.activeGroupId = targetGroupId
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

      // Insert after pinned tabs
      const lastPinnedIndex = group.tabs.reduce(
        (acc, t, i) => (t.isPinned ? i + 1 : acc),
        0
      )
      group.tabs.splice(lastPinnedIndex, 0, newTab)
      group.activeTabId = newTab.id
      state.activeGroupId = targetGroupId

      return newTab.id
    },

    // Find tab by URI
    findTabByUri(uri: string, groupId?: string): { groupId: string; tab: EditorTab } | null {
      if (groupId) {
        const group = state.layout.groups[groupId]
        if (group) {
          const tab = group.tabs.find((t) => t.uri === uri)
          if (tab) return { groupId, tab }
        }
        return null
      }
      // Search all groups
      for (const gid of Object.keys(state.layout.groups)) {
        const group = state.layout.groups[gid]
        const tab = group.tabs.find((t) => t.uri === uri)
        if (tab) return { groupId: gid, tab }
      }
      return null
    },

    // Pin a tab
    pinTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = state.layout.groups[targetGroupId]
      if (!group) return

      const tabIndex = group.tabs.findIndex((t) => t.id === tabId)
      if (tabIndex === -1) return

      const tab = group.tabs[tabIndex]
      if (tab.isPinned) return // Already pinned

      tab.isPinned = true
      tab.isPreview = false // Pinning promotes from preview

      // Move to end of pinned section
      const lastPinnedIndex = group.tabs.reduce(
        (acc, t, i) => (t.isPinned && i !== tabIndex ? i + 1 : acc),
        0
      )
      if (tabIndex !== lastPinnedIndex) {
        // Remove from current position and insert at pinned section
        group.tabs.splice(tabIndex, 1)
        group.tabs.splice(lastPinnedIndex, 0, tab)
      }
    },

    // Unpin a tab
    unpinTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = state.layout.groups[targetGroupId]
      if (!group) return

      const tab = group.tabs.find((t) => t.id === tabId)
      if (tab) {
        tab.isPinned = false
      }
    },

    // Promote preview tab to regular tab
    promoteFromPreview(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = state.layout.groups[targetGroupId]
      if (!group) return

      const tab = group.tabs.find((t) => t.id === tabId)
      if (tab) {
        tab.isPreview = false
      }
    },

    // Close a tab
    closeTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = state.layout.groups[targetGroupId]
      if (!group) return

      const tabIndex = group.tabs.findIndex((t) => t.id === tabId)
      if (tabIndex === -1) return

      group.tabs.splice(tabIndex, 1)

      // Update active tab if needed
      if (group.activeTabId === tabId) {
        if (group.tabs.length > 0) {
          // Select adjacent tab
          const newIndex = Math.min(tabIndex, group.tabs.length - 1)
          group.activeTabId = group.tabs[newIndex].id
        } else {
          group.activeTabId = null
        }
      }

      // If group is now empty and it's not the last group, remove it
      if (group.tabs.length === 0 && Object.keys(state.layout.groups).length > 1) {
        this.removeGroup(targetGroupId)
      }
    },

    // Set active tab in a group
    setActiveTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = state.layout.groups[targetGroupId]
      if (!group) return

      if (group.tabs.some((t) => t.id === tabId)) {
        group.activeTabId = tabId
        state.activeGroupId = targetGroupId
      }
    },

    // Find which group contains a tab
    findGroupContainingTab(tabId: string): string | null {
      for (const gid of Object.keys(state.layout.groups)) {
        const group = state.layout.groups[gid]
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

      const sourceGroup = state.layout.groups[sourceGroupId]
      const targetGroup = state.layout.groups[targetGroupId]
      if (!sourceGroup || !targetGroup) return

      const tabIndex = sourceGroup.tabs.findIndex((t) => t.id === tabId)
      if (tabIndex === -1) return

      const tab = sourceGroup.tabs[tabIndex]

      // Remove from source
      sourceGroup.tabs.splice(tabIndex, 1)

      // Add to target
      if (index !== undefined && index >= 0) {
        targetGroup.tabs.splice(index, 0, tab)
      } else {
        targetGroup.tabs.push(tab)
      }

      targetGroup.activeTabId = tabId
      state.activeGroupId = targetGroupId

      // Clean up empty source group
      if (sourceGroup.tabs.length === 0 && Object.keys(state.layout.groups).length > 1) {
        this.removeGroup(sourceGroupId)
      }
    },

    // Split current group
    splitGroup(direction: 'horizontal' | 'vertical', groupId?: string): string {
      const targetGroupId = groupId ?? state.activeGroupId
      const newGroupId = generateGroupId()

      // Create new empty group
      state.layout.groups[newGroupId] = {
        id: newGroupId,
        tabs: [],
        activeTabId: null
      }

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

      state.layout.root = updateNode(state.layout.root)
      state.activeGroupId = newGroupId

      return newGroupId
    },

    // Remove a group from the layout
    removeGroup(groupId: string) {
      const groupIds = Object.keys(state.layout.groups)
      if (!state.layout.groups[groupId] || groupIds.length <= 1) return

      delete state.layout.groups[groupId]

      // Update layout tree to remove the group
      const removeFromNode = (node: EditorLayoutNode): EditorLayoutNode | null => {
        if (node.type === 'group') {
          return node.groupId === groupId ? null : node
        }

        const newChildren = node.children.map(removeFromNode).filter((n): n is EditorLayoutNode => n !== null)

        if (newChildren.length === 0) {
          return null
        }
        if (newChildren.length === 1) {
          return newChildren[0]
        }

        // Redistribute sizes
        const totalSize = node.sizes.reduce((a, b) => a + b, 0)
        const remainingIndices = node.children
          .map((child, i) => (child.type === 'group' && child.groupId === groupId ? -1 : i))
          .filter((i) => i >= 0)

        const newSizes = remainingIndices.map((i) => (node.sizes[i] / totalSize) * 100)

        return {
          ...node,
          children: newChildren,
          sizes: newSizes
        }
      }

      const newRoot = removeFromNode(state.layout.root)
      if (newRoot) {
        state.layout.root = newRoot
      }

      // Update active group if it was removed
      if (state.activeGroupId === groupId) {
        state.activeGroupId = Object.keys(state.layout.groups)[0] ?? ''
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

      state.layout.root = updateNode(state.layout.root, path)
    },

    // Mark a tab as dirty (unsaved)
    setTabDirty(tabId: string, dirty: boolean) {
      const groupId = this.findGroupContainingTab(tabId)
      if (!groupId) return

      const group = state.layout.groups[groupId]
      if (!group) return

      const tab = group.tabs.find((t) => t.id === tabId)
      if (tab) {
        tab.dirty = dirty
      }
    }
  }
}

export const editorStore = createEditorStore()

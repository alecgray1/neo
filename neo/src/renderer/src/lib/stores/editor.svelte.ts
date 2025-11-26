export interface EditorTab {
  id: string
  title: string
  icon?: string
  dirty?: boolean
  content: unknown
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
  groups: Map<string, EditorGroup>
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
  const initialGroups = new Map<string, EditorGroup>()
  initialGroups.set(initialGroupId, {
    id: initialGroupId,
    tabs: [],
    activeTabId: null
  })

  let layout = $state<EditorLayout>({
    root: { type: 'group', groupId: initialGroupId },
    groups: initialGroups
  })

  let activeGroupId = $state<string>(initialGroupId)

  return {
    get layout() {
      return layout
    },

    get activeGroupId() {
      return activeGroupId
    },

    get activeGroup(): EditorGroup | undefined {
      return layout.groups.get(activeGroupId)
    },

    setActiveGroup(groupId: string) {
      if (layout.groups.has(groupId)) {
        activeGroupId = groupId
      }
    },

    // Open a new tab in the active group
    openTab(tab: Omit<EditorTab, 'id'>, groupId?: string): string {
      const targetGroupId = groupId ?? activeGroupId
      const group = layout.groups.get(targetGroupId)
      if (!group) return ''

      const newTab: EditorTab = {
        ...tab,
        id: generateTabId()
      }

      group.tabs = [...group.tabs, newTab]
      group.activeTabId = newTab.id
      activeGroupId = targetGroupId

      return newTab.id
    },

    // Close a tab
    closeTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = layout.groups.get(targetGroupId)
      if (!group) return

      const tabIndex = group.tabs.findIndex((t) => t.id === tabId)
      if (tabIndex === -1) return

      group.tabs = group.tabs.filter((t) => t.id !== tabId)

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
      if (group.tabs.length === 0 && layout.groups.size > 1) {
        this.removeGroup(targetGroupId)
      }
    },

    // Set active tab in a group
    setActiveTab(tabId: string, groupId?: string) {
      const targetGroupId = groupId ?? this.findGroupContainingTab(tabId)
      if (!targetGroupId) return

      const group = layout.groups.get(targetGroupId)
      if (!group) return

      if (group.tabs.some((t) => t.id === tabId)) {
        group.activeTabId = tabId
        activeGroupId = targetGroupId
      }
    },

    // Find which group contains a tab
    findGroupContainingTab(tabId: string): string | null {
      for (const [groupId, group] of layout.groups) {
        if (group.tabs.some((t) => t.id === tabId)) {
          return groupId
        }
      }
      return null
    },

    // Move a tab to another group
    moveTab(tabId: string, targetGroupId: string, index?: number) {
      const sourceGroupId = this.findGroupContainingTab(tabId)
      if (!sourceGroupId) return

      const sourceGroup = layout.groups.get(sourceGroupId)
      const targetGroup = layout.groups.get(targetGroupId)
      if (!sourceGroup || !targetGroup) return

      const tab = sourceGroup.tabs.find((t) => t.id === tabId)
      if (!tab) return

      // Remove from source
      sourceGroup.tabs = sourceGroup.tabs.filter((t) => t.id !== tabId)

      // Add to target
      if (index !== undefined && index >= 0) {
        targetGroup.tabs = [...targetGroup.tabs.slice(0, index), tab, ...targetGroup.tabs.slice(index)]
      } else {
        targetGroup.tabs = [...targetGroup.tabs, tab]
      }

      targetGroup.activeTabId = tabId
      activeGroupId = targetGroupId

      // Clean up empty source group
      if (sourceGroup.tabs.length === 0 && layout.groups.size > 1) {
        this.removeGroup(sourceGroupId)
      }
    },

    // Split current group
    splitGroup(direction: 'horizontal' | 'vertical', groupId?: string): string {
      const targetGroupId = groupId ?? activeGroupId
      const newGroupId = generateGroupId()

      // Create new empty group
      layout.groups.set(newGroupId, {
        id: newGroupId,
        tabs: [],
        activeTabId: null
      })

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

      layout.root = updateNode(layout.root)
      activeGroupId = newGroupId

      return newGroupId
    },

    // Remove a group from the layout
    removeGroup(groupId: string) {
      if (!layout.groups.has(groupId) || layout.groups.size <= 1) return

      layout.groups.delete(groupId)

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

      const newRoot = removeFromNode(layout.root)
      if (newRoot) {
        layout.root = newRoot
      }

      // Update active group if it was removed
      if (activeGroupId === groupId) {
        activeGroupId = layout.groups.keys().next().value ?? ''
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

      layout.root = updateNode(layout.root, path)
    },

    // Mark a tab as dirty (unsaved)
    setTabDirty(tabId: string, dirty: boolean) {
      const groupId = this.findGroupContainingTab(tabId)
      if (!groupId) return

      const group = layout.groups.get(groupId)
      if (!group) return

      const tab = group.tabs.find((t) => t.id === tabId)
      if (tab) {
        tab.dirty = dirty
      }
    }
  }
}

export const editorStore = createEditorStore()

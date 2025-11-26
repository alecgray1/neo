// Keybindings editor store - reactive state for the editor UI

import {
  KeybindingsEditorModel,
  type IKeybindingItemEntry,
  type SortMode
} from '$lib/keybindings/editorModel'
import { getUserKeybindingsService } from '$lib/keybindings/userKeybindings'
import { getKeybindingService } from '$lib/keybindings/registry'
import type { IParsedKeybinding } from '$lib/keybindings/types'

/**
 * Editor state
 */
interface IKeybindingsEditorState {
  entries: IKeybindingItemEntry[]
  searchQuery: string
  sortMode: SortMode
  selectedId: string | null
  isRecordMode: boolean
  recordedKeybinding: IParsedKeybinding | null
  isDefineDialogOpen: boolean
  defineDialogCommandId: string | null
  defineDialogExistingKey: string | null
}

/**
 * Create the keybindings editor store
 */
function createKeybindingsEditorStore() {
  const model = new KeybindingsEditorModel()

  let state = $state<IKeybindingsEditorState>({
    entries: model.entries,
    searchQuery: '',
    sortMode: 'command',
    selectedId: null,
    isRecordMode: false,
    recordedKeybinding: null,
    isDefineDialogOpen: false,
    defineDialogCommandId: null,
    defineDialogExistingKey: null
  })

  // Subscribe to keybinding changes
  const keybindingService = getKeybindingService()
  keybindingService.onDidChange(() => {
    model.refresh()
    state.entries = [...model.entries]
  })

  return {
    get state() {
      return state
    },

    get entries() {
      return state.entries
    },

    get searchQuery() {
      return state.searchQuery
    },

    get sortMode() {
      return state.sortMode
    },

    get selectedId() {
      return state.selectedId
    },

    get isRecordMode() {
      return state.isRecordMode
    },

    get recordedKeybinding() {
      return state.recordedKeybinding
    },

    get isDefineDialogOpen() {
      return state.isDefineDialogOpen
    },

    get defineDialogCommandId() {
      return state.defineDialogCommandId
    },

    get defineDialogExistingKey() {
      return state.defineDialogExistingKey
    },

    /**
     * Set search query and filter entries
     */
    setSearchQuery(query: string): void {
      state.searchQuery = query
      model.filter(query)
      state.entries = [...model.entries]
    },

    /**
     * Set sort mode
     */
    setSortMode(mode: SortMode): void {
      state.sortMode = mode
      model.setSortMode(mode)
      state.entries = [...model.entries]
    },

    /**
     * Select an entry
     */
    selectEntry(id: string | null): void {
      state.selectedId = id
    },

    /**
     * Toggle record mode
     */
    toggleRecordMode(): void {
      state.isRecordMode = !state.isRecordMode
      if (!state.isRecordMode) {
        state.recordedKeybinding = null
      }
    },

    /**
     * Exit record mode
     */
    exitRecordMode(): void {
      state.isRecordMode = false
      state.recordedKeybinding = null
    },

    /**
     * Record a keybinding (in record mode)
     */
    recordKeybinding(parsed: IParsedKeybinding): void {
      state.recordedKeybinding = parsed
      model.filterByKeybinding(parsed)
      state.entries = [...model.entries]
    },

    /**
     * Open define keybinding dialog
     */
    openDefineDialog(commandId: string, existingKey: string | null = null): void {
      state.defineDialogCommandId = commandId
      state.defineDialogExistingKey = existingKey
      state.isDefineDialogOpen = true
    },

    /**
     * Close define keybinding dialog
     */
    closeDefineDialog(): void {
      state.isDefineDialogOpen = false
      state.defineDialogCommandId = null
      state.defineDialogExistingKey = null
    },

    /**
     * Add or change a keybinding
     */
    async saveKeybinding(commandId: string, key: string, when?: string): Promise<void> {
      const userService = getUserKeybindingsService()

      if (state.defineDialogExistingKey) {
        // Editing existing
        await userService.editKeybinding(commandId, state.defineDialogExistingKey, key, when)
      } else {
        // Adding new
        await userService.addKeybinding(commandId, key, when)
      }

      this.closeDefineDialog()
      model.refresh()
      state.entries = [...model.entries]
    },

    /**
     * Remove a keybinding
     */
    async removeKeybinding(commandId: string, key: string, when?: string): Promise<void> {
      const userService = getUserKeybindingsService()
      await userService.removeKeybinding(commandId, key, when)

      model.refresh()
      state.entries = [...model.entries]
    },

    /**
     * Reset keybinding to default
     */
    async resetKeybinding(commandId: string, key?: string): Promise<void> {
      const userService = getUserKeybindingsService()
      await userService.resetKeybinding(commandId, key)

      model.refresh()
      state.entries = [...model.entries]
    },

    /**
     * Find conflicts for a keybinding
     */
    findConflicts(key: string, excludeCommandId?: string): IKeybindingItemEntry[] {
      return model.findConflicts(key, excludeCommandId)
    },

    /**
     * Refresh entries from services
     */
    refresh(): void {
      model.refresh()
      state.entries = [...model.entries]
    }
  }
}

// Singleton store instance
let _store: ReturnType<typeof createKeybindingsEditorStore> | null = null

/**
 * Get the keybindings editor store
 */
export function getKeybindingsEditorStore() {
  if (!_store) {
    _store = createKeybindingsEditorStore()
  }
  return _store
}

/**
 * Reset the store (for testing)
 */
export function resetKeybindingsEditorStore(): void {
  _store = null
}

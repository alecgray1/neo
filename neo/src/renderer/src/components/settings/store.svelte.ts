/**
 * Settings editor store - reactive state for the settings editor UI
 */

import {
  SettingsEditorModel,
  type ISettingItemEntry
} from '$lib/settings/editorModel'
import type { ISettingCategory, ISettingsGroup } from '$lib/settings/types'
import { getUserSettingsService } from '$lib/settings/userSettings'
import { getSettingsRegistry, registerBuiltinSettings } from '$lib/settings'

/**
 * Editor state
 */
interface ISettingsEditorState {
  settings: ISettingItemEntry[]
  groups: ISettingsGroup[]
  categories: ISettingCategory[]
  searchQuery: string
  selectedCategory: string | null
  expandedCategories: Set<string>
  modifiedCount: number
  totalCount: number
}

/**
 * Create the settings editor store
 */
function createSettingsEditorStore() {
  // Register built-in settings
  registerBuiltinSettings()

  const model = new SettingsEditorModel()

  let state = $state<ISettingsEditorState>({
    settings: model.settings,
    groups: model.groups,
    categories: model.categories,
    searchQuery: '',
    selectedCategory: null,
    expandedCategories: new Set<string>(),
    modifiedCount: model.modifiedCount,
    totalCount: model.totalCount
  })

  // Subscribe to user settings changes
  const userSettings = getUserSettingsService()
  userSettings.onDidChange(() => {
    model.refresh()
    updateState()
  })

  // Subscribe to registry changes
  const registry = getSettingsRegistry()
  registry.onDidChange(() => {
    model.refresh()
    updateState()
  })

  function updateState() {
    state.settings = [...model.settings]
    state.groups = [...model.groups]
    state.categories = [...model.categories]
    state.modifiedCount = model.modifiedCount
    state.totalCount = model.totalCount
  }

  return {
    get state() {
      return state
    },

    get settings() {
      return state.settings
    },

    get groups() {
      return state.groups
    },

    get categories() {
      return state.categories
    },

    get searchQuery() {
      return state.searchQuery
    },

    get selectedCategory() {
      return state.selectedCategory
    },

    get expandedCategories() {
      return state.expandedCategories
    },

    get modifiedCount() {
      return state.modifiedCount
    },

    get totalCount() {
      return state.totalCount
    },

    /**
     * Set search query and filter settings
     */
    setSearchQuery(query: string): void {
      state.searchQuery = query
      model.filter(query)
      updateState()
    },

    /**
     * Select a category
     */
    selectCategory(categoryId: string | null): void {
      state.selectedCategory = categoryId
      model.selectCategory(categoryId)
      updateState()
    },

    /**
     * Toggle category expansion in TOC
     */
    toggleCategory(categoryId: string): void {
      const expanded = new Set(state.expandedCategories)
      if (expanded.has(categoryId)) {
        expanded.delete(categoryId)
      } else {
        expanded.add(categoryId)
      }
      state.expandedCategories = expanded
    },

    /**
     * Expand a category
     */
    expandCategory(categoryId: string): void {
      const expanded = new Set(state.expandedCategories)
      expanded.add(categoryId)
      state.expandedCategories = expanded
    },

    /**
     * Update a setting value
     */
    async updateSetting(id: string, value: unknown): Promise<void> {
      await userSettings.setValue(id, value)
      model.refresh()
      updateState()
    },

    /**
     * Reset a setting to default
     */
    async resetSetting(id: string): Promise<void> {
      await userSettings.resetValue(id)
      model.refresh()
      updateState()
    },

    /**
     * Get a specific setting
     */
    getSetting(id: string) {
      return model.getSetting(id)
    },

    /**
     * Refresh from services
     */
    refresh(): void {
      model.refresh()
      updateState()
    },

    /**
     * Initialize the store (load user settings)
     */
    async initialize(): Promise<void> {
      await userSettings.initialize()
      model.refresh()
      updateState()
    }
  }
}

// Singleton store instance
let _store: ReturnType<typeof createSettingsEditorStore> | null = null

/**
 * Get the settings editor store
 */
export function getSettingsEditorStore() {
  if (!_store) {
    _store = createSettingsEditorStore()
  }
  return _store
}

/**
 * Reset the store (for testing)
 */
export function resetSettingsEditorStore(): void {
  _store = null
}

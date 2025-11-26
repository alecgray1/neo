/**
 * Settings editor model - data layer for the settings editor UI
 */

import type { ISettingMatch, ISettingItemEntry, ISetting, ISettingsGroup, ISettingCategory } from './types'
import { getSettingsRegistry } from './registry'
import { getUserSettingsService } from './userSettings'
import { fuzzyMatch } from '$lib/quickaccess/filter'

/**
 * Search filter type
 */
export interface ISearchFilter {
  type: 'text' | 'id' | 'tag' | 'modified'
  value: string
}

/**
 * Parse search query into filters
 * Supports: @modified, @tag:name, @id:pattern, or plain text
 */
function parseSearchQuery(query: string): ISearchFilter[] {
  const filters: ISearchFilter[] = []
  const parts = query.split(/\s+/)

  for (const part of parts) {
    if (!part) continue

    if (part === '@modified') {
      filters.push({ type: 'modified', value: '' })
    } else if (part.startsWith('@tag:')) {
      filters.push({ type: 'tag', value: part.slice(5).toLowerCase() })
    } else if (part.startsWith('@id:')) {
      filters.push({ type: 'id', value: part.slice(4).toLowerCase() })
    } else {
      // Plain text search
      filters.push({ type: 'text', value: part })
    }
  }

  return filters
}

/**
 * Settings editor model
 */
export class SettingsEditorModel {
  private _allSettings: ISetting[] = []
  private _filteredSettings: ISettingItemEntry[] = []
  private _groups: ISettingsGroup[] = []
  private _categories: ISettingCategory[] = []
  private _searchQuery = ''
  private _selectedCategory: string | null = null

  constructor() {
    this.refresh()
  }

  get settings(): ISettingItemEntry[] {
    return this._filteredSettings
  }

  get groups(): ISettingsGroup[] {
    return this._groups
  }

  get categories(): ISettingCategory[] {
    return this._categories
  }

  get searchQuery(): string {
    return this._searchQuery
  }

  get selectedCategory(): string | null {
    return this._selectedCategory
  }

  get modifiedCount(): number {
    return this._allSettings.filter((s) => s.isModified).length
  }

  get totalCount(): number {
    return this._filteredSettings.length
  }

  /**
   * Refresh entries from the registry and user settings
   */
  refresh(): void {
    const registry = getSettingsRegistry()
    const userSettings = getUserSettingsService()

    const definitions = registry.getAllSettings()
    this._categories = registry.getCategories()

    // Build settings with values
    this._allSettings = definitions.map((def) => {
      const userValue = userSettings.getUserValue(def.id)
      const defaultValue = def.schema.default
      const isModified = userValue !== undefined

      return {
        ...def,
        value: isModified ? userValue : defaultValue,
        defaultValue,
        userValue,
        isModified,
        source: isModified ? 'user' : 'default'
      } as ISetting
    })

    // Apply current filter
    this._applyFilter()
  }

  /**
   * Filter settings by text search
   */
  filter(query: string): void {
    this._searchQuery = query
    this._applyFilter()
  }

  /**
   * Select a category for filtering
   */
  selectCategory(categoryId: string | null): void {
    this._selectedCategory = categoryId
    this._applyFilter()
  }

  /**
   * Apply filter to settings
   */
  private _applyFilter(): void {
    let filtered = this._allSettings

    // Apply category filter
    if (this._selectedCategory) {
      const categoryPath = this._selectedCategory.split('.')
      filtered = filtered.filter((s) => {
        // Check if setting's category starts with selected category path
        if (s.category.length < categoryPath.length) return false
        for (let i = 0; i < categoryPath.length; i++) {
          if (s.category[i] !== categoryPath[i]) return false
        }
        return true
      })
    }

    // Apply search filters
    if (this._searchQuery.trim()) {
      const filters = parseSearchQuery(this._searchQuery)
      const results: ISettingItemEntry[] = []

      for (const setting of filtered) {
        let matches = true
        let idMatches: ISettingMatch[] | undefined
        let descriptionMatches: ISettingMatch[] | undefined
        let categoryMatches: ISettingMatch[] | undefined

        for (const filter of filters) {
          switch (filter.type) {
            case 'modified': {
              if (!setting.isModified) {
                matches = false
              }
              break
            }

            case 'tag': {
              const tags = setting.schema.tags || []
              if (!tags.some((t) => t.toLowerCase().includes(filter.value))) {
                matches = false
              }
              break
            }

            case 'id': {
              if (!setting.id.toLowerCase().includes(filter.value)) {
                matches = false
              } else {
                // Highlight the match
                const idx = setting.id.toLowerCase().indexOf(filter.value)
                if (idx >= 0) {
                  idMatches = [{ start: idx, end: idx + filter.value.length }]
                }
              }
              break
            }

            case 'text': {
              let found = false

              // Search setting ID
              const idMatch = fuzzyMatch(filter.value, setting.id)
              if (idMatch) {
                found = true
                idMatches = idMatch.highlights
              }

              // Search description
              const desc = setting.schema.description || ''
              if (!found && desc) {
                const descMatch = fuzzyMatch(filter.value, desc)
                if (descMatch) {
                  found = true
                  descriptionMatches = descMatch.highlights
                }
              }

              // Search category
              const categoryStr = setting.category.join(' ')
              if (!found) {
                const catMatch = fuzzyMatch(filter.value, categoryStr)
                if (catMatch) {
                  found = true
                  categoryMatches = catMatch.highlights
                }
              }

              if (!found) {
                matches = false
              }
              break
            }
          }

          if (!matches) break
        }

        if (matches) {
          results.push({
            ...setting,
            idMatches,
            descriptionMatches,
            categoryMatches
          })
        }
      }

      this._filteredSettings = results
    } else {
      this._filteredSettings = filtered.map((s) => ({ ...s }))
    }

    // Build groups from filtered settings
    this._buildGroups()
  }

  /**
   * Build groups from filtered settings
   */
  private _buildGroups(): void {
    const groupMap = new Map<string, ISettingsGroup>()

    for (const setting of this._filteredSettings) {
      const groupId = setting.category.join('.')
      const groupLabel = setting.category[setting.category.length - 1] || 'General'

      if (!groupMap.has(groupId)) {
        groupMap.set(groupId, {
          id: groupId,
          label: groupLabel,
          path: setting.category,
          settings: []
        })
      }

      groupMap.get(groupId)!.settings.push(setting)
    }

    // Sort groups by category order and name
    this._groups = Array.from(groupMap.values()).sort((a, b) => {
      // Compare by path segments
      const minLen = Math.min(a.path.length, b.path.length)
      for (let i = 0; i < minLen; i++) {
        const cmp = a.path[i].localeCompare(b.path[i])
        if (cmp !== 0) return cmp
      }
      return a.path.length - b.path.length
    })

    // Sort settings within each group by ID
    for (const group of this._groups) {
      group.settings.sort((a, b) => {
        // Sort by order if specified
        const orderA = a.schema.order ?? 999
        const orderB = b.schema.order ?? 999
        if (orderA !== orderB) return orderA - orderB
        return a.id.localeCompare(b.id)
      })
    }
  }

  /**
   * Get a specific setting by ID
   */
  getSetting(id: string): ISetting | undefined {
    return this._allSettings.find((s) => s.id === id)
  }
}

/**
 * Create a new settings editor model
 */
export function createSettingsEditorModel(): SettingsEditorModel {
  return new SettingsEditorModel()
}

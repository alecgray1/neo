/**
 * Settings Registry - manages registration of setting definitions
 */

import type { ISettingDefinition, ISettingCategory } from './types'

type ChangeListener = () => void

class SettingsRegistry {
  private _definitions: Map<string, ISettingDefinition> = new Map()
  private _listeners: Set<ChangeListener> = new Set()
  private _categoriesCache: ISettingCategory[] | null = null

  /**
   * Register a setting definition
   */
  register(definition: ISettingDefinition): () => void {
    this._definitions.set(definition.id, definition)
    this._categoriesCache = null
    this._notifyListeners()

    // Return dispose function
    return () => {
      this._definitions.delete(definition.id)
      this._categoriesCache = null
      this._notifyListeners()
    }
  }

  /**
   * Register multiple settings at once
   */
  registerMany(definitions: ISettingDefinition[]): () => void {
    for (const def of definitions) {
      this._definitions.set(def.id, def)
    }
    this._categoriesCache = null
    this._notifyListeners()

    return () => {
      for (const def of definitions) {
        this._definitions.delete(def.id)
      }
      this._categoriesCache = null
      this._notifyListeners()
    }
  }

  /**
   * Get all registered settings
   */
  getAllSettings(): ISettingDefinition[] {
    return Array.from(this._definitions.values())
  }

  /**
   * Get a specific setting by ID
   */
  getSetting(id: string): ISettingDefinition | undefined {
    return this._definitions.get(id)
  }

  /**
   * Get default value for a setting
   */
  getDefaultValue(id: string): unknown {
    const def = this._definitions.get(id)
    return def?.schema.default
  }

  /**
   * Build and return category tree from setting definitions
   */
  getCategories(): ISettingCategory[] {
    if (this._categoriesCache) {
      return this._categoriesCache
    }

    const categoryMap = new Map<string, ISettingCategory>()
    const rootCategories: ISettingCategory[] = []

    // Count settings per category path
    const categoryCounts = new Map<string, number>()
    for (const def of this._definitions.values()) {
      const path = def.category.join('.')
      categoryCounts.set(path, (categoryCounts.get(path) || 0) + 1)

      // Also count for parent paths
      for (let i = 1; i < def.category.length; i++) {
        const parentPath = def.category.slice(0, i).join('.')
        categoryCounts.set(parentPath, (categoryCounts.get(parentPath) || 0) + 1)
      }
    }

    // Build category nodes
    for (const def of this._definitions.values()) {
      let currentPath: string[] = []

      for (let i = 0; i < def.category.length; i++) {
        const segment = def.category[i]
        currentPath = [...currentPath, segment]
        const pathKey = currentPath.join('.')

        if (!categoryMap.has(pathKey)) {
          const category: ISettingCategory = {
            id: pathKey,
            label: segment,
            path: [...currentPath],
            children: [],
            settingCount: categoryCounts.get(pathKey) || 0,
            order: def.categoryOrder
          }
          categoryMap.set(pathKey, category)

          // Add to parent or root
          if (currentPath.length === 1) {
            rootCategories.push(category)
          } else {
            const parentPath = currentPath.slice(0, -1).join('.')
            const parent = categoryMap.get(parentPath)
            if (parent && !parent.children.find((c) => c.id === category.id)) {
              parent.children.push(category)
            }
          }
        }
      }
    }

    // Sort categories alphabetically, but respect order if specified
    const sortCategories = (cats: ISettingCategory[]) => {
      cats.sort((a, b) => {
        if (a.order !== undefined && b.order !== undefined) {
          return a.order - b.order
        }
        if (a.order !== undefined) return -1
        if (b.order !== undefined) return 1
        return a.label.localeCompare(b.label)
      })
      for (const cat of cats) {
        sortCategories(cat.children)
      }
    }
    sortCategories(rootCategories)

    this._categoriesCache = rootCategories
    return rootCategories
  }

  /**
   * Subscribe to registry changes
   */
  onDidChange(listener: ChangeListener): () => void {
    this._listeners.add(listener)
    return () => this._listeners.delete(listener)
  }

  private _notifyListeners(): void {
    for (const listener of this._listeners) {
      listener()
    }
  }
}

// Singleton instance
let _instance: SettingsRegistry | null = null

export function getSettingsRegistry(): SettingsRegistry {
  if (!_instance) {
    _instance = new SettingsRegistry()
  }
  return _instance
}

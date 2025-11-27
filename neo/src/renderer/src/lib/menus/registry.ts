// Menu registry - central storage for all menu definitions

import { Emitter, toDisposable, type IDisposable } from '../services/types'
import { MenuId } from './menuId'
import type { MenuEntry, IMenuChangeEvent } from './types'

/**
 * Menu registry - stores all registered menu items
 * Menu items are registered per MenuId and can have "when" clauses
 * for conditional visibility
 */
class MenuRegistry {
  private _items = new Map<MenuId, MenuEntry[]>()
  private _onDidChange = new Emitter<IMenuChangeEvent>()

  /**
   * Event fired when menu items change
   */
  readonly onDidChangeMenu = this._onDidChange.event

  /**
   * Append a menu item to a menu
   * Returns a disposable that removes the item when disposed
   */
  appendMenuItem(menuId: MenuId, item: MenuEntry): IDisposable {
    const items = this._items.get(menuId) ?? []
    items.push(item)
    this._items.set(menuId, items)
    this._onDidChange.fire({ menuId })

    return toDisposable(() => this.removeMenuItem(menuId, item))
  }

  /**
   * Append multiple menu items to a menu
   * Returns a disposable that removes all items when disposed
   */
  appendMenuItems(menuId: MenuId, items: MenuEntry[]): IDisposable {
    const existingItems = this._items.get(menuId) ?? []
    existingItems.push(...items)
    this._items.set(menuId, existingItems)
    this._onDidChange.fire({ menuId })

    return toDisposable(() => {
      for (const item of items) {
        this.removeMenuItem(menuId, item)
      }
    })
  }

  /**
   * Remove a specific menu item
   */
  removeMenuItem(menuId: MenuId, item: MenuEntry): void {
    const items = this._items.get(menuId)
    if (!items) return

    const idx = items.indexOf(item)
    if (idx >= 0) {
      items.splice(idx, 1)
      this._onDidChange.fire({ menuId })
    }
  }

  /**
   * Get all menu items for a menu ID
   * Returns a copy to prevent external modification
   */
  getMenuItems(menuId: MenuId): readonly MenuEntry[] {
    return [...(this._items.get(menuId) ?? [])]
  }

  /**
   * Check if a menu has any items
   */
  hasMenuItems(menuId: MenuId): boolean {
    const items = this._items.get(menuId)
    return !!items && items.length > 0
  }

  /**
   * Clear all items for a menu
   */
  clearMenu(menuId: MenuId): void {
    if (this._items.has(menuId)) {
      this._items.delete(menuId)
      this._onDidChange.fire({ menuId })
    }
  }

  /**
   * Clear all menus
   */
  clearAll(): void {
    this._items.clear()
  }
}

/**
 * Global menu registry singleton
 */
export const menuRegistry = new MenuRegistry()

// Menu system type definitions

import type { MenuId } from './menuId'

/**
 * A menu item that executes a command
 */
export interface IMenuItem {
  command: {
    id: string
    title: string
    icon?: string
  }
  /** When clause expression - item only shows if this evaluates to true */
  when?: string
  /** Group name for organizing menu items (e.g., 'navigation', '1_modification') */
  group?: string
  /** Order within the group (lower = earlier) */
  order?: number
}

/**
 * A submenu item that opens another menu
 */
export interface ISubmenuItem {
  submenu: MenuId
  title: string
  icon?: string
  when?: string
  group?: string
  order?: number
}

/**
 * A menu entry can be either a command item or a submenu
 */
export type MenuEntry = IMenuItem | ISubmenuItem

/**
 * Event fired when a menu's items change
 */
export interface IMenuChangeEvent {
  menuId: MenuId
}

/**
 * Type guard to check if a menu entry is a command item
 */
export function isMenuItem(entry: MenuEntry): entry is IMenuItem {
  return 'command' in entry
}

/**
 * Type guard to check if a menu entry is a submenu
 */
export function isSubmenuItem(entry: MenuEntry): entry is ISubmenuItem {
  return 'submenu' in entry
}

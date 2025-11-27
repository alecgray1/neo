// Context menu utilities

import type { IContextKeyService } from '$lib/services/context'
import { getContextFromElement } from '$lib/services/context'
import type { MenuId } from '$lib/menus/menuId'
import { getMenuActions, type IMenuActionGroup } from '$lib/menus/service'

export interface ContextMenuState {
  /** Whether the menu is open */
  open: boolean
  /** X position for the menu */
  x: number
  /** Y position for the menu */
  y: number
  /** Menu groups to display */
  groups: IMenuActionGroup[]
}

/**
 * Create a context menu state object
 */
export function createContextMenuState(): ContextMenuState {
  return {
    open: false,
    x: 0,
    y: 0,
    groups: []
  }
}

/**
 * Build menu actions for a menu at a given element
 * Uses the DOM-bound context system to find the appropriate context
 */
export function buildMenuForElement(
  menuId: MenuId,
  element: HTMLElement | null
): IMenuActionGroup[] {
  const ctx = getContextFromElement(element)
  return getMenuActions(menuId, ctx)
}

/**
 * Build menu actions with a specific context service
 */
export function buildMenuWithContext(
  menuId: MenuId,
  contextKeyService: IContextKeyService
): IMenuActionGroup[] {
  return getMenuActions(menuId, contextKeyService)
}

/**
 * Show context menu at mouse event position
 * Returns the menu state to be used with a context menu component
 */
export function showContextMenuAtEvent(
  menuId: MenuId,
  event: MouseEvent,
  contextKeyService?: IContextKeyService
): ContextMenuState {
  const ctx = contextKeyService ?? getContextFromElement(event.target as HTMLElement)
  const groups = getMenuActions(menuId, ctx)

  return {
    open: true,
    x: event.clientX,
    y: event.clientY,
    groups
  }
}

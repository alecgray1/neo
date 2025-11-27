// Menu service - builds action lists from registry with context filtering

import type { IContextKeyService } from '../services/context'
import { executeCommand } from '../commands/registry'
import { MenuId } from './menuId'
import { menuRegistry } from './registry'
import { isMenuItem, isSubmenuItem, type MenuEntry } from './types'

/**
 * Options for getting menu actions
 */
export interface IMenuActionOptions {
  /** Context argument to pass to commands when they run */
  arg?: unknown
  /** Whether to forward additional runtime args */
  shouldForwardArgs?: boolean
}

/**
 * A resolved menu action ready for display
 */
export interface IMenuAction {
  id: string
  label: string
  icon?: string
  enabled: boolean
  run: (...args: unknown[]) => void | Promise<void>
}

/**
 * A group of menu actions with a group identifier
 */
export interface IMenuActionGroup {
  group: string
  actions: IMenuAction[]
}

/**
 * A resolved submenu ready for display
 */
export interface IMenuSubmenu {
  id: string
  label: string
  icon?: string
  menuId: MenuId
}

/**
 * Parse a group string to extract sort order
 * Format: "order_name" where order is a number (e.g., "1_modification")
 * If no order prefix, defaults to Infinity
 */
function parseGroupOrder(group: string): { order: number; name: string } {
  const match = group.match(/^(\d+)_(.*)$/)
  if (match) {
    return { order: parseInt(match[1], 10), name: match[2] }
  }
  return { order: Infinity, name: group }
}

/**
 * Sort groups by their numeric prefix
 */
function sortGroups(groups: Map<string, IMenuAction[]>): IMenuActionGroup[] {
  const entries = Array.from(groups.entries())

  // Sort by group order (numeric prefix)
  entries.sort((a, b) => {
    const orderA = parseGroupOrder(a[0]).order
    const orderB = parseGroupOrder(b[0]).order
    return orderA - orderB
  })

  return entries.map(([group, actions]) => ({
    group,
    actions
  }))
}

/**
 * Get menu actions for a menu, filtered by context
 * Returns grouped and sorted actions ready for rendering
 *
 * @param menuId - Which menu to get actions for
 * @param contextKeyService - Context for evaluating "when" clauses
 * @param options - Options including arg to pass to commands (VS Code pattern)
 */
export function getMenuActions(
  menuId: MenuId,
  contextKeyService: IContextKeyService,
  options?: IMenuActionOptions
): IMenuActionGroup[] {
  const items = menuRegistry.getMenuItems(menuId)
  const groups = new Map<string, IMenuAction[]>()

  // Capture the arg at menu build time (VS Code pattern - closure captures context)
  const capturedArg = options?.arg
  const shouldForwardArgs = options?.shouldForwardArgs ?? false

  // Collect items into groups, filtering by "when" clause
  for (const item of items) {
    // Evaluate "when" clause - skip if evaluates to false
    if (item.when && !contextKeyService.evaluate(item.when)) {
      continue
    }

    const group = item.group ?? ''
    const actions = groups.get(group) ?? []

    if (isMenuItem(item)) {
      const commandId = item.command.id
      actions.push({
        id: commandId,
        label: item.command.title,
        icon: item.command.icon,
        enabled: true, // Could add precondition check later
        // Closure captures capturedArg - available when run() is called later
        run: (...runtimeArgs: unknown[]) => {
          const args: unknown[] = []
          if (capturedArg !== undefined) {
            args.push(capturedArg)
          }
          if (shouldForwardArgs) {
            args.push(...runtimeArgs)
          }
          return executeCommand(commandId, ...args)
        }
      })
    }
    // Submenus are handled separately

    groups.set(group, actions)
  }

  // Sort actions within each group by order
  for (const actions of groups.values()) {
    actions.sort((a, b) => {
      const itemA = items.find(
        (i) => isMenuItem(i) && i.command.id === a.id
      ) as (MenuEntry & { order?: number }) | undefined
      const itemB = items.find(
        (i) => isMenuItem(i) && i.command.id === b.id
      ) as (MenuEntry & { order?: number }) | undefined
      return (itemA?.order ?? Infinity) - (itemB?.order ?? Infinity)
    })
  }

  return sortGroups(groups)
}

/**
 * Get submenus for a menu, filtered by context
 */
export function getMenuSubmenus(
  menuId: MenuId,
  contextKeyService: IContextKeyService
): IMenuSubmenu[] {
  const items = menuRegistry.getMenuItems(menuId)
  const submenus: IMenuSubmenu[] = []

  for (const item of items) {
    if (!isSubmenuItem(item)) continue

    // Evaluate "when" clause
    if (item.when && !contextKeyService.evaluate(item.when)) {
      continue
    }

    submenus.push({
      id: item.submenu.id,
      label: item.title,
      icon: item.icon,
      menuId: item.submenu
    })
  }

  return submenus
}

/**
 * Check if a menu has any visible items for the given context
 */
export function hasVisibleMenuItems(
  menuId: MenuId,
  contextKeyService: IContextKeyService
): boolean {
  const items = menuRegistry.getMenuItems(menuId)

  for (const item of items) {
    // If no when clause, item is visible
    if (!item.when) {
      return true
    }
    // If when clause evaluates to true, item is visible
    if (contextKeyService.evaluate(item.when)) {
      return true
    }
  }

  return false
}

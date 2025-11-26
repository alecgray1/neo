// Commands quick access provider ("> " prefix)

import type { IQuickAccessProvider, IQuickAccessItem } from '../types'
import { getCommandRegistry } from '$lib/commands/registry'
import { getKeybindingService } from '$lib/keybindings/registry'
import { formatCommandTitle } from '$lib/commands/types'
import { filterItems } from '../filter'

/**
 * Commands provider - lists available commands
 * Activated by ">" prefix
 */
export class CommandsQuickAccessProvider implements IQuickAccessProvider {
  readonly prefix = '>'
  readonly placeholder = 'Type a command name...'

  private _mruCommands: string[] = []
  private readonly MRU_KEY = 'neo.quickaccess.commands.mru'
  private readonly MRU_LIMIT = 50

  constructor() {
    this._loadMru()
  }

  private _loadMru(): void {
    try {
      const stored = localStorage.getItem(this.MRU_KEY)
      if (stored) {
        this._mruCommands = JSON.parse(stored)
      }
    } catch {
      this._mruCommands = []
    }
  }

  private _saveMru(): void {
    try {
      localStorage.setItem(this.MRU_KEY, JSON.stringify(this._mruCommands))
    } catch {
      // Ignore storage errors
    }
  }

  private _addToMru(commandId: string): void {
    // Remove if exists
    const index = this._mruCommands.indexOf(commandId)
    if (index >= 0) {
      this._mruCommands.splice(index, 1)
    }
    // Add to front
    this._mruCommands.unshift(commandId)
    // Trim to limit
    if (this._mruCommands.length > this.MRU_LIMIT) {
      this._mruCommands = this._mruCommands.slice(0, this.MRU_LIMIT)
    }
    this._saveMru()
  }

  provide(filter: string): IQuickAccessItem[] {
    const registry = getCommandRegistry()
    const keybindingService = getKeybindingService()

    // Get available commands
    const commands = registry.getAvailableMeta()

    // Build items
    const items = commands.map((cmd) => {
      const keybinding = keybindingService.getKeybindingLabel(cmd.id)
      const mruIndex = this._mruCommands.indexOf(cmd.id)

      return {
        id: cmd.id,
        label: formatCommandTitle(cmd),
        description: cmd.category,
        icon: cmd.icon,
        keybinding,
        data: cmd,
        // MRU items get negative order (appear first)
        order: mruIndex >= 0 ? -1000 + mruIndex : 0
      }
    })

    // Filter if there's a query - only match on label
    if (filter.trim()) {
      const filtered = filterItems(
        items,
        filter,
        (item) => item.label
      )

      return filtered.map(({ item, highlights }) => ({
        ...item,
        labelHighlights: highlights
      }))
    }

    // No filter - sort by MRU then alphabetically
    items.sort((a, b) => {
      if (a.order !== b.order) {
        return a.order - b.order
      }
      return a.label.localeCompare(b.label)
    })

    return items
  }

  accept(item: IQuickAccessItem): void {
    const registry = getCommandRegistry()
    this._addToMru(item.id)
    registry.execute(item.id).catch((error) => {
      console.error(`Failed to execute command "${item.id}":`, error)
    })
  }
}

/**
 * Create and return the commands provider
 */
export function createCommandsProvider(): IQuickAccessProvider {
  return new CommandsQuickAccessProvider()
}

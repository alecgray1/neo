// Keybindings editor model - data layer for the keybindings editor UI

import type { IHighlight } from '$lib/quickaccess/types'
import type { IKeybindingEntry, KeybindingSource, IParsedKeybinding } from './types'
import { getKeybindingService } from './registry'
import { getCommandRegistry } from '$lib/commands/registry'
import { formatCommandTitle } from '$lib/commands/types'
import { formatKeybindingForDisplay, parseKeybinding, keybindingsMatch } from './parser'
import { isMac } from './types'
import { fuzzyMatch } from '$lib/quickaccess/filter'

/**
 * Keybinding item entry for display in the editor
 */
export interface IKeybindingItemEntry {
  id: string
  commandId: string
  commandLabel: string
  commandCategory?: string
  keybinding: string | null // Raw keybinding string
  keybindingLabel: string | null // Formatted for display
  when: string | null
  source: KeybindingSource
  weight: number

  // Search match highlights
  commandMatches?: IHighlight[]
  keybindingMatches?: IHighlight[]
  whenMatches?: IHighlight[]
}

/**
 * Sort mode for keybindings
 */
export type SortMode = 'command' | 'precedence'

/**
 * Search filter type
 */
export interface ISearchFilter {
  type: 'text' | 'command' | 'source' | 'keybinding' | 'when'
  value: string
}

/**
 * Parse search query into filters
 * Supports: @command:id, @source:user, @keybinding:key, @when:expr, or plain text
 */
function parseSearchQuery(query: string): ISearchFilter[] {
  const filters: ISearchFilter[] = []
  const parts = query.split(/\s+/)

  for (const part of parts) {
    if (!part) continue

    if (part.startsWith('@command:')) {
      filters.push({ type: 'command', value: part.slice(9) })
    } else if (part.startsWith('@source:')) {
      filters.push({ type: 'source', value: part.slice(8).toLowerCase() })
    } else if (part.startsWith('@keybinding:')) {
      filters.push({ type: 'keybinding', value: part.slice(12) })
    } else if (part.startsWith('@when:')) {
      filters.push({ type: 'when', value: part.slice(6) })
    } else {
      // Plain text search
      filters.push({ type: 'text', value: part })
    }
  }

  return filters
}

/**
 * Keybindings editor model
 */
export class KeybindingsEditorModel {
  private _allEntries: IKeybindingItemEntry[] = []
  private _filteredEntries: IKeybindingItemEntry[] = []
  private _sortMode: SortMode = 'command'
  private _searchQuery = ''

  constructor() {
    this.refresh()
  }

  get entries(): IKeybindingItemEntry[] {
    return this._filteredEntries
  }

  get sortMode(): SortMode {
    return this._sortMode
  }

  get searchQuery(): string {
    return this._searchQuery
  }

  /**
   * Refresh entries from the keybinding service
   */
  refresh(): void {
    const keybindingService = getKeybindingService()
    const commandRegistry = getCommandRegistry()
    const mac = isMac()

    const keybindingEntries = keybindingService.getAllEntries()
    const commands = commandRegistry.getAvailableMeta()

    // Create a map of command ID to keybinding entries
    const keybindingsByCommand = new Map<string, IKeybindingEntry[]>()
    for (const entry of keybindingEntries) {
      if (!keybindingsByCommand.has(entry.commandId)) {
        keybindingsByCommand.set(entry.commandId, [])
      }
      keybindingsByCommand.get(entry.commandId)!.push(entry)
    }

    // Build entries for all commands
    this._allEntries = []

    for (const cmd of commands) {
      const keybindings = keybindingsByCommand.get(cmd.id) || []

      if (keybindings.length === 0) {
        // Command with no keybinding
        this._allEntries.push({
          id: `${cmd.id}-unbound`,
          commandId: cmd.id,
          commandLabel: formatCommandTitle(cmd),
          commandCategory: cmd.category,
          keybinding: null,
          keybindingLabel: null,
          when: null,
          source: 'default',
          weight: 999
        })
      } else {
        // Add entry for each keybinding
        for (const kb of keybindings) {
          const keyStr = mac && kb.mac ? kb.mac : kb.key
          this._allEntries.push({
            id: kb.id,
            commandId: cmd.id,
            commandLabel: formatCommandTitle(cmd),
            commandCategory: cmd.category,
            keybinding: keyStr,
            keybindingLabel: formatKeybindingForDisplay(keyStr),
            when: kb.when || null,
            source: kb.source,
            weight: kb.weight
          })
        }
      }
    }

    // Apply current filter and sort
    this._applyFilterAndSort()
  }

  /**
   * Filter entries by text search
   */
  filter(query: string): void {
    this._searchQuery = query
    this._applyFilterAndSort()
  }

  /**
   * Filter entries by pressed keybinding (record mode)
   */
  filterByKeybinding(parsed: IParsedKeybinding): void {
    this._filteredEntries = []

    for (const entry of this._allEntries) {
      if (!entry.keybinding) continue

      const entryParsed = parseKeybinding(entry.keybinding)
      if (keybindingsMatch(parsed, entryParsed)) {
        this._filteredEntries.push({
          ...entry,
          keybindingMatches: [{ start: 0, end: entry.keybindingLabel?.length || 0 }]
        })
      }
    }

    this._applySort()
  }

  /**
   * Set sort mode
   */
  setSortMode(mode: SortMode): void {
    this._sortMode = mode
    this._applySort()
  }

  /**
   * Apply filter and sort to entries
   */
  private _applyFilterAndSort(): void {
    if (!this._searchQuery.trim()) {
      // No filter - show all entries
      this._filteredEntries = this._allEntries.map((e) => ({ ...e }))
    } else {
      // Parse and apply filters
      const filters = parseSearchQuery(this._searchQuery)
      this._filteredEntries = []

      for (const entry of this._allEntries) {
        let matches = true
        let commandMatches: IHighlight[] | undefined
        let keybindingMatches: IHighlight[] | undefined
        let whenMatches: IHighlight[] | undefined

        for (const filter of filters) {
          switch (filter.type) {
            case 'command': {
              // Exact command ID filter
              if (!entry.commandId.toLowerCase().includes(filter.value.toLowerCase())) {
                matches = false
              }
              break
            }

            case 'source': {
              // Source filter (default, user, extension)
              if (entry.source !== filter.value) {
                matches = false
              }
              break
            }

            case 'keybinding': {
              // Keybinding filter
              if (!entry.keybinding) {
                matches = false
              } else {
                const kbMatch = fuzzyMatch(filter.value, entry.keybinding)
                if (!kbMatch) {
                  matches = false
                } else {
                  keybindingMatches = kbMatch.highlights
                }
              }
              break
            }

            case 'when': {
              // When clause filter
              if (!entry.when) {
                matches = false
              } else {
                const whenMatch = fuzzyMatch(filter.value, entry.when)
                if (!whenMatch) {
                  matches = false
                } else {
                  whenMatches = whenMatch.highlights
                }
              }
              break
            }

            case 'text': {
              // General text search - search across command label, ID, keybinding, when
              let found = false

              // Search command label
              const labelMatch = fuzzyMatch(filter.value, entry.commandLabel)
              if (labelMatch) {
                found = true
                commandMatches = labelMatch.highlights
              }

              // Search command ID
              if (!found) {
                const idMatch = fuzzyMatch(filter.value, entry.commandId)
                if (idMatch) {
                  found = true
                }
              }

              // Search keybinding
              if (!found && entry.keybinding) {
                const kbMatch = fuzzyMatch(filter.value, entry.keybinding)
                if (kbMatch) {
                  found = true
                  keybindingMatches = kbMatch.highlights
                }
              }

              // Search when clause
              if (!found && entry.when) {
                const whenMatch = fuzzyMatch(filter.value, entry.when)
                if (whenMatch) {
                  found = true
                  whenMatches = whenMatch.highlights
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
          this._filteredEntries.push({
            ...entry,
            commandMatches,
            keybindingMatches,
            whenMatches
          })
        }
      }
    }

    this._applySort()
  }

  /**
   * Apply sort to filtered entries
   */
  private _applySort(): void {
    if (this._sortMode === 'command') {
      // Sort alphabetically by command label
      this._filteredEntries.sort((a, b) => a.commandLabel.localeCompare(b.commandLabel))
    } else {
      // Sort by weight (precedence) - lower weight first
      this._filteredEntries.sort((a, b) => {
        if (a.weight !== b.weight) {
          return a.weight - b.weight
        }
        return a.commandLabel.localeCompare(b.commandLabel)
      })
    }
  }

  /**
   * Find conflicts for a given keybinding
   */
  findConflicts(key: string, excludeCommandId?: string): IKeybindingItemEntry[] {
    const parsed = parseKeybinding(key)
    const conflicts: IKeybindingItemEntry[] = []

    for (const entry of this._allEntries) {
      if (!entry.keybinding) continue
      if (excludeCommandId && entry.commandId === excludeCommandId) continue

      const entryParsed = parseKeybinding(entry.keybinding)
      if (keybindingsMatch(parsed, entryParsed)) {
        conflicts.push(entry)
      }
    }

    return conflicts
  }
}

/**
 * Create a new keybindings editor model
 */
export function createKeybindingsEditorModel(): KeybindingsEditorModel {
  return new KeybindingsEditorModel()
}

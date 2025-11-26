// Keybinding registry - manages keybinding registrations

import { createServiceId, type IDisposable, Emitter, type Event } from '$lib/services/types'
import { getContextKeyService } from '$lib/services/context'
import type {
  IKeybindingService,
  IKeybindingRegistration,
  IParsedKeybinding,
  IKeybindingEntry,
  IUserKeybinding,
  KeybindingSource
} from './types'
import { isMac, KeybindingWeight, USER_KEYBINDING_WEIGHT } from './types'
import {
  parseKeybinding,
  normalizeKeybindingStr,
  keyboardEventToKeybinding,
  keybindingsMatch,
  formatKeybindingForDisplay
} from './parser'

/**
 * Keybinding service identifier
 */
export const IKeybindingServiceId = createServiceId<IKeybindingService>('IKeybindingService')

/**
 * Internal keybinding with weight info
 */
interface IInternalKeybinding extends IKeybindingRegistration {
  id: string
  weight: number
  source: KeybindingSource
  isRemoval?: boolean
}

/**
 * Keybinding registry implementation with weight-based precedence
 */
class KeybindingRegistry implements IKeybindingService, IDisposable {
  private _defaultBindings: IInternalKeybinding[] = []
  private _userBindings: IInternalKeybinding[] = []
  private _onDidChange = new Emitter<void>()
  private _parsedCache = new Map<string, IParsedKeybinding>()
  private _idCounter = 0

  get onDidChange(): Event<void> {
    return this._onDidChange.event
  }

  private _generateId(): string {
    return `kb-${++this._idCounter}`
  }

  register(
    commandId: string,
    keybinding: {
      key: string
      mac?: string
      when?: string
      args?: unknown[]
      weight?: KeybindingWeight
    }
  ): IDisposable {
    const registration: IInternalKeybinding = {
      id: this._generateId(),
      commandId,
      key: normalizeKeybindingStr(keybinding.key),
      mac: keybinding.mac ? normalizeKeybindingStr(keybinding.mac) : undefined,
      when: keybinding.when,
      args: keybinding.args,
      weight: keybinding.weight ?? KeybindingWeight.WorkbenchContrib,
      source: 'default'
    }

    this._defaultBindings.push(registration)
    this._onDidChange.fire()

    return {
      dispose: () => {
        const index = this._defaultBindings.indexOf(registration)
        if (index >= 0) {
          this._defaultBindings.splice(index, 1)
          this._onDidChange.fire()
        }
      }
    }
  }

  setUserKeybindings(keybindings: IUserKeybinding[]): void {
    // Clear existing user bindings
    this._userBindings = []

    for (const kb of keybindings) {
      const isRemoval = kb.command.startsWith('-')
      const commandId = isRemoval ? kb.command.slice(1) : kb.command

      const registration: IInternalKeybinding = {
        id: this._generateId(),
        commandId,
        key: normalizeKeybindingStr(kb.key),
        when: kb.when,
        args: kb.args ? [kb.args] : undefined,
        weight: USER_KEYBINDING_WEIGHT,
        source: 'user',
        isRemoval
      }

      this._userBindings.push(registration)
    }

    this._onDidChange.fire()
  }

  /**
   * Get all bindings merged, with user overrides and removals applied
   */
  private _getMergedBindings(): IInternalKeybinding[] {
    const result: IInternalKeybinding[] = []
    const mac = isMac()

    // Build a set of removals from user bindings
    const removals = new Set<string>()
    for (const ub of this._userBindings) {
      if (ub.isRemoval) {
        // Key for removal: commandId + keyStr + when
        const keyStr = mac && ub.mac ? ub.mac : ub.key
        const removalKey = `${ub.commandId}|${keyStr}|${ub.when || ''}`
        removals.add(removalKey)
      }
    }

    // Add default bindings that aren't removed
    for (const db of this._defaultBindings) {
      const keyStr = mac && db.mac ? db.mac : db.key
      const removalKey = `${db.commandId}|${keyStr}|${db.when || ''}`

      // Also check removal without when clause (removes all instances)
      const removalKeyNoWhen = `${db.commandId}|${keyStr}|`

      if (!removals.has(removalKey) && !removals.has(removalKeyNoWhen)) {
        result.push(db)
      }
    }

    // Add non-removal user bindings
    for (const ub of this._userBindings) {
      if (!ub.isRemoval) {
        result.push(ub)
      }
    }

    return result
  }

  resolve(event: KeyboardEvent): { commandId: string; args?: unknown[] } | undefined {
    const eventBinding = keyboardEventToKeybinding(event)
    const contextKeyService = getContextKeyService()
    const mac = isMac()

    // Get all merged bindings
    const bindings = this._getMergedBindings()

    // Find all matching bindings
    const matches: IInternalKeybinding[] = []

    for (const registration of bindings) {
      // Use mac key if on Mac and available
      const keyStr = mac && registration.mac ? registration.mac : registration.key

      // Get or parse the keybinding
      let parsed = this._parsedCache.get(keyStr)
      if (!parsed) {
        parsed = parseKeybinding(keyStr)
        this._parsedCache.set(keyStr, parsed)
      }

      // Check if keys match
      if (!keybindingsMatch(eventBinding, parsed)) {
        continue
      }

      // Check "when" condition
      if (registration.when && !contextKeyService.evaluate(registration.when)) {
        continue
      }

      matches.push(registration)
    }

    if (matches.length === 0) {
      return undefined
    }

    // Sort by weight (lower wins), then by registration order (later wins for same weight)
    matches.sort((a, b) => {
      if (a.weight !== b.weight) {
        return a.weight - b.weight // Lower weight = higher priority
      }
      // For same weight, user bindings registered later take precedence
      // Since user bindings are added after defaults, they naturally come later
      return 0
    })

    const winner = matches[0]
    return {
      commandId: winner.commandId,
      args: winner.args
    }
  }

  getKeybindings(commandId: string): IKeybindingRegistration[] {
    const bindings = this._getMergedBindings()
    return bindings.filter((b) => b.commandId === commandId)
  }

  getKeybindingLabel(commandId: string): string | undefined {
    const bindings = this.getKeybindings(commandId)
    if (bindings.length === 0) return undefined

    // Sort by weight to get highest priority binding
    const sorted = [...bindings].sort((a, b) => (a.weight ?? 200) - (b.weight ?? 200))
    const binding = sorted[0]
    const mac = isMac()
    const keyStr = mac && binding.mac ? binding.mac : binding.key

    return formatKeybindingForDisplay(keyStr)
  }

  getAllEntries(): IKeybindingEntry[] {
    const bindings = this._getMergedBindings()

    return bindings.map((b) => ({
      id: b.id,
      commandId: b.commandId,
      key: b.key,
      mac: b.mac,
      when: b.when,
      args: b.args,
      weight: b.weight,
      source: b.source,
      isRemoval: b.isRemoval
    }))
  }

  dispose(): void {
    this._onDidChange.dispose()
    this._defaultBindings = []
    this._userBindings = []
    this._parsedCache.clear()
  }
}

// Global keybinding registry instance
let _globalKeybindingRegistry: KeybindingRegistry | null = null

/**
 * Get or create the global keybinding registry
 */
export function getKeybindingService(): IKeybindingService {
  if (!_globalKeybindingRegistry) {
    _globalKeybindingRegistry = new KeybindingRegistry()
  }
  return _globalKeybindingRegistry
}

/**
 * Reset the global keybinding registry (useful for testing)
 */
export function resetKeybindingService(): void {
  _globalKeybindingRegistry?.dispose()
  _globalKeybindingRegistry = null
}

/**
 * Convenience function to register a keybinding
 */
export function registerKeybinding(
  commandId: string,
  keybinding: { key: string; mac?: string; when?: string; args?: unknown[] }
): IDisposable {
  return getKeybindingService().register(commandId, keybinding)
}

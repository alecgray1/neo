// Keybinding registry - manages keybinding registrations

import { createServiceId, type IDisposable, DisposableStore, Emitter, type Event } from '$lib/services/types'
import { getContextKeyService } from '$lib/services/context'
import type { IKeybindingService, IKeybindingRegistration, IParsedKeybinding } from './types'
import { isMac } from './types'
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
 * Keybinding registry implementation
 */
class KeybindingRegistry implements IKeybindingService, IDisposable {
  private _bindings: IKeybindingRegistration[] = []
  private _onDidChange = new Emitter<void>()
  private _parsedCache = new Map<string, IParsedKeybinding>()

  get onDidChange(): Event<void> {
    return this._onDidChange.event
  }

  register(
    commandId: string,
    keybinding: { key: string; mac?: string; when?: string; args?: unknown[] }
  ): IDisposable {
    const registration: IKeybindingRegistration = {
      commandId,
      key: normalizeKeybindingStr(keybinding.key),
      mac: keybinding.mac ? normalizeKeybindingStr(keybinding.mac) : undefined,
      when: keybinding.when,
      args: keybinding.args
    }

    this._bindings.push(registration)
    this._onDidChange.fire()

    return {
      dispose: () => {
        const index = this._bindings.indexOf(registration)
        if (index >= 0) {
          this._bindings.splice(index, 1)
          this._onDidChange.fire()
        }
      }
    }
  }

  resolve(event: KeyboardEvent): { commandId: string; args?: unknown[] } | undefined {
    const eventBinding = keyboardEventToKeybinding(event)
    const contextKeyService = getContextKeyService()
    const mac = isMac()

    // Find matching binding (last registered wins for duplicates)
    for (let i = this._bindings.length - 1; i >= 0; i--) {
      const registration = this._bindings[i]

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

      return {
        commandId: registration.commandId,
        args: registration.args
      }
    }

    return undefined
  }

  getKeybindings(commandId: string): IKeybindingRegistration[] {
    return this._bindings.filter(b => b.commandId === commandId)
  }

  getKeybindingLabel(commandId: string): string | undefined {
    const bindings = this.getKeybindings(commandId)
    if (bindings.length === 0) return undefined

    const binding = bindings[0]
    const mac = isMac()
    const keyStr = mac && binding.mac ? binding.mac : binding.key

    return formatKeybindingForDisplay(keyStr)
  }

  dispose(): void {
    this._onDidChange.dispose()
    this._bindings = []
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

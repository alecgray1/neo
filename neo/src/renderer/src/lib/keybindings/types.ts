// Keybinding types

import type { IDisposable, Event } from '$lib/services/types'

/**
 * Parsed keybinding representation
 */
export interface IParsedKeybinding {
  ctrl: boolean
  alt: boolean
  shift: boolean
  meta: boolean
  key: string // Lowercase key name
}

/**
 * Keybinding registration
 */
export interface IKeybindingRegistration {
  commandId: string
  key: string // Normalized key string
  mac?: string // Mac-specific key string
  when?: string // Context expression
  args?: unknown[]
}

/**
 * Keybinding service interface
 */
export interface IKeybindingService {
  /** Register a keybinding for a command */
  register(commandId: string, keybinding: { key: string; mac?: string; when?: string; args?: unknown[] }): IDisposable
  /** Resolve a keyboard event to a command ID */
  resolve(event: KeyboardEvent): { commandId: string; args?: unknown[] } | undefined
  /** Get all keybindings for a command */
  getKeybindings(commandId: string): IKeybindingRegistration[]
  /** Get the display string for a command's keybinding */
  getKeybindingLabel(commandId: string): string | undefined
  /** Event fired when keybindings change */
  onDidChange: Event<void>
}

/**
 * Check if running on Mac
 */
export function isMac(): boolean {
  return typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform)
}

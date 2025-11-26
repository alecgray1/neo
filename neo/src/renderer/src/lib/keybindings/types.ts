// Keybinding types

import type { IDisposable, Event } from '$lib/services/types'

/**
 * Keybinding weight/priority (VS Code style)
 * Lower weight = higher priority
 */
export enum KeybindingWeight {
  EditorCore = 0,         // Core editor bindings (highest priority)
  EditorContrib = 100,    // Editor contributions
  WorkbenchContrib = 200, // Workbench features (default)
  BuiltinExtension = 300, // Built-in extensions
  ExternalExtension = 400 // Third-party extensions (lowest priority)
}

/** User keybindings always win over defaults */
export const USER_KEYBINDING_WEIGHT = -1000

/**
 * Keybinding source
 */
export type KeybindingSource = 'default' | 'user' | 'extension'

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
  weight?: KeybindingWeight // Priority (default: WorkbenchContrib)
  source?: KeybindingSource // Where this binding came from
}

/**
 * Full keybinding entry with resolved information (for editor UI)
 */
export interface IKeybindingEntry {
  id: string // Unique identifier
  commandId: string
  key: string
  mac?: string
  when?: string
  args?: unknown[]
  weight: number
  source: KeybindingSource
  isRemoval?: boolean // true if this is a "-command" removal entry
}

/**
 * User keybinding from keybindings.json
 */
export interface IUserKeybinding {
  key: string
  command: string // Command ID, or "-commandId" for removal
  when?: string
  args?: unknown
}

/**
 * Keybinding service interface
 */
export interface IKeybindingService {
  /** Register a keybinding for a command */
  register(
    commandId: string,
    keybinding: {
      key: string
      mac?: string
      when?: string
      args?: unknown[]
      weight?: KeybindingWeight
    }
  ): IDisposable

  /** Resolve a keyboard event to a command ID */
  resolve(event: KeyboardEvent): { commandId: string; args?: unknown[] } | undefined

  /** Get all keybindings for a command */
  getKeybindings(commandId: string): IKeybindingRegistration[]

  /** Get the display string for a command's keybinding */
  getKeybindingLabel(commandId: string): string | undefined

  /** Get all keybinding entries (for editor UI) */
  getAllEntries(): IKeybindingEntry[]

  /** Add user keybindings (from keybindings.json) */
  setUserKeybindings(keybindings: IUserKeybinding[]): void

  /** Event fired when keybindings change */
  onDidChange: Event<void>
}

/**
 * Check if running on Mac
 */
export function isMac(): boolean {
  return typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/.test(navigator.platform)
}

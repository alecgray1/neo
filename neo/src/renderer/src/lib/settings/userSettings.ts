/**
 * User Settings Service - manages settings.json file
 */

import { Emitter, type Event, type IDisposable } from '$lib/services/types'
import { getSettingsRegistry } from './registry'

/**
 * User settings service interface
 */
export interface IUserSettingsService {
  /** Current user settings as a map */
  readonly settings: Map<string, unknown>
  /** Event fired when settings change (includes list of changed setting IDs) */
  readonly onDidChange: Event<string[]>

  /** Initialize the service (load from file) */
  initialize(): Promise<void>
  /** Reload settings from file */
  reload(): Promise<void>
  /** Save current settings to file */
  save(): Promise<void>

  /** Get the effective value of a setting (user value or default) */
  getValue<T>(id: string): T
  /** Get only the user-set value (undefined if not set) */
  getUserValue<T>(id: string): T | undefined
  /** Set a setting value */
  setValue(id: string, value: unknown): Promise<void>
  /** Reset a setting to its default value */
  resetValue(id: string): Promise<void>
  /** Check if a setting has been modified from default */
  isModified(id: string): boolean
}

/**
 * Storage key for localStorage fallback
 */
const STORAGE_KEY = 'neo.settings.user'

/**
 * User settings service implementation
 */
class UserSettingsService implements IUserSettingsService, IDisposable {
  private _settings: Map<string, unknown> = new Map()
  private _onDidChange = new Emitter<string[]>()
  private _initialized = false

  constructor() {
    // Load from localStorage synchronously on construction
    // This ensures settings are available immediately
    this._loadFromLocalStorageSync()
  }

  private _loadFromLocalStorageSync(): void {
    try {
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        this._parseSettings(stored)
      }
    } catch (error) {
      console.error('Failed to load settings from localStorage:', error)
    }
  }

  get settings(): Map<string, unknown> {
    return new Map(this._settings)
  }

  get onDidChange(): Event<string[]> {
    return this._onDidChange.event
  }

  async initialize(): Promise<void> {
    if (this._initialized) return

    await this.reload()
    this._initialized = true
  }

  async reload(): Promise<void> {
    try {
      // Try to load from Electron IPC first (file system)
      if (window.electronAPI?.readSettingsFile) {
        const content = await window.electronAPI.readSettingsFile()
        if (content) {
          this._parseSettings(content)
          this._onDidChange.fire(Array.from(this._settings.keys()))
          return
        }
      }

      // Fall back to localStorage
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        this._parseSettings(stored)
      } else {
        this._settings.clear()
      }

      this._onDidChange.fire(Array.from(this._settings.keys()))
    } catch (error) {
      console.error('Failed to load user settings:', error)
      this._settings.clear()
    }
  }

  async save(): Promise<void> {
    try {
      const obj: Record<string, unknown> = {}
      for (const [key, value] of this._settings) {
        obj[key] = value
      }
      const content = JSON.stringify(obj, null, 2)

      // Try to save via Electron IPC first
      if (window.electronAPI?.writeSettingsFile) {
        await window.electronAPI.writeSettingsFile(content)
      } else {
        // Fall back to localStorage
        localStorage.setItem(STORAGE_KEY, content)
      }
    } catch (error) {
      console.error('Failed to save user settings:', error)
      throw error
    }
  }

  getValue<T>(id: string): T {
    // Return user value if set
    if (this._settings.has(id)) {
      return this._settings.get(id) as T
    }

    // Return default value from registry
    const registry = getSettingsRegistry()
    return registry.getDefaultValue(id) as T
  }

  getUserValue<T>(id: string): T | undefined {
    if (this._settings.has(id)) {
      return this._settings.get(id) as T
    }
    return undefined
  }

  async setValue(id: string, value: unknown): Promise<void> {
    const registry = getSettingsRegistry()
    const defaultValue = registry.getDefaultValue(id)

    // If setting to default value, remove user override
    if (this._deepEquals(value, defaultValue)) {
      if (this._settings.has(id)) {
        this._settings.delete(id)
        await this.save()
        this._onDidChange.fire([id])
      }
      return
    }

    // Set the value
    this._settings.set(id, value)
    await this.save()
    this._onDidChange.fire([id])
  }

  async resetValue(id: string): Promise<void> {
    if (this._settings.has(id)) {
      this._settings.delete(id)
      await this.save()
      this._onDidChange.fire([id])
    }
  }

  isModified(id: string): boolean {
    return this._settings.has(id)
  }

  private _parseSettings(content: string): void {
    try {
      const parsed = JSON.parse(content)
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        console.warn('settings.json should be an object')
        this._settings.clear()
        return
      }

      this._settings.clear()
      for (const [key, value] of Object.entries(parsed)) {
        this._settings.set(key, value)
      }
    } catch (error) {
      console.error('Failed to parse settings JSON:', error)
      this._settings.clear()
    }
  }

  private _deepEquals(a: unknown, b: unknown): boolean {
    if (a === b) return true
    if (typeof a !== typeof b) return false
    if (a === null || b === null) return a === b
    if (typeof a !== 'object') return a === b

    if (Array.isArray(a) && Array.isArray(b)) {
      if (a.length !== b.length) return false
      return a.every((val, i) => this._deepEquals(val, b[i]))
    }

    if (Array.isArray(a) || Array.isArray(b)) return false

    const aKeys = Object.keys(a as object)
    const bKeys = Object.keys(b as object)
    if (aKeys.length !== bKeys.length) return false

    return aKeys.every((key) =>
      this._deepEquals(
        (a as Record<string, unknown>)[key],
        (b as Record<string, unknown>)[key]
      )
    )
  }

  dispose(): void {
    this._onDidChange.dispose()
  }
}

// Global instance
let _instance: UserSettingsService | null = null

/**
 * Get the user settings service
 */
export function getUserSettingsService(): IUserSettingsService {
  if (!_instance) {
    _instance = new UserSettingsService()
  }
  return _instance
}

/**
 * Reset the user settings service (for testing)
 */
export function resetUserSettingsService(): void {
  _instance?.dispose()
  _instance = null
}

// Extend Window interface for Electron IPC
declare global {
  interface Window {
    electronAPI?: {
      readSettingsFile?: () => Promise<string | null>
      writeSettingsFile?: (content: string) => Promise<void>
      onSettingsFileChanged?: (callback: () => void) => () => void
      // ... existing keybindings methods
      readKeybindingsFile?: () => Promise<string | null>
      writeKeybindingsFile?: (content: string) => Promise<void>
      onKeybindingsFileChanged?: (callback: () => void) => () => void
    }
  }
}

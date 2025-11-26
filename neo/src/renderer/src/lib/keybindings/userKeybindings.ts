// User keybindings service - manages keybindings.json file

import { Emitter, type Event, type IDisposable } from '$lib/services/types'
import type { IUserKeybinding } from './types'
import { getKeybindingService } from './registry'

/**
 * User keybindings service interface
 */
export interface IUserKeybindingsService {
  /** Current user keybindings */
  readonly keybindings: IUserKeybinding[]
  /** Event fired when user keybindings change */
  readonly onDidChange: Event<void>

  /** Initialize the service (load from file) */
  initialize(): Promise<void>
  /** Reload keybindings from file */
  reload(): Promise<void>
  /** Save current keybindings to file */
  save(): Promise<void>

  /** Add a new keybinding */
  addKeybinding(commandId: string, key: string, when?: string): Promise<void>
  /** Edit an existing keybinding */
  editKeybinding(commandId: string, oldKey: string, newKey: string, when?: string): Promise<void>
  /** Remove a keybinding (adds removal entry for defaults) */
  removeKeybinding(commandId: string, key: string, when?: string): Promise<void>
  /** Reset a keybinding to default (remove user override) */
  resetKeybinding(commandId: string, key?: string): Promise<void>
}

/**
 * Default keybindings.json location
 * In Electron, this would be in the user data directory
 * For now, we'll use localStorage as a fallback
 */
const STORAGE_KEY = 'neo.keybindings.user'

/**
 * User keybindings service implementation
 */
class UserKeybindingsService implements IUserKeybindingsService, IDisposable {
  private _keybindings: IUserKeybinding[] = []
  private _onDidChange = new Emitter<void>()
  private _initialized = false

  get keybindings(): IUserKeybinding[] {
    return [...this._keybindings]
  }

  get onDidChange(): Event<void> {
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
      if (window.electronAPI?.readKeybindingsFile) {
        const content = await window.electronAPI.readKeybindingsFile()
        if (content) {
          this._keybindings = this._parseKeybindings(content)
          this._syncToRegistry()
          this._onDidChange.fire()
          return
        }
      }

      // Fall back to localStorage
      const stored = localStorage.getItem(STORAGE_KEY)
      if (stored) {
        this._keybindings = this._parseKeybindings(stored)
      } else {
        this._keybindings = []
      }

      this._syncToRegistry()
      this._onDidChange.fire()
    } catch (error) {
      console.error('Failed to load user keybindings:', error)
      this._keybindings = []
    }
  }

  async save(): Promise<void> {
    try {
      const content = JSON.stringify(this._keybindings, null, 2)

      // Try to save via Electron IPC first
      if (window.electronAPI?.writeKeybindingsFile) {
        await window.electronAPI.writeKeybindingsFile(content)
      } else {
        // Fall back to localStorage
        localStorage.setItem(STORAGE_KEY, content)
      }

      this._syncToRegistry()
      this._onDidChange.fire()
    } catch (error) {
      console.error('Failed to save user keybindings:', error)
      throw error
    }
  }

  async addKeybinding(commandId: string, key: string, when?: string): Promise<void> {
    const newBinding: IUserKeybinding = {
      key,
      command: commandId
    }
    if (when) {
      newBinding.when = when
    }

    this._keybindings.push(newBinding)
    await this.save()
  }

  async editKeybinding(
    commandId: string,
    oldKey: string,
    newKey: string,
    when?: string
  ): Promise<void> {
    // Find existing binding
    const index = this._keybindings.findIndex(
      (kb) => kb.command === commandId && kb.key.toLowerCase() === oldKey.toLowerCase()
    )

    if (index >= 0) {
      // Update existing
      this._keybindings[index] = {
        key: newKey,
        command: commandId,
        ...(when && { when })
      }
    } else {
      // Add new
      await this.addKeybinding(commandId, newKey, when)
      return
    }

    await this.save()
  }

  async removeKeybinding(commandId: string, key: string, when?: string): Promise<void> {
    // Check if this is a user-added binding or a default
    const userIndex = this._keybindings.findIndex(
      (kb) =>
        kb.command === commandId &&
        kb.key.toLowerCase() === key.toLowerCase() &&
        !kb.command.startsWith('-')
    )

    if (userIndex >= 0) {
      // It's a user binding - just remove it
      this._keybindings.splice(userIndex, 1)
    } else {
      // It's a default binding - add a removal entry
      const removal: IUserKeybinding = {
        key,
        command: `-${commandId}`
      }
      if (when) {
        removal.when = when
      }
      this._keybindings.push(removal)
    }

    await this.save()
  }

  async resetKeybinding(commandId: string, key?: string): Promise<void> {
    // Remove any user overrides for this command
    this._keybindings = this._keybindings.filter((kb) => {
      const cmd = kb.command.startsWith('-') ? kb.command.slice(1) : kb.command
      if (cmd !== commandId) return true
      if (key && kb.key.toLowerCase() !== key.toLowerCase()) return true
      return false
    })

    await this.save()
  }

  private _parseKeybindings(content: string): IUserKeybinding[] {
    try {
      const parsed = JSON.parse(content)
      if (!Array.isArray(parsed)) {
        console.warn('keybindings.json should be an array')
        return []
      }

      // Validate and filter valid entries
      return parsed.filter((item): item is IUserKeybinding => {
        if (typeof item !== 'object' || item === null) return false
        if (typeof item.key !== 'string') return false
        if (typeof item.command !== 'string') return false
        return true
      })
    } catch (error) {
      console.error('Failed to parse keybindings JSON:', error)
      return []
    }
  }

  private _syncToRegistry(): void {
    const service = getKeybindingService()
    service.setUserKeybindings(this._keybindings)
  }

  dispose(): void {
    this._onDidChange.dispose()
  }
}

// Global instance
let _instance: UserKeybindingsService | null = null

/**
 * Get the user keybindings service
 */
export function getUserKeybindingsService(): IUserKeybindingsService {
  if (!_instance) {
    _instance = new UserKeybindingsService()
  }
  return _instance
}

/**
 * Reset the user keybindings service (for testing)
 */
export function resetUserKeybindingsService(): void {
  _instance?.dispose()
  _instance = null
}

// Extend Window interface for Electron IPC
declare global {
  interface Window {
    electronAPI?: {
      readKeybindingsFile?: () => Promise<string | null>
      writeKeybindingsFile?: (content: string) => Promise<void>
      onKeybindingsFileChanged?: (callback: () => void) => () => void
    }
  }
}

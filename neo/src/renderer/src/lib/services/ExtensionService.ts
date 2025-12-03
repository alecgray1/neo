/**
 * Extension Service
 *
 * Manages extension contributions in the renderer process.
 * Loads contributions from the main process and registers them
 * with the appropriate registries.
 */

import { createServiceId, type IDisposable, DisposableStore, Emitter, type Event } from './types'
import { getCommandRegistry } from '$lib/commands/registry'
import { getKeybindingService } from '$lib/keybindings/registry'
import { menuRegistry } from '$lib/menus/registry'
import { MenuId } from '$lib/menus/menuId'
import { getContextKeyService } from './context'
// Types are from the preload declarations (exposed on window)
interface CommandContribution {
  id: string
  title: string
  category?: string
  icon?: string
  enablement?: string
  extensionId: string
}

interface KeybindingContribution {
  command: string
  key: string
  mac?: string
  when?: string
  extensionId: string
}

interface MenuContribution {
  command: string
  when?: string
  group?: string
  extensionId: string
}

interface ViewContainerContribution {
  id: string
  title: string
  icon: string
  extensionId: string
}

interface ViewContribution {
  id: string
  name: string
  type?: 'tree' | 'webview'
  when?: string
  icon?: string
  extensionId: string
}

interface ExtensionInfo {
  id: string
  name: string
  displayName: string
  version: string
  description: string
}

interface CollectedContributions {
  commands: CommandContribution[]
  viewsContainers: {
    activitybar: ViewContainerContribution[]
    panel: ViewContainerContribution[]
  }
  views: Record<string, ViewContribution[]>
  menus: Record<string, MenuContribution[]>
  keybindings: KeybindingContribution[]
}

export interface IExtensionService extends IDisposable {
  /**
   * Load contributions from extensions
   */
  loadContributions(): Promise<void>

  /**
   * Get all installed extensions
   */
  getExtensions(): Promise<ExtensionInfo[]>

  /**
   * Get all contributions
   */
  getContributions(): CollectedContributions | null

  /**
   * Execute an extension command
   */
  executeCommand<T = unknown>(id: string, ...args: unknown[]): Promise<T>

  /**
   * Event fired when contributions are loaded
   */
  onDidLoadContributions: Event<CollectedContributions>

  /**
   * Event fired when a new command is registered dynamically
   */
  onDidRegisterCommand: Event<string>

  /**
   * Event fired when a command is unregistered
   */
  onDidUnregisterCommand: Event<string>
}

export const IExtensionServiceId = createServiceId<IExtensionService>('IExtensionService')

class ExtensionService implements IExtensionService {
  private _contributions: CollectedContributions | null = null
  private _disposables = new DisposableStore()
  private _contributionDisposables = new DisposableStore() // Separate store for contribution registrations
  private _extensionCommands = new Set<string>()

  private _onDidLoadContributions = new Emitter<CollectedContributions>()
  private _onDidRegisterCommand = new Emitter<string>()
  private _onDidUnregisterCommand = new Emitter<string>()

  get onDidLoadContributions(): Event<CollectedContributions> {
    return this._onDidLoadContributions.event
  }

  get onDidRegisterCommand(): Event<string> {
    return this._onDidRegisterCommand.event
  }

  get onDidUnregisterCommand(): Event<string> {
    return this._onDidUnregisterCommand.event
  }

  constructor() {
    this._setupEventListeners()
  }

  private _setupEventListeners(): void {
    // Listen for dynamic command registration
    const unsubCommandReg = window.extensionAPI.onCommandRegistered(({ id }) => {
      this._extensionCommands.add(id)
      this._onDidRegisterCommand.fire(id)
    })
    this._disposables.add({ dispose: unsubCommandReg })

    const unsubCommandUnreg = window.extensionAPI.onCommandUnregistered(({ id }) => {
      this._extensionCommands.delete(id)
      this._onDidUnregisterCommand.fire(id)
    })
    this._disposables.add({ dispose: unsubCommandUnreg })

    // Listen for context changes from extensions
    const unsubContext = window.extensionAPI.onContextSet(({ key, value }) => {
      const contextKeyService = getContextKeyService()
      contextKeyService.setContext(key, value)
    })
    this._disposables.add({ dispose: unsubContext })

    // Listen for extension reload to refresh contributions
    const unsubReloaded = window.extensionAPI.onExtensionReloaded?.(() => {
      console.log('[ExtensionService] Extension reloaded, refreshing contributions...')
      this.loadContributions()
    })
    if (unsubReloaded) {
      this._disposables.add({ dispose: unsubReloaded })
    }
  }

  async loadContributions(): Promise<void> {
    try {
      // Clear old contribution registrations before loading new ones
      this._contributionDisposables.clear()
      this._extensionCommands.clear()

      this._contributions = await window.extensionAPI.getContributions()
      console.log('[ExtensionService] Loaded contributions:', this._contributions)

      // Register commands
      this._registerCommands(this._contributions.commands)

      // Register keybindings
      this._registerKeybindings(this._contributions.keybindings)

      // Register menus
      this._registerMenus(this._contributions.menus)

      // Note: View containers and views are handled by the layout system
      // They read contributions directly when needed

      this._onDidLoadContributions.fire(this._contributions)
    } catch (err) {
      console.error('[ExtensionService] Failed to load contributions:', err)
    }
  }

  private _registerCommands(commands: CommandContribution[]): void {
    const commandRegistry = getCommandRegistry()

    for (const cmd of commands) {
      // Register command metadata (the handler is in the extension host)
      const disposable = commandRegistry.register({
        id: cmd.id,
        title: cmd.title,
        category: cmd.category,
        icon: cmd.icon,
        when: cmd.enablement,
        handler: async (_accessor, ...args) => {
          // Forward to extension host
          return window.extensionAPI.executeCommand(cmd.id, ...args)
        }
      })
      this._contributionDisposables.add(disposable)
      this._extensionCommands.add(cmd.id)
    }
  }

  private _registerKeybindings(keybindings: KeybindingContribution[]): void {
    const keybindingService = getKeybindingService()

    for (const kb of keybindings) {
      const disposable = keybindingService.register(kb.command, {
        key: kb.key,
        mac: kb.mac,
        when: kb.when
      })
      this._contributionDisposables.add(disposable)
    }
  }

  private _registerMenus(menus: Record<string, MenuContribution[]>): void {
    const commandRegistry = getCommandRegistry()

    for (const [menuIdStr, items] of Object.entries(menus)) {
      const menuId = MenuId.getOrCreate(menuIdStr)

      for (const item of items) {
        // Look up command metadata
        const cmd = commandRegistry.getMeta(item.command)
        if (!cmd) {
          console.warn(`[ExtensionService] Menu item references unknown command: ${item.command}`)
          continue
        }

        const disposable = menuRegistry.appendMenuItem(menuId, {
          command: {
            id: item.command,
            title: cmd.title,
            icon: cmd.icon
          },
          when: item.when,
          group: item.group
        })
        this._contributionDisposables.add(disposable)
      }
    }
  }

  async getExtensions(): Promise<ExtensionInfo[]> {
    return window.extensionAPI.getExtensions()
  }

  getContributions(): CollectedContributions | null {
    return this._contributions
  }

  async executeCommand<T = unknown>(id: string, ...args: unknown[]): Promise<T> {
    if (this._extensionCommands.has(id)) {
      return window.extensionAPI.executeCommand(id, ...args)
    }
    // Fall back to local command registry
    const commandRegistry = getCommandRegistry()
    return commandRegistry.execute(id, ...args) as Promise<T>
  }

  dispose(): void {
    this._contributionDisposables.dispose()
    this._disposables.dispose()
    this._onDidLoadContributions.dispose()
    this._onDidRegisterCommand.dispose()
    this._onDidUnregisterCommand.dispose()
  }
}

// Singleton instance
let _extensionService: ExtensionService | null = null

export function getExtensionService(): IExtensionService {
  if (!_extensionService) {
    _extensionService = new ExtensionService()
  }
  return _extensionService
}

export function resetExtensionService(): void {
  _extensionService?.dispose()
  _extensionService = null
}

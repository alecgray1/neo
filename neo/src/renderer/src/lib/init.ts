// Application initialization - sets up command system, keybindings, and quick access

import type { IDisposable } from './services/types'
import { DisposableStore } from './services/types'
import { registerBuiltinCommands } from './commands/builtin'
import { startKeybindingListener } from './keybindings/listener'
import { registerQuickAccessProvider } from './quickaccess/registry'
import { createCommandsProvider, createHelpProvider } from './quickaccess'
import { registerBuiltinMenus } from './menus/builtinMenus'
import { serverStore } from './stores/server.svelte'
import { getExtensionService } from './services/ExtensionService'
import { getUserSettingsService } from './settings/userSettings'
import { getContextKeyService } from './services/context'

let _initialized = false
let _disposables: DisposableStore | null = null

/**
 * Initialize the application command system
 * Call this once at app startup
 */
export function initializeCommandSystem(): IDisposable {
  if (_initialized) {
    console.warn('Command system already initialized')
    return { dispose: () => {} }
  }

  _disposables = new DisposableStore()

  // Register built-in commands
  _disposables.add(registerBuiltinCommands())

  // Register built-in menus
  registerBuiltinMenus()

  // Register quick access providers
  _disposables.add(registerQuickAccessProvider(createCommandsProvider()))
  _disposables.add(registerQuickAccessProvider(createHelpProvider()))

  // Start keybinding listener
  _disposables.add(startKeybindingListener())

  // Initialize server store
  serverStore.init()
  _disposables.add({ dispose: () => serverStore.destroy() })

  // Load extension contributions when extension host is ready
  const extensionService = getExtensionService()
  _disposables.add(extensionService)

  // Listen for extensions:ready event from main process
  const unsubReady = window.extensionAPI?.onExtensionsReady?.(() => {
    console.log('[Init] Extensions ready, loading contributions...')
    extensionService.loadContributions().catch((err) => {
      console.error('Failed to load extension contributions:', err)
    })
  })
  if (unsubReady) {
    _disposables.add({ dispose: unsubReady })
  } else {
    // Fallback: if API not available, try loading after a delay
    setTimeout(() => {
      extensionService.loadContributions().catch((err) => {
        console.error('Failed to load extension contributions:', err)
      })
    }, 1000)
  }

  // Initialize developer mode context key
  _disposables.add(initializeDeveloperModeContext())

  _initialized = true

  console.log('Command system initialized')

  return {
    dispose: () => {
      _disposables?.dispose()
      _disposables = null
      _initialized = false
    }
  }
}

/**
 * Check if the command system is initialized
 */
export function isCommandSystemInitialized(): boolean {
  return _initialized
}

/**
 * Initialize the isDeveloperMode context key based on the developer.devMode setting
 * and watch for changes to keep them in sync
 */
function initializeDeveloperModeContext(): IDisposable {
  const contextKeyService = getContextKeyService()
  const userSettings = getUserSettingsService()

  // Set initial value
  const initialValue = userSettings.getValue<boolean>('developer.devMode') ?? false
  console.log('[Init] Developer mode initial value:', initialValue)
  contextKeyService.set('isDeveloperMode', initialValue)

  // Notify main process of initial dev mode state (for ExtensionDevServer on startup)
  // Always call setDevMode to ensure main process is in sync
  if (window.developerAPI?.setDevMode) {
    console.log('[Init] Notifying main process of dev mode:', initialValue)
    window.developerAPI.setDevMode(initialValue)
  }

  // Watch for setting changes
  const subscription = userSettings.onDidChange((changedIds) => {
    if (changedIds.includes('developer.devMode')) {
      const newValue = userSettings.getValue<boolean>('developer.devMode') ?? false
      contextKeyService.set('isDeveloperMode', newValue)

      // Notify main process of dev mode change (for ExtensionDevServer)
      if (window.developerAPI?.setDevMode) {
        window.developerAPI.setDevMode(newValue)
      }
    }
  })

  return {
    dispose: () => {
      subscription.dispose()
      contextKeyService.delete('isDeveloperMode')
    }
  }
}

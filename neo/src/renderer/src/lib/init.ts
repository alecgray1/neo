// Application initialization - sets up command system, keybindings, and quick access

import type { IDisposable } from './services/types'
import { DisposableStore } from './services/types'
import { registerBuiltinCommands } from './commands/builtin'
import { startKeybindingListener } from './keybindings/listener'
import { registerQuickAccessProvider } from './quickaccess/registry'
import { createCommandsProvider, createHelpProvider } from './quickaccess'
import { registerBuiltinMenus } from './menus/builtinMenus'

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

// Built-in commands registration

import type { IDisposable } from '$lib/services/types'
import { DisposableStore } from '$lib/services/types'
import { getCommandRegistry } from '../registry'
import { registerKeybinding } from '$lib/keybindings/registry'

import { viewCommands } from './view'
import { fileCommands } from './file'
import { editCommands } from './edit'
import { tabCommands } from './tab'
import { explorerCommands } from './explorer'
import { developerCommands } from './developer'

/**
 * All built-in commands
 */
export const builtinCommands = [...viewCommands, ...fileCommands, ...editCommands, ...tabCommands, ...explorerCommands, ...developerCommands]

/**
 * Register all built-in commands
 */
export function registerBuiltinCommands(): IDisposable {
  const disposables = new DisposableStore()
  const registry = getCommandRegistry()

  for (const command of builtinCommands) {
    // Register the command
    disposables.add(registry.register(command))

    // Register keybinding if present
    if (command.keybinding) {
      disposables.add(
        registerKeybinding(command.id, {
          key: command.keybinding.key,
          mac: command.keybinding.mac,
          when: command.when,
          args: command.keybinding.args
        })
      )
    }
  }

  return disposables
}

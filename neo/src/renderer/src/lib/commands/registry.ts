// Command registry - central registry for all commands

import { createServiceId, type IDisposable, DisposableStore, Emitter, type Event } from '$lib/services/types'
import { createServiceAccessor } from '$lib/services/container'
import { getContextKeyService } from '$lib/services/context'
import type { ICommand, ICommandMeta, ICommandRegistry } from './types'
import { commandToMeta } from './types'

/**
 * Command registry service identifier
 */
export const ICommandRegistryService = createServiceId<ICommandRegistry>('ICommandRegistry')

/**
 * Command registry implementation
 */
class CommandRegistry implements ICommandRegistry, IDisposable {
  private _commands = new Map<string, ICommand>()
  private _onDidRegister = new Emitter<string>()
  private _onDidUnregister = new Emitter<string>()

  get onDidRegister(): Event<string> {
    return this._onDidRegister.event
  }

  get onDidUnregister(): Event<string> {
    return this._onDidUnregister.event
  }

  register(command: ICommand): IDisposable {
    if (this._commands.has(command.id)) {
      console.warn(`Command "${command.id}" is already registered. Overwriting.`)
    }
    this._commands.set(command.id, command)
    this._onDidRegister.fire(command.id)

    return {
      dispose: () => {
        if (this._commands.get(command.id) === command) {
          this._commands.delete(command.id)
          this._onDidUnregister.fire(command.id)
        }
      }
    }
  }

  registerMany(commands: ICommand[]): IDisposable {
    const disposables = new DisposableStore()
    for (const command of commands) {
      disposables.add(this.register(command))
    }
    return disposables
  }

  async execute(id: string, ...args: unknown[]): Promise<unknown> {
    const command = this._commands.get(id)
    if (!command) {
      throw new Error(`Command "${id}" not found`)
    }

    // Check "when" expression
    if (command.when) {
      const contextKeyService = getContextKeyService()
      if (!contextKeyService.evaluate(command.when)) {
        console.warn(`Command "${id}" is not available in the current context`)
        return undefined
      }
    }

    // Execute with service accessor
    const accessor = createServiceAccessor()
    try {
      return await command.handler(accessor, ...args)
    } catch (error) {
      console.error(`Error executing command "${id}":`, error)
      throw error
    }
  }

  get(id: string): ICommand | undefined {
    return this._commands.get(id)
  }

  getMeta(id: string): ICommandMeta | undefined {
    const command = this._commands.get(id)
    return command ? commandToMeta(command) : undefined
  }

  getAll(): ICommand[] {
    return Array.from(this._commands.values())
  }

  getAllMeta(): ICommandMeta[] {
    return this.getAll().map(commandToMeta)
  }

  has(id: string): boolean {
    return this._commands.has(id)
  }

  /**
   * Get commands that are available in the current context
   */
  getAvailable(): ICommand[] {
    const contextKeyService = getContextKeyService()
    return this.getAll().filter((cmd) => {
      if (!cmd.when) return true
      return contextKeyService.evaluate(cmd.when)
    })
  }

  /**
   * Get metadata for commands available in current context
   */
  getAvailableMeta(): ICommandMeta[] {
    return this.getAvailable().map(commandToMeta)
  }

  dispose(): void {
    this._onDidRegister.dispose()
    this._onDidUnregister.dispose()
    this._commands.clear()
  }
}

// Global command registry instance
let _globalCommandRegistry: CommandRegistry | null = null

/**
 * Get or create the global command registry
 */
export function getCommandRegistry(): ICommandRegistry & { getAvailable(): ICommand[]; getAvailableMeta(): ICommandMeta[] } {
  if (!_globalCommandRegistry) {
    _globalCommandRegistry = new CommandRegistry()
  }
  return _globalCommandRegistry
}

/**
 * Reset the global command registry (useful for testing)
 */
export function resetCommandRegistry(): void {
  _globalCommandRegistry?.dispose()
  _globalCommandRegistry = null
}

/**
 * Convenience function to register a command
 */
export function registerCommand(command: ICommand): IDisposable {
  return getCommandRegistry().register(command)
}

/**
 * Convenience function to execute a command
 */
export function executeCommand(id: string, ...args: unknown[]): Promise<unknown> {
  return getCommandRegistry().execute(id, ...args)
}

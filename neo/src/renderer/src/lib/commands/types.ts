// Command types

import type { IServiceAccessor } from '$lib/services/container'
import type { IDisposable, Event } from '$lib/services/types'

/**
 * Keybinding definition
 */
export interface IKeybinding {
  /** Primary key combination, e.g. "ctrl+shift+p" */
  key: string
  /** Mac-specific override, e.g. "cmd+shift+p" */
  mac?: string
  /** Context expression for when this keybinding is active */
  when?: string
  /** Arguments to pass to the command handler */
  args?: unknown[]
}

/**
 * Command handler function
 * Receives a service accessor for dependency injection
 */
export type ICommandHandler = (accessor: IServiceAccessor, ...args: unknown[]) => unknown

/**
 * Command definition
 */
export interface ICommand {
  /** Unique command identifier, e.g. "neo.file.save" */
  id: string
  /** Display title for the command palette */
  title: string
  /** Category for grouping, e.g. "File", "View" */
  category?: string
  /** Icon identifier or component */
  icon?: string
  /** Default keybinding */
  keybinding?: IKeybinding
  /** Context expression for when this command is available */
  when?: string
  /** The command handler function */
  handler: ICommandHandler
}

/**
 * Command metadata (without handler) for display purposes
 */
export interface ICommandMeta {
  id: string
  title: string
  category?: string
  icon?: string
  keybinding?: IKeybinding
  when?: string
}

/**
 * Command registry interface
 */
export interface ICommandRegistry {
  /** Register a command */
  register(command: ICommand): IDisposable
  /** Register multiple commands at once */
  registerMany(commands: ICommand[]): IDisposable
  /** Execute a command by ID */
  execute(id: string, ...args: unknown[]): Promise<unknown>
  /** Get a command by ID */
  get(id: string): ICommand | undefined
  /** Get command metadata by ID */
  getMeta(id: string): ICommandMeta | undefined
  /** Get all registered commands */
  getAll(): ICommand[]
  /** Get all command metadata */
  getAllMeta(): ICommandMeta[]
  /** Check if a command is registered */
  has(id: string): boolean
  /** Event fired when a command is registered */
  onDidRegister: Event<string>
  /** Event fired when a command is unregistered */
  onDidUnregister: Event<string>
}

/**
 * Convert a command to metadata (strips the handler)
 */
export function commandToMeta(command: ICommand): ICommandMeta {
  return {
    id: command.id,
    title: command.title,
    category: command.category,
    icon: command.icon,
    keybinding: command.keybinding,
    when: command.when
  }
}

/**
 * Format a command's display title with category
 */
export function formatCommandTitle(command: ICommandMeta): string {
  if (command.category) {
    return `${command.category}: ${command.title}`
  }
  return command.title
}

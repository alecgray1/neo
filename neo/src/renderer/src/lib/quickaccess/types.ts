// Quick access types

import type { IDisposable, Event } from '$lib/services/types'

/**
 * Highlight range for fuzzy match display
 */
export interface IHighlight {
  start: number
  end: number
}

/**
 * Quick access item displayed in the palette
 */
export interface IQuickAccessItem {
  /** Unique item identifier */
  id: string
  /** Primary display text */
  label: string
  /** Secondary description text */
  description?: string
  /** Additional detail (shown below) */
  detail?: string
  /** Icon identifier */
  icon?: string
  /** Keybinding display string */
  keybinding?: string
  /** Fuzzy match highlights for the label */
  labelHighlights?: IHighlight[]
  /** Fuzzy match highlights for the description */
  descriptionHighlights?: IHighlight[]
  /** Provider-specific data */
  data?: unknown
  /** Group/category for separators */
  group?: string
  /** Sort order within group (lower = higher priority) */
  order?: number
}

/**
 * Quick access provider interface
 */
export interface IQuickAccessProvider {
  /** Prefix that activates this provider (e.g., ">" for commands) */
  readonly prefix: string
  /** Placeholder text when this provider is active */
  readonly placeholder: string

  /**
   * Provide items for the given filter
   * @param filter - User input without the prefix
   * @returns Items to display, or a promise that resolves to items
   */
  provide(filter: string): IQuickAccessItem[] | Promise<IQuickAccessItem[]>

  /**
   * Called when an item is selected/accepted
   * @param item - The selected item
   */
  accept(item: IQuickAccessItem): void

  /**
   * Optional: Called when the provider becomes active
   */
  onActivate?(): void

  /**
   * Optional: Called when the provider becomes inactive
   */
  onDeactivate?(): void
}

/**
 * Quick access registry interface
 */
export interface IQuickAccessRegistry {
  /** Register a provider */
  registerProvider(provider: IQuickAccessProvider): IDisposable
  /** Get provider for a prefix */
  getProvider(prefix: string): IQuickAccessProvider | undefined
  /** Get all registered providers */
  getProviders(): IQuickAccessProvider[]
  /** Get the default provider (empty prefix) */
  getDefaultProvider(): IQuickAccessProvider | undefined
  /** Event fired when providers change */
  onDidChange: Event<void>
}

/**
 * Quick access controller state
 */
export interface IQuickAccessState {
  /** Whether the quick access is visible */
  visible: boolean
  /** Current input value */
  value: string
  /** Active provider prefix */
  prefix: string
  /** Current items */
  items: IQuickAccessItem[]
  /** Selected item index */
  selectedIndex: number
  /** Whether loading */
  loading: boolean
  /** Placeholder text */
  placeholder: string
}

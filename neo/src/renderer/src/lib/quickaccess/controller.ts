// Quick access controller - manages the quick access state and logic

import { createServiceId, type IDisposable, DisposableStore, Emitter, type Event } from '$lib/services/types'
import type { IQuickAccessItem, IQuickAccessProvider, IQuickAccessState } from './types'
import { getQuickAccessRegistry } from './registry'
import { getContextKeyService } from '$lib/services/context'

/**
 * Quick access controller service identifier
 */
export const IQuickAccessControllerService = createServiceId<IQuickAccessController>('IQuickAccessController')

/**
 * Quick access controller interface
 */
export interface IQuickAccessController {
  /** Current state */
  readonly state: IQuickAccessState
  /** Show the quick access */
  show(initialValue?: string): void
  /** Hide the quick access */
  hide(): void
  /** Toggle visibility */
  toggle(initialValue?: string): void
  /** Update the input value */
  setValue(value: string): void
  /** Select an item by index */
  selectIndex(index: number): void
  /** Move selection up */
  selectPrevious(): void
  /** Move selection down */
  selectNext(): void
  /** Accept the current selection */
  acceptSelected(): void
  /** Event fired when state changes */
  onDidChange: Event<IQuickAccessState>
}

/**
 * Quick access controller implementation
 */
class QuickAccessController implements IQuickAccessController, IDisposable {
  private _state: IQuickAccessState = {
    visible: false,
    value: '',
    prefix: '',
    items: [],
    selectedIndex: 0,
    loading: false,
    placeholder: 'Type to search...'
  }

  private _activeProvider: IQuickAccessProvider | undefined
  private _onDidChange = new Emitter<IQuickAccessState>()
  private _disposables = new DisposableStore()

  get state(): IQuickAccessState {
    return this._state
  }

  get onDidChange(): Event<IQuickAccessState> {
    return this._onDidChange.event
  }

  show(initialValue?: string): void {
    const value = initialValue ?? ''
    this._state = {
      ...this._state,
      visible: true,
      value,
      selectedIndex: 0
    }

    // Set context key
    getContextKeyService().set('quickAccessVisible', true)

    // Process initial value
    this._processValue(value)
    this._notifyChange()
  }

  hide(): void {
    if (this._activeProvider?.onDeactivate) {
      this._activeProvider.onDeactivate()
    }
    this._activeProvider = undefined

    this._state = {
      ...this._state,
      visible: false,
      value: '',
      prefix: '',
      items: [],
      selectedIndex: 0,
      loading: false
    }

    // Clear context key
    getContextKeyService().set('quickAccessVisible', false)

    this._notifyChange()
  }

  toggle(initialValue?: string): void {
    if (this._state.visible) {
      this.hide()
    } else {
      this.show(initialValue)
    }
  }

  setValue(value: string): void {
    this._state = {
      ...this._state,
      value,
      selectedIndex: 0
    }
    this._processValue(value)
    this._notifyChange()
  }

  selectIndex(index: number): void {
    const maxIndex = Math.max(0, this._state.items.length - 1)
    this._state = {
      ...this._state,
      selectedIndex: Math.max(0, Math.min(index, maxIndex))
    }
    this._notifyChange()
  }

  selectPrevious(): void {
    this.selectIndex(this._state.selectedIndex - 1)
  }

  selectNext(): void {
    this.selectIndex(this._state.selectedIndex + 1)
  }

  acceptSelected(): void {
    const item = this._state.items[this._state.selectedIndex]
    if (item && this._activeProvider) {
      this._activeProvider.accept(item)
      this.hide()
    }
  }

  private async _processValue(value: string): Promise<void> {
    const registry = getQuickAccessRegistry()

    // Detect prefix
    let prefix = ''
    let filter = value

    // Check for known prefixes
    const providers = registry.getProviders()
    for (const provider of providers) {
      if (provider.prefix && value.startsWith(provider.prefix)) {
        prefix = provider.prefix
        filter = value.slice(prefix.length)
        break
      }
    }

    // Get the appropriate provider
    const provider = registry.getProvider(prefix) || registry.getDefaultProvider()

    // Handle provider change
    if (provider !== this._activeProvider) {
      if (this._activeProvider?.onDeactivate) {
        this._activeProvider.onDeactivate()
      }
      this._activeProvider = provider
      if (provider?.onActivate) {
        provider.onActivate()
      }
    }

    if (!provider) {
      this._state = {
        ...this._state,
        prefix,
        items: [],
        loading: false,
        placeholder: 'No provider available'
      }
      this._notifyChange()
      return
    }

    // Update placeholder
    this._state = {
      ...this._state,
      prefix,
      placeholder: provider.placeholder,
      loading: true
    }
    this._notifyChange()

    // Get items from provider
    try {
      const items = await provider.provide(filter)
      this._state = {
        ...this._state,
        items,
        loading: false,
        selectedIndex: Math.min(this._state.selectedIndex, Math.max(0, items.length - 1))
      }
    } catch (error) {
      console.error('Quick access provider error:', error)
      this._state = {
        ...this._state,
        items: [],
        loading: false
      }
    }
    this._notifyChange()
  }

  private _notifyChange(): void {
    this._onDidChange.fire(this._state)
  }

  dispose(): void {
    this._disposables.dispose()
    this._onDidChange.dispose()
  }
}

// Global quick access controller instance
let _globalQuickAccessController: QuickAccessController | null = null

/**
 * Get or create the global quick access controller
 */
export function getQuickAccessController(): IQuickAccessController {
  if (!_globalQuickAccessController) {
    _globalQuickAccessController = new QuickAccessController()
  }
  return _globalQuickAccessController
}

/**
 * Reset the global quick access controller (useful for testing)
 */
export function resetQuickAccessController(): void {
  _globalQuickAccessController?.dispose()
  _globalQuickAccessController = null
}

// Quick access reactive store for Svelte

import { getQuickAccessController, type IQuickAccessController } from '$lib/quickaccess/controller'
import type { IQuickAccessState } from '$lib/quickaccess/types'

/**
 * Reactive quick access state for Svelte components
 */
class QuickAccessStore {
  private _controller: IQuickAccessController
  private _state = $state<IQuickAccessState>({
    visible: false,
    value: '',
    prefix: '',
    items: [],
    selectedIndex: 0,
    loading: false,
    placeholder: 'Type to search...'
  })

  constructor() {
    this._controller = getQuickAccessController()

    // Subscribe to state changes
    this._controller.onDidChange((newState) => {
      this._state = newState
    })
  }

  get state(): IQuickAccessState {
    return this._state
  }

  get visible(): boolean {
    return this._state.visible
  }

  get value(): string {
    return this._state.value
  }

  get items() {
    return this._state.items
  }

  get selectedIndex(): number {
    return this._state.selectedIndex
  }

  get loading(): boolean {
    return this._state.loading
  }

  get placeholder(): string {
    return this._state.placeholder
  }

  show(initialValue?: string): void {
    this._controller.show(initialValue)
  }

  hide(): void {
    this._controller.hide()
  }

  toggle(initialValue?: string): void {
    this._controller.toggle(initialValue)
  }

  setValue(value: string): void {
    this._controller.setValue(value)
  }

  selectIndex(index: number): void {
    this._controller.selectIndex(index)
  }

  selectPrevious(): void {
    this._controller.selectPrevious()
  }

  selectNext(): void {
    this._controller.selectNext()
  }

  acceptSelected(): void {
    this._controller.acceptSelected()
  }
}

// Singleton store instance
let _store: QuickAccessStore | null = null

/**
 * Get the quick access store
 */
export function getQuickAccessStore(): QuickAccessStore {
  if (!_store) {
    _store = new QuickAccessStore()
  }
  return _store
}

// Export a reactive reference for direct use
export const quickAccessStore = getQuickAccessStore()

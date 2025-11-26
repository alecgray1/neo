// Quick access provider registry

import { createServiceId, type IDisposable, Emitter, type Event } from '$lib/services/types'
import type { IQuickAccessProvider, IQuickAccessRegistry } from './types'

/**
 * Quick access registry service identifier
 */
export const IQuickAccessRegistryService = createServiceId<IQuickAccessRegistry>('IQuickAccessRegistry')

/**
 * Quick access registry implementation
 */
class QuickAccessRegistry implements IQuickAccessRegistry, IDisposable {
  private _providers = new Map<string, IQuickAccessProvider>()
  private _onDidChange = new Emitter<void>()

  get onDidChange(): Event<void> {
    return this._onDidChange.event
  }

  registerProvider(provider: IQuickAccessProvider): IDisposable {
    if (this._providers.has(provider.prefix)) {
      console.warn(`Quick access provider for prefix "${provider.prefix}" already registered. Overwriting.`)
    }

    this._providers.set(provider.prefix, provider)
    this._onDidChange.fire()

    return {
      dispose: () => {
        if (this._providers.get(provider.prefix) === provider) {
          this._providers.delete(provider.prefix)
          this._onDidChange.fire()
        }
      }
    }
  }

  getProvider(prefix: string): IQuickAccessProvider | undefined {
    return this._providers.get(prefix)
  }

  getProviders(): IQuickAccessProvider[] {
    return Array.from(this._providers.values())
  }

  getDefaultProvider(): IQuickAccessProvider | undefined {
    return this._providers.get('')
  }

  dispose(): void {
    this._onDidChange.dispose()
    this._providers.clear()
  }
}

// Global quick access registry instance
let _globalQuickAccessRegistry: QuickAccessRegistry | null = null

/**
 * Get or create the global quick access registry
 */
export function getQuickAccessRegistry(): IQuickAccessRegistry {
  if (!_globalQuickAccessRegistry) {
    _globalQuickAccessRegistry = new QuickAccessRegistry()
  }
  return _globalQuickAccessRegistry
}

/**
 * Reset the global quick access registry (useful for testing)
 */
export function resetQuickAccessRegistry(): void {
  _globalQuickAccessRegistry?.dispose()
  _globalQuickAccessRegistry = null
}

/**
 * Convenience function to register a provider
 */
export function registerQuickAccessProvider(provider: IQuickAccessProvider): IDisposable {
  return getQuickAccessRegistry().registerProvider(provider)
}

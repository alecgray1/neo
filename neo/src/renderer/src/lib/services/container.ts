// Service container - VS Code style dependency injection

import type { ServiceIdentifier, IDisposable } from './types'
import { toDisposable } from './types'

/**
 * Service accessor passed to command handlers for dependency injection
 */
export interface IServiceAccessor {
  get<T>(id: ServiceIdentifier<T>): T
}

/**
 * Service container for registering and resolving services
 */
export interface IServiceContainer extends IServiceAccessor {
  register<T>(id: ServiceIdentifier<T>, instance: T): IDisposable
  has(id: ServiceIdentifier<unknown>): boolean
  createChild(): IServiceContainer
}

/**
 * Service container implementation
 */
class ServiceContainer implements IServiceContainer {
  private _services = new Map<symbol, unknown>()
  private _parent: ServiceContainer | null

  constructor(parent: ServiceContainer | null = null) {
    this._parent = parent
  }

  register<T>(id: ServiceIdentifier<T>, instance: T): IDisposable {
    this._services.set(id, instance)
    return toDisposable(() => this._services.delete(id))
  }

  get<T>(id: ServiceIdentifier<T>): T {
    const service = this._services.get(id) as T | undefined
    if (service !== undefined) {
      return service
    }
    if (this._parent) {
      return this._parent.get(id)
    }
    throw new Error(`Service not found: ${String(id)}`)
  }

  has(id: ServiceIdentifier<unknown>): boolean {
    if (this._services.has(id)) {
      return true
    }
    if (this._parent) {
      return this._parent.has(id)
    }
    return false
  }

  /**
   * Create a child container that inherits from this container
   * Child container lookups will fall through to parent if not found locally
   */
  createChild(): IServiceContainer {
    return new ServiceContainer(this)
  }
}

// Global service container instance
let _globalContainer: IServiceContainer | null = null

/**
 * Get or create the global service container
 */
export function getServiceContainer(): IServiceContainer {
  if (!_globalContainer) {
    _globalContainer = new ServiceContainer()
  }
  return _globalContainer
}

/**
 * Create a service accessor for command handlers
 */
export function createServiceAccessor(): IServiceAccessor {
  const container = getServiceContainer()
  return {
    get: <T>(id: ServiceIdentifier<T>) => container.get(id)
  }
}

/**
 * Reset the global container (useful for testing)
 */
export function resetServiceContainer(): void {
  _globalContainer = null
}

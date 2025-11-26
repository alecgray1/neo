// Service container types - VS Code style dependency injection

/**
 * Service identifier - use Symbol for type-safe service lookup
 * @example
 * export const ILayoutService = Symbol('ILayoutService')
 */
export type ServiceIdentifier<T> = symbol & { __type?: T }

/**
 * Create a typed service identifier
 */
export function createServiceId<T>(name: string): ServiceIdentifier<T> {
  return Symbol(name) as ServiceIdentifier<T>
}

/**
 * Disposable interface for cleanup
 */
export interface IDisposable {
  dispose(): void
}

/**
 * Create a disposable from a cleanup function
 */
export function toDisposable(fn: () => void): IDisposable {
  return { dispose: fn }
}

/**
 * Combine multiple disposables into one
 */
export class DisposableStore implements IDisposable {
  private _disposables = new Set<IDisposable>()
  private _isDisposed = false

  add<T extends IDisposable>(disposable: T): T {
    if (this._isDisposed) {
      disposable.dispose()
      return disposable
    }
    this._disposables.add(disposable)
    return disposable
  }

  delete(disposable: IDisposable): void {
    this._disposables.delete(disposable)
  }

  dispose(): void {
    if (this._isDisposed) return
    this._isDisposed = true
    for (const d of this._disposables) {
      d.dispose()
    }
    this._disposables.clear()
  }
}

/**
 * Simple event emitter interface
 */
export interface Event<T> {
  (listener: (e: T) => void): IDisposable
}

/**
 * Simple event emitter implementation
 */
export class Emitter<T> implements IDisposable {
  private _listeners = new Set<(e: T) => void>()

  get event(): Event<T> {
    return (listener) => {
      this._listeners.add(listener)
      return toDisposable(() => this._listeners.delete(listener))
    }
  }

  fire(event: T): void {
    for (const listener of this._listeners) {
      listener(event)
    }
  }

  dispose(): void {
    this._listeners.clear()
  }
}

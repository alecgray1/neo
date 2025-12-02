/**
 * VSCode-style typed proxy RPC protocol
 *
 * This provides compile-time type safety for all RPC calls between
 * the main process and extension host.
 */

// Proxy identifier - used to get typed proxies
export interface ProxyIdentifier<T> {
  readonly id: string
  readonly _brand: T // Phantom type for type inference
}

export function createProxyIdentifier<T>(id: string): ProxyIdentifier<T> {
  return { id } as ProxyIdentifier<T>
}

// Convert a shape interface to a "Proxied" version where all methods return Promises
export type Proxied<T> = {
  [K in keyof T]: T[K] extends (...args: infer A) => infer R
    ? (...args: A) => R extends Promise<unknown> ? R : Promise<R>
    : never
}

// RPC message types
export interface RPCRequest {
  type: 'request'
  id: number
  proxyId: string
  method: string
  args: unknown[]
}

export interface RPCResponse {
  type: 'response'
  id: number
  result?: unknown
  error?: { message: string; stack?: string }
}

export interface RPCNotification {
  type: 'notification'
  proxyId: string
  method: string
  args: unknown[]
}

export type RPCMessage = RPCRequest | RPCResponse | RPCNotification

/**
 * RPC Protocol implementation
 *
 * Provides typed proxies for cross-process communication over IPC.
 * All method calls on proxies become async RPC requests.
 */
export class RPCProtocol {
  private _requestId = 0
  private _pendingRequests = new Map<
    number,
    {
      resolve: (value: unknown) => void
      reject: (error: Error) => void
      timer: NodeJS.Timeout
    }
  >()
  private _handlers = new Map<string, object>()
  private _proxies = new Map<string, object>()
  private _sendMessage: (msg: RPCMessage) => void

  constructor(sendMessage: (msg: RPCMessage) => void) {
    this._sendMessage = sendMessage
  }

  /**
   * Get a typed proxy for a remote service
   */
  getProxy<T>(identifier: ProxyIdentifier<T>): Proxied<T> {
    if (!this._proxies.has(identifier.id)) {
      const proxy = new Proxy(
        {},
        {
          get: (_target, method: string) => {
            return (...args: unknown[]) => this._remoteCall(identifier.id, method, args)
          }
        }
      )
      this._proxies.set(identifier.id, proxy)
    }
    return this._proxies.get(identifier.id) as Proxied<T>
  }

  /**
   * Register a local handler for incoming RPC calls
   */
  set<T extends object>(identifier: ProxyIdentifier<T>, instance: T): T {
    this._handlers.set(identifier.id, instance)
    return instance
  }

  /**
   * Handle an incoming RPC message
   */
  async handleMessage(msg: RPCMessage): Promise<void> {
    switch (msg.type) {
      case 'request':
        await this._handleRequest(msg)
        break
      case 'response':
        this._handleResponse(msg)
        break
      case 'notification':
        await this._handleNotification(msg)
        break
    }
  }

  private async _remoteCall(proxyId: string, method: string, args: unknown[]): Promise<unknown> {
    const id = ++this._requestId

    return new Promise((resolve, reject) => {
      // Set timeout for request
      const timer = setTimeout(() => {
        this._pendingRequests.delete(id)
        reject(new Error(`RPC timeout: ${proxyId}.${method}`))
      }, 30000)

      this._pendingRequests.set(id, { resolve, reject, timer })

      const request: RPCRequest = {
        type: 'request',
        id,
        proxyId,
        method,
        args
      }

      this._sendMessage(request)
    })
  }

  private async _handleRequest(msg: RPCRequest): Promise<void> {
    const handler = this._handlers.get(msg.proxyId)

    let result: unknown
    let error: { message: string; stack?: string } | undefined

    if (!handler) {
      error = { message: `No handler registered for: ${msg.proxyId}` }
    } else {
      const method = (handler as Record<string, unknown>)[msg.method]
      if (typeof method !== 'function') {
        error = { message: `Method not found: ${msg.proxyId}.${msg.method}` }
      } else {
        try {
          result = await method.apply(handler, msg.args)
        } catch (e) {
          const err = e instanceof Error ? e : new Error(String(e))
          error = { message: err.message, stack: err.stack }
        }
      }
    }

    const response: RPCResponse = {
      type: 'response',
      id: msg.id,
      result,
      error
    }

    this._sendMessage(response)
  }

  private _handleResponse(msg: RPCResponse): void {
    const pending = this._pendingRequests.get(msg.id)
    if (!pending) return

    clearTimeout(pending.timer)
    this._pendingRequests.delete(msg.id)

    if (msg.error) {
      const error = new Error(msg.error.message)
      if (msg.error.stack) error.stack = msg.error.stack
      pending.reject(error)
    } else {
      pending.resolve(msg.result)
    }
  }

  private async _handleNotification(msg: RPCNotification): Promise<void> {
    const handler = this._handlers.get(msg.proxyId)
    if (!handler) return

    const method = (handler as Record<string, unknown>)[msg.method]
    if (typeof method === 'function') {
      try {
        await method.apply(handler, msg.args)
      } catch (e) {
        console.error(`Notification handler error: ${msg.proxyId}.${msg.method}`, e)
      }
    }
  }

  /**
   * Send a one-way notification (no response expected)
   */
  notify(proxyId: string, method: string, args: unknown[]): void {
    const notification: RPCNotification = {
      type: 'notification',
      proxyId,
      method,
      args
    }
    this._sendMessage(notification)
  }

  /**
   * Dispose all pending requests
   */
  dispose(): void {
    for (const [id, pending] of this._pendingRequests) {
      clearTimeout(pending.timer)
      pending.reject(new Error('Protocol disposed'))
      this._pendingRequests.delete(id)
    }
  }
}

/**
 * Extension Host Process
 *
 * This is the entry point for the forked Node.js process that runs extensions.
 * It loads extensions, manages their lifecycle, and handles RPC communication
 * with the main process.
 */

import { RPCProtocol, RPCMessage } from './protocol'
import {
  MainContext,
  ExtHostContext,
  ExtHostCommandsShape,
  ExtHostWebviewsShape,
  ExtHostServerShape,
  ExtHostExtensionsShape,
  ChangeEvent
} from './extHost.protocol'

// Extension info from main process
interface ExtensionInfo {
  id: string
  name: string
  version: string
  extensionPath: string
  mainPath: string
}

// Loaded extension instance
interface LoadedExtension {
  info: ExtensionInfo
  module: ExtensionModule | null
  context: ExtensionContext
  activated: boolean
  disposables: Disposable[]
}

// Extension module exports
interface ExtensionModule {
  activate?: (context: ExtensionContext) => void | Promise<void>
  deactivate?: () => void | Promise<void>
}

// Disposable interface
interface Disposable {
  dispose(): void
}

// Extension context passed to activate()
interface ExtensionContext {
  readonly extensionPath: string
  readonly subscriptions: Disposable[]
  readonly globalState: Memento
  readonly workspaceState: Memento
}

// Simple key-value storage
interface Memento {
  get<T>(key: string, defaultValue?: T): T | undefined
  update(key: string, value: unknown): Promise<void>
  keys(): readonly string[]
}

class ExtensionHostProcess {
  private _protocol: RPCProtocol
  private _extensions = new Map<string, LoadedExtension>()

  // Command handlers registered by extensions
  private _commandHandlers = new Map<string, (...args: unknown[]) => unknown>()

  // Webview view providers
  private _webviewViewProviders = new Map<
    string,
    {
      resolveWebviewView: (
        webviewView: WebviewView,
        context: WebviewViewResolveContext
      ) => void | Promise<void>
    }
  >()

  // Server event listeners
  private _serverEventListeners: ((event: ChangeEvent) => void)[] = []
  private _connectionStateListeners: ((connected: boolean) => void)[] = []

  constructor() {
    // Set up RPC protocol
    this._protocol = new RPCProtocol((msg) => {
      process.send?.(msg)
    })

    process.on('message', (msg: RPCMessage | { type: string; extensions?: ExtensionInfo[] }) => {
      // Check for init message (has 'extensions' field)
      if (msg.type === 'init' && 'extensions' in msg && msg.extensions) {
        this._initialize(msg.extensions)
      } else if (msg.type === 'request' || msg.type === 'response' || msg.type === 'notification') {
        // RPC message
        this._protocol.handleMessage(msg as RPCMessage)
      }
    })

    // Register extension host handlers
    this._registerHandlers()
  }

  private async _initialize(extensionInfos: ExtensionInfo[]): Promise<void> {
    console.log(`[ExtHostProcess] Initializing with ${extensionInfos.length} extensions`)

    // Create extension instances
    for (const info of extensionInfos) {
      const context = this._createExtensionContext(info)
      this._extensions.set(info.id, {
        info,
        module: null,
        context,
        activated: false,
        disposables: []
      })
    }

    // Signal ready
    process.send?.({ type: 'ready' })
  }

  private _registerHandlers(): void {
    // Commands
    this._protocol.set<ExtHostCommandsShape>(ExtHostContext.ExtHostCommands, {
      $executeContributedCommand: async <T>(id: string, args: unknown[]): Promise<T> => {
        const handler = this._commandHandlers.get(id)
        if (!handler) {
          throw new Error(`Command not found: ${id}`)
        }
        return (await handler(...args)) as T
      }
    })

    // Webviews
    this._protocol.set<ExtHostWebviewsShape>(ExtHostContext.ExtHostWebviews, {
      $onDidReceiveMessage: (handle, message) => {
        // Find the webview and dispatch message
        // This requires tracking webview instances
        console.log(`[ExtHostProcess] Webview message: ${handle}`, message)
      },

      $onDidDisposeWebview: (handle) => {
        console.log(`[ExtHostProcess] Webview disposed: ${handle}`)
      },

      $onDidChangeWebviewVisibility: (handle, visible) => {
        console.log(`[ExtHostProcess] Webview visibility: ${handle} = ${visible}`)
      },

      $resolveWebviewView: async (handle, viewId, title, state) => {
        const provider = this._webviewViewProviders.get(viewId)
        if (!provider) {
          throw new Error(`No provider registered for view: ${viewId}`)
        }

        const webviewView: WebviewView = {
          webview: this._createWebview(handle),
          title: title ?? '',
          description: undefined,
          onDidChangeVisibility: () => ({ dispose: () => {} }),
          onDidDispose: () => ({ dispose: () => {} }),
          show: (preserveFocus?: boolean) => {
            // Notify main thread
          }
        }

        await provider.resolveWebviewView(webviewView, { state })
      },

      $onDidDisposeWebviewView: (handle) => {
        console.log(`[ExtHostProcess] WebviewView disposed: ${handle}`)
      },

      $onDidChangeWebviewViewVisibility: (handle, visible) => {
        console.log(`[ExtHostProcess] WebviewView visibility: ${handle} = ${visible}`)
      }
    })

    // Server events
    this._protocol.set<ExtHostServerShape>(ExtHostContext.ExtHostServer, {
      $onServerEvent: (event) => {
        for (const listener of this._serverEventListeners) {
          try {
            listener(event)
          } catch (err) {
            console.error('[ExtHostProcess] Server event listener error:', err)
          }
        }
      },

      $onConnectionStateChanged: (connected) => {
        for (const listener of this._connectionStateListeners) {
          try {
            listener(connected)
          } catch (err) {
            console.error('[ExtHostProcess] Connection state listener error:', err)
          }
        }
      }
    })

    // Extensions lifecycle
    this._protocol.set<ExtHostExtensionsShape>(ExtHostContext.ExtHostExtensions, {
      $activateExtension: async (extensionId) => {
        await this._activateExtension(extensionId)
      },

      $deactivateExtension: async (extensionId) => {
        await this._deactivateExtension(extensionId)
      }
    })
  }

  private async _activateExtension(extensionId: string): Promise<void> {
    const ext = this._extensions.get(extensionId)
    if (!ext) {
      throw new Error(`Extension not found: ${extensionId}`)
    }

    if (ext.activated) {
      return
    }

    const mainThreadExtensions = this._protocol.getProxy(MainContext.MainThreadExtensions)

    try {
      console.log(`[ExtHostProcess] Activating extension: ${extensionId}`)

      // Load the extension module
      const module = await this._loadExtensionModule(ext.info.mainPath)
      ext.module = module

      // Create the neo.* API for this extension
      const api = this._createExtensionAPI(ext)

      // Make API available globally
      ;(globalThis as Record<string, unknown>).neo = api

      // Call activate if present
      if (module.activate) {
        await module.activate(ext.context)
      }

      ext.activated = true
      mainThreadExtensions.$onExtensionActivated(extensionId)

      console.log(`[ExtHostProcess] Extension activated: ${extensionId}`)
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      console.error(`[ExtHostProcess] Failed to activate ${extensionId}:`, err)
      mainThreadExtensions.$onExtensionActivationFailed(extensionId, message)
      throw err
    }
  }

  private async _deactivateExtension(extensionId: string): Promise<void> {
    const ext = this._extensions.get(extensionId)
    if (!ext || !ext.activated) {
      return
    }

    console.log(`[ExtHostProcess] Deactivating extension: ${extensionId}`)

    // Call deactivate if present
    if (ext.module?.deactivate) {
      await ext.module.deactivate()
    }

    // Dispose all subscriptions
    for (const disposable of ext.context.subscriptions) {
      try {
        disposable.dispose()
      } catch (err) {
        console.error(`[ExtHostProcess] Disposal error in ${extensionId}:`, err)
      }
    }
    ext.context.subscriptions.length = 0

    // Dispose extension-level disposables
    for (const disposable of ext.disposables) {
      try {
        disposable.dispose()
      } catch (err) {
        console.error(`[ExtHostProcess] Disposal error in ${extensionId}:`, err)
      }
    }
    ext.disposables.length = 0

    ext.activated = false
    console.log(`[ExtHostProcess] Extension deactivated: ${extensionId}`)
  }

  private async _loadExtensionModule(mainPath: string): Promise<ExtensionModule> {
    try {
      // Use dynamic import for ES modules
      // Add cache buster to force reload on hot reload
      const cacheBuster = `?t=${Date.now()}`
      const module = await import(mainPath + cacheBuster)
      return module.default ?? module
    } catch (err) {
      console.error(`[ExtHostProcess] Failed to load module: ${mainPath}`, err)
      throw err
    }
  }

  private _createExtensionContext(info: ExtensionInfo): ExtensionContext {
    const subscriptions: Disposable[] = []

    // Simple in-memory storage (could be persisted)
    const createMemento = (): Memento => {
      const storage = new Map<string, unknown>()
      return {
        get: <T>(key: string, defaultValue?: T) => {
          return (storage.get(key) as T) ?? defaultValue
        },
        update: async (key, value) => {
          storage.set(key, value)
        },
        keys: () => Array.from(storage.keys())
      }
    }

    return {
      extensionPath: info.extensionPath,
      subscriptions,
      globalState: createMemento(),
      workspaceState: createMemento()
    }
  }

  private _createExtensionAPI(ext: LoadedExtension): NeoAPI {
    const mainThreadCommands = this._protocol.getProxy(MainContext.MainThreadCommands)
    const mainThreadWindow = this._protocol.getProxy(MainContext.MainThreadWindow)
    const mainThreadWebviews = this._protocol.getProxy(MainContext.MainThreadWebviews)
    const mainThreadContext = this._protocol.getProxy(MainContext.MainThreadContext)
    const mainThreadServer = this._protocol.getProxy(MainContext.MainThreadServer)

    return {
      version: '1.0.0',

      commands: {
        registerCommand: (id: string, handler: (...args: unknown[]) => unknown) => {
          this._commandHandlers.set(id, handler)
          mainThreadCommands.$registerCommand(id)

          return {
            dispose: () => {
              this._commandHandlers.delete(id)
              mainThreadCommands.$unregisterCommand(id)
            }
          }
        },

        executeCommand: async <T>(id: string, ...args: unknown[]): Promise<T> => {
          return mainThreadCommands.$executeCommand(id, args) as Promise<T>
        }
      },

      window: {
        showInformationMessage: (message: string, ...items: string[]) => {
          return mainThreadWindow.$showMessage('info', message, items)
        },

        showWarningMessage: (message: string, ...items: string[]) => {
          return mainThreadWindow.$showMessage('warning', message, items)
        },

        showErrorMessage: (message: string, ...items: string[]) => {
          return mainThreadWindow.$showMessage('error', message, items)
        },

        showQuickPick: (items, options) => {
          return mainThreadWindow.$showQuickPick(items, options)
        },

        showInputBox: (options) => {
          return mainThreadWindow.$showInputBox(options)
        },

        createWebviewPanel: (viewType, title, column, options) => {
          const handle = `panel-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
          mainThreadWebviews.$createWebviewPanel(handle, viewType, title, column, options)

          return {
            webview: this._createWebview(handle),
            viewType,
            title,
            visible: true,
            active: true,
            onDidChangeViewState: () => ({ dispose: () => {} }),
            onDidDispose: () => ({ dispose: () => {} }),
            reveal: (viewColumn?: number, preserveFocus?: boolean) => {
              mainThreadWebviews.$reveal(handle, viewColumn, preserveFocus)
            },
            dispose: () => {
              mainThreadWebviews.$disposeWebview(handle)
            }
          }
        },

        registerWebviewViewProvider: (viewId, provider) => {
          this._webviewViewProviders.set(viewId, provider)
          mainThreadWebviews.$registerWebviewViewProvider(viewId)

          return {
            dispose: () => {
              this._webviewViewProviders.delete(viewId)
              mainThreadWebviews.$unregisterWebviewViewProvider(viewId)
            }
          }
        }
      },

      context: {
        set: (key: string, value: unknown) => {
          mainThreadContext.$setContext(key, value)
        },

        get: <T>(key: string): T | undefined => {
          // Context is typically set-only from extensions
          // Reading would require additional protocol
          return undefined
        }
      },

      server: {
        connected: false, // Will be updated by events

        onDidConnect: (listener: () => void) => {
          const wrappedListener = (connected: boolean): void => {
            if (connected) listener()
          }
          this._connectionStateListeners.push(wrappedListener)
          return {
            dispose: () => {
              const idx = this._connectionStateListeners.indexOf(wrappedListener)
              if (idx >= 0) this._connectionStateListeners.splice(idx, 1)
            }
          }
        },

        onDidDisconnect: (listener: () => void) => {
          const wrappedListener = (connected: boolean): void => {
            if (!connected) listener()
          }
          this._connectionStateListeners.push(wrappedListener)
          return {
            dispose: () => {
              const idx = this._connectionStateListeners.indexOf(wrappedListener)
              if (idx >= 0) this._connectionStateListeners.splice(idx, 1)
            }
          }
        },

        request: async <T>(path: string, params?: Record<string, unknown>): Promise<T> => {
          return mainThreadServer.$request(path, params) as Promise<T>
        },

        subscribe: async (paths: string[]) => {
          const id = await mainThreadServer.$subscribe(paths)
          return {
            dispose: () => {
              mainThreadServer.$unsubscribe(id)
            }
          }
        },

        onDidReceiveChange: (listener: (event: ChangeEvent) => void) => {
          this._serverEventListeners.push(listener)
          return {
            dispose: () => {
              const idx = this._serverEventListeners.indexOf(listener)
              if (idx >= 0) this._serverEventListeners.splice(idx, 1)
            }
          }
        }
      }
    }
  }

  private _createWebview(handle: string): Webview {
    const mainThreadWebviews = this._protocol.getProxy(MainContext.MainThreadWebviews)
    let _html = ''

    return {
      get html() {
        return _html
      },
      set html(value: string) {
        _html = value
        mainThreadWebviews.$setWebviewHtml(handle, value)
      },
      onDidReceiveMessage: (listener: (message: unknown) => void) => {
        // Messages are routed through ExtHostWebviews.$onDidReceiveMessage
        return { dispose: () => {} }
      },
      postMessage: async (message: unknown) => {
        return mainThreadWebviews.$postMessage(handle, message)
      },
      asWebviewUri: (localResource: string) => {
        // Convert local file path to webview-safe URI
        return `vscode-webview:///${localResource}`
      }
    }
  }
}

// Types for the API
interface NeoAPI {
  version: string
  commands: {
    registerCommand(id: string, handler: (...args: unknown[]) => unknown): Disposable
    executeCommand<T>(id: string, ...args: unknown[]): Promise<T>
  }
  window: {
    showInformationMessage(message: string, ...items: string[]): Promise<string | undefined>
    showWarningMessage(message: string, ...items: string[]): Promise<string | undefined>
    showErrorMessage(message: string, ...items: string[]): Promise<string | undefined>
    showQuickPick(items: unknown[], options?: unknown): Promise<unknown>
    showInputBox(options?: unknown): Promise<string | undefined>
    createWebviewPanel(
      viewType: string,
      title: string,
      column: number,
      options?: unknown
    ): WebviewPanel
    registerWebviewViewProvider(viewId: string, provider: unknown): Disposable
  }
  context: {
    set(key: string, value: unknown): void
    get<T>(key: string): T | undefined
  }
  server: {
    connected: boolean
    onDidConnect(listener: () => void): Disposable
    onDidDisconnect(listener: () => void): Disposable
    request<T>(path: string, params?: Record<string, unknown>): Promise<T>
    subscribe(paths: string[]): Promise<Disposable>
    onDidReceiveChange(listener: (event: ChangeEvent) => void): Disposable
  }
}

interface Webview {
  html: string
  onDidReceiveMessage(listener: (message: unknown) => void): Disposable
  postMessage(message: unknown): Promise<boolean>
  asWebviewUri(localResource: string): string
}

interface WebviewPanel {
  webview: Webview
  viewType: string
  title: string
  visible: boolean
  active: boolean
  onDidChangeViewState: () => Disposable
  onDidDispose: () => Disposable
  reveal(viewColumn?: number, preserveFocus?: boolean): void
  dispose(): void
}

interface WebviewView {
  webview: Webview
  title: string
  description: string | undefined
  onDidChangeVisibility: () => Disposable
  onDidDispose: () => Disposable
  show(preserveFocus?: boolean): void
}

interface WebviewViewResolveContext {
  state: unknown
}

// Start the extension host
new ExtensionHostProcess()

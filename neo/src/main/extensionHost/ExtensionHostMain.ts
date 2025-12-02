/**
 * Extension Host Main
 *
 * Manages the extension host process from the main process side.
 * Spawns a Node.js fork and establishes RPC communication.
 */

import { fork, ChildProcess } from 'child_process'
import { join } from 'path'
import { app, BrowserWindow, dialog } from 'electron'
import { RPCProtocol, RPCMessage } from './protocol'
import {
  MainContext,
  ExtHostContext,
  MainThreadCommandsShape,
  MainThreadWindowShape,
  MainThreadWebviewsShape,
  MainThreadContextShape,
  MainThreadServerShape,
  MainThreadExtensionsShape,
  ExtensionInfo,
  QuickPickItem,
  QuickPickOptions,
  InputBoxOptions,
  WebviewOptions,
  ChangeEvent
} from './extHost.protocol'
import {
  ExtensionScanner,
  ScannedExtension,
  ExtensionContributes,
  CommandContribution,
  ViewContainerContribution,
  ViewContribution,
  MenuContribution,
  KeybindingContribution
} from './ExtensionScanner'
import { ipcMain } from 'electron'

/**
 * Collected contributions from all extensions
 */
export interface CollectedContributions {
  commands: Array<CommandContribution & { extensionId: string }>
  viewsContainers: {
    activitybar: Array<ViewContainerContribution & { extensionId: string }>
    panel: Array<ViewContainerContribution & { extensionId: string }>
  }
  views: Record<string, Array<ViewContribution & { extensionId: string }>>
  menus: Record<string, Array<MenuContribution & { extensionId: string }>>
  keybindings: Array<KeybindingContribution & { extensionId: string }>
}

export class ExtensionHostMain {
  private _process: ChildProcess | null = null
  private _protocol: RPCProtocol | null = null
  private _mainWindow: BrowserWindow | null = null
  private _extensions: ScannedExtension[] = []
  private _extensionsPath: string
  private _isDisposed = false

  // Command handlers registered by extensions
  private _commandHandlers = new Set<string>()

  // Active webview handles
  private _webviewHandles = new Map<string, { viewType: string; title: string }>()

  // Collected contributions from all extensions
  private _contributions: CollectedContributions = {
    commands: [],
    viewsContainers: { activitybar: [], panel: [] },
    views: {},
    menus: {},
    keybindings: []
  }

  constructor(extensionsPath?: string) {
    this._extensionsPath = extensionsPath ?? join(app.getPath('userData'), 'extensions')
    this._registerIPCHandlers()
  }

  /**
   * Register IPC handlers for renderer queries
   */
  private _registerIPCHandlers(): void {
    ipcMain.handle('extension:getContributions', () => {
      return this._contributions
    })

    ipcMain.handle('extension:getExtensions', () => {
      return this._extensions.map((e) => ({
        id: e.id,
        name: e.manifest.name,
        displayName: e.manifest.displayName ?? e.manifest.name,
        version: e.manifest.version,
        description: e.manifest.description ?? ''
      }))
    })

    ipcMain.handle('extension:executeCommand', async (_event, id: string, args: unknown[]) => {
      return this.executeCommand(id, ...args)
    })
  }

  /**
   * Collect contributions from all scanned extensions
   */
  private _collectContributions(): void {
    // Reset contributions
    this._contributions = {
      commands: [],
      viewsContainers: { activitybar: [], panel: [] },
      views: {},
      menus: {},
      keybindings: []
    }

    for (const ext of this._extensions) {
      const contributes = ext.manifest.neo?.app?.contributes
      if (!contributes) continue

      // Commands
      if (contributes.commands) {
        for (const cmd of contributes.commands) {
          this._contributions.commands.push({ ...cmd, extensionId: ext.id })
        }
      }

      // View containers
      if (contributes.viewsContainers) {
        if (contributes.viewsContainers.activitybar) {
          for (const container of contributes.viewsContainers.activitybar) {
            this._contributions.viewsContainers.activitybar.push({
              ...container,
              extensionId: ext.id
            })
          }
        }
        if (contributes.viewsContainers.panel) {
          for (const container of contributes.viewsContainers.panel) {
            this._contributions.viewsContainers.panel.push({
              ...container,
              extensionId: ext.id
            })
          }
        }
      }

      // Views
      if (contributes.views) {
        for (const [containerId, views] of Object.entries(contributes.views)) {
          if (!this._contributions.views[containerId]) {
            this._contributions.views[containerId] = []
          }
          for (const view of views) {
            this._contributions.views[containerId].push({ ...view, extensionId: ext.id })
          }
        }
      }

      // Menus
      if (contributes.menus) {
        for (const [menuId, items] of Object.entries(contributes.menus)) {
          if (!this._contributions.menus[menuId]) {
            this._contributions.menus[menuId] = []
          }
          for (const item of items) {
            this._contributions.menus[menuId].push({ ...item, extensionId: ext.id })
          }
        }
      }

      // Keybindings
      if (contributes.keybindings) {
        for (const keybinding of contributes.keybindings) {
          this._contributions.keybindings.push({ ...keybinding, extensionId: ext.id })
        }
      }
    }

    console.log(
      `[ExtensionHost] Collected contributions: ${this._contributions.commands.length} commands, ` +
        `${this._contributions.viewsContainers.activitybar.length + this._contributions.viewsContainers.panel.length} view containers`
    )
  }

  setMainWindow(window: BrowserWindow): void {
    this._mainWindow = window
  }

  async start(): Promise<void> {
    if (this._process) {
      console.log('[ExtensionHost] Already running')
      return
    }

    // Scan for extensions
    const scanner = new ExtensionScanner(this._extensionsPath)
    this._extensions = await scanner.scan()
    console.log(`[ExtensionHost] Found ${this._extensions.length} extensions`)

    // Collect contributions from all extensions
    this._collectContributions()

    // Fork the extension host process
    const extensionHostPath = join(__dirname, 'ExtensionHostProcess.js')

    this._process = fork(extensionHostPath, [], {
      execArgv: ['--enable-source-maps'],
      env: {
        ...process.env,
        NEO_EXTENSION_HOST: '1',
        NEO_EXTENSIONS_PATH: this._extensionsPath
      },
      stdio: ['pipe', 'pipe', 'pipe', 'ipc']
    })

    // Log extension host output
    this._process.stdout?.on('data', (data) => {
      console.log(`[ExtHost] ${data.toString().trim()}`)
    })

    this._process.stderr?.on('data', (data) => {
      console.error(`[ExtHost] ${data.toString().trim()}`)
    })

    // Handle process exit
    this._process.on('exit', (code, signal) => {
      console.log(`[ExtensionHost] Process exited with code ${code}, signal ${signal}`)
      this._process = null
      this._protocol = null

      // Restart if not intentionally disposed
      if (!this._isDisposed) {
        console.log('[ExtensionHost] Restarting...')
        setTimeout(() => this.start(), 1000)
      }
    })

    // Set up RPC protocol
    this._protocol = new RPCProtocol((msg) => {
      this._process?.send(msg)
    })

    this._process.on('message', (msg: RPCMessage) => {
      this._protocol?.handleMessage(msg)
    })

    // Register main thread handlers
    this._registerMainThreadHandlers()

    // Wait for extension host to be ready
    await this._waitForReady()

    // Send extension list
    const extHostExtensions = this._protocol.getProxy(ExtHostContext.ExtHostExtensions)

    // Activate extensions with onStartupFinished event
    for (const ext of this._extensions) {
      if (ext.manifest.neo?.app?.activationEvents?.includes('onStartupFinished')) {
        try {
          await extHostExtensions.$activateExtension(ext.id)
        } catch (err) {
          console.error(`[ExtensionHost] Failed to activate ${ext.id}:`, err)
        }
      }
    }

    console.log('[ExtensionHost] Started successfully')
  }

  private _waitForReady(): Promise<void> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Extension host startup timeout'))
      }, 10000)

      const handler = (msg: { type: string }): void => {
        if (msg.type === 'ready') {
          clearTimeout(timeout)
          this._process?.off('message', handler)
          resolve()
        }
      }

      this._process?.on('message', handler)

      // Send init message with extension list
      this._process?.send({
        type: 'init',
        extensions: this._extensions.map((e) => ({
          id: e.id,
          name: e.manifest.name,
          version: e.manifest.version,
          extensionPath: e.path,
          mainPath: join(e.path, e.manifest.neo?.app?.entry ?? 'dist/app/index.js')
        }))
      })
    })
  }

  private _registerMainThreadHandlers(): void {
    if (!this._protocol) return

    // Commands
    this._protocol.set<MainThreadCommandsShape>(MainContext.MainThreadCommands, {
      $registerCommand: (id) => {
        this._commandHandlers.add(id)
        // Notify renderer of new command
        this._notifyRenderer('extension:commandRegistered', { id })
      },

      $unregisterCommand: (id) => {
        this._commandHandlers.delete(id)
        this._notifyRenderer('extension:commandUnregistered', { id })
      },

      $executeCommand: async <T>(id: string, args: unknown[]): Promise<T> => {
        // Check if it's an extension command
        if (this._commandHandlers.has(id)) {
          const extHostCommands = this._protocol!.getProxy(ExtHostContext.ExtHostCommands)
          return extHostCommands.$executeContributedCommand(id, args)
        }
        // Forward to renderer for built-in commands
        return this._invokeRenderer('command:execute', { id, args })
      },

      $getCommands: async () => {
        const builtIn = await this._invokeRenderer<string[]>('command:getAll', {})
        return [...builtIn, ...Array.from(this._commandHandlers)]
      }
    })

    // Window/UI
    this._protocol.set<MainThreadWindowShape>(MainContext.MainThreadWindow, {
      $showMessage: async (type, message, items) => {
        const buttons = items.length > 0 ? items : ['OK']
        const result = await dialog.showMessageBox(this._mainWindow!, {
          type: type === 'warning' ? 'warning' : type === 'error' ? 'error' : 'info',
          message,
          buttons
        })
        return items[result.response]
      },

      $showQuickPick: async (items, options) => {
        // Forward to renderer for QuickPick UI
        return this._invokeRenderer('quickPick:show', { items, options })
      },

      $showInputBox: async (options) => {
        // Forward to renderer for InputBox UI
        return this._invokeRenderer('inputBox:show', { options })
      }
    })

    // Webviews
    this._protocol.set<MainThreadWebviewsShape>(MainContext.MainThreadWebviews, {
      $createWebviewPanel: (handle, viewType, title, column, options) => {
        this._webviewHandles.set(handle, { viewType, title })
        this._notifyRenderer('webview:create', { handle, viewType, title, column, options })
      },

      $disposeWebview: (handle) => {
        this._webviewHandles.delete(handle)
        this._notifyRenderer('webview:dispose', { handle })
      },

      $setWebviewHtml: (handle, html) => {
        this._notifyRenderer('webview:setHtml', { handle, html })
      },

      $setWebviewTitle: (handle, title) => {
        const info = this._webviewHandles.get(handle)
        if (info) info.title = title
        this._notifyRenderer('webview:setTitle', { handle, title })
      },

      $postMessage: async (handle, message) => {
        this._notifyRenderer('webview:postMessage', { handle, message })
        return true
      },

      $reveal: (handle, column, preserveFocus) => {
        this._notifyRenderer('webview:reveal', { handle, column, preserveFocus })
      },

      $registerWebviewViewProvider: (viewId) => {
        this._notifyRenderer('webviewView:registerProvider', { viewId })
      },

      $unregisterWebviewViewProvider: (viewId) => {
        this._notifyRenderer('webviewView:unregisterProvider', { viewId })
      },

      $setWebviewViewHtml: (handle, html) => {
        this._notifyRenderer('webviewView:setHtml', { handle, html })
      },

      $setWebviewViewTitle: (handle, title) => {
        this._notifyRenderer('webviewView:setTitle', { handle, title })
      },

      $setWebviewViewDescription: (handle, description) => {
        this._notifyRenderer('webviewView:setDescription', { handle, description })
      }
    })

    // Context
    this._protocol.set<MainThreadContextShape>(MainContext.MainThreadContext, {
      $setContext: (key, value) => {
        this._notifyRenderer('context:set', { key, value })
      },

      $removeContext: (key) => {
        this._notifyRenderer('context:remove', { key })
      }
    })

    // Server communication
    this._protocol.set<MainThreadServerShape>(MainContext.MainThreadServer, {
      $request: async (path, params) => {
        return this._invokeRenderer('server:request', { path, params })
      },

      $subscribe: async (paths) => {
        const id = `ext-sub-${Date.now()}`
        await this._invokeRenderer('server:subscribe', { id, paths })
        return id
      },

      $unsubscribe: (subscriptionId) => {
        this._notifyRenderer('server:unsubscribe', { subscriptionId })
      },

      $getConnectionState: async () => {
        return this._invokeRenderer('server:getConnectionState', {})
      }
    })

    // Extensions
    this._protocol.set<MainThreadExtensionsShape>(MainContext.MainThreadExtensions, {
      $onExtensionActivated: (extensionId) => {
        console.log(`[ExtensionHost] Extension activated: ${extensionId}`)
        this._notifyRenderer('extension:activated', { extensionId })
      },

      $onExtensionActivationFailed: (extensionId, error) => {
        console.error(`[ExtensionHost] Extension activation failed: ${extensionId}`, error)
        this._notifyRenderer('extension:activationFailed', { extensionId, error })
      },

      $getExtensions: async () => {
        return this._extensions.map((e) => ({
          id: e.id,
          name: e.manifest.name,
          version: e.manifest.version,
          extensionPath: e.path
        }))
      }
    })
  }

  /**
   * Forward server events to extension host
   */
  onServerEvent(event: ChangeEvent): void {
    if (!this._protocol) return
    const extHostServer = this._protocol.getProxy(ExtHostContext.ExtHostServer)
    extHostServer.$onServerEvent(event)
  }

  /**
   * Forward connection state changes to extension host
   */
  onConnectionStateChanged(connected: boolean): void {
    if (!this._protocol) return
    const extHostServer = this._protocol.getProxy(ExtHostContext.ExtHostServer)
    extHostServer.$onConnectionStateChanged(connected)
  }

  /**
   * Activate extension by event
   */
  async activateByEvent(event: string): Promise<void> {
    if (!this._protocol) return

    const extHostExtensions = this._protocol.getProxy(ExtHostContext.ExtHostExtensions)

    for (const ext of this._extensions) {
      const activationEvents = ext.manifest.neo?.app?.activationEvents ?? []
      if (activationEvents.includes(event)) {
        try {
          await extHostExtensions.$activateExtension(ext.id)
        } catch (err) {
          console.error(`[ExtensionHost] Failed to activate ${ext.id} for event ${event}:`, err)
        }
      }
    }
  }

  /**
   * Execute a command (may be extension or built-in)
   */
  async executeCommand<T = unknown>(id: string, ...args: unknown[]): Promise<T> {
    if (!this._protocol) throw new Error('Extension host not running')

    // Check if it's an extension command
    if (this._commandHandlers.has(id)) {
      const extHostCommands = this._protocol.getProxy(ExtHostContext.ExtHostCommands)
      return extHostCommands.$executeContributedCommand(id, args) as Promise<T>
    }

    // Forward to renderer for built-in commands
    return this._invokeRenderer('command:execute', { id, args })
  }

  /**
   * Get list of installed extensions
   */
  getInstalledExtensions(): ScannedExtension[] {
    return this._extensions
  }

  private _notifyRenderer(channel: string, data: unknown): void {
    if (this._mainWindow && !this._mainWindow.isDestroyed()) {
      this._mainWindow.webContents.send(channel, data)
    }
  }

  private async _invokeRenderer<T>(channel: string, data: unknown): Promise<T> {
    // This requires setting up invoke handlers in the renderer
    // For now, use a simple request/response pattern
    return new Promise((resolve, reject) => {
      const responseChannel = `${channel}:response:${Date.now()}`

      const handler = (_event: Electron.IpcMainEvent, response: { result?: T; error?: string }): void => {
        if (response.error) {
          reject(new Error(response.error))
        } else {
          resolve(response.result as T)
        }
      }

      const { ipcMain } = require('electron')
      ipcMain.once(responseChannel, handler)

      this._mainWindow?.webContents.send(channel, { ...data, responseChannel })

      // Timeout
      setTimeout(() => {
        ipcMain.removeListener(responseChannel, handler)
        reject(new Error(`Renderer invoke timeout: ${channel}`))
      }, 30000)
    })
  }

  dispose(): void {
    this._isDisposed = true
    this._protocol?.dispose()

    if (this._process) {
      this._process.kill()
      this._process = null
    }
  }
}

// Singleton instance
let extensionHostMain: ExtensionHostMain | null = null

export function getExtensionHostMain(): ExtensionHostMain {
  if (!extensionHostMain) {
    extensionHostMain = new ExtensionHostMain()
  }
  return extensionHostMain
}

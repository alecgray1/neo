import { contextBridge, ipcRenderer } from 'electron'
import { electronAPI } from '@electron-toolkit/preload'

// Custom APIs for renderer
const api = {}

// Theme API for renderer
const themeAPI = {
  getCurrentTheme: (): Promise<unknown> => ipcRenderer.invoke('theme:get-current'),
  getAvailableThemes: (): Promise<unknown> => ipcRenderer.invoke('theme:get-available'),
  setTheme: (themeId: string): Promise<boolean> => ipcRenderer.invoke('theme:set', themeId),
  onThemeChanged: (callback: (theme: unknown) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, theme: unknown): void => callback(theme)
    ipcRenderer.on('theme:changed', handler)
    return (): void => {
      ipcRenderer.removeListener('theme:changed', handler)
    }
  }
}

// Layout API for renderer
const layoutAPI = {
  getLayout: (): Promise<unknown> => ipcRenderer.invoke('layout:get'),
  setLayout: (layout: unknown): Promise<void> => ipcRenderer.invoke('layout:set', layout),
  resetLayout: (): Promise<void> => ipcRenderer.invoke('layout:reset')
}

// Window API for custom window controls
const windowAPI = {
  minimize: (): Promise<void> => ipcRenderer.invoke('window:minimize'),
  maximize: (): Promise<void> => ipcRenderer.invoke('window:maximize'),
  close: (): Promise<void> => ipcRenderer.invoke('window:close'),
  isMaximized: (): Promise<boolean> => ipcRenderer.invoke('window:isMaximized'),
  isMac: (): Promise<boolean> => ipcRenderer.invoke('window:isMac'),
  onMaximizedChange: (callback: (maximized: boolean) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, maximized: boolean): void =>
      callback(maximized)
    ipcRenderer.on('window:maximized-change', handler)
    return (): void => {
      ipcRenderer.removeListener('window:maximized-change', handler)
    }
  }
}

// Server API for WebSocket connection
export interface ConnectionState {
  state: 'disconnected' | 'connecting' | 'connected' | 'reconnecting'
  reconnectAttempts: number
}

export interface ServerConfig {
  host: string
  port: number
}

export interface ChangeEvent {
  path: string
  changeType: 'created' | 'updated' | 'deleted'
  data: unknown
}

const serverAPI = {
  // Connection management
  connect: (config?: Partial<ServerConfig>): Promise<boolean> =>
    ipcRenderer.invoke('server:connect', config),
  disconnect: (): Promise<void> => ipcRenderer.invoke('server:disconnect'),
  getState: (): Promise<ConnectionState> => ipcRenderer.invoke('server:getState'),
  getConfig: (): Promise<ServerConfig> => ipcRenderer.invoke('server:getConfig'),
  setConfig: (config: Partial<ServerConfig>): Promise<void> =>
    ipcRenderer.invoke('server:setConfig', config),

  // Request/response
  request: <T = unknown>(path: string, params?: Record<string, unknown>): Promise<T> =>
    ipcRenderer.invoke('server:request', path, params),

  // Subscriptions
  subscribe: (paths: string[]): Promise<void> => ipcRenderer.invoke('server:subscribe', paths),
  unsubscribe: (paths: string[]): Promise<void> => ipcRenderer.invoke('server:unsubscribe', paths),
  getSubscriptions: (): Promise<string[]> => ipcRenderer.invoke('server:getSubscriptions'),

  // Event listeners
  onStateChanged: (callback: (state: ConnectionState) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, state: ConnectionState): void =>
      callback(state)
    ipcRenderer.on('server:state-changed', handler)
    return (): void => {
      ipcRenderer.removeListener('server:state-changed', handler)
    }
  },
  onChange: (callback: (event: ChangeEvent) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: ChangeEvent): void => callback(data)
    ipcRenderer.on('server:change', handler)
    return (): void => {
      ipcRenderer.removeListener('server:change', handler)
    }
  }
}

// Extension API for querying extension contributions
export interface ExtensionInfo {
  id: string
  name: string
  displayName: string
  version: string
  description: string
}

export interface CommandContribution {
  id: string
  title: string
  category?: string
  icon?: string
  enablement?: string
  extensionId: string
}

export interface ViewContainerContribution {
  id: string
  title: string
  icon: string
  extensionId: string
}

export interface ViewContribution {
  id: string
  name: string
  type?: 'tree' | 'webview'
  when?: string
  icon?: string
  extensionId: string
}

export interface MenuContribution {
  command: string
  when?: string
  group?: string
  extensionId: string
}

export interface KeybindingContribution {
  command: string
  key: string
  mac?: string
  when?: string
  extensionId: string
}

export interface CollectedContributions {
  commands: CommandContribution[]
  viewsContainers: {
    activitybar: ViewContainerContribution[]
    panel: ViewContainerContribution[]
  }
  views: Record<string, ViewContribution[]>
  menus: Record<string, MenuContribution[]>
  keybindings: KeybindingContribution[]
}

const extensionAPI = {
  // Get all contributions from extensions
  getContributions: (): Promise<CollectedContributions> =>
    ipcRenderer.invoke('extension:getContributions'),

  // Get list of installed extensions
  getExtensions: (): Promise<ExtensionInfo[]> => ipcRenderer.invoke('extension:getExtensions'),

  // Execute an extension command
  executeCommand: <T = unknown>(id: string, ...args: unknown[]): Promise<T> =>
    ipcRenderer.invoke('extension:executeCommand', id, args),

  // Listen for extension events
  onExtensionActivated: (
    callback: (data: { extensionId: string }) => void
  ): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: { extensionId: string }): void =>
      callback(data)
    ipcRenderer.on('extension:activated', handler)
    return (): void => {
      ipcRenderer.removeListener('extension:activated', handler)
    }
  },

  onCommandRegistered: (callback: (data: { id: string }) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: { id: string }): void =>
      callback(data)
    ipcRenderer.on('extension:commandRegistered', handler)
    return (): void => {
      ipcRenderer.removeListener('extension:commandRegistered', handler)
    }
  },

  onCommandUnregistered: (callback: (data: { id: string }) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: { id: string }): void =>
      callback(data)
    ipcRenderer.on('extension:commandUnregistered', handler)
    return (): void => {
      ipcRenderer.removeListener('extension:commandUnregistered', handler)
    }
  },

  // Webview events
  onWebviewCreate: (
    callback: (data: {
      handle: string
      viewType: string
      title: string
      column: number
      options: unknown
    }) => void
  ): (() => void) => {
    const handler = (
      _event: Electron.IpcRendererEvent,
      data: { handle: string; viewType: string; title: string; column: number; options: unknown }
    ): void => callback(data)
    ipcRenderer.on('webview:create', handler)
    return (): void => {
      ipcRenderer.removeListener('webview:create', handler)
    }
  },

  onWebviewSetHtml: (callback: (data: { handle: string; html: string }) => void): (() => void) => {
    const handler = (
      _event: Electron.IpcRendererEvent,
      data: { handle: string; html: string }
    ): void => callback(data)
    ipcRenderer.on('webview:setHtml', handler)
    return (): void => {
      ipcRenderer.removeListener('webview:setHtml', handler)
    }
  },

  onWebviewDispose: (callback: (data: { handle: string }) => void): (() => void) => {
    const handler = (_event: Electron.IpcRendererEvent, data: { handle: string }): void =>
      callback(data)
    ipcRenderer.on('webview:dispose', handler)
    return (): void => {
      ipcRenderer.removeListener('webview:dispose', handler)
    }
  },

  // Context events
  onContextSet: (callback: (data: { key: string; value: unknown }) => void): (() => void) => {
    const handler = (
      _event: Electron.IpcRendererEvent,
      data: { key: string; value: unknown }
    ): void => callback(data)
    ipcRenderer.on('context:set', handler)
    return (): void => {
      ipcRenderer.removeListener('context:set', handler)
    }
  },

  // Extension reload event
  onExtensionReloaded: (callback: (data: { extensionId: string }) => void): (() => void) => {
    const handler = (
      _event: Electron.IpcRendererEvent,
      data: { extensionId: string }
    ): void => callback(data)
    ipcRenderer.on('extension:reloaded', handler)
    return (): void => {
      ipcRenderer.removeListener('extension:reloaded', handler)
    }
  },

  // Extensions ready event (fired after extension host starts)
  onExtensionsReady: (callback: () => void): (() => void) => {
    const handler = (): void => callback()
    ipcRenderer.on('extensions:ready', handler)
    return (): void => {
      ipcRenderer.removeListener('extensions:ready', handler)
    }
  }
}

// Developer API - dev tools and extension hot reload
const developerAPI = {
  // Toggle Chrome DevTools
  toggleDevTools: (): void => ipcRenderer.send('developer:toggleDevTools'),

  // Set developer mode (starts/stops ExtensionDevServer)
  setDevMode: (enabled: boolean): void => ipcRenderer.send('developer:setDevMode', enabled),

  // Set extension logs only mode
  setExtensionLogsOnly: (enabled: boolean): void =>
    ipcRenderer.send('developer:setExtensionLogsOnly', enabled),

  // Reload all extensions
  reloadExtensions: (): Promise<void> => ipcRenderer.invoke('developer:reloadExtensions'),

  // Reload a specific extension
  reloadExtension: (extensionId: string): Promise<void> =>
    ipcRenderer.invoke('developer:reloadExtension', extensionId)
}

// Project API - high-level data operations
const projectAPI = {
  // Project info
  getProject: (): Promise<unknown> => ipcRenderer.invoke('server:request', '/project'),

  // Devices
  getDevices: (): Promise<unknown[]> => ipcRenderer.invoke('server:request', '/devices'),
  getDevice: (id: string): Promise<unknown> => ipcRenderer.invoke('server:request', `/devices/${id}`),

  // Blueprints
  getBlueprints: (): Promise<unknown[]> => ipcRenderer.invoke('server:request', '/blueprints'),
  getBlueprint: (id: string): Promise<unknown> =>
    ipcRenderer.invoke('server:request', `/blueprints/${id}`),

  // Schedules
  getSchedules: (): Promise<unknown[]> => ipcRenderer.invoke('server:request', '/schedules'),
  getSchedule: (id: string): Promise<unknown> =>
    ipcRenderer.invoke('server:request', `/schedules/${id}`)
}

// Use `contextBridge` APIs to expose Electron APIs to
// renderer only if context isolation is enabled, otherwise
// just add to the DOM global.
if (process.contextIsolated) {
  try {
    contextBridge.exposeInMainWorld('electron', electronAPI)
    contextBridge.exposeInMainWorld('api', api)
    contextBridge.exposeInMainWorld('themeAPI', themeAPI)
    contextBridge.exposeInMainWorld('layoutAPI', layoutAPI)
    contextBridge.exposeInMainWorld('windowAPI', windowAPI)
    contextBridge.exposeInMainWorld('serverAPI', serverAPI)
    contextBridge.exposeInMainWorld('projectAPI', projectAPI)
    contextBridge.exposeInMainWorld('extensionAPI', extensionAPI)
    contextBridge.exposeInMainWorld('developerAPI', developerAPI)
  } catch (error) {
    console.error(error)
  }
} else {
  // @ts-ignore (define in dts)
  window.electron = electronAPI
  // @ts-ignore (define in dts)
  window.api = api
  // @ts-ignore (define in dts)
  window.themeAPI = themeAPI
  // @ts-ignore (define in dts)
  window.layoutAPI = layoutAPI
  // @ts-ignore (define in dts)
  window.windowAPI = windowAPI
  // @ts-ignore (define in dts)
  window.serverAPI = serverAPI
  // @ts-ignore (define in dts)
  window.projectAPI = projectAPI
  // @ts-ignore (define in dts)
  window.extensionAPI = extensionAPI
  // @ts-ignore (define in dts)
  window.developerAPI = developerAPI
}

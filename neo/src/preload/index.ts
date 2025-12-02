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
}

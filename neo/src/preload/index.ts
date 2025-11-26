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
}

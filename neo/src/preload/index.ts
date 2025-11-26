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

// Use `contextBridge` APIs to expose Electron APIs to
// renderer only if context isolation is enabled, otherwise
// just add to the DOM global.
if (process.contextIsolated) {
  try {
    contextBridge.exposeInMainWorld('electron', electronAPI)
    contextBridge.exposeInMainWorld('api', api)
    contextBridge.exposeInMainWorld('themeAPI', themeAPI)
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
}

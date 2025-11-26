import { ipcMain } from 'electron'
import Store from 'electron-store'

export type PanelPosition = 'bottom' | 'top' | 'left' | 'right'

export interface LayoutConfig {
  primarySidebarVisible: boolean
  auxiliaryBarVisible: boolean
  panelVisible: boolean
  panelPosition: PanelPosition
  activeActivityItem: string | null
  activePanelTab: string
}

const defaultLayout: LayoutConfig = {
  primarySidebarVisible: true,
  auxiliaryBarVisible: false,
  panelVisible: true,
  panelPosition: 'bottom',
  activeActivityItem: 'explorer',
  activePanelTab: 'terminal'
}

class LayoutService {
  private store: Store

  constructor() {
    this.store = new Store({ name: 'layout' })
  }

  registerIPC(): void {
    ipcMain.handle('layout:get', () => this.getLayout())
    ipcMain.handle('layout:set', (_, layout: Partial<LayoutConfig>) => this.setLayout(layout))
    ipcMain.handle('layout:reset', () => this.resetLayout())
  }

  getLayout(): LayoutConfig {
    return this.store.get('layout', defaultLayout) as LayoutConfig
  }

  setLayout(layout: Partial<LayoutConfig>): void {
    const current = this.getLayout()
    this.store.set('layout', { ...current, ...layout })
  }

  resetLayout(): void {
    this.store.set('layout', defaultLayout)
  }
}

export const layoutService = new LayoutService()

import type { PaneAPI } from 'paneforge'

export type PanelPosition = 'bottom' | 'top' | 'left' | 'right'

export interface LayoutState {
  primarySidebarVisible: boolean
  auxiliaryBarVisible: boolean
  panelVisible: boolean
  panelPosition: PanelPosition
  activeActivityItem: string | null
  activePanelTab: string
}

const defaultLayoutState: LayoutState = {
  primarySidebarVisible: true,
  auxiliaryBarVisible: false,
  panelVisible: true,
  panelPosition: 'bottom',
  activeActivityItem: 'explorer',
  activePanelTab: 'terminal'
}

interface PaneAPIs {
  primarySidebar?: PaneAPI
  auxiliaryBar?: PaneAPI
  panel?: PaneAPI
}

function createLayoutStore() {
  let state = $state<LayoutState>({ ...defaultLayoutState })
  let paneAPIs: PaneAPIs = {}

  return {
    get state() {
      return state
    },

    setPaneAPIs(apis: PaneAPIs) {
      paneAPIs = apis
    },

    togglePrimarySidebar() {
      if (paneAPIs.primarySidebar) {
        if (paneAPIs.primarySidebar.isCollapsed()) {
          paneAPIs.primarySidebar.expand()
        } else {
          paneAPIs.primarySidebar.collapse()
        }
      }
    },

    toggleAuxiliaryBar() {
      if (paneAPIs.auxiliaryBar) {
        if (paneAPIs.auxiliaryBar.isCollapsed()) {
          paneAPIs.auxiliaryBar.expand()
        } else {
          paneAPIs.auxiliaryBar.collapse()
        }
      }
    },

    togglePanel() {
      if (paneAPIs.panel) {
        if (paneAPIs.panel.isCollapsed()) {
          paneAPIs.panel.expand()
        } else {
          paneAPIs.panel.collapse()
        }
      }
    },

    setPanelPosition(position: PanelPosition) {
      state.panelPosition = position
    },

    setActiveActivityItem(item: string | null) {
      if (state.activeActivityItem === item) {
        this.togglePrimarySidebar()
      } else {
        state.activeActivityItem = item
        // Expand if collapsed
        if (paneAPIs.primarySidebar?.isCollapsed()) {
          paneAPIs.primarySidebar.expand()
        }
      }
    },

    setActivePanelTab(tab: string) {
      state.activePanelTab = tab
      // Expand if collapsed
      if (paneAPIs.panel?.isCollapsed()) {
        paneAPIs.panel.expand()
      }
    },

    setLayout(newState: Partial<LayoutState>) {
      state = { ...state, ...newState }
    },

    resetLayout() {
      state = { ...defaultLayoutState }
    }
  }
}

export const layoutStore = createLayoutStore()

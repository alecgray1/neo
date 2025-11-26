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

function createLayoutStore() {
  let state = $state<LayoutState>({ ...defaultLayoutState })

  return {
    get state() {
      return state
    },

    togglePrimarySidebar() {
      state.primarySidebarVisible = !state.primarySidebarVisible
    },

    toggleAuxiliaryBar() {
      state.auxiliaryBarVisible = !state.auxiliaryBarVisible
    },

    togglePanel() {
      state.panelVisible = !state.panelVisible
    },

    setPanelPosition(position: PanelPosition) {
      state.panelPosition = position
    },

    setActiveActivityItem(item: string | null) {
      if (state.activeActivityItem === item) {
        // Clicking same item toggles sidebar
        state.primarySidebarVisible = !state.primarySidebarVisible
      } else {
        state.activeActivityItem = item
        state.primarySidebarVisible = true
      }
    },

    setActivePanelTab(tab: string) {
      state.activePanelTab = tab
      if (!state.panelVisible) {
        state.panelVisible = true
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

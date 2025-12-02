import { ElectronAPI } from '@electron-toolkit/preload'

// Theme types
export interface ThemeColors {
  background: string
  foreground: string
  primary: string
  primaryForeground: string
  'titleBar.background': string
  'titleBar.foreground': string
  'titleBar.inactiveBackground': string
  'titleBar.inactiveForeground': string
  'sideBar.background': string
  'sideBar.foreground': string
  'sideBar.border': string
  'activityBar.background': string
  'activityBar.foreground': string
  'activityBar.inactiveForeground': string
  'panel.background': string
  'panel.border': string
  'statusBar.background': string
  'statusBar.foreground': string
  'statusBar.noFolderBackground': string
  'input.background': string
  'input.foreground': string
  'input.border': string
  'input.placeholderForeground': string
  'button.background': string
  'button.foreground': string
  'button.hoverBackground': string
  'button.secondaryBackground': string
  'button.secondaryForeground': string
  'button.secondaryHoverBackground': string
  'list.activeSelectionBackground': string
  'list.activeSelectionForeground': string
  'list.hoverBackground': string
  'list.focusBackground': string
  'scrollbar.thumb': string
  'scrollbar.thumbHover': string
  'scrollbar.track': string
  border: string
  focusBorder: string
  separator: string
  'dropdown.background': string
  'dropdown.foreground': string
  'dropdown.border': string
  'checkbox.background': string
  'checkbox.border': string
  'text.link': string
  'text.codeBackground': string
  error: string
  warning: string
  info: string
  success: string
  [key: string]: string // Allow additional custom colors
}

export interface Theme {
  id: string
  name: string
  type: 'dark' | 'light'
  colors: ThemeColors
}

export interface ThemeInfo {
  id: string
  name: string
  type: 'dark' | 'light'
}

export interface ThemeAPI {
  getCurrentTheme(): Promise<Theme | null>
  getAvailableThemes(): Promise<ThemeInfo[]>
  setTheme(themeId: string): Promise<boolean>
  onThemeChanged(callback: (theme: Theme) => void): () => void
}

// Layout types
export type PanelPosition = 'bottom' | 'top' | 'left' | 'right'

export interface LayoutConfig {
  primarySidebarVisible: boolean
  auxiliaryBarVisible: boolean
  panelVisible: boolean
  panelPosition: PanelPosition
  activeActivityItem: string | null
  activePanelTab: string
}

export interface LayoutAPI {
  getLayout(): Promise<LayoutConfig>
  setLayout(layout: Partial<LayoutConfig>): Promise<void>
  resetLayout(): Promise<void>
}

// Window control types
export interface WindowAPI {
  minimize(): Promise<void>
  maximize(): Promise<void>
  close(): Promise<void>
  isMaximized(): Promise<boolean>
  isMac(): Promise<boolean>
  onMaximizedChange(callback: (maximized: boolean) => void): () => void
}

// Server connection types
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

export interface ServerAPI {
  // Connection management
  connect(config?: Partial<ServerConfig>): Promise<boolean>
  disconnect(): Promise<void>
  getState(): Promise<ConnectionState>
  getConfig(): Promise<ServerConfig>
  setConfig(config: Partial<ServerConfig>): Promise<void>

  // Request/response
  request<T = unknown>(path: string, params?: Record<string, unknown>): Promise<T>

  // Subscriptions
  subscribe(paths: string[]): Promise<void>
  unsubscribe(paths: string[]): Promise<void>
  getSubscriptions(): Promise<string[]>

  // Event listeners
  onStateChanged(callback: (state: ConnectionState) => void): () => void
  onChange(callback: (event: ChangeEvent) => void): () => void
}

// Project data types
export interface ProjectAPI {
  getProject(): Promise<unknown>
  getDevices(): Promise<unknown[]>
  getDevice(id: string): Promise<unknown>
  getBlueprints(): Promise<unknown[]>
  getBlueprint(id: string): Promise<unknown>
  getSchedules(): Promise<unknown[]>
  getSchedule(id: string): Promise<unknown>
}

declare global {
  interface Window {
    electron: ElectronAPI
    api: unknown
    themeAPI: ThemeAPI
    layoutAPI: LayoutAPI
    windowAPI: WindowAPI
    serverAPI: ServerAPI
    projectAPI: ProjectAPI
  }
}

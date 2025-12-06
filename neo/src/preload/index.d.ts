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

  // Send arbitrary message to server
  send<T = unknown>(message: Record<string, unknown>): Promise<T>

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

// Extension types
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

export interface ExtensionAPI {
  // Get all contributions from extensions
  getContributions(): Promise<CollectedContributions>

  // Get list of installed extensions
  getExtensions(): Promise<ExtensionInfo[]>

  // Execute an extension command
  executeCommand<T = unknown>(id: string, ...args: unknown[]): Promise<T>

  // Listen for extension events
  onExtensionActivated(callback: (data: { extensionId: string }) => void): () => void
  onCommandRegistered(callback: (data: { id: string }) => void): () => void
  onCommandUnregistered(callback: (data: { id: string }) => void): () => void

  // Webview events
  onWebviewCreate(
    callback: (data: {
      handle: string
      viewType: string
      title: string
      column: number
      options: unknown
    }) => void
  ): () => void
  onWebviewSetHtml(callback: (data: { handle: string; html: string }) => void): () => void
  onWebviewDispose(callback: (data: { handle: string }) => void): () => void

  // Context events
  onContextSet(callback: (data: { key: string; value: unknown }) => void): () => void
}

// BACnet types
export interface DiscoveredDevice {
  device_id: number
  address: string
  max_apdu: number
  vendor_id: number
  segmentation: string
  vendor_name?: string
  model_name?: string
  object_name?: string
}

export interface DiscoveryOptions {
  lowLimit?: number
  highLimit?: number
  duration?: number
}

export interface DeviceAddedResult {
  deviceId: number
  entityId: number
}

export interface BACnetAPI {
  // Discovery
  startDiscovery(options?: DiscoveryOptions): Promise<void>
  stopDiscovery(): Promise<void>
  onDiscoveryStarted(callback: (id: string) => void): () => void
  onDeviceFound(callback: (device: DiscoveredDevice, alreadyExists: boolean) => void): () => void
  onDiscoveryComplete(callback: (devicesFound: number) => void): () => void

  // Device management
  addDevice(device: DiscoveredDevice): Promise<DeviceAddedResult>
  removeDevice(deviceId: number): Promise<void>
  onDeviceAdded(callback: (deviceId: number, entityId: number) => void): () => void
  onDeviceRemoved(callback: (deviceId: number) => void): () => void
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
    extensionAPI: ExtensionAPI
    bacnetAPI: BACnetAPI
  }
}

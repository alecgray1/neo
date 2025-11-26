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

declare global {
  interface Window {
    electron: ElectronAPI
    api: unknown
    themeAPI: ThemeAPI
  }
}

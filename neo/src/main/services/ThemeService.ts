import { app, ipcMain, BrowserWindow } from 'electron'
import * as chokidar from 'chokidar'
import * as fs from 'fs'
import * as path from 'path'
import Store from 'electron-store'

export interface Theme {
  name: string
  type: 'dark' | 'light'
  colors: Record<string, string>
}

interface ThemeManifest {
  id: string
  path: string
  theme: Theme
}

export interface ThemeWithId extends Theme {
  id: string
}

export interface ThemeInfo {
  id: string
  name: string
  type: 'dark' | 'light'
}

class ThemeService {
  private themes: Map<string, ThemeManifest> = new Map()
  private currentThemeId: string
  private watcher: chokidar.FSWatcher | null = null
  private store: Store
  private mainWindow: BrowserWindow | null = null

  constructor() {
    this.store = new Store({ name: 'settings' })
    this.currentThemeId = this.store.get('theme', 'neo-dark') as string
  }

  /**
   * Get the themes directory path
   * In development: project/themes
   * In production: resources/themes
   */
  private getThemesDir(): string {
    if (app.isPackaged) {
      return path.join(process.resourcesPath, 'themes')
    }
    return path.join(app.getAppPath(), 'themes')
  }

  /**
   * Load all themes from the themes directory
   */
  loadThemes(): void {
    const themesDir = this.getThemesDir()

    if (!fs.existsSync(themesDir)) {
      console.warn(`Themes directory not found: ${themesDir}`)
      return
    }

    const files = fs.readdirSync(themesDir).filter((f) => {
      return f.endsWith('.json') && f !== 'theme.schema.json'
    })

    for (const file of files) {
      const id = path.basename(file, '.json')
      const themePath = path.join(themesDir, file)
      this.loadThemeFile(id, themePath)
    }

    console.log(`Loaded ${this.themes.size} themes`)

    // Validate current theme exists, fallback to neo-dark
    if (!this.themes.has(this.currentThemeId)) {
      console.warn(`Theme "${this.currentThemeId}" not found, falling back to neo-dark`)
      this.currentThemeId = 'neo-dark'
      this.store.set('theme', this.currentThemeId)
    }
  }

  /**
   * Load a single theme file
   */
  private loadThemeFile(id: string, themePath: string): void {
    try {
      const content = fs.readFileSync(themePath, 'utf-8')
      const theme = JSON.parse(content) as Theme
      this.themes.set(id, { id, path: themePath, theme })
      console.log(`Loaded theme: ${theme.name} (${id})`)
    } catch (e) {
      console.error(`Failed to load theme ${id}:`, e)
    }
  }

  /**
   * Start watching the themes directory for changes
   */
  startWatching(): void {
    const themesDir = this.getThemesDir()

    this.watcher = chokidar.watch(path.join(themesDir, '*.json'), {
      ignoreInitial: true,
      awaitWriteFinish: {
        stabilityThreshold: 100,
        pollInterval: 50
      }
    })

    this.watcher.on('change', (filePath) => {
      const id = path.basename(filePath, '.json')
      if (id === 'theme.schema') return

      console.log(`Theme file changed: ${id}`)
      this.loadThemeFile(id, filePath)

      // If the changed theme is the current theme, notify renderer
      if (id === this.currentThemeId) {
        this.notifyThemeChanged()
      }
    })

    this.watcher.on('add', (filePath) => {
      const id = path.basename(filePath, '.json')
      if (id === 'theme.schema') return

      console.log(`New theme file detected: ${id}`)
      this.loadThemeFile(id, filePath)
    })

    this.watcher.on('unlink', (filePath) => {
      const id = path.basename(filePath, '.json')
      if (this.themes.has(id)) {
        console.log(`Theme file removed: ${id}`)
        this.themes.delete(id)

        // If the removed theme was current, switch to default
        if (id === this.currentThemeId) {
          this.setTheme('neo-dark')
        }
      }
    })

    console.log(`Watching for theme changes in: ${themesDir}`)
  }

  /**
   * Stop watching for theme changes
   */
  stopWatching(): void {
    if (this.watcher) {
      this.watcher.close()
      this.watcher = null
    }
  }

  /**
   * Register IPC handlers for theme operations
   */
  registerIPC(): void {
    ipcMain.handle('theme:get-current', () => this.getCurrentTheme())
    ipcMain.handle('theme:get-available', () => this.getAvailableThemes())
    ipcMain.handle('theme:set', (_, themeId: string) => this.setTheme(themeId))
  }

  /**
   * Get the current theme with its ID
   */
  getCurrentTheme(): ThemeWithId | null {
    const manifest = this.themes.get(this.currentThemeId)
    if (!manifest) return null

    return {
      id: this.currentThemeId,
      ...manifest.theme
    }
  }

  /**
   * Get list of available themes
   */
  getAvailableThemes(): ThemeInfo[] {
    return Array.from(this.themes.entries()).map(([id, manifest]) => ({
      id,
      name: manifest.theme.name,
      type: manifest.theme.type
    }))
  }

  /**
   * Set the current theme
   */
  setTheme(themeId: string): boolean {
    if (!this.themes.has(themeId)) {
      console.warn(`Theme not found: ${themeId}`)
      return false
    }

    this.currentThemeId = themeId
    this.store.set('theme', themeId)
    this.notifyThemeChanged()
    console.log(`Theme changed to: ${themeId}`)
    return true
  }

  /**
   * Notify the renderer process that the theme has changed
   */
  private notifyThemeChanged(): void {
    const theme = this.getCurrentTheme()
    if (theme && this.mainWindow && !this.mainWindow.isDestroyed()) {
      this.mainWindow.webContents.send('theme:changed', theme)
    }
  }

  /**
   * Set the main window reference for IPC communication
   */
  setMainWindow(window: BrowserWindow): void {
    this.mainWindow = window
  }
}

// Export singleton instance
export const themeService = new ThemeService()

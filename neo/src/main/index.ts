import { app, shell, BrowserWindow, ipcMain } from 'electron'
import { join } from 'path'
import { electronApp, optimizer, is } from '@electron-toolkit/utils'
import icon from '../../resources/icon.png?asset'
import { themeService } from './services/ThemeService'
import { layoutService } from './services/LayoutService'
import { webSocketService } from './services/WebSocketService'
import { getExtensionHostMain } from './extensionHost'

const isMac = process.platform === 'darwin'

function createWindow(): BrowserWindow {
  // Create the browser window with platform-specific frameless settings
  const mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    show: false,
    autoHideMenuBar: true,
    // Platform-specific frameless window settings (like VS Code)
    frame: isMac, // Keep frame on macOS for native traffic lights
    titleBarStyle: isMac ? 'hiddenInset' : undefined,
    trafficLightPosition: isMac ? { x: 10, y: 10 } : undefined,
    ...(process.platform === 'linux' ? { icon } : {}),
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      sandbox: false
    }
  })

  mainWindow.on('ready-to-show', () => {
    mainWindow.show()
  })

  mainWindow.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  // HMR for renderer base on electron-vite cli.
  // Load the remote URL for development or the local html file for production.
  if (is.dev && process.env['ELECTRON_RENDERER_URL']) {
    mainWindow.loadURL(process.env['ELECTRON_RENDERER_URL'])
  } else {
    mainWindow.loadFile(join(__dirname, '../renderer/index.html'))
  }

  return mainWindow
}

// This method will be called when Electron has finished
// initialization and is ready to create browser windows.
// Some APIs can only be used after this event occurs.
app.whenReady().then(() => {
  // Set app user model id for windows
  electronApp.setAppUserModelId('com.electron')

  // Default open or close DevTools by F12 in development
  // and ignore CommandOrControl + R in production.
  // see https://github.com/alex8088/electron-toolkit/tree/master/packages/utils
  app.on('browser-window-created', (_, window) => {
    optimizer.watchWindowShortcuts(window)
  })

  // IPC test
  ipcMain.on('ping', () => console.log('pong'))

  // Initialize theme service
  themeService.loadThemes()
  themeService.registerIPC()
  themeService.startWatching()

  // Initialize layout service
  layoutService.registerIPC()

  // Initialize WebSocket service
  webSocketService.registerIPC()

  // Create window and connect to services
  const mainWindow = createWindow()
  themeService.setMainWindow(mainWindow)
  webSocketService.setMainWindow(mainWindow)

  // Initialize extension host
  const extensionHost = getExtensionHostMain()
  extensionHost.setMainWindow(mainWindow)

  // Start extension host after window is ready
  mainWindow.webContents.once('did-finish-load', async () => {
    try {
      await extensionHost.start()
      console.log('[Main] Extension host started')
    } catch (err) {
      console.error('[Main] Failed to start extension host:', err)
    }
  })

  // Window control IPC handlers
  ipcMain.handle('window:minimize', () => mainWindow.minimize())
  ipcMain.handle('window:maximize', () => {
    if (mainWindow.isMaximized()) {
      mainWindow.unmaximize()
    } else {
      mainWindow.maximize()
    }
  })
  ipcMain.handle('window:close', () => mainWindow.close())
  ipcMain.handle('window:isMaximized', () => mainWindow.isMaximized())
  ipcMain.handle('window:isMac', () => isMac)

  // Notify renderer of maximize state changes
  mainWindow.on('maximize', () => mainWindow.webContents.send('window:maximized-change', true))
  mainWindow.on('unmaximize', () => mainWindow.webContents.send('window:maximized-change', false))

  app.on('activate', function () {
    // On macOS it's common to re-create a window in the app when the
    // dock icon is clicked and there are no other windows open.
    if (BrowserWindow.getAllWindows().length === 0) {
      const newWindow = createWindow()
      themeService.setMainWindow(newWindow)
    }
  })
})

// Quit when all windows are closed, except on macOS. There, it's common
// for applications and their menu bar to stay active until the user quits
// explicitly with Cmd + Q.
app.on('window-all-closed', () => {
  // Clean up extension host
  getExtensionHostMain().dispose()

  if (process.platform !== 'darwin') {
    app.quit()
  }
})

// In this file you can include the rest of your app's specific main process
// code. You can also put them in separate files and require them here.

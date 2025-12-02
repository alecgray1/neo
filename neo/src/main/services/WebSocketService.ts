import { ipcMain, BrowserWindow } from 'electron'
import WebSocket from 'ws'
import Store from 'electron-store'

// Message types matching Rust server protocol
export interface ClientMessage {
  type: 'Subscribe' | 'Unsubscribe' | 'Get' | 'Update' | 'Create' | 'Delete' | 'Ping'
  id: string
  paths?: string[]
  path?: string
  data?: unknown
}

export interface ServerMessage {
  type: 'Connected' | 'Response' | 'Change' | 'Error' | 'Pong'
  session_id?: string
  server_version?: string
  id?: string
  success?: boolean
  data?: unknown
  error?: string
  path?: string
  change_type?: 'created' | 'updated' | 'deleted'
  code?: string
  message?: string
}

export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting'

export interface ServerConfig {
  host: string
  port: number
}

interface PendingRequest {
  resolve: (value: unknown) => void
  reject: (error: Error) => void
  timeout: NodeJS.Timeout
}

class WebSocketService {
  private ws: WebSocket | null = null
  private mainWindow: BrowserWindow | null = null
  private store: Store
  private config: ServerConfig
  private connectionState: ConnectionState = 'disconnected'
  private reconnectAttempts = 0
  private maxReconnectAttempts = 10
  private reconnectInterval = 3000
  private reconnectTimer: NodeJS.Timeout | null = null
  private pendingRequests = new Map<string, PendingRequest>()
  private subscriptions = new Set<string>()
  private requestIdCounter = 0

  constructor() {
    this.store = new Store({ name: 'server' })
    this.config = {
      host: this.store.get('host', 'localhost') as string,
      port: this.store.get('port', 9600) as number
    }
  }

  setMainWindow(window: BrowserWindow): void {
    this.mainWindow = window
  }

  registerIPC(): void {
    // Connection management
    ipcMain.handle('server:connect', (_, config?: Partial<ServerConfig>) => this.connect(config))
    ipcMain.handle('server:disconnect', () => this.disconnect())
    ipcMain.handle('server:getState', () => this.getConnectionState())
    ipcMain.handle('server:getConfig', () => this.config)
    ipcMain.handle('server:setConfig', (_, config: Partial<ServerConfig>) => this.setConfig(config))

    // Request/response
    ipcMain.handle('server:request', (_, path: string, params?: Record<string, unknown>) =>
      this.request(path, params)
    )

    // Subscriptions
    ipcMain.handle('server:subscribe', (_, paths: string[]) => this.subscribe(paths))
    ipcMain.handle('server:unsubscribe', (_, paths: string[]) => this.unsubscribe(paths))
    ipcMain.handle('server:getSubscriptions', () => Array.from(this.subscriptions))
  }

  private async connect(config?: Partial<ServerConfig>): Promise<boolean> {
    if (config) {
      Object.assign(this.config, config)
      this.store.set('host', this.config.host)
      this.store.set('port', this.config.port)
    }

    if (this.ws?.readyState === WebSocket.OPEN) {
      return true
    }

    this.setConnectionState('connecting')

    return new Promise((resolve) => {
      const url = `ws://${this.config.host}:${this.config.port}/ws`
      console.log(`Connecting to ${url}...`)

      try {
        this.ws = new WebSocket(url)

        this.ws.on('open', () => {
          console.log('WebSocket connected')
          this.setConnectionState('connected')
          this.reconnectAttempts = 0
          // Resubscribe to all active subscriptions
          this.resubscribeAll()
          resolve(true)
        })

        this.ws.on('close', () => {
          console.log('WebSocket closed')
          this.handleDisconnect()
          resolve(false)
        })

        this.ws.on('error', (error) => {
          console.error('WebSocket error:', error)
          this.handleDisconnect()
          resolve(false)
        })

        this.ws.on('message', (data) => {
          this.handleMessage(data.toString())
        })
      } catch (error) {
        console.error('Failed to create WebSocket:', error)
        this.setConnectionState('disconnected')
        resolve(false)
      }
    })
  }

  private disconnect(): void {
    this.clearReconnectTimer()
    this.subscriptions.clear()
    this.pendingRequests.forEach((pending) => {
      clearTimeout(pending.timeout)
      pending.reject(new Error('Disconnected'))
    })
    this.pendingRequests.clear()

    if (this.ws) {
      this.ws.close()
      this.ws = null
    }
    this.setConnectionState('disconnected')
  }

  private handleDisconnect(): void {
    if (this.connectionState === 'disconnected') return

    this.ws = null

    // Reject all pending requests
    this.pendingRequests.forEach((pending) => {
      clearTimeout(pending.timeout)
      pending.reject(new Error('Connection lost'))
    })
    this.pendingRequests.clear()

    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.setConnectionState('reconnecting')
      this.scheduleReconnect()
    } else {
      this.setConnectionState('disconnected')
    }
  }

  private scheduleReconnect(): void {
    this.clearReconnectTimer()
    this.reconnectTimer = setTimeout(() => {
      this.reconnectAttempts++
      console.log(`Reconnect attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts}`)
      this.connect()
    }, this.reconnectInterval)
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }

  private handleMessage(data: string): void {
    try {
      const message: ServerMessage = JSON.parse(data)

      switch (message.type) {
        case 'Connected':
          console.log(`Connected to server, session: ${message.session_id}`)
          break

        case 'Response':
          if (message.id) {
            const pending = this.pendingRequests.get(message.id)
            if (pending) {
              clearTimeout(pending.timeout)
              this.pendingRequests.delete(message.id)
              if (message.success) {
                pending.resolve(message.data)
              } else {
                pending.reject(new Error(message.error || 'Request failed'))
              }
            }
          }
          break

        case 'Change':
          // Forward change events to renderer
          this.notifyRenderer('server:change', {
            path: message.path,
            changeType: message.change_type,
            data: message.data
          })
          break

        case 'Error':
          if (message.id) {
            const pending = this.pendingRequests.get(message.id)
            if (pending) {
              clearTimeout(pending.timeout)
              this.pendingRequests.delete(message.id)
              pending.reject(new Error(message.message || 'Unknown error'))
            }
          }
          console.error('Server error:', message.message)
          break

        case 'Pong':
          // Keep-alive response, nothing to do
          break
      }
    } catch (e) {
      console.error('Failed to parse server message:', e)
    }
  }

  private async request(path: string, params?: Record<string, unknown>): Promise<unknown> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error('Not connected to server')
    }

    const id = this.generateRequestId()

    // Determine message type based on params
    let message: ClientMessage
    if (params?.action === 'create') {
      message = { type: 'Create', id, path, data: params.data }
    } else if (params?.action === 'update') {
      message = { type: 'Update', id, path, data: params.data }
    } else if (params?.action === 'delete') {
      message = { type: 'Delete', id, path }
    } else {
      message = { type: 'Get', id, path }
    }

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id)
        reject(new Error('Request timeout'))
      }, 30000)

      this.pendingRequests.set(id, { resolve, reject, timeout })
      this.ws!.send(JSON.stringify(message))
    })
  }

  private async subscribe(paths: string[]): Promise<void> {
    paths.forEach((p) => this.subscriptions.add(p))

    if (this.ws?.readyState === WebSocket.OPEN) {
      const id = this.generateRequestId()
      const message: ClientMessage = { type: 'Subscribe', id, paths }
      this.ws.send(JSON.stringify(message))
    }
  }

  private async unsubscribe(paths: string[]): Promise<void> {
    paths.forEach((p) => this.subscriptions.delete(p))

    if (this.ws?.readyState === WebSocket.OPEN) {
      const id = this.generateRequestId()
      const message: ClientMessage = { type: 'Unsubscribe', id, paths }
      this.ws.send(JSON.stringify(message))
    }
  }

  private resubscribeAll(): void {
    if (this.subscriptions.size > 0 && this.ws?.readyState === WebSocket.OPEN) {
      const id = this.generateRequestId()
      const message: ClientMessage = {
        type: 'Subscribe',
        id,
        paths: Array.from(this.subscriptions)
      }
      this.ws.send(JSON.stringify(message))
    }
  }

  private generateRequestId(): string {
    return `req-${++this.requestIdCounter}`
  }

  private setConnectionState(state: ConnectionState): void {
    this.connectionState = state
    this.notifyRenderer('server:state-changed', {
      state,
      reconnectAttempts: this.reconnectAttempts
    })
  }

  private getConnectionState(): { state: ConnectionState; reconnectAttempts: number } {
    return { state: this.connectionState, reconnectAttempts: this.reconnectAttempts }
  }

  private setConfig(config: Partial<ServerConfig>): void {
    Object.assign(this.config, config)
    this.store.set('host', this.config.host)
    this.store.set('port', this.config.port)
  }

  private notifyRenderer(channel: string, data: unknown): void {
    if (this.mainWindow && !this.mainWindow.isDestroyed()) {
      this.mainWindow.webContents.send(channel, data)
    }
  }
}

export const webSocketService = new WebSocketService()

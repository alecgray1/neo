/**
 * ExtensionDevServer - WebSocket server for extension hot reload
 *
 * When developer mode is enabled, this server listens on port 9601
 * for messages from the vite plugin to trigger extension reloads.
 *
 * Protocol:
 * - Vite plugin connects when building app target in watch mode
 * - On rebuild, vite sends: { type: "extension:reload", extensionId }
 * - Server triggers extension reload in ExtensionHostMain
 */

import { WebSocketServer, WebSocket } from 'ws'
import { getExtensionHostMain } from './index'

const DEV_SERVER_PORT = 9601

interface DevMessage {
  type: string
  extensionId?: string
}

class ExtensionDevServer {
  private _wss: WebSocketServer | null = null
  private _clients: Set<WebSocket> = new Set()

  /**
   * Start the WebSocket server
   */
  start(port: number = DEV_SERVER_PORT): void {
    if (this._wss) {
      console.log('[ExtensionDevServer] Already running')
      return
    }

    try {
      this._wss = new WebSocketServer({ port })

      this._wss.on('connection', (ws) => {
        console.log('[ExtensionDevServer] Client connected')
        this._clients.add(ws)

        ws.on('message', async (data) => {
          try {
            const msg: DevMessage = JSON.parse(data.toString())
            await this._handleMessage(msg)
          } catch (err) {
            console.error('[ExtensionDevServer] Failed to parse message:', err)
          }
        })

        ws.on('close', () => {
          console.log('[ExtensionDevServer] Client disconnected')
          this._clients.delete(ws)
        })

        ws.on('error', (err) => {
          console.error('[ExtensionDevServer] WebSocket error:', err)
          this._clients.delete(ws)
        })
      })

      this._wss.on('error', (err) => {
        console.error('[ExtensionDevServer] Server error:', err)
      })

      console.log(`[ExtensionDevServer] Started on port ${port}`)
    } catch (err) {
      console.error('[ExtensionDevServer] Failed to start:', err)
    }
  }

  /**
   * Stop the WebSocket server
   */
  stop(): void {
    if (!this._wss) {
      return
    }

    // Close all client connections
    for (const client of this._clients) {
      client.close()
    }
    this._clients.clear()

    // Close the server
    this._wss.close()
    this._wss = null

    console.log('[ExtensionDevServer] Stopped')
  }

  /**
   * Check if the server is running
   */
  isRunning(): boolean {
    return this._wss !== null
  }

  /**
   * Handle incoming messages
   */
  private async _handleMessage(msg: DevMessage): Promise<void> {
    switch (msg.type) {
      case 'extension:reload':
        if (msg.extensionId) {
          console.log(`[ExtensionDevServer] Reloading extension: ${msg.extensionId}`)
          const extensionHost = getExtensionHostMain()
          await extensionHost.reloadExtension(msg.extensionId)
        }
        break

      case 'extension:reloadAll':
        console.log('[ExtensionDevServer] Reloading all extensions')
        const extensionHost = getExtensionHostMain()
        await extensionHost.reloadAllExtensions()
        break

      case 'ping':
        // Heartbeat - broadcast pong to all clients
        this._broadcast({ type: 'pong' })
        break

      default:
        console.log('[ExtensionDevServer] Unknown message type:', msg.type)
    }
  }

  /**
   * Broadcast a message to all connected clients
   */
  private _broadcast(msg: object): void {
    const data = JSON.stringify(msg)
    for (const client of this._clients) {
      if (client.readyState === WebSocket.OPEN) {
        client.send(data)
      }
    }
  }
}

// Singleton instance
let _instance: ExtensionDevServer | null = null

/**
 * Get the ExtensionDevServer singleton
 */
export function getExtensionDevServer(): ExtensionDevServer {
  if (!_instance) {
    _instance = new ExtensionDevServer()
  }
  return _instance
}

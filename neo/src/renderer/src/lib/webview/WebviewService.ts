/**
 * Webview Service
 *
 * Manages webview instances for extension-contributed webviews.
 * Handles webview lifecycle, content updates, and messaging.
 */

import { type IDisposable, DisposableStore, Emitter, type Event } from '$lib/services/types'

/**
 * Webview panel instance
 */
export interface WebviewInstance {
  readonly handle: string
  readonly viewType: string
  title: string
  column: number
  html: string
  visible: boolean
  active: boolean
  options?: WebviewOptions
}

/**
 * Webview options
 */
export interface WebviewOptions {
  enableScripts?: boolean
  enableForms?: boolean
  localResourceRoots?: string[]
}

/**
 * Webview view instance (sidebar/panel views)
 */
export interface WebviewViewInstance {
  readonly handle: string
  readonly viewId: string
  title: string
  description?: string
  html: string
  visible: boolean
}

/**
 * Events for webview changes
 */
export interface WebviewChangeEvent {
  handle: string
  type: 'created' | 'updated' | 'disposed' | 'revealed' | 'html-changed'
}

export interface IWebviewService extends IDisposable {
  /**
   * Get all webview panels
   */
  getPanels(): ReadonlyArray<WebviewInstance>

  /**
   * Get a specific webview panel by handle
   */
  getPanel(handle: string): WebviewInstance | undefined

  /**
   * Get all webview views
   */
  getViews(): ReadonlyArray<WebviewViewInstance>

  /**
   * Get a specific webview view by handle
   */
  getView(handle: string): WebviewViewInstance | undefined

  /**
   * Event fired when webview panels change
   */
  onDidChangePanels: Event<WebviewChangeEvent>

  /**
   * Event fired when webview views change
   */
  onDidChangeViews: Event<WebviewChangeEvent>
}

class WebviewService implements IWebviewService {
  private _panels = new Map<string, WebviewInstance>()
  private _views = new Map<string, WebviewViewInstance>()
  private _disposables = new DisposableStore()

  private _onDidChangePanels = new Emitter<WebviewChangeEvent>()
  private _onDidChangeViews = new Emitter<WebviewChangeEvent>()

  get onDidChangePanels(): Event<WebviewChangeEvent> {
    return this._onDidChangePanels.event
  }

  get onDidChangeViews(): Event<WebviewChangeEvent> {
    return this._onDidChangeViews.event
  }

  constructor() {
    this._setupEventListeners()
  }

  private _setupEventListeners(): void {
    // Listen for webview panel events from the main process
    const unsubCreate = window.extensionAPI.onWebviewCreate((data) => {
      const panel: WebviewInstance = {
        handle: data.handle,
        viewType: data.viewType,
        title: data.title,
        column: data.column,
        html: '',
        visible: true,
        active: true,
        options: data.options as WebviewOptions
      }
      this._panels.set(data.handle, panel)
      this._onDidChangePanels.fire({ handle: data.handle, type: 'created' })
    })
    this._disposables.add({ dispose: unsubCreate })

    const unsubSetHtml = window.extensionAPI.onWebviewSetHtml((data) => {
      const panel = this._panels.get(data.handle)
      if (panel) {
        panel.html = data.html
        this._onDidChangePanels.fire({ handle: data.handle, type: 'html-changed' })
      }

      const view = this._views.get(data.handle)
      if (view) {
        view.html = data.html
        this._onDidChangeViews.fire({ handle: data.handle, type: 'html-changed' })
      }
    })
    this._disposables.add({ dispose: unsubSetHtml })

    const unsubDispose = window.extensionAPI.onWebviewDispose((data) => {
      if (this._panels.has(data.handle)) {
        this._panels.delete(data.handle)
        this._onDidChangePanels.fire({ handle: data.handle, type: 'disposed' })
      }

      if (this._views.has(data.handle)) {
        this._views.delete(data.handle)
        this._onDidChangeViews.fire({ handle: data.handle, type: 'disposed' })
      }
    })
    this._disposables.add({ dispose: unsubDispose })
  }

  getPanels(): ReadonlyArray<WebviewInstance> {
    return Array.from(this._panels.values())
  }

  getPanel(handle: string): WebviewInstance | undefined {
    return this._panels.get(handle)
  }

  getViews(): ReadonlyArray<WebviewViewInstance> {
    return Array.from(this._views.values())
  }

  getView(handle: string): WebviewViewInstance | undefined {
    return this._views.get(handle)
  }

  /**
   * Create a webview view (called when a view provider is resolved)
   */
  createView(handle: string, viewId: string, title: string): WebviewViewInstance {
    const view: WebviewViewInstance = {
      handle,
      viewId,
      title,
      html: '',
      visible: true
    }
    this._views.set(handle, view)
    this._onDidChangeViews.fire({ handle, type: 'created' })
    return view
  }

  dispose(): void {
    this._disposables.dispose()
    this._onDidChangePanels.dispose()
    this._onDidChangeViews.dispose()
    this._panels.clear()
    this._views.clear()
  }
}

// Singleton instance
let _webviewService: WebviewService | null = null

export function getWebviewService(): IWebviewService {
  if (!_webviewService) {
    _webviewService = new WebviewService()
  }
  return _webviewService
}

export function resetWebviewService(): void {
  _webviewService?.dispose()
  _webviewService = null
}

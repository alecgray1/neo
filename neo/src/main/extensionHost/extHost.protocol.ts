/**
 * Extension Host Protocol Definitions
 *
 * Defines the shape interfaces for RPC communication between
 * the main process and extension host.
 *
 * Convention:
 * - MainThread* shapes are implemented in main process, called from extension host
 * - ExtHost* shapes are implemented in extension host, called from main process
 * - Methods prefixed with $ are RPC-callable
 */

import { createProxyIdentifier } from './protocol'

// ============================================================================
// Common Types
// ============================================================================

export interface QuickPickItem {
  label: string
  description?: string
  detail?: string
  picked?: boolean
  alwaysShow?: boolean
}

export interface QuickPickOptions {
  title?: string
  placeholder?: string
  canPickMany?: boolean
  matchOnDescription?: boolean
  matchOnDetail?: boolean
}

export interface InputBoxOptions {
  title?: string
  placeholder?: string
  value?: string
  prompt?: string
  password?: boolean
  validateInput?: (value: string) => string | undefined
}

export interface WebviewOptions {
  enableScripts?: boolean
  enableForms?: boolean
  localResourceRoots?: string[]
  retainContextWhenHidden?: boolean
}

export interface ChangeEvent {
  path: string
  changeType: 'created' | 'updated' | 'deleted'
  data: unknown
}

export interface ExtensionInfo {
  id: string
  name: string
  version: string
  extensionPath: string
}

// ============================================================================
// Main Thread Shapes (implemented in main process)
// ============================================================================

/**
 * Command registration and execution
 */
export interface MainThreadCommandsShape {
  $registerCommand(id: string): void
  $unregisterCommand(id: string): void
  $executeCommand<T = unknown>(id: string, args: unknown[]): Promise<T>
  $getCommands(): Promise<string[]>
}

/**
 * Window/UI operations
 */
export interface MainThreadWindowShape {
  $showMessage(
    type: 'info' | 'warning' | 'error',
    message: string,
    items: string[]
  ): Promise<string | undefined>
  $showQuickPick(items: QuickPickItem[], options?: QuickPickOptions): Promise<QuickPickItem | undefined>
  $showInputBox(options?: InputBoxOptions): Promise<string | undefined>
}

/**
 * Webview management
 */
export interface MainThreadWebviewsShape {
  $createWebviewPanel(
    handle: string,
    viewType: string,
    title: string,
    column: number,
    options?: WebviewOptions
  ): void
  $disposeWebview(handle: string): void
  $setWebviewHtml(handle: string, html: string): void
  $setWebviewTitle(handle: string, title: string): void
  $postMessage(handle: string, message: unknown): Promise<boolean>
  $reveal(handle: string, column?: number, preserveFocus?: boolean): void

  // Webview views (sidebar panels)
  $registerWebviewViewProvider(viewId: string): void
  $unregisterWebviewViewProvider(viewId: string): void
  $setWebviewViewHtml(handle: string, html: string): void
  $setWebviewViewTitle(handle: string, title: string): void
  $setWebviewViewDescription(handle: string, description: string | undefined): void
}

/**
 * Context key management
 */
export interface MainThreadContextShape {
  $setContext(key: string, value: unknown): void
  $removeContext(key: string): void
}

/**
 * Neo server communication
 */
export interface MainThreadServerShape {
  $request(path: string, params?: Record<string, unknown>): Promise<unknown>
  $subscribe(paths: string[]): Promise<string> // Returns subscription ID
  $unsubscribe(subscriptionId: string): void
  $getConnectionState(): Promise<{ connected: boolean }>
}

/**
 * Extension lifecycle management
 */
export interface MainThreadExtensionsShape {
  $onExtensionActivated(extensionId: string): void
  $onExtensionActivationFailed(extensionId: string, error: string): void
  $getExtensions(): Promise<ExtensionInfo[]>
}

// ============================================================================
// Extension Host Shapes (implemented in extension host)
// ============================================================================

/**
 * Command execution in extension host
 */
export interface ExtHostCommandsShape {
  $executeContributedCommand<T = unknown>(id: string, args: unknown[]): Promise<T>
}

/**
 * Webview message handling
 */
export interface ExtHostWebviewsShape {
  $onDidReceiveMessage(handle: string, message: unknown): void
  $onDidDisposeWebview(handle: string): void
  $onDidChangeWebviewVisibility(handle: string, visible: boolean): void

  // Webview views
  $resolveWebviewView(
    handle: string,
    viewId: string,
    title: string | undefined,
    state: unknown
  ): Promise<void>
  $onDidDisposeWebviewView(handle: string): void
  $onDidChangeWebviewViewVisibility(handle: string, visible: boolean): void
}

/**
 * Server event handling
 */
export interface ExtHostServerShape {
  $onServerEvent(event: ChangeEvent): void
  $onConnectionStateChanged(connected: boolean): void
}

/**
 * Extension lifecycle
 */
export interface ExtHostExtensionsShape {
  $activateExtension(extensionId: string): Promise<void>
  $deactivateExtension(extensionId: string): Promise<void>
}

// ============================================================================
// Proxy Identifiers
// ============================================================================

/**
 * Main thread proxies - call these from extension host
 */
export const MainContext = {
  MainThreadCommands: createProxyIdentifier<MainThreadCommandsShape>('MainThreadCommands'),
  MainThreadWindow: createProxyIdentifier<MainThreadWindowShape>('MainThreadWindow'),
  MainThreadWebviews: createProxyIdentifier<MainThreadWebviewsShape>('MainThreadWebviews'),
  MainThreadContext: createProxyIdentifier<MainThreadContextShape>('MainThreadContext'),
  MainThreadServer: createProxyIdentifier<MainThreadServerShape>('MainThreadServer'),
  MainThreadExtensions: createProxyIdentifier<MainThreadExtensionsShape>('MainThreadExtensions')
}

/**
 * Extension host proxies - call these from main thread
 */
export const ExtHostContext = {
  ExtHostCommands: createProxyIdentifier<ExtHostCommandsShape>('ExtHostCommands'),
  ExtHostWebviews: createProxyIdentifier<ExtHostWebviewsShape>('ExtHostWebviews'),
  ExtHostServer: createProxyIdentifier<ExtHostServerShape>('ExtHostServer'),
  ExtHostExtensions: createProxyIdentifier<ExtHostExtensionsShape>('ExtHostExtensions')
}

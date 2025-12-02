/**
 * Neo Extension API
 *
 * This module provides TypeScript type definitions for Neo app extensions.
 * Extensions run in a separate Node.js process and communicate with the
 * main Electron process via a typed RPC protocol.
 */

// ============================================================================
// Core Types
// ============================================================================

/**
 * Disposable interface for cleanup
 */
export interface Disposable {
  dispose(): void
}

/**
 * Event emitter pattern
 */
export interface Event<T> {
  (listener: (e: T) => void): Disposable
}

/**
 * Simple key-value storage
 */
export interface Memento {
  /**
   * Get a value from storage
   */
  get<T>(key: string, defaultValue?: T): T | undefined

  /**
   * Update a value in storage
   */
  update(key: string, value: unknown): Promise<void>

  /**
   * Get all keys in storage
   */
  keys(): readonly string[]
}

// ============================================================================
// Extension Context
// ============================================================================

/**
 * Context passed to extension's activate() function
 */
export interface ExtensionContext {
  /**
   * The absolute path of the extension's installation directory
   */
  readonly extensionPath: string

  /**
   * An array to which disposables can be added.
   * When this extension is deactivated, all disposables will be disposed.
   */
  readonly subscriptions: Disposable[]

  /**
   * A memento object that stores state independent of the workspace
   */
  readonly globalState: Memento

  /**
   * A memento object that stores state specific to the workspace
   */
  readonly workspaceState: Memento
}

// ============================================================================
// Commands API
// ============================================================================

/**
 * Commands namespace for registering and executing commands
 */
export interface CommandsAPI {
  /**
   * Register a command handler
   * @param id The command identifier (e.g., "myExtension.doSomething")
   * @param handler The function to execute when the command is invoked
   * @returns A disposable that unregisters the command when disposed
   */
  registerCommand(id: string, handler: (...args: unknown[]) => unknown): Disposable

  /**
   * Execute a command by identifier
   * @param id The command identifier
   * @param args Arguments to pass to the command handler
   * @returns A promise that resolves to the command's return value
   */
  executeCommand<T = unknown>(id: string, ...args: unknown[]): Promise<T>
}

// ============================================================================
// Window API
// ============================================================================

/**
 * Options for quick pick
 */
export interface QuickPickOptions {
  /**
   * An optional string that represents the title of the quick pick
   */
  title?: string

  /**
   * An optional string to show as placeholder in the input box
   */
  placeHolder?: string

  /**
   * Set to true to keep the picker open when focus moves to another part of the editor
   */
  ignoreFocusOut?: boolean

  /**
   * An optional flag to include the description when filtering the picks
   */
  matchOnDescription?: boolean

  /**
   * An optional flag to include the detail when filtering the picks
   */
  matchOnDetail?: boolean

  /**
   * An optional flag to make the picker accept multiple selections
   */
  canPickMany?: boolean
}

/**
 * Quick pick item
 */
export interface QuickPickItem {
  /**
   * A human-readable string which is rendered prominent
   */
  label: string

  /**
   * A human-readable string which is rendered less prominent
   */
  description?: string

  /**
   * A human-readable string which is rendered less prominent in a separate line
   */
  detail?: string

  /**
   * Optional flag indicating if this item is picked initially
   */
  picked?: boolean

  /**
   * Always show this item
   */
  alwaysShow?: boolean
}

/**
 * Options for input box
 */
export interface InputBoxOptions {
  /**
   * An optional string that represents the title of the input box
   */
  title?: string

  /**
   * The value to prefill in the input box
   */
  value?: string

  /**
   * Selection of the prefilled value
   */
  valueSelection?: [number, number]

  /**
   * The text to display underneath the input box
   */
  prompt?: string

  /**
   * An optional string to show as placeholder in the input box
   */
  placeHolder?: string

  /**
   * Set to true to show a password prompt that will not show the typed value
   */
  password?: boolean

  /**
   * Set to true to keep the input box open when focus moves to another part of the editor
   */
  ignoreFocusOut?: boolean

  /**
   * An optional function that will be called to validate input
   */
  validateInput?(value: string): string | undefined | null | Promise<string | undefined | null>
}

/**
 * Content settings for a webview
 */
export interface WebviewOptions {
  /**
   * Enable scripts in the webview
   */
  enableScripts?: boolean

  /**
   * Enable forms in the webview
   */
  enableForms?: boolean

  /**
   * Root paths from which the webview can load local resources
   */
  localResourceRoots?: string[]

  /**
   * Mapping of local paths to webview URIs
   */
  portMapping?: { webviewPort: number; extensionHostPort: number }[]
}

/**
 * Panel options and settings
 */
export interface WebviewPanelOptions {
  /**
   * Enable find widget in the webview
   */
  enableFindWidget?: boolean

  /**
   * Controls if the panel's content should be kept when the panel becomes hidden
   */
  retainContextWhenHidden?: boolean
}

/**
 * A webview displays html content in an iframe
 */
export interface Webview {
  /**
   * HTML content displayed in the webview
   */
  html: string

  /**
   * Event fired when the webview sends a message
   */
  onDidReceiveMessage: Event<unknown>

  /**
   * Post a message to the webview content
   * @returns A promise indicating if the message was successfully posted
   */
  postMessage(message: unknown): Promise<boolean>

  /**
   * Convert a URI for use in the webview
   * @param localResource Path to a local resource
   * @returns A URI string usable within the webview
   */
  asWebviewUri(localResource: string): string
}

/**
 * A panel that contains a webview
 */
export interface WebviewPanel {
  /**
   * Identifies the type of the webview panel
   */
  readonly viewType: string

  /**
   * Title of the panel shown in UI
   */
  title: string

  /**
   * The webview belonging to the panel
   */
  readonly webview: Webview

  /**
   * Whether the panel is active (focused)
   */
  readonly active: boolean

  /**
   * Whether the panel is visible
   */
  readonly visible: boolean

  /**
   * Event fired when the panel's view state changes
   */
  onDidChangeViewState: Event<{ active: boolean; visible: boolean }>

  /**
   * Event fired when the panel is disposed
   */
  onDidDispose: Event<void>

  /**
   * Show the webview panel
   * @param viewColumn The column to show the panel in
   * @param preserveFocus When true, the panel will not be focused
   */
  reveal(viewColumn?: ViewColumn, preserveFocus?: boolean): void

  /**
   * Dispose of the webview panel
   */
  dispose(): void
}

/**
 * A webview based view
 */
export interface WebviewView {
  /**
   * The webview belonging to the view
   */
  readonly webview: Webview

  /**
   * Title of the view shown in UI
   */
  title: string

  /**
   * Optional human-readable description of the view
   */
  description?: string

  /**
   * Event fired when the visibility of the view changes
   */
  onDidChangeVisibility: Event<boolean>

  /**
   * Event fired when the view is disposed
   */
  onDidDispose: Event<void>

  /**
   * Show the view
   * @param preserveFocus When true, the view will not be focused
   */
  show(preserveFocus?: boolean): void
}

/**
 * Context for resolving a webview view
 */
export interface WebviewViewResolveContext<T = unknown> {
  /**
   * Persisted state from the view's previous session
   */
  readonly state: T | undefined
}

/**
 * Provider for webview views
 */
export interface WebviewViewProvider {
  /**
   * Resolve a webview view
   * @param webviewView The webview view to resolve
   * @param context Context for resolving the view
   */
  resolveWebviewView(
    webviewView: WebviewView,
    context: WebviewViewResolveContext
  ): void | Promise<void>
}

/**
 * View column for positioning panels
 */
export enum ViewColumn {
  Active = -1,
  Beside = -2,
  One = 1,
  Two = 2,
  Three = 3,
  Four = 4,
  Five = 5,
  Six = 6,
  Seven = 7,
  Eight = 8,
  Nine = 9
}

/**
 * Window namespace for UI operations
 */
export interface WindowAPI {
  /**
   * Show an information message
   * @param message The message to show
   * @param items Optional action items
   * @returns A promise that resolves to the selected item or undefined
   */
  showInformationMessage(message: string, ...items: string[]): Promise<string | undefined>

  /**
   * Show a warning message
   * @param message The message to show
   * @param items Optional action items
   * @returns A promise that resolves to the selected item or undefined
   */
  showWarningMessage(message: string, ...items: string[]): Promise<string | undefined>

  /**
   * Show an error message
   * @param message The message to show
   * @param items Optional action items
   * @returns A promise that resolves to the selected item or undefined
   */
  showErrorMessage(message: string, ...items: string[]): Promise<string | undefined>

  /**
   * Show a quick pick selection list
   * @param items An array of items to pick from
   * @param options Optional configuration for the quick pick
   * @returns A promise that resolves to the selected item or undefined
   */
  showQuickPick(
    items: QuickPickItem[] | string[],
    options?: QuickPickOptions
  ): Promise<QuickPickItem | string | undefined>

  /**
   * Show an input box to get user input
   * @param options Optional configuration for the input box
   * @returns A promise that resolves to the input value or undefined
   */
  showInputBox(options?: InputBoxOptions): Promise<string | undefined>

  /**
   * Create a webview panel
   * @param viewType Identifies the type of the webview panel
   * @param title Title of the panel
   * @param showOptions Where to show the panel
   * @param options Settings for the webview
   * @returns A new webview panel
   */
  createWebviewPanel(
    viewType: string,
    title: string,
    showOptions: ViewColumn | { viewColumn: ViewColumn; preserveFocus?: boolean },
    options?: WebviewOptions & WebviewPanelOptions
  ): WebviewPanel

  /**
   * Register a provider for webview views
   * @param viewId The unique id of the view
   * @param provider The webview view provider
   * @returns A disposable that unregisters the provider when disposed
   */
  registerWebviewViewProvider(viewId: string, provider: WebviewViewProvider): Disposable
}

// ============================================================================
// Context API
// ============================================================================

/**
 * Context namespace for setting and getting context values
 * Context values control when clause evaluation in menus, keybindings, etc.
 */
export interface ContextAPI {
  /**
   * Set a context value
   * @param key The context key
   * @param value The value to set
   */
  set(key: string, value: unknown): void

  /**
   * Get a context value (typically returns undefined as context is write-only from extensions)
   * @param key The context key
   */
  get<T>(key: string): T | undefined
}

// ============================================================================
// Server API
// ============================================================================

/**
 * Change event from the server
 */
export interface ChangeEvent {
  /**
   * The path that changed
   */
  path: string

  /**
   * The type of change
   */
  type: 'create' | 'update' | 'delete'

  /**
   * The new value (if applicable)
   */
  value?: unknown

  /**
   * The previous value (if applicable)
   */
  previousValue?: unknown
}

/**
 * Server namespace for communicating with the Neo backend server
 */
export interface ServerAPI {
  /**
   * Whether the server is currently connected
   */
  readonly connected: boolean

  /**
   * Event fired when the server connection is established
   */
  onDidConnect: Event<void>

  /**
   * Event fired when the server connection is lost
   */
  onDidDisconnect: Event<void>

  /**
   * Make a request to the server
   * @param path The request path
   * @param params Optional parameters
   * @returns A promise that resolves to the response
   */
  request<T = unknown>(path: string, params?: Record<string, unknown>): Promise<T>

  /**
   * Subscribe to changes at specific paths
   * @param paths The paths to subscribe to
   * @returns A disposable that unsubscribes when disposed
   */
  subscribe(paths: string[]): Promise<Disposable>

  /**
   * Event fired when a subscribed path changes
   */
  onDidReceiveChange: Event<ChangeEvent>
}

// ============================================================================
// Main API Object
// ============================================================================

/**
 * The main Neo extension API
 */
export interface NeoAPI {
  /**
   * The version of the Neo extension API
   */
  readonly version: string

  /**
   * Namespace for registering and executing commands
   */
  readonly commands: CommandsAPI

  /**
   * Namespace for UI operations like messages, pickers, and webviews
   */
  readonly window: WindowAPI

  /**
   * Namespace for setting context values
   */
  readonly context: ContextAPI

  /**
   * Namespace for server communication
   */
  readonly server: ServerAPI
}

// ============================================================================
// Global Declaration
// ============================================================================

/**
 * Extend the global scope to include the neo API
 */
declare global {
  /**
   * The Neo extension API is available globally as `neo`
   */
  const neo: NeoAPI
}

// ============================================================================
// Extension Module Exports
// ============================================================================

/**
 * Interface for extension modules
 */
export interface ExtensionModule {
  /**
   * Called when the extension is activated
   * @param context The extension context
   */
  activate?(context: ExtensionContext): void | Promise<void>

  /**
   * Called when the extension is deactivated
   */
  deactivate?(): void | Promise<void>
}


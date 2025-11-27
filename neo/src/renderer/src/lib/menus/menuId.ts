// Menu ID definitions - unique identifiers for each menu location

/**
 * Menu ID class - identifies a specific menu location
 * Uses a singleton pattern to ensure each ID is unique
 */
export class MenuId {
  private static _instances = new Map<string, MenuId>()

  // Editor tab context menu (right-click on tab)
  static readonly EditorTabContext = new MenuId('EditorTabContext')

  // File explorer context menu
  static readonly ExplorerContext = new MenuId('ExplorerContext')

  // Editor area context menu (right-click in editor)
  static readonly EditorContext = new MenuId('EditorContext')

  // Panel context menu
  static readonly PanelContext = new MenuId('PanelContext')

  // Command palette menu
  static readonly CommandPalette = new MenuId('CommandPalette')

  // Editor title menu (tab bar actions)
  static readonly EditorTitle = new MenuId('EditorTitle')

  // Activity bar context menu
  static readonly ActivityBarContext = new MenuId('ActivityBarContext')

  // View title menu (sidebar view actions)
  static readonly ViewTitle = new MenuId('ViewTitle')

  /** The unique string identifier */
  readonly id: string

  private constructor(id: string) {
    this.id = id
    MenuId._instances.set(id, this)
  }

  /**
   * Get or create a MenuId by string
   * This is useful for dynamic menu creation
   */
  static getOrCreate(id: string): MenuId {
    const existing = MenuId._instances.get(id)
    if (existing) {
      return existing
    }
    // Create new instance (constructor is private, so use Object.create)
    const menuId = Object.create(MenuId.prototype) as MenuId
    ;(menuId as { id: string }).id = id
    MenuId._instances.set(id, menuId)
    return menuId
  }

  /**
   * Get a MenuId by string, returns undefined if not found
   */
  static get(id: string): MenuId | undefined {
    return MenuId._instances.get(id)
  }

  toString(): string {
    return this.id
  }
}

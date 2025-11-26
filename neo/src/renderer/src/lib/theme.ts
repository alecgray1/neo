import { writable, derived } from 'svelte/store'
import type { Theme, ThemeInfo } from '../../../preload/index.d'

// Current theme store
export const currentTheme = writable<Theme | null>(null)

// Derived store for theme type (for conditional styling)
export const isDarkTheme = derived(currentTheme, ($theme) => $theme?.type === 'dark')

// Derived store for theme name
export const themeName = derived(currentTheme, ($theme) => $theme?.name ?? 'Unknown')

/**
 * Apply theme colors as CSS variables to the document root
 * Integrates with shadcn by setting both neo-* and shadcn variables
 */
function applyTheme(theme: Theme): void {
  const root = document.documentElement

  // Toggle dark class for shadcn (it uses .dark class, not data attribute)
  if (theme.type === 'dark') {
    root.classList.add('dark')
  } else {
    root.classList.remove('dark')
  }

  // Set theme type attribute for our own CSS selectors
  root.setAttribute('data-theme', theme.type)

  // Apply all color tokens as CSS variables (neo-* prefix)
  for (const [key, value] of Object.entries(theme.colors)) {
    const varName = `--neo-${key.replace(/\./g, '-')}`
    root.style.setProperty(varName, value)
  }

  // Map our theme colors to shadcn's expected variables
  // shadcn uses these base variable names (without --color- prefix)
  const shadcnMappings: Record<string, string | undefined> = {
    '--background': theme.colors.background,
    '--foreground': theme.colors.foreground,
    '--primary': theme.colors.primary,
    '--primary-foreground': theme.colors.primaryForeground,
    '--secondary': theme.colors['button.secondaryBackground'],
    '--secondary-foreground': theme.colors['button.secondaryForeground'],
    '--muted': theme.colors['sideBar.background'],
    '--muted-foreground': theme.colors['input.placeholderForeground'],
    '--accent': theme.colors['list.hoverBackground'],
    '--accent-foreground': theme.colors.foreground,
    '--destructive': theme.colors.error,
    '--border': theme.colors.border,
    '--input': theme.colors['input.border'],
    '--ring': theme.colors.focusBorder,
    '--card': theme.colors['panel.background'],
    '--card-foreground': theme.colors.foreground,
    '--popover': theme.colors['panel.background'],
    '--popover-foreground': theme.colors.foreground,
    '--sidebar': theme.colors['sideBar.background'],
    '--sidebar-foreground': theme.colors['sideBar.foreground'],
    '--sidebar-primary': theme.colors.primary,
    '--sidebar-primary-foreground': theme.colors.primaryForeground,
    '--sidebar-accent': theme.colors['list.hoverBackground'],
    '--sidebar-accent-foreground': theme.colors.foreground,
    '--sidebar-border': theme.colors['sideBar.border'] || theme.colors.border,
    '--sidebar-ring': theme.colors.focusBorder,
    '--scrollbar': theme.colors['scrollbar.thumb'],
    '--scrollbar-hover': theme.colors['scrollbar.thumbHover'],
    '--scrollbar-active': theme.colors['scrollbar.thumbHover']
  }

  // Apply shadcn mappings
  for (const [varName, value] of Object.entries(shadcnMappings)) {
    if (value) {
      root.style.setProperty(varName, value)
    }
  }

  console.log(`Applied theme: ${theme.name} (${theme.type})`)
}

/**
 * Initialize the theme system
 * Should be called before mounting the app to prevent FOUC
 */
export async function initTheme(): Promise<void> {
  try {
    // Load the initial theme
    const theme = (await window.themeAPI.getCurrentTheme()) as Theme | null
    if (theme) {
      currentTheme.set(theme)
      applyTheme(theme)
    }

    // Listen for theme changes (hot reload from file changes or user switching)
    window.themeAPI.onThemeChanged((theme: Theme) => {
      currentTheme.set(theme)
      applyTheme(theme)
    })
  } catch (error) {
    console.error('Failed to initialize theme:', error)
  }
}

/**
 * Change to a different theme
 */
export async function setTheme(themeId: string): Promise<boolean> {
  try {
    return await window.themeAPI.setTheme(themeId)
  } catch (error) {
    console.error('Failed to set theme:', error)
    return false
  }
}

/**
 * Get list of available themes
 */
export async function getAvailableThemes(): Promise<ThemeInfo[]> {
  try {
    return (await window.themeAPI.getAvailableThemes()) as ThemeInfo[]
  } catch (error) {
    console.error('Failed to get available themes:', error)
    return []
  }
}

/**
 * Toggle between dark and light themes
 */
export async function toggleTheme(): Promise<void> {
  const themes = await getAvailableThemes()
  const current = await window.themeAPI.getCurrentTheme()

  if (!current) return

  // Find a theme of the opposite type
  const oppositeType = current.type === 'dark' ? 'light' : 'dark'
  const oppositeTheme = themes.find((t) => t.type === oppositeType)

  if (oppositeTheme) {
    await setTheme(oppositeTheme.id)
  }
}

/**
 * Extension Scanner
 *
 * Discovers and validates installed extensions from the extensions directory.
 */

import { readdir, readFile, access, stat } from 'fs/promises'
import { join } from 'path'
import { constants } from 'fs'

// Extension manifest structure (from package.json)
export interface ExtensionManifest {
  name: string
  displayName?: string
  version: string
  description?: string
  main?: string
  neo?: {
    id: string
    name: string
    description?: string

    // Server-side plugin configuration
    server?: {
      entry: string
      subscriptions?: string[]
      tickInterval?: number
      config?: Record<string, unknown>
    }

    // App-side extension configuration
    app?: {
      entry: string
      activationEvents?: string[]
      contributes?: ExtensionContributes
    }
  }
}

export interface ExtensionContributes {
  commands?: CommandContribution[]
  viewsContainers?: {
    activitybar?: ViewContainerContribution[]
    panel?: ViewContainerContribution[]
  }
  views?: Record<string, ViewContribution[]>
  menus?: Record<string, MenuContribution[]>
  keybindings?: KeybindingContribution[]
  configuration?: ConfigurationContribution
}

export interface CommandContribution {
  id: string
  title: string
  category?: string
  icon?: string
  enablement?: string
}

export interface ViewContainerContribution {
  id: string
  title: string
  icon: string
}

export interface ViewContribution {
  id: string
  name: string
  type?: 'tree' | 'webview'
  when?: string
  icon?: string
}

export interface MenuContribution {
  command: string
  when?: string
  group?: string
}

export interface KeybindingContribution {
  command: string
  key: string
  mac?: string
  when?: string
}

export interface ConfigurationContribution {
  title?: string
  properties?: Record<
    string,
    {
      type: string
      default?: unknown
      description?: string
      enum?: unknown[]
      enumDescriptions?: string[]
    }
  >
}

export interface ScannedExtension {
  id: string
  path: string
  manifest: ExtensionManifest
}

export class ExtensionScanner {
  constructor(private _extensionsPath: string) {}

  async scan(): Promise<ScannedExtension[]> {
    const extensions: ScannedExtension[] = []

    // Check if extensions directory exists
    try {
      await access(this._extensionsPath, constants.R_OK)
    } catch {
      console.log(`[ExtensionScanner] Extensions directory does not exist: ${this._extensionsPath}`)
      return []
    }

    try {
      const entries = await readdir(this._extensionsPath, { withFileTypes: true })

      for (const entry of entries) {
        const extPath = join(this._extensionsPath, entry.name)

        // Check if it's a directory (or symlink to directory)
        try {
          const stats = await stat(extPath)
          if (!stats.isDirectory()) continue
        } catch {
          continue
        }

        const extension = await this._scanExtension(extPath)

        if (extension) {
          extensions.push(extension)
        }
      }
    } catch (err) {
      console.error('[ExtensionScanner] Failed to scan extensions directory:', err)
    }

    return extensions
  }

  private async _scanExtension(extPath: string): Promise<ScannedExtension | null> {
    // Try package.json first (unified manifest)
    const packageJsonPath = join(extPath, 'package.json')

    try {
      const content = await readFile(packageJsonPath, 'utf-8')
      const manifest = JSON.parse(content) as ExtensionManifest

      // Check if it has neo.app configuration (app extension)
      if (!manifest.neo?.app) {
        return null
      }

      // Validate required fields
      if (!manifest.neo.id || !manifest.name || !manifest.version) {
        console.warn(`[ExtensionScanner] Invalid manifest at ${extPath}: missing required fields`)
        return null
      }

      // Check if entry file exists
      const entryPath = join(extPath, manifest.neo.app.entry)
      try {
        await access(entryPath, constants.R_OK)
      } catch {
        console.warn(`[ExtensionScanner] Extension entry not found: ${entryPath}`)
        return null
      }

      console.log(`[ExtensionScanner] Found extension: ${manifest.neo.id} at ${extPath}`)

      return {
        id: manifest.neo.id,
        path: extPath,
        manifest
      }
    } catch {
      // Not a valid extension, skip silently
      return null
    }
  }

  /**
   * Scan a single extension by path
   */
  async scanSingle(extPath: string): Promise<ScannedExtension | null> {
    return this._scanExtension(extPath)
  }

  /**
   * Validate an extension manifest
   */
  validateManifest(manifest: ExtensionManifest): string[] {
    const errors: string[] = []

    if (!manifest.neo) {
      errors.push('Missing "neo" field in package.json')
      return errors
    }

    if (!manifest.neo.id) {
      errors.push('Missing "neo.id" field')
    }

    if (!manifest.name) {
      errors.push('Missing "name" field')
    }

    if (!manifest.version) {
      errors.push('Missing "version" field')
    }

    if (manifest.neo.app) {
      if (!manifest.neo.app.entry) {
        errors.push('Missing "neo.app.entry" field')
      }

      // Validate contributions
      const contributes = manifest.neo.app.contributes
      if (contributes) {
        // Validate commands
        if (contributes.commands) {
          for (const cmd of contributes.commands) {
            if (!cmd.id) {
              errors.push(`Command missing "id" field`)
            }
            if (!cmd.title) {
              errors.push(`Command "${cmd.id}" missing "title" field`)
            }
          }
        }

        // Validate views
        if (contributes.views) {
          for (const [containerId, views] of Object.entries(contributes.views)) {
            for (const view of views) {
              if (!view.id) {
                errors.push(`View in container "${containerId}" missing "id" field`)
              }
              if (!view.name) {
                errors.push(`View "${view.id}" missing "name" field`)
              }
            }
          }
        }

        // Validate view containers
        if (contributes.viewsContainers) {
          const containers = [
            ...(contributes.viewsContainers.activitybar ?? []),
            ...(contributes.viewsContainers.panel ?? [])
          ]

          for (const container of containers) {
            if (!container.id) {
              errors.push('View container missing "id" field')
            }
            if (!container.title) {
              errors.push(`View container "${container.id}" missing "title" field`)
            }
            if (!container.icon) {
              errors.push(`View container "${container.id}" missing "icon" field`)
            }
          }
        }
      }
    }

    return errors
  }
}

// Help quick access provider ("?" prefix)

import type { IQuickAccessProvider, IQuickAccessItem } from '../types'
import { getQuickAccessRegistry } from '../registry'
import { getQuickAccessController } from '../controller'
import { filterItems } from '../filter'

/**
 * Help provider - shows available quick access prefixes
 * Activated by "?" prefix
 */
export class HelpQuickAccessProvider implements IQuickAccessProvider {
  readonly prefix = '?'
  readonly placeholder = 'Which picker do you want to use?'

  provide(filter: string): IQuickAccessItem[] {
    const registry = getQuickAccessRegistry()
    const providers = registry.getProviders()

    // Build items from providers
    const items: IQuickAccessItem[] = providers
      .filter((p) => p.prefix !== '?') // Don't show help in help
      .map((p) => ({
        id: p.prefix || 'files',
        label: this._getPrefixLabel(p.prefix),
        description: p.placeholder,
        data: p.prefix
      }))

    // Add common help items
    items.push(
      {
        id: 'commands',
        label: '> Commands',
        description: 'Run commands',
        data: '>'
      },
      {
        id: 'files',
        label: 'Go to File',
        description: 'Open a file by name',
        data: ''
      }
    )

    // Deduplicate by id
    const seen = new Set<string>()
    const uniqueItems = items.filter((item) => {
      if (seen.has(item.id)) return false
      seen.add(item.id)
      return true
    })

    // Filter if there's a query
    if (filter.trim()) {
      const filtered = filterItems(
        uniqueItems,
        filter,
        (item) => item.label,
        (item) => item.description
      )

      return filtered.map(({ item, highlights, secondaryHighlights }) => ({
        ...item,
        labelHighlights: highlights,
        descriptionHighlights: secondaryHighlights
      }))
    }

    return uniqueItems
  }

  accept(item: IQuickAccessItem): void {
    const controller = getQuickAccessController()
    const prefix = item.data as string
    controller.setValue(prefix)
  }

  private _getPrefixLabel(prefix: string): string {
    switch (prefix) {
      case '>':
        return '> Commands'
      case '@':
        return '@ Go to Symbol'
      case ':':
        return ': Go to Line'
      case '#':
        return '# Search Symbols'
      case '':
        return 'Go to File'
      default:
        return prefix
    }
  }
}

/**
 * Create and return the help provider
 */
export function createHelpProvider(): IQuickAccessProvider {
  return new HelpQuickAccessProvider()
}

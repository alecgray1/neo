// Global keyboard event listener for keybindings

import type { IDisposable } from '$lib/services/types'
import { toDisposable, DisposableStore } from '$lib/services/types'
import { getKeybindingService } from './registry'
import { executeCommand } from '$lib/commands/registry'

let _listenerDisposable: IDisposable | null = null

/**
 * Start listening for keyboard events and dispatching to commands
 */
export function startKeybindingListener(): IDisposable {
  if (_listenerDisposable) {
    return _listenerDisposable
  }

  const disposables = new DisposableStore()

  const handleKeydown = async (event: KeyboardEvent) => {
    // Don't intercept if we're in an input element (unless it's a command shortcut)
    const target = event.target as HTMLElement
    const isInput = target.tagName === 'INPUT' ||
                    target.tagName === 'TEXTAREA' ||
                    target.isContentEditable

    // Always check for keybinding match
    const keybindingService = getKeybindingService()
    const match = keybindingService.resolve(event)

    if (match) {
      // For inputs, only allow certain "global" commands that should work anywhere
      // Quick access commands should always work
      const globalCommands = [
        'neo.quickAccess.show',
        'neo.quickAccess.showCommands',
        'neo.quickAccess.showFiles',
        'neo.view.togglePrimarySidebar',
        'neo.view.toggleSecondarySidebar',
        'neo.view.togglePanel',
        'neo.file.save',
        'neo.file.saveAll',
        'neo.preferences.openKeybindings',
        'neo.preferences.openSettings',
      ]

      if (isInput && !globalCommands.includes(match.commandId)) {
        return
      }

      event.preventDefault()
      event.stopPropagation()

      try {
        if (match.args) {
          await executeCommand(match.commandId, ...match.args)
        } else {
          await executeCommand(match.commandId)
        }
      } catch (error) {
        console.error(`Failed to execute command "${match.commandId}":`, error)
      }
    }
  }

  // Capture phase to get events before other handlers
  document.addEventListener('keydown', handleKeydown, true)
  disposables.add(toDisposable(() => document.removeEventListener('keydown', handleKeydown, true)))

  _listenerDisposable = {
    dispose: () => {
      disposables.dispose()
      _listenerDisposable = null
    }
  }

  return _listenerDisposable
}

/**
 * Stop listening for keyboard events
 */
export function stopKeybindingListener(): void {
  _listenerDisposable?.dispose()
}

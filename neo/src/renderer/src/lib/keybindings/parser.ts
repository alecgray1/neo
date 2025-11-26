// Keybinding parser - parses "ctrl+shift+p" style strings

import type { IParsedKeybinding } from './types'
import { isMac } from './types'

// Map of special key names
const KEY_ALIASES: Record<string, string> = {
  esc: 'escape',
  return: 'enter',
  space: ' ',
  spacebar: ' ',
  up: 'arrowup',
  down: 'arrowdown',
  left: 'arrowleft',
  right: 'arrowright',
  del: 'delete',
  ins: 'insert',
  pageup: 'pageup',
  pagedown: 'pagedown',
}

/**
 * Parse a keybinding string like "ctrl+shift+p" into parts
 */
export function parseKeybinding(str: string): IParsedKeybinding {
  const parts = str.toLowerCase().split('+').map(s => s.trim())

  const result: IParsedKeybinding = {
    ctrl: false,
    alt: false,
    shift: false,
    meta: false,
    key: ''
  }

  for (const part of parts) {
    switch (part) {
      case 'ctrl':
      case 'control':
        result.ctrl = true
        break
      case 'alt':
      case 'option':
        result.alt = true
        break
      case 'shift':
        result.shift = true
        break
      case 'meta':
      case 'cmd':
      case 'command':
      case 'win':
      case 'super':
        result.meta = true
        break
      default:
        // This is the key
        result.key = KEY_ALIASES[part] || part
        break
    }
  }

  return result
}

/**
 * Convert a parsed keybinding back to a normalized string
 * Always uses order: ctrl+alt+shift+meta+key
 */
export function normalizeKeybinding(parsed: IParsedKeybinding): string {
  const parts: string[] = []
  if (parsed.ctrl) parts.push('ctrl')
  if (parsed.alt) parts.push('alt')
  if (parsed.shift) parts.push('shift')
  if (parsed.meta) parts.push('meta')
  parts.push(parsed.key)
  return parts.join('+')
}

/**
 * Parse and normalize a keybinding string
 */
export function normalizeKeybindingStr(str: string): string {
  return normalizeKeybinding(parseKeybinding(str))
}

/**
 * Convert a keyboard event to a parsed keybinding
 */
export function keyboardEventToKeybinding(event: KeyboardEvent): IParsedKeybinding {
  return {
    ctrl: event.ctrlKey,
    alt: event.altKey,
    shift: event.shiftKey,
    meta: event.metaKey,
    key: event.key.toLowerCase()
  }
}

/**
 * Check if two parsed keybindings match
 */
export function keybindingsMatch(a: IParsedKeybinding, b: IParsedKeybinding): boolean {
  return (
    a.ctrl === b.ctrl &&
    a.alt === b.alt &&
    a.shift === b.shift &&
    a.meta === b.meta &&
    a.key === b.key
  )
}

/**
 * Format a keybinding for display (platform-aware)
 */
export function formatKeybindingForDisplay(str: string): string {
  const parsed = parseKeybinding(str)
  const mac = isMac()

  const parts: string[] = []

  if (mac) {
    // Mac: Use symbols
    if (parsed.ctrl) parts.push('⌃')
    if (parsed.alt) parts.push('⌥')
    if (parsed.shift) parts.push('⇧')
    if (parsed.meta) parts.push('⌘')
  } else {
    // Windows/Linux: Use text
    if (parsed.ctrl) parts.push('Ctrl')
    if (parsed.alt) parts.push('Alt')
    if (parsed.shift) parts.push('Shift')
    if (parsed.meta) parts.push('Win')
  }

  // Format the key
  let key = parsed.key
  if (key === ' ') key = 'Space'
  else if (key === 'arrowup') key = '↑'
  else if (key === 'arrowdown') key = '↓'
  else if (key === 'arrowleft') key = '←'
  else if (key === 'arrowright') key = '→'
  else if (key === 'escape') key = 'Esc'
  else if (key === 'enter') key = mac ? '↵' : 'Enter'
  else if (key === 'backspace') key = mac ? '⌫' : 'Backspace'
  else if (key === 'delete') key = mac ? '⌦' : 'Delete'
  else if (key === 'tab') key = mac ? '⇥' : 'Tab'
  else if (key.length === 1) key = key.toUpperCase()
  else key = key.charAt(0).toUpperCase() + key.slice(1)

  parts.push(key)

  return mac ? parts.join('') : parts.join('+')
}

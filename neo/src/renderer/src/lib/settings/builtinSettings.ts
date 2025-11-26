/**
 * Built-in Neo settings definitions
 */

import type { ISettingDefinition } from './types'
import { getSettingsRegistry } from './registry'

export const builtinSettings: ISettingDefinition[] = [
  // ============================================================
  // EDITOR SETTINGS
  // ============================================================

  // Editor > Font
  {
    id: 'editor.fontSize',
    category: ['Editor', 'Font'],
    categoryOrder: 0,
    schema: {
      type: 'number',
      default: 14,
      minimum: 6,
      maximum: 72,
      description: 'Controls the font size in pixels.'
    }
  },
  {
    id: 'editor.fontFamily',
    category: ['Editor', 'Font'],
    categoryOrder: 0,
    schema: {
      type: 'string',
      default: "'Fira Code', 'Cascadia Code', Consolas, 'Courier New', monospace",
      description: 'Controls the font family.'
    }
  },
  {
    id: 'editor.fontWeight',
    category: ['Editor', 'Font'],
    categoryOrder: 0,
    schema: {
      type: 'string',
      default: 'normal',
      enum: ['normal', 'bold', '100', '200', '300', '400', '500', '600', '700', '800', '900'],
      enumDescriptions: [
        'Normal font weight',
        'Bold font weight',
        'Thin (100)',
        'Extra Light (200)',
        'Light (300)',
        'Normal (400)',
        'Medium (500)',
        'Semi Bold (600)',
        'Bold (700)',
        'Extra Bold (800)',
        'Black (900)'
      ],
      description: 'Controls the font weight.'
    }
  },
  {
    id: 'editor.lineHeight',
    category: ['Editor', 'Font'],
    categoryOrder: 0,
    schema: {
      type: 'number',
      default: 0,
      minimum: 0,
      maximum: 150,
      description:
        'Controls the line height. Use 0 to automatically compute from font size.'
    }
  },
  {
    id: 'editor.fontLigatures',
    category: ['Editor', 'Font'],
    categoryOrder: 0,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Enables/disables font ligatures.'
    }
  },

  // Editor > Cursor
  {
    id: 'editor.cursorStyle',
    category: ['Editor', 'Cursor'],
    categoryOrder: 1,
    schema: {
      type: 'string',
      default: 'line',
      enum: ['line', 'block', 'underline', 'line-thin', 'block-outline', 'underline-thin'],
      enumDescriptions: [
        'Line cursor',
        'Block cursor',
        'Underline cursor',
        'Thin line cursor',
        'Block outline cursor',
        'Thin underline cursor'
      ],
      description: 'Controls the cursor style.'
    }
  },
  {
    id: 'editor.cursorBlinking',
    category: ['Editor', 'Cursor'],
    categoryOrder: 1,
    schema: {
      type: 'string',
      default: 'blink',
      enum: ['blink', 'smooth', 'phase', 'expand', 'solid'],
      enumDescriptions: [
        'Standard blinking',
        'Smooth fading animation',
        'Phase animation',
        'Expand animation',
        'No blinking'
      ],
      description: 'Controls the cursor animation style.'
    }
  },
  {
    id: 'editor.cursorWidth',
    category: ['Editor', 'Cursor'],
    categoryOrder: 1,
    schema: {
      type: 'integer',
      default: 2,
      minimum: 1,
      maximum: 10,
      description: 'Controls the width of the cursor when cursorStyle is line.'
    }
  },

  // Editor > Formatting
  {
    id: 'editor.tabSize',
    category: ['Editor', 'Formatting'],
    categoryOrder: 2,
    schema: {
      type: 'integer',
      default: 4,
      minimum: 1,
      maximum: 16,
      description: 'The number of spaces a tab is equal to.'
    }
  },
  {
    id: 'editor.insertSpaces',
    category: ['Editor', 'Formatting'],
    categoryOrder: 2,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Insert spaces when pressing Tab.'
    }
  },
  {
    id: 'editor.detectIndentation',
    category: ['Editor', 'Formatting'],
    categoryOrder: 2,
    schema: {
      type: 'boolean',
      default: true,
      description:
        'Controls whether tabSize and insertSpaces will be automatically detected when a file is opened.'
    }
  },
  {
    id: 'editor.trimAutoWhitespace',
    category: ['Editor', 'Formatting'],
    categoryOrder: 2,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Remove trailing auto-inserted whitespace.'
    }
  },

  // Editor > Display
  {
    id: 'editor.wordWrap',
    category: ['Editor', 'Display'],
    categoryOrder: 3,
    schema: {
      type: 'string',
      default: 'off',
      enum: ['off', 'on', 'wordWrapColumn', 'bounded'],
      enumDescriptions: [
        'Lines will never wrap',
        'Lines will wrap at the viewport width',
        'Lines will wrap at wordWrapColumn',
        'Lines will wrap at the minimum of viewport and wordWrapColumn'
      ],
      description: 'Controls how lines should wrap.'
    }
  },
  {
    id: 'editor.wordWrapColumn',
    category: ['Editor', 'Display'],
    categoryOrder: 3,
    schema: {
      type: 'integer',
      default: 80,
      minimum: 1,
      maximum: 1000,
      description: 'Controls the wrapping column of the editor when wordWrap is wordWrapColumn or bounded.'
    }
  },
  {
    id: 'editor.lineNumbers',
    category: ['Editor', 'Display'],
    categoryOrder: 3,
    schema: {
      type: 'string',
      default: 'on',
      enum: ['off', 'on', 'relative', 'interval'],
      enumDescriptions: [
        'Line numbers are not rendered',
        'Line numbers are rendered as absolute',
        'Line numbers are rendered relative to cursor',
        'Line numbers are rendered every 10 lines'
      ],
      description: 'Controls the display of line numbers.'
    }
  },
  {
    id: 'editor.renderWhitespace',
    category: ['Editor', 'Display'],
    categoryOrder: 3,
    schema: {
      type: 'string',
      default: 'selection',
      enum: ['none', 'boundary', 'selection', 'trailing', 'all'],
      enumDescriptions: [
        'Whitespace is not rendered',
        'Render whitespace except single spaces between words',
        'Render whitespace only on selected text',
        'Render only trailing whitespace',
        'Render all whitespace characters'
      ],
      description: 'Controls how whitespace characters are rendered.'
    }
  },

  // Editor > Minimap
  {
    id: 'editor.minimap.enabled',
    category: ['Editor', 'Minimap'],
    categoryOrder: 4,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Controls whether the minimap is shown.'
    }
  },
  {
    id: 'editor.minimap.side',
    category: ['Editor', 'Minimap'],
    categoryOrder: 4,
    schema: {
      type: 'string',
      default: 'right',
      enum: ['left', 'right'],
      description: 'Controls the side where to render the minimap.'
    }
  },
  {
    id: 'editor.minimap.scale',
    category: ['Editor', 'Minimap'],
    categoryOrder: 4,
    schema: {
      type: 'integer',
      default: 1,
      minimum: 1,
      maximum: 3,
      description: 'Scale of content drawn in the minimap (1, 2, or 3).'
    }
  },
  {
    id: 'editor.minimap.maxColumn',
    category: ['Editor', 'Minimap'],
    categoryOrder: 4,
    schema: {
      type: 'integer',
      default: 120,
      minimum: 1,
      maximum: 500,
      description: 'Limit the width of the minimap.'
    }
  },

  // ============================================================
  // WORKBENCH SETTINGS
  // ============================================================

  // Workbench > Appearance
  {
    id: 'workbench.colorTheme',
    category: ['Workbench', 'Appearance'],
    categoryOrder: 10,
    schema: {
      type: 'string',
      default: 'neo-dark',
      description: 'Specifies the color theme used in the workbench.'
    }
  },
  {
    id: 'workbench.iconTheme',
    category: ['Workbench', 'Appearance'],
    categoryOrder: 10,
    schema: {
      type: 'string',
      default: 'neo-icons',
      description: 'Specifies the file icon theme used in the workbench.'
    }
  },
  {
    id: 'workbench.sideBar.location',
    category: ['Workbench', 'Appearance'],
    categoryOrder: 10,
    schema: {
      type: 'string',
      default: 'left',
      enum: ['left', 'right'],
      description: 'Controls the location of the sidebar.'
    }
  },
  {
    id: 'workbench.activityBar.visible',
    category: ['Workbench', 'Appearance'],
    categoryOrder: 10,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Controls the visibility of the activity bar.'
    }
  },
  {
    id: 'workbench.statusBar.visible',
    category: ['Workbench', 'Appearance'],
    categoryOrder: 10,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Controls the visibility of the status bar.'
    }
  },

  // Workbench > Editor Management
  {
    id: 'workbench.editor.showTabs',
    category: ['Workbench', 'Editor Management'],
    categoryOrder: 11,
    schema: {
      type: 'string',
      default: 'multiple',
      enum: ['multiple', 'single', 'none'],
      enumDescriptions: [
        'Show tabs for all open editors',
        'Show single tab for active editor',
        'Do not show tabs'
      ],
      description: 'Controls whether editor tabs are shown.'
    }
  },
  {
    id: 'workbench.editor.tabCloseButton',
    category: ['Workbench', 'Editor Management'],
    categoryOrder: 11,
    schema: {
      type: 'string',
      default: 'right',
      enum: ['off', 'left', 'right'],
      description: 'Controls the position of the tab close button.'
    }
  },
  {
    id: 'workbench.editor.enablePreview',
    category: ['Workbench', 'Editor Management'],
    categoryOrder: 11,
    schema: {
      type: 'boolean',
      default: true,
      description:
        'Controls whether opened editors show as preview. Preview editors are reused until they are pinned.'
    }
  },
  {
    id: 'workbench.editor.closeOnFileDelete',
    category: ['Workbench', 'Editor Management'],
    categoryOrder: 11,
    schema: {
      type: 'boolean',
      default: false,
      description: 'Controls whether editors showing a file should close when the file is deleted.'
    }
  },

  // ============================================================
  // FILES SETTINGS
  // ============================================================

  {
    id: 'files.autoSave',
    category: ['Files'],
    categoryOrder: 20,
    schema: {
      type: 'string',
      default: 'off',
      enum: ['off', 'afterDelay', 'onFocusChange', 'onWindowChange'],
      enumDescriptions: [
        'Files are not auto-saved',
        'Files are auto-saved after a configurable delay',
        'Files are auto-saved when the editor loses focus',
        'Files are auto-saved when the window loses focus'
      ],
      description: 'Controls auto save of editors.'
    }
  },
  {
    id: 'files.autoSaveDelay',
    category: ['Files'],
    categoryOrder: 20,
    schema: {
      type: 'integer',
      default: 1000,
      minimum: 0,
      description: 'Controls the delay in milliseconds after which a file is auto-saved.'
    }
  },
  {
    id: 'files.encoding',
    category: ['Files'],
    categoryOrder: 20,
    schema: {
      type: 'string',
      default: 'utf8',
      enum: ['utf8', 'utf16le', 'utf16be', 'windows1252', 'iso88591'],
      enumDescriptions: [
        'UTF-8',
        'UTF-16 Little Endian',
        'UTF-16 Big Endian',
        'Windows 1252',
        'ISO 8859-1 (Latin 1)'
      ],
      description: 'The default character set encoding to use when reading and writing files.'
    }
  },
  {
    id: 'files.eol',
    category: ['Files'],
    categoryOrder: 20,
    schema: {
      type: 'string',
      default: 'auto',
      enum: ['\\n', '\\r\\n', 'auto'],
      enumDescriptions: ['LF (Unix)', 'CRLF (Windows)', 'Use OS default'],
      description: 'The default end of line character.'
    }
  },
  {
    id: 'files.trimTrailingWhitespace',
    category: ['Files'],
    categoryOrder: 20,
    schema: {
      type: 'boolean',
      default: false,
      description: 'When enabled, will trim trailing whitespace when saving a file.'
    }
  },
  {
    id: 'files.insertFinalNewline',
    category: ['Files'],
    categoryOrder: 20,
    schema: {
      type: 'boolean',
      default: false,
      description: 'When enabled, insert a final new line at the end of the file when saving it.'
    }
  },
  {
    id: 'files.exclude',
    category: ['Files'],
    categoryOrder: 20,
    schema: {
      type: 'object',
      default: {
        '**/.git': true,
        '**/.svn': true,
        '**/.hg': true,
        '**/CVS': true,
        '**/.DS_Store': true,
        '**/Thumbs.db': true,
        '**/node_modules': true
      },
      additionalProperties: { type: 'boolean' },
      description: 'Configure glob patterns for excluding files and folders.'
    }
  },

  // ============================================================
  // TERMINAL SETTINGS
  // ============================================================

  {
    id: 'terminal.integrated.fontSize',
    category: ['Terminal'],
    categoryOrder: 30,
    schema: {
      type: 'integer',
      default: 14,
      minimum: 6,
      maximum: 72,
      description: 'Controls the font size in pixels of the terminal.'
    }
  },
  {
    id: 'terminal.integrated.fontFamily',
    category: ['Terminal'],
    categoryOrder: 30,
    schema: {
      type: 'string',
      default: '',
      description:
        'Controls the font family of the terminal. Defaults to editor font family if empty.'
    }
  },
  {
    id: 'terminal.integrated.fontWeight',
    category: ['Terminal'],
    categoryOrder: 30,
    schema: {
      type: 'string',
      default: 'normal',
      enum: ['normal', 'bold', '100', '200', '300', '400', '500', '600', '700', '800', '900'],
      description: 'Controls the font weight of the terminal.'
    }
  },
  {
    id: 'terminal.integrated.lineHeight',
    category: ['Terminal'],
    categoryOrder: 30,
    schema: {
      type: 'number',
      default: 1,
      minimum: 1,
      maximum: 2,
      description: 'Controls the line height of the terminal.'
    }
  },
  {
    id: 'terminal.integrated.cursorStyle',
    category: ['Terminal'],
    categoryOrder: 30,
    schema: {
      type: 'string',
      default: 'block',
      enum: ['block', 'underline', 'line'],
      description: 'Controls the style of terminal cursor.'
    }
  },
  {
    id: 'terminal.integrated.cursorBlinking',
    category: ['Terminal'],
    categoryOrder: 30,
    schema: {
      type: 'boolean',
      default: false,
      description: 'Controls whether the terminal cursor blinks.'
    }
  },
  {
    id: 'terminal.integrated.scrollback',
    category: ['Terminal'],
    categoryOrder: 30,
    schema: {
      type: 'integer',
      default: 1000,
      minimum: 0,
      maximum: 100000,
      description: 'Controls the maximum number of lines the terminal keeps in its buffer.'
    }
  },

  // ============================================================
  // SEARCH SETTINGS
  // ============================================================

  {
    id: 'search.exclude',
    category: ['Search'],
    categoryOrder: 40,
    schema: {
      type: 'object',
      default: {
        '**/node_modules': true,
        '**/bower_components': true,
        '**/*.code-search': true
      },
      additionalProperties: { type: 'boolean' },
      description: 'Configure glob patterns for excluding files and folders in searches.'
    }
  },
  {
    id: 'search.useIgnoreFiles',
    category: ['Search'],
    categoryOrder: 40,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Controls whether to use .gitignore and .ignore files when searching.'
    }
  },
  {
    id: 'search.followSymlinks',
    category: ['Search'],
    categoryOrder: 40,
    schema: {
      type: 'boolean',
      default: true,
      description: 'Controls whether to follow symlinks while searching.'
    }
  }
]

/**
 * Register all built-in settings
 */
export function registerBuiltinSettings(): () => void {
  const registry = getSettingsRegistry()
  return registry.registerMany(builtinSettings)
}

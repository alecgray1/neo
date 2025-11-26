/**
 * Settings types - JSON Schema based like VS Code
 */

/**
 * Setting value types (JSON Schema style)
 */
export type SettingType = 'boolean' | 'string' | 'number' | 'integer' | 'array' | 'object' | 'null'

/**
 * Setting scope - where the setting applies
 */
export type SettingScope = 'application' | 'window' | 'resource'

/**
 * Setting source - where the current value comes from
 */
export type SettingSource = 'default' | 'user'

/**
 * Setting schema definition (JSON Schema subset)
 */
export interface ISettingSchema {
  type: SettingType | SettingType[]
  default?: unknown
  description?: string
  markdownDescription?: string

  // String constraints
  pattern?: string
  patternErrorMessage?: string
  minLength?: number
  maxLength?: number

  // Number constraints
  minimum?: number
  maximum?: number
  multipleOf?: number

  // Enum options
  enum?: unknown[]
  enumDescriptions?: string[]
  enumItemLabels?: string[]

  // Array schema
  items?: ISettingSchema
  minItems?: number
  maxItems?: number
  uniqueItems?: boolean

  // Object schema
  properties?: Record<string, ISettingSchema>
  additionalProperties?: boolean | ISettingSchema
  required?: string[]

  // UI hints
  order?: number
  tags?: string[]
  scope?: SettingScope
  editPresentation?: 'singlelineText' | 'multilineText'

  // Deprecation
  deprecationMessage?: string
  markdownDeprecationMessage?: string
}

/**
 * Setting definition - registered by extensions or built-in
 */
export interface ISettingDefinition {
  id: string
  schema: ISettingSchema
  category: string[]
  categoryOrder?: number
  extensionId?: string
}

/**
 * Setting with runtime value - used in editor
 */
export interface ISetting extends ISettingDefinition {
  value: unknown
  defaultValue: unknown
  userValue?: unknown
  isModified: boolean
  source: SettingSource
}

/**
 * Category tree node for TOC
 */
export interface ISettingCategory {
  id: string
  label: string
  path: string[]
  children: ISettingCategory[]
  settingCount: number
  order?: number
}

/**
 * Group of settings for display
 */
export interface ISettingsGroup {
  id: string
  label: string
  path: string[]
  settings: ISetting[]
}

/**
 * User setting entry (stored in settings.json)
 */
export interface IUserSetting {
  key: string
  value: unknown
}

/**
 * Search/filter match highlight
 */
export interface ISettingMatch {
  start: number
  end: number
}

/**
 * Setting item for editor display with match highlights
 */
export interface ISettingItemEntry extends ISetting {
  idMatches?: ISettingMatch[]
  descriptionMatches?: ISettingMatch[]
  categoryMatches?: ISettingMatch[]
}

// Context key service - conditional command availability via "when" expressions

import { createServiceId, type IDisposable, toDisposable, Emitter, type Event } from './types'

/**
 * Context key service identifier
 */
export const IContextKeyService = createServiceId<IContextKeyService>('IContextKeyService')

/**
 * Context key service interface
 */
export interface IContextKeyService {
  /** Set a context value */
  set(key: string, value: unknown): void
  /** Get a context value */
  get(key: string): unknown
  /** Delete a context value */
  delete(key: string): void
  /** Evaluate a "when" expression */
  evaluate(expression: string): boolean
  /** Event fired when context changes */
  onDidChange: Event<string>
  /** Create a scoped context that inherits from this one */
  createScoped(): IContextKeyService
}

// Token types for the expression parser
type Token =
  | { type: 'identifier'; value: string }
  | { type: 'string'; value: string }
  | { type: 'number'; value: number }
  | { type: 'boolean'; value: boolean }
  | { type: 'operator'; value: string }
  | { type: 'lparen' }
  | { type: 'rparen' }
  | { type: 'eof' }

/**
 * Tokenize a "when" expression
 */
function tokenize(expr: string): Token[] {
  const tokens: Token[] = []
  let i = 0

  while (i < expr.length) {
    const ch = expr[i]

    // Skip whitespace
    if (/\s/.test(ch)) {
      i++
      continue
    }

    // Parentheses
    if (ch === '(') {
      tokens.push({ type: 'lparen' })
      i++
      continue
    }
    if (ch === ')') {
      tokens.push({ type: 'rparen' })
      i++
      continue
    }

    // Operators
    if (ch === '!' && expr[i + 1] !== '=') {
      tokens.push({ type: 'operator', value: '!' })
      i++
      continue
    }
    if (ch === '&' && expr[i + 1] === '&') {
      tokens.push({ type: 'operator', value: '&&' })
      i += 2
      continue
    }
    if (ch === '|' && expr[i + 1] === '|') {
      tokens.push({ type: 'operator', value: '||' })
      i += 2
      continue
    }
    if (ch === '=' && expr[i + 1] === '=') {
      tokens.push({ type: 'operator', value: '==' })
      i += 2
      continue
    }
    if (ch === '!' && expr[i + 1] === '=') {
      tokens.push({ type: 'operator', value: '!=' })
      i += 2
      continue
    }

    // String literal
    if (ch === "'" || ch === '"') {
      const quote = ch
      let str = ''
      i++
      while (i < expr.length && expr[i] !== quote) {
        str += expr[i]
        i++
      }
      i++ // skip closing quote
      tokens.push({ type: 'string', value: str })
      continue
    }

    // Number
    if (/\d/.test(ch)) {
      let num = ''
      while (i < expr.length && /[\d.]/.test(expr[i])) {
        num += expr[i]
        i++
      }
      tokens.push({ type: 'number', value: parseFloat(num) })
      continue
    }

    // Identifier (including true/false)
    if (/[a-zA-Z_]/.test(ch)) {
      let id = ''
      while (i < expr.length && /[a-zA-Z0-9_.]/.test(expr[i])) {
        id += expr[i]
        i++
      }
      if (id === 'true') {
        tokens.push({ type: 'boolean', value: true })
      } else if (id === 'false') {
        tokens.push({ type: 'boolean', value: false })
      } else {
        tokens.push({ type: 'identifier', value: id })
      }
      continue
    }

    // Unknown character, skip
    i++
  }

  tokens.push({ type: 'eof' })
  return tokens
}

/**
 * Parse and evaluate a "when" expression
 * Supports: &&, ||, !, ==, !=, parentheses
 * Example: "editorFocus && !inputFocus"
 */
function parseExpression(
  tokens: Token[],
  context: Map<string, unknown>,
  pos: { i: number }
): boolean {
  return parseOr(tokens, context, pos)
}

function parseOr(tokens: Token[], context: Map<string, unknown>, pos: { i: number }): boolean {
  let left = parseAnd(tokens, context, pos)

  while (tokens[pos.i]?.type === 'operator' && tokens[pos.i].value === '||') {
    pos.i++
    const right = parseAnd(tokens, context, pos)
    left = left || right
  }

  return left
}

function parseAnd(tokens: Token[], context: Map<string, unknown>, pos: { i: number }): boolean {
  let left = parseNot(tokens, context, pos)

  while (tokens[pos.i]?.type === 'operator' && tokens[pos.i].value === '&&') {
    pos.i++
    const right = parseNot(tokens, context, pos)
    left = left && right
  }

  return left
}

function parseNot(tokens: Token[], context: Map<string, unknown>, pos: { i: number }): boolean {
  if (tokens[pos.i]?.type === 'operator' && tokens[pos.i].value === '!') {
    pos.i++
    return !parseNot(tokens, context, pos)
  }
  return parseComparison(tokens, context, pos)
}

function parseComparison(
  tokens: Token[],
  context: Map<string, unknown>,
  pos: { i: number }
): boolean {
  const left = parsePrimary(tokens, context, pos)

  const token = tokens[pos.i]
  if (token?.type === 'operator' && (token.value === '==' || token.value === '!=')) {
    pos.i++
    const right = parsePrimary(tokens, context, pos)
    if (token.value === '==') {
      return left === right
    } else {
      return left !== right
    }
  }

  // If it's just an identifier, treat truthy/falsy as boolean
  return Boolean(left)
}

function parsePrimary(tokens: Token[], context: Map<string, unknown>, pos: { i: number }): unknown {
  const token = tokens[pos.i]

  if (!token || token.type === 'eof') {
    return false
  }

  // Parentheses
  if (token.type === 'lparen') {
    pos.i++
    const result = parseExpression(tokens, context, pos)
    if (tokens[pos.i]?.type === 'rparen') {
      pos.i++
    }
    return result
  }

  // Literals
  if (token.type === 'string') {
    pos.i++
    return token.value
  }
  if (token.type === 'number') {
    pos.i++
    return token.value
  }
  if (token.type === 'boolean') {
    pos.i++
    return token.value
  }

  // Identifier - look up in context
  if (token.type === 'identifier') {
    pos.i++
    return context.get(token.value)
  }

  pos.i++
  return false
}

/**
 * Context key service implementation
 */
class ContextKeyService implements IContextKeyService, IDisposable {
  private _context = new Map<string, unknown>()
  private _parent: ContextKeyService | null
  private _onDidChange = new Emitter<string>()

  constructor(parent: ContextKeyService | null = null) {
    this._parent = parent
  }

  get onDidChange(): Event<string> {
    return this._onDidChange.event
  }

  set(key: string, value: unknown): void {
    this._context.set(key, value)
    this._onDidChange.fire(key)
  }

  get(key: string): unknown {
    if (this._context.has(key)) {
      return this._context.get(key)
    }
    if (this._parent) {
      return this._parent.get(key)
    }
    return undefined
  }

  delete(key: string): void {
    this._context.delete(key)
    this._onDidChange.fire(key)
  }

  evaluate(expression: string): boolean {
    if (!expression || expression.trim() === '') {
      return true
    }

    try {
      // Build full context including parent
      const fullContext = new Map<string, unknown>()
      if (this._parent) {
        const parentCtx = this._parent
        // Walk up the parent chain
        const collectContext = (svc: ContextKeyService): void => {
          if (svc._parent) {
            collectContext(svc._parent)
          }
          for (const [k, v] of svc._context) {
            fullContext.set(k, v)
          }
        }
        collectContext(parentCtx)
      }
      for (const [k, v] of this._context) {
        fullContext.set(k, v)
      }

      const tokens = tokenize(expression)
      return parseExpression(tokens, fullContext, { i: 0 })
    } catch (e) {
      console.warn(`Failed to evaluate context expression: "${expression}"`, e)
      return false
    }
  }

  createScoped(): IContextKeyService {
    return new ContextKeyService(this)
  }

  dispose(): void {
    this._onDidChange.dispose()
  }
}

// Global context key service instance
let _globalContextKeyService: ContextKeyService | null = null

/**
 * Get or create the global context key service
 */
export function getContextKeyService(): IContextKeyService {
  if (!_globalContextKeyService) {
    _globalContextKeyService = new ContextKeyService()
  }
  return _globalContextKeyService
}

/**
 * Reset the global context key service (useful for testing)
 */
export function resetContextKeyService(): void {
  _globalContextKeyService?.dispose()
  _globalContextKeyService = null
}

/**
 * Helper to create a context key binding
 * Returns a disposable that removes the binding when disposed
 */
export function bindContextKey(key: string, value: unknown): IDisposable {
  const ctx = getContextKeyService()
  ctx.set(key, value)
  return toDisposable(() => ctx.delete(key))
}

// Fuzzy filtering utilities for quick access

import type { IHighlight } from './types'

/**
 * Fuzzy match result
 */
export interface IFuzzyMatch {
  score: number
  highlights: IHighlight[]
}

/**
 * Perform fuzzy matching on a string (VS Code style)
 * Matches characters in sequence, preferring word boundaries
 * Returns null if no match, otherwise returns score and highlights
 */
export function fuzzyMatch(pattern: string, text: string): IFuzzyMatch | null {
  if (!pattern) {
    return { score: 0, highlights: [] }
  }

  const patternLower = pattern.toLowerCase()
  const textLower = text.toLowerCase()

  // First try exact substring match (highest score)
  const exactIndex = textLower.indexOf(patternLower)
  if (exactIndex >= 0) {
    return {
      score: 100 + (exactIndex === 0 ? 50 : 0),
      highlights: [{ start: exactIndex, end: exactIndex + pattern.length }]
    }
  }

  // Try fuzzy matching - all pattern chars must appear in order
  const highlights: IHighlight[] = []
  let patternIdx = 0
  let score = 0
  let textIdx = 0

  while (textIdx < textLower.length && patternIdx < patternLower.length) {
    if (textLower[textIdx] === patternLower[patternIdx]) {
      // Found a match
      const isWordStart =
        textIdx === 0 ||
        textLower[textIdx - 1] === ' ' ||
        textLower[textIdx - 1] === ':' ||
        textLower[textIdx - 1] === '_' ||
        textLower[textIdx - 1] === '-'

      score += isWordStart ? 10 : 1

      // Add to highlights
      if (highlights.length > 0 && highlights[highlights.length - 1].end === textIdx) {
        highlights[highlights.length - 1].end = textIdx + 1
      } else {
        highlights.push({ start: textIdx, end: textIdx + 1 })
      }

      patternIdx++
    }
    textIdx++
  }

  // All pattern characters must be found
  if (patternIdx !== patternLower.length) {
    return null
  }

  return { score, highlights }
}

/**
 * Filter and score items
 */
export function filterItems<T>(
  items: T[],
  pattern: string,
  getText: (item: T) => string,
  getSecondaryText?: (item: T) => string | undefined
): Array<{ item: T; score: number; highlights: IHighlight[]; secondaryHighlights?: IHighlight[] }> {
  if (!pattern) {
    return items.map(item => ({ item, score: 0, highlights: [] }))
  }

  const results: Array<{ item: T; score: number; highlights: IHighlight[]; secondaryHighlights?: IHighlight[] }> = []

  for (const item of items) {
    const text = getText(item)
    const match = fuzzyMatch(pattern, text)

    let secondaryHighlights: IHighlight[] | undefined
    let secondaryMatch: IFuzzyMatch | null = null

    // Also try matching secondary text
    if (getSecondaryText) {
      const secondary = getSecondaryText(item)
      if (secondary) {
        secondaryMatch = fuzzyMatch(pattern, secondary)
        if (secondaryMatch) {
          secondaryHighlights = secondaryMatch.highlights
        }
      }
    }

    // Include if either primary or secondary matches
    if (match || secondaryMatch) {
      const primaryScore = match?.score ?? 0
      const secondaryScore = (secondaryMatch?.score ?? 0) * 0.5
      const totalScore = Math.max(primaryScore, secondaryScore)

      results.push({
        item,
        score: totalScore,
        highlights: match?.highlights ?? [],
        secondaryHighlights
      })
    }
  }

  // Sort by score descending
  results.sort((a, b) => b.score - a.score)

  return results
}

<script lang="ts">
  interface Props {
    content: string
    showLineNumbers?: boolean
  }

  let { content, showLineNumbers = true }: Props = $props()

  // Parse and tokenize JSON for syntax highlighting
  interface Token {
    type: 'key' | 'string' | 'number' | 'boolean' | 'null' | 'punctuation'
    value: string
  }

  function tokenizeLine(line: string): Token[] {
    const tokens: Token[] = []
    let remaining = line

    while (remaining.length > 0) {
      // Skip whitespace (preserve it)
      const wsMatch = remaining.match(/^(\s+)/)
      if (wsMatch) {
        tokens.push({ type: 'punctuation', value: wsMatch[1] })
        remaining = remaining.slice(wsMatch[1].length)
        continue
      }

      // Object key (with quotes and colon)
      const keyMatch = remaining.match(/^("(?:[^"\\]|\\.)*")\s*:/)
      if (keyMatch) {
        tokens.push({ type: 'key', value: keyMatch[1] })
        tokens.push({ type: 'punctuation', value: ':' })
        remaining = remaining.slice(keyMatch[0].length).trimStart()
        // Add space after colon if there was one
        if (keyMatch[0].includes(': ')) {
          tokens.push({ type: 'punctuation', value: ' ' })
        }
        continue
      }

      // String value
      const strMatch = remaining.match(/^"(?:[^"\\]|\\.)*"/)
      if (strMatch) {
        tokens.push({ type: 'string', value: strMatch[0] })
        remaining = remaining.slice(strMatch[0].length)
        continue
      }

      // Number
      const numMatch = remaining.match(/^-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/)
      if (numMatch) {
        tokens.push({ type: 'number', value: numMatch[0] })
        remaining = remaining.slice(numMatch[0].length)
        continue
      }

      // Boolean
      const boolMatch = remaining.match(/^(true|false)/)
      if (boolMatch) {
        tokens.push({ type: 'boolean', value: boolMatch[0] })
        remaining = remaining.slice(boolMatch[0].length)
        continue
      }

      // Null
      const nullMatch = remaining.match(/^null/)
      if (nullMatch) {
        tokens.push({ type: 'null', value: 'null' })
        remaining = remaining.slice(4)
        continue
      }

      // Punctuation (brackets, braces, commas)
      const punctMatch = remaining.match(/^[{}\[\],]/)
      if (punctMatch) {
        tokens.push({ type: 'punctuation', value: punctMatch[0] })
        remaining = remaining.slice(1)
        continue
      }

      // Fallback: take one character
      tokens.push({ type: 'punctuation', value: remaining[0] })
      remaining = remaining.slice(1)
    }

    return tokens
  }

  let lines = $derived(content.split('\n'))
  let tokenizedLines = $derived(lines.map(tokenizeLine))
  let lineNumberWidth = $derived(Math.max(3, String(lines.length).length))
</script>

<div class="json-viewer font-mono text-sm leading-6">
  {#each tokenizedLines as tokens, lineIndex}
    <div class="line flex">
      {#if showLineNumbers}
        <span
          class="line-number select-none text-right pr-4 shrink-0"
          style="width: {lineNumberWidth + 2}ch; color: var(--neo-editorLineNumber-foreground);"
        >
          {lineIndex + 1}
        </span>
      {/if}
      <span class="line-content whitespace-pre">
        {#each tokens as token}
          <span class="token-{token.type}">{token.value}</span>
        {/each}
      </span>
    </div>
  {/each}
</div>

<style>
  .json-viewer {
    padding: 0.5rem 0;
  }

  .line:hover {
    background: var(--neo-editor-lineHighlightBackground);
  }

  .token-key {
    color: var(--neo-token-key, #9cdcfe);
  }

  .token-string {
    color: var(--neo-token-string, #ce9178);
  }

  .token-number {
    color: var(--neo-token-number, #b5cea8);
  }

  .token-boolean {
    color: var(--neo-token-boolean, #569cd6);
  }

  .token-null {
    color: var(--neo-token-null, #569cd6);
  }

  .token-punctuation {
    color: var(--neo-foreground);
  }
</style>

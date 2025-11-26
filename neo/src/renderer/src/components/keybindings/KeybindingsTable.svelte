<script lang="ts">
  import { Pencil, Plus, Trash2, RotateCcw, Copy } from '@lucide/svelte'
  import type { IKeybindingItemEntry } from '$lib/keybindings/editorModel'
  import type { IHighlight } from '$lib/quickaccess/types'
  import * as ContextMenu from '$lib/components/ui/context-menu'

  interface Props {
    entries: IKeybindingItemEntry[]
    selectedId: string | null
    onselect: (id: string | null) => void
    onedit: (commandId: string, key: string | null) => void
    onadd: (commandId: string) => void
    onremove: (commandId: string, key: string, when?: string) => void
    onreset: (commandId: string, key?: string) => void
  }

  let { entries, selectedId, onselect, onedit, onadd, onremove, onreset }: Props = $props()

  // Render text with highlights
  function renderHighlighted(text: string, highlights?: IHighlight[]): string {
    if (!highlights || highlights.length === 0) {
      return escapeHtml(text)
    }

    let result = ''
    let lastIndex = 0

    for (const hl of highlights) {
      if (hl.start > lastIndex) {
        result += escapeHtml(text.slice(lastIndex, hl.start))
      }
      result += `<mark class="highlight">${escapeHtml(text.slice(hl.start, hl.end))}</mark>`
      lastIndex = hl.end
    }

    if (lastIndex < text.length) {
      result += escapeHtml(text.slice(lastIndex))
    }

    return result
  }

  function escapeHtml(text: string): string {
    return text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
  }

  // Copy to clipboard
  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text)
  }

  // Generate JSON for keybinding
  function getKeybindingJson(entry: IKeybindingItemEntry): string {
    const obj: Record<string, unknown> = {
      key: entry.keybinding,
      command: entry.commandId
    }
    if (entry.when) {
      obj.when = entry.when
    }
    return JSON.stringify(obj, null, 2)
  }
</script>

<div class="keybindings-list">
  <!-- Header -->
  <div class="keybindings-header">
    <div class="col-actions"></div>
    <div class="col-command">Command</div>
    <div class="col-keybinding">Keybinding</div>
    <div class="col-when">When</div>
    <div class="col-source">Source</div>
  </div>

  <!-- Rows -->
  <div class="keybindings-body">
    {#each entries as entry (entry.id)}
      <ContextMenu.Root>
        <ContextMenu.Trigger>
          <div
            class="keybindings-row"
            class:selected={entry.id === selectedId}
            onclick={() => onselect(entry.id)}
            ondblclick={() => {
              if (entry.keybinding) {
                onedit(entry.commandId, entry.keybinding)
              } else {
                onadd(entry.commandId)
              }
            }}
            role="button"
            tabindex="0"
            onkeydown={(e) => {
              if (e.key === 'Enter') {
                if (entry.keybinding) {
                  onedit(entry.commandId, entry.keybinding)
                } else {
                  onadd(entry.commandId)
                }
              }
            }}
          >
            <!-- Actions -->
            <div class="col-actions">
              {#if entry.keybinding}
                <button
                  class="action-btn"
                  onclick={(e) => {
                    e.stopPropagation()
                    onedit(entry.commandId, entry.keybinding)
                  }}
                  title="Edit keybinding"
                >
                  <Pencil class="w-3.5 h-3.5" />
                </button>
              {:else}
                <button
                  class="action-btn"
                  onclick={(e) => {
                    e.stopPropagation()
                    onadd(entry.commandId)
                  }}
                  title="Add keybinding"
                >
                  <Plus class="w-3.5 h-3.5" />
                </button>
              {/if}
            </div>

            <!-- Command -->
            <div class="col-command">
              <span class="command-label">
                <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                {@html renderHighlighted(entry.commandLabel, entry.commandMatches)}
              </span>
              <span class="command-id">{entry.commandId}</span>
            </div>

            <!-- Keybinding -->
            <div class="col-keybinding">
              {#if entry.keybindingLabel}
                <kbd class="keybinding-badge">
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  {@html renderHighlighted(entry.keybindingLabel, entry.keybindingMatches)}
                </kbd>
              {:else}
                <span class="empty-value">—</span>
              {/if}
            </div>

            <!-- When -->
            <div class="col-when">
              {#if entry.when}
                <code class="when-expression">
                  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
                  {@html renderHighlighted(entry.when, entry.whenMatches)}
                </code>
              {:else}
                <span class="empty-value">—</span>
              {/if}
            </div>

            <!-- Source -->
            <div class="col-source">
              <span class="source-badge" class:user={entry.source === 'user'}>
                {entry.source}
              </span>
            </div>
          </div>
        </ContextMenu.Trigger>

        <ContextMenu.Content>
          {#if entry.keybinding}
            <ContextMenu.Item onclick={() => onedit(entry.commandId, entry.keybinding)}>
              <Pencil class="w-4 h-4 mr-2" />
              Change Keybinding
            </ContextMenu.Item>
          {/if}
          <ContextMenu.Item onclick={() => onadd(entry.commandId)}>
            <Plus class="w-4 h-4 mr-2" />
            Add Keybinding
          </ContextMenu.Item>

          {#if entry.keybinding}
            <ContextMenu.Separator />
            <ContextMenu.Item onclick={() => onremove(entry.commandId, entry.keybinding!, entry.when ?? undefined)}>
              <Trash2 class="w-4 h-4 mr-2" />
              Remove Keybinding
            </ContextMenu.Item>
            {#if entry.source === 'user'}
              <ContextMenu.Item onclick={() => onreset(entry.commandId, entry.keybinding ?? undefined)}>
                <RotateCcw class="w-4 h-4 mr-2" />
                Reset to Default
              </ContextMenu.Item>
            {/if}
          {/if}

          <ContextMenu.Separator />
          <ContextMenu.Item onclick={() => copyToClipboard(entry.commandId)}>
            <Copy class="w-4 h-4 mr-2" />
            Copy Command ID
          </ContextMenu.Item>
          {#if entry.keybinding}
            <ContextMenu.Item onclick={() => copyToClipboard(getKeybindingJson(entry))}>
              <Copy class="w-4 h-4 mr-2" />
              Copy as JSON
            </ContextMenu.Item>
          {/if}
        </ContextMenu.Content>
      </ContextMenu.Root>
    {/each}
  </div>

  {#if entries.length === 0}
    <div class="empty-state">
      No keybindings found
    </div>
  {/if}
</div>

<style>
  .keybindings-list {
    display: flex;
    flex-direction: column;
    width: 100%;
    font-size: 13px;
  }

  .keybindings-header {
    display: flex;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid var(--neo-border);
    color: color-mix(in srgb, var(--neo-foreground) 70%, transparent);
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    position: sticky;
    top: 0;
    background: #1e1e1e;
    z-index: 10;
  }

  .keybindings-body {
    display: flex;
    flex-direction: column;
  }

  .keybindings-row {
    display: flex;
    align-items: center;
    padding: 6px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--neo-border) 50%, transparent);
    cursor: pointer;
    transition: background-color 0.1s;
  }

  .keybindings-row:hover {
    background: color-mix(in srgb, var(--neo-foreground) 8%, transparent);
  }

  .keybindings-row.selected {
    background: var(--neo-primary);
    color: var(--neo-primaryForeground);
  }

  .keybindings-row:hover .action-btn {
    opacity: 0.6;
  }

  /* Column widths */
  .col-actions {
    width: 40px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .col-command {
    flex: 1;
    min-width: 200px;
    padding-right: 16px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .col-keybinding {
    width: 160px;
    flex-shrink: 0;
    padding-right: 16px;
  }

  .col-when {
    width: 180px;
    flex-shrink: 0;
    padding-right: 16px;
  }

  .col-source {
    width: 70px;
    flex-shrink: 0;
  }

  /* Action button */
  .action-btn {
    opacity: 0;
    padding: 4px;
    border-radius: 4px;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .action-btn:hover {
    opacity: 1 !important;
    background: color-mix(in srgb, var(--neo-foreground) 15%, transparent);
  }

  /* Command column */
  .command-label {
    color: var(--neo-foreground);
  }

  .command-id {
    font-size: 11px;
    color: var(--neo-foreground);
    opacity: 0.5;
  }

  /* Keybinding badge */
  .keybinding-badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 6px;
    font-size: 11px;
    font-family: var(--neo-font-family);
    background: var(--neo-keybindingLabel-background, rgba(128, 128, 128, 0.17));
    color: var(--neo-keybindingLabel-foreground, var(--neo-foreground));
    border: 1px solid var(--neo-keybindingLabel-border, rgba(128, 128, 128, 0.2));
    border-radius: 3px;
    box-shadow: inset 0 -1px 0 var(--neo-keybindingLabel-bottomBorder, rgba(128, 128, 128, 0.2));
  }

  /* When expression */
  .when-expression {
    font-size: 11px;
    color: var(--neo-foreground);
    opacity: 0.7;
    background: color-mix(in srgb, var(--neo-foreground) 10%, transparent);
    padding: 2px 6px;
    border-radius: 3px;
    display: inline-block;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Source badge */
  .source-badge {
    font-size: 11px;
    text-transform: capitalize;
    color: var(--neo-foreground);
    opacity: 0.6;
  }

  .source-badge.user {
    color: var(--neo-primary);
    opacity: 1;
  }

  /* Empty values */
  .empty-value {
    color: var(--neo-foreground);
    opacity: 0.2;
  }

  /* Empty state */
  .empty-state {
    padding: 48px 0;
    text-align: center;
    font-size: 13px;
    color: var(--neo-foreground);
    opacity: 0.5;
  }

  /* Highlight for search matches */
  :global(.highlight) {
    background: transparent;
    color: var(--neo-primary);
    font-weight: 600;
  }
</style>

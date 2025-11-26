<script lang="ts">
  import { PaneGroup, Pane, PaneResizer } from 'paneforge'
  import { editorStore, type EditorLayoutNode } from '$lib/stores/editor.svelte'
  import EditorGroup from './EditorGroup.svelte'

  let layout = $derived(editorStore.state.layout)

  // Resizer classes
  const hResizerClass =
    'w-1 bg-transparent hover:bg-[var(--neo-focusBorder)] active:bg-[var(--neo-focusBorder)] transition-colors cursor-col-resize'
  const vResizerClass =
    'h-1 bg-transparent hover:bg-[var(--neo-focusBorder)] active:bg-[var(--neo-focusBorder)] transition-colors cursor-row-resize'
</script>

<div class="editor-area h-full" style="background: var(--neo-background);">
  {@render layoutNode(layout.root, [])}
</div>

{#snippet layoutNode(node: EditorLayoutNode, path: number[])}
  {#if node.type === 'group'}
    <EditorGroup groupId={node.groupId} />
  {:else if node.type === 'split'}
    <PaneGroup
      direction={node.direction}
      onLayoutChange={(sizes) => editorStore.updateSplitSizes(path, sizes)}
    >
      {#each node.children as child, index}
        {#if index > 0}
          <PaneResizer class={node.direction === 'horizontal' ? hResizerClass : vResizerClass} />
        {/if}
        <Pane defaultSize={node.sizes[index]} minSize={10}>
          {@render layoutNode(child, [...path, index])}
        </Pane>
      {/each}
    </PaneGroup>
  {/if}
{/snippet}

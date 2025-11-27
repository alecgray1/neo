<script lang="ts">
  import { PaneResizer } from 'paneforge'
</script>

<PaneResizer class="resizer" />

<style>
  :global(.resizer) {
    width: 4px;
    background: transparent;
    transition: background-color 0.15s;
    cursor: col-resize;
  }

  :global(.resizer:hover),
  :global(.resizer[data-active]) {
    background: var(--neo-focusBorder);
  }

  :global([data-direction='vertical'] > .resizer) {
    width: auto;
    height: 4px;
    cursor: row-resize;
  }
</style>

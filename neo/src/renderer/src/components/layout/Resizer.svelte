<script lang="ts">
  import { PaneResizer } from 'paneforge'
</script>

<PaneResizer class="resizer" />

<style>
  :global(.resizer) {
    position: relative;
    z-index: 10;
    width: 4px;
    margin-left: -2px;
    margin-right: -2px;
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
    margin-left: 0;
    margin-right: 0;
    margin-top: -2px;
    margin-bottom: -2px;
    cursor: row-resize;
  }
</style>

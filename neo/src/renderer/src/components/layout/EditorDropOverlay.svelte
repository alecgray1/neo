<script lang="ts">
  export type DropZone = 'left' | 'right' | 'top' | 'bottom' | 'center' | null

  interface Props {
    visible: boolean
    zone: DropZone
  }

  let { visible, zone }: Props = $props()
</script>

{#if visible}
  <div class="drop-overlay">
    <div
      class="drop-indicator"
      class:left={zone === 'left'}
      class:right={zone === 'right'}
      class:top={zone === 'top'}
      class:bottom={zone === 'bottom'}
      class:center={zone === 'center'}
    ></div>
  </div>
{/if}

<style>
  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 1000;
    pointer-events: none;
  }

  .drop-indicator {
    position: absolute;
    background: var(--neo-editor-dropIntoPromptBackground, rgba(83, 89, 93, 0.5));
    border: 2px dashed var(--neo-focusBorder, #007fd4);
    opacity: 0;
    transition: opacity 150ms ease-out, top 70ms ease-out, left 70ms ease-out, width 70ms ease-out, height 70ms ease-out;
  }

  .drop-indicator.left {
    opacity: 1;
    top: 0;
    left: 0;
    width: 50%;
    height: 100%;
  }

  .drop-indicator.right {
    opacity: 1;
    top: 0;
    left: 50%;
    width: 50%;
    height: 100%;
  }

  .drop-indicator.top {
    opacity: 1;
    top: 0;
    left: 0;
    width: 100%;
    height: 50%;
  }

  .drop-indicator.bottom {
    opacity: 1;
    top: 50%;
    left: 0;
    width: 100%;
    height: 50%;
  }

  .drop-indicator.center {
    opacity: 1;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
  }
</style>

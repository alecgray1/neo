<script lang="ts">
  import { GitBranch, AlertCircle, AlertTriangle, CheckCircle, Bell } from '@lucide/svelte'
</script>

<div
  class="status-bar flex items-center justify-between px-2 h-[22px] text-xs select-none"
  style="background: var(--neo-statusBar-background); color: var(--neo-statusBar-foreground);"
>
  <div class="flex items-center gap-3">
    <button class="status-item flex items-center gap-1 hover:bg-white/10 px-1 rounded">
      <GitBranch class="w-3.5 h-3.5" />
      <span>main</span>
    </button>

    <button class="status-item flex items-center gap-1 hover:bg-white/10 px-1 rounded">
      <AlertCircle class="w-3.5 h-3.5" />
      <span>0</span>
      <AlertTriangle class="w-3.5 h-3.5" />
      <span>0</span>
    </button>
  </div>

  <div class="flex items-center gap-3">
    <button class="status-item hover:bg-white/10 px-1 rounded">Ln 1, Col 1</button>
    <button class="status-item hover:bg-white/10 px-1 rounded">Spaces: 2</button>
    <button class="status-item hover:bg-white/10 px-1 rounded">UTF-8</button>
    <button class="status-item hover:bg-white/10 px-1 rounded">Svelte</button>
    <button class="status-item flex items-center gap-1 hover:bg-white/10 px-1 rounded">
      <CheckCircle class="w-3.5 h-3.5" />
      <span>Prettier</span>
    </button>
    <button class="status-item hover:bg-white/10 px-1 rounded">
      <Bell class="w-3.5 h-3.5" />
    </button>
  </div>
</div>

<style>
  .status-item {
    cursor: pointer;
  }
</style>

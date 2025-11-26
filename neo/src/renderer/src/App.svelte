<script lang="ts">
  import { onMount } from 'svelte'
  import AppLayout from './components/layout/AppLayout.svelte'
  import QuickAccess from './components/quickaccess/QuickAccess.svelte'
  import { initializeCommandSystem } from '$lib/init'

  // Initialize command system on mount
  onMount(() => {
    const disposable = initializeCommandSystem()
    return () => disposable.dispose()
  })
</script>

<AppLayout />
<QuickAccess />

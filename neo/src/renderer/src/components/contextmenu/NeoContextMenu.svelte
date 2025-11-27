<script lang="ts">
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { getMenuActions, type IMenuActionGroup } from '$lib/menus/service'
  import type { MenuId } from '$lib/menus/menuId'
  import type { IContextKeyService } from '$lib/services/context'
  import type { Snippet } from 'svelte'

  interface Props {
    /** Which menu to render */
    menuId: MenuId
    /** Context service for evaluating when clauses */
    contextKeyService: IContextKeyService
    /** Context argument to pass to commands (VS Code pattern) */
    arg?: unknown
    /** Content to wrap */
    children: Snippet
    /** Optional callback when menu opens */
    onOpen?: () => void
    /** Optional callback when menu closes */
    onClose?: () => void
  }

  let { menuId, contextKeyService, arg, children, onOpen, onClose }: Props = $props()

  // Menu actions state (recalculated when menu opens)
  let groups = $state<IMenuActionGroup[]>([])

  function handleOpenChange(open: boolean) {
    if (open) {
      // Build menu actions with context arg captured at this moment (VS Code pattern)
      // The arg is closed over in each action's run() function
      groups = getMenuActions(menuId, contextKeyService, { arg })
      onOpen?.()
    } else {
      onClose?.()
    }
  }
</script>

<ContextMenu.Root onOpenChange={handleOpenChange}>
  <ContextMenu.Trigger>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Content>
    {#if groups.length === 0}
      <ContextMenu.Item disabled>No actions available</ContextMenu.Item>
    {:else}
      {#each groups as group, groupIndex}
        {#if groupIndex > 0}
          <ContextMenu.Separator />
        {/if}
        {#each group.actions as action}
          <ContextMenu.Item onclick={action.run} disabled={!action.enabled}>
            {action.label}
          </ContextMenu.Item>
        {/each}
      {/each}
    {/if}
  </ContextMenu.Content>
</ContextMenu.Root>

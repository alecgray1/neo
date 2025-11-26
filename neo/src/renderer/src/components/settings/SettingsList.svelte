<script lang="ts">
  import type { ISettingsGroup } from '$lib/settings/types'
  import SettingRow from './SettingRow.svelte'

  interface Props {
    groups: ISettingsGroup[]
    onupdate: (id: string, value: unknown) => void
    onreset: (id: string) => void
  }

  let { groups, onupdate, onreset }: Props = $props()
</script>

<div class="settings-list">
  {#if groups.length === 0}
    <div class="empty-state">
      <p>No settings found.</p>
      <p class="hint">Try adjusting your search or filter criteria.</p>
    </div>
  {:else}
    {#each groups as group (group.id)}
      <div class="settings-group">
        <h3 class="group-title">{group.path.join(' > ')}</h3>
        <div class="group-settings">
          {#each group.settings as setting (setting.id)}
            <SettingRow
              {setting}
              {onupdate}
              {onreset}
            />
          {/each}
        </div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .settings-list {
    padding: 0 24px 24px 24px;
  }

  .empty-state {
    padding: 48px 0;
    text-align: center;
  }

  .empty-state p {
    margin: 0;
    font-size: 14px;
    color: var(--neo-foreground);
  }

  .empty-state .hint {
    margin-top: 8px;
    font-size: 13px;
    opacity: 0.7;
  }

  .settings-group {
    margin-bottom: 0;
  }

  .group-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--neo-settings-headerForeground, var(--neo-foreground));
    margin: 0;
    padding: 20px 14px 10px 14px;
  }

  .group-settings {
    display: flex;
    flex-direction: column;
  }
</style>

<script lang="ts">
  import { onMount } from 'svelte'
  import { getSettingsEditorStore } from './store.svelte'
  import SettingsSearch from './SettingsSearch.svelte'
  import SettingsTOC from './SettingsTOC.svelte'
  import SettingsList from './SettingsList.svelte'
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'

  const store = getSettingsEditorStore()

  onMount(async () => {
    await store.initialize()
  })
</script>

<div class="settings-editor">
  <div class="settings-editor-inner">
    <!-- Header -->
    <div class="settings-header">
      <div class="settings-title">
        <h2>Settings</h2>
        {#if store.modifiedCount > 0}
          <span class="modified-badge">{store.modifiedCount} modified</span>
        {/if}
      </div>

      <!-- Search -->
      <div class="settings-search-container">
        <SettingsSearch
          value={store.searchQuery}
          onchange={(query) => store.setSearchQuery(query)}
        />
      </div>
    </div>

    <!-- Content -->
    <div class="settings-body">
      <!-- TOC Sidebar -->
      <div class="settings-toc-container">
        <ScrollArea class="h-full">
          <SettingsTOC
            categories={store.categories}
            selectedCategory={store.selectedCategory}
            expandedCategories={store.expandedCategories}
            onselect={(id) => store.selectCategory(id)}
            ontoggle={(id) => store.toggleCategory(id)}
          />
        </ScrollArea>
      </div>

      <!-- Settings List -->
      <div class="settings-tree-container">
        <ScrollArea class="h-full">
          <SettingsList
            groups={store.groups}
            onupdate={(id, value) => store.updateSetting(id, value)}
            onreset={(id) => store.resetSetting(id)}
          />
        </ScrollArea>
      </div>
    </div>
  </div>
</div>

<style>
  .settings-editor {
    height: 100%;
    overflow: hidden;
    background: var(--neo-editor-background);
    color: var(--neo-foreground);
  }

  .settings-editor-inner {
    display: flex;
    flex-direction: column;
    height: 100%;
    max-width: 1200px;
    margin: 0 auto;
  }

  .settings-header {
    padding: 16px 24px 0 24px;
    flex-shrink: 0;
  }

  .settings-title {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .settings-title h2 {
    font-size: 20px;
    font-weight: 400;
    margin: 0;
    color: var(--neo-foreground);
  }

  .modified-badge {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--neo-badge-background, #4d4d4d);
    color: var(--neo-badge-foreground, #fff);
  }

  .settings-search-container {
    margin-bottom: 12px;
  }

  .settings-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .settings-toc-container {
    width: 200px;
    flex-shrink: 0;
    height: 100%;
    padding-top: 8px;
  }

  .settings-tree-container {
    flex: 1;
    height: 100%;
    border-left: 1px solid var(--neo-panel-border, var(--neo-border));
  }
</style>

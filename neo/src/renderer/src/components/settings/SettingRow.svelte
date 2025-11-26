<script lang="ts">
  import { RotateCcw } from '@lucide/svelte'
  import type { ISettingItemEntry } from '$lib/settings/editorModel'
  import BooleanControl from './controls/BooleanControl.svelte'
  import StringControl from './controls/StringControl.svelte'
  import NumberControl from './controls/NumberControl.svelte'
  import EnumControl from './controls/EnumControl.svelte'
  import ArrayControl from './controls/ArrayControl.svelte'
  import ObjectControl from './controls/ObjectControl.svelte'

  interface Props {
    setting: ISettingItemEntry
    onupdate: (id: string, value: unknown) => void
    onreset: (id: string) => void
  }

  let { setting, onupdate, onreset }: Props = $props()

  function getSettingType(): string {
    const schema = setting.schema
    const type = Array.isArray(schema.type) ? schema.type[0] : schema.type

    // Check for enum first
    if (schema.enum && schema.enum.length > 0) {
      return 'enum'
    }

    return type || 'string'
  }

  function handleUpdate(value: unknown) {
    onupdate(setting.id, value)
  }

  // Extract the setting name (last part of id)
  function getSettingName(): string {
    const parts = setting.id.split('.')
    return parts[parts.length - 1]
  }

  // Extract the category (all but last part)
  function getCategory(): string {
    const parts = setting.id.split('.')
    return parts.slice(0, -1).join(' > ')
  }
</script>

<div class="setting-row" class:modified={setting.isModified}>
  {#if setting.isModified}
    <div class="modified-indicator"></div>
  {/if}

  <div class="setting-contents">
    <div class="setting-header">
      <div class="setting-title">
        <span class="setting-category">{getCategory()}</span>
        <span class="setting-name">{getSettingName()}</span>
      </div>
      <div class="setting-toolbar">
        {#if setting.isModified}
          <button
            class="reset-btn"
            onclick={() => onreset(setting.id)}
            title="Reset to default"
          >
            <RotateCcw class="w-4 h-4" />
          </button>
        {/if}
      </div>
    </div>

    {#if setting.schema.description}
      <p class="setting-description">{setting.schema.description}</p>
    {/if}

    {#if setting.schema.deprecationMessage}
      <p class="setting-deprecated">{setting.schema.deprecationMessage}</p>
    {/if}

    <div class="setting-control">
      {#if getSettingType() === 'boolean'}
        <BooleanControl
          value={setting.value as boolean}
          onupdate={handleUpdate}
        />
      {:else if getSettingType() === 'enum'}
        <EnumControl
          value={setting.value as string}
          options={setting.schema.enum || []}
          descriptions={setting.schema.enumDescriptions}
          labels={setting.schema.enumItemLabels}
          onupdate={handleUpdate}
        />
      {:else if getSettingType() === 'number' || getSettingType() === 'integer'}
        <NumberControl
          value={setting.value as number}
          minimum={setting.schema.minimum}
          maximum={setting.schema.maximum}
          isInteger={getSettingType() === 'integer'}
          onupdate={handleUpdate}
        />
      {:else if getSettingType() === 'array'}
        <ArrayControl
          value={setting.value as unknown[]}
          itemSchema={setting.schema.items}
          onupdate={handleUpdate}
        />
      {:else if getSettingType() === 'object'}
        <ObjectControl
          value={setting.value as Record<string, unknown>}
          schema={setting.schema}
          onupdate={handleUpdate}
        />
      {:else}
        <StringControl
          value={setting.value as string}
          multiline={setting.schema.editPresentation === 'multilineText'}
          pattern={setting.schema.pattern}
          onupdate={handleUpdate}
        />
      {/if}
    </div>
  </div>
</div>

<style>
  .setting-row {
    position: relative;
    padding: 12px 14px 18px 14px;
    border-radius: 0;
    background: transparent;
    transition: background-color 0.1s;
  }

  .setting-row:hover {
    background: var(--neo-settings-rowHoverBackground, rgba(255, 255, 255, 0.03));
  }

  .setting-row.modified {
    /* No special background, just the indicator */
  }

  .modified-indicator {
    position: absolute;
    left: 6px;
    top: 15px;
    bottom: 18px;
    width: 0;
    border-left: 2px solid var(--neo-settings-modifiedItemIndicator, #0078d4);
  }

  .setting-contents {
    padding-left: 10px;
  }

  .setting-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 2px;
  }

  .setting-title {
    display: flex;
    align-items: baseline;
    gap: 0;
    flex-wrap: wrap;
  }

  .setting-category {
    font-size: 13px;
    color: var(--neo-descriptionForeground, rgba(255, 255, 255, 0.7));
    margin-right: 4px;
  }

  .setting-category::after {
    content: ':';
  }

  .setting-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--neo-settings-headerForeground, var(--neo-foreground));
  }

  .setting-toolbar {
    opacity: 0;
    transition: opacity 0.1s;
  }

  .setting-row:hover .setting-toolbar {
    opacity: 1;
  }

  .reset-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    background: none;
    border: none;
    color: var(--neo-foreground);
    opacity: 0.7;
    cursor: pointer;
    border-radius: 4px;
  }

  .reset-btn:hover {
    opacity: 1;
    background: var(--neo-toolbar-hoverBackground, rgba(255, 255, 255, 0.1));
  }

  .setting-description {
    font-size: 13px;
    color: var(--neo-foreground);
    opacity: 0.9;
    margin: 0 0 8px 0;
    line-height: 1.4;
  }

  .setting-deprecated {
    font-size: 12px;
    color: var(--neo-editorWarning-foreground, #cca700);
    margin: 4px 0 8px 0;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--neo-editorWarning-foreground, #cca700) 15%, transparent);
    border-radius: 4px;
  }

  .setting-control {
    margin-top: 6px;
  }
</style>

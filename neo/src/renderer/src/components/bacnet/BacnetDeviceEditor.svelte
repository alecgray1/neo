<script lang="ts">
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Radio, RefreshCw, MapPin, Cpu, Hash, Settings2, Activity, Trash2 } from '@lucide/svelte'
  import { serverStore, type BacnetObject, type BacnetObjectValue } from '$lib/stores/server.svelte'
  import { editorStore } from '$lib/stores/editor.svelte'

  interface Props {
    content: string
    uri: string
  }

  let { content, uri }: Props = $props()

  let showRemoveDialog = $state(false)
  let isRemoving = $state(false)

  // Parse device data from JSON content
  let device = $derived(() => {
    try {
      return JSON.parse(content)
    } catch {
      return null
    }
  })

  // Get device ID for looking up objects
  let deviceId = $derived(() => device()?.device_id as number | undefined)

  // Get objects from the server store
  let objects = $derived(() => {
    const id = deviceId()
    return id !== undefined ? serverStore.getBacnetDeviceObjects(id) : []
  })

  // Get object value from store
  function getObjectValue(obj: BacnetObject): BacnetObjectValue | undefined {
    const id = deviceId()
    if (id === undefined) return undefined
    return serverStore.getBacnetObjectValue(id, obj.object_type, obj.instance)
  }

  // Format value for display
  function formatValue(value: unknown): string {
    if (value === null || value === undefined) return '—'
    if (typeof value === 'number') {
      // Format floats to 2 decimal places
      return Number.isInteger(value) ? value.toString() : value.toFixed(2)
    }
    if (typeof value === 'boolean') return value ? 'ON' : 'OFF'
    if (typeof value === 'object') return JSON.stringify(value)
    return String(value)
  }

  // Check if object type is readable (has present-value)
  function isReadableType(objectType: string): boolean {
    const normalized = objectType.toLowerCase()
    return [
      'analoginput', 'analogoutput', 'analogvalue',
      'binaryinput', 'binaryoutput', 'binaryvalue',
      'multistateinput', 'multistateoutput', 'multistatevalue'
    ].includes(normalized)
  }

  let isLoadingObjects = $state(false)

  async function refreshObjects() {
    const id = deviceId()
    if (id === undefined) return

    isLoadingObjects = true
    try {
      await serverStore.readBacnetDeviceObjects(id)
    } catch (e) {
      console.error('Failed to read objects:', e)
    }
    // Note: Loading state will be cleared when objects arrive via subscription
    // But we add a timeout fallback in case the request fails silently
    setTimeout(() => {
      isLoadingObjects = false
    }, 10000)
  }

  // Clear loading state when objects arrive
  $effect(() => {
    if (objects().length > 0) {
      isLoadingObjects = false
    }
  })

  async function handleRemoveDevice() {
    const id = deviceId()
    if (id === undefined) return

    isRemoving = true
    try {
      // Close this editor tab
      editorStore.closeTabByUri(uri)
      // Remove the device
      await serverStore.removeBacnetDevice(id)
    } catch (e) {
      console.error('Failed to remove device:', e)
      isRemoving = false
    }
    showRemoveDialog = false
  }
</script>

<ScrollArea class="h-full">
  <div class="p-6 max-w-4xl mx-auto">
    {#if device()}
      <!-- Device Header -->
      <div class="flex items-center gap-4 mb-6">
        <div
          class="w-16 h-16 rounded-lg flex items-center justify-center"
          style="background: var(--neo-badge-background);"
        >
          <Radio class="w-8 h-8" style="color: var(--neo-textLink-foreground);" />
        </div>
        <div class="flex-1">
          <h1 class="text-2xl font-semibold" style="color: var(--neo-editor-foreground);">
            Device {device().device_id}
          </h1>
          <p class="text-sm opacity-60">BACnet/IP Device</p>
        </div>
        <Dialog.Root bind:open={showRemoveDialog}>
          <Dialog.Trigger>
            <Button variant="destructive" size="sm">
              <Trash2 class="w-4 h-4 mr-2" />
              Remove Device
            </Button>
          </Dialog.Trigger>
          <Dialog.Content>
            <Dialog.Header>
              <Dialog.Title>Remove Device</Dialog.Title>
              <Dialog.Description>
                Are you sure you want to remove Device {device().device_id} from the system?
                This will stop polling and remove all associated data.
              </Dialog.Description>
            </Dialog.Header>
            <Dialog.Footer>
              <Button variant="outline" onclick={() => showRemoveDialog = false}>
                Cancel
              </Button>
              <Button variant="destructive" onclick={handleRemoveDevice} disabled={isRemoving}>
                {isRemoving ? 'Removing...' : 'Remove'}
              </Button>
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Root>
      </div>

      <!-- Device Info Cards -->
      <div class="grid grid-cols-2 gap-4 mb-6">
        <!-- Address Card -->
        <div
          class="p-4 rounded-lg"
          style="background: var(--neo-editorWidget-background); border: 1px solid var(--neo-editorWidget-border);"
        >
          <div class="flex items-center gap-2 mb-2">
            <MapPin class="w-4 h-4 opacity-60" />
            <span class="text-xs uppercase tracking-wide opacity-60">Network Address</span>
          </div>
          <div class="font-mono text-lg" style="color: var(--neo-textLink-foreground);">
            {device().address}
          </div>
        </div>

        <!-- Vendor Card -->
        <div
          class="p-4 rounded-lg"
          style="background: var(--neo-editorWidget-background); border: 1px solid var(--neo-editorWidget-border);"
        >
          <div class="flex items-center gap-2 mb-2">
            <Cpu class="w-4 h-4 opacity-60" />
            <span class="text-xs uppercase tracking-wide opacity-60">Vendor ID</span>
          </div>
          <div class="font-mono text-lg">
            {device().vendor_id}
          </div>
        </div>

        <!-- Max APDU Card -->
        <div
          class="p-4 rounded-lg"
          style="background: var(--neo-editorWidget-background); border: 1px solid var(--neo-editorWidget-border);"
        >
          <div class="flex items-center gap-2 mb-2">
            <Hash class="w-4 h-4 opacity-60" />
            <span class="text-xs uppercase tracking-wide opacity-60">Max APDU</span>
          </div>
          <div class="font-mono text-lg">
            {device().max_apdu} bytes
          </div>
        </div>

        <!-- Segmentation Card -->
        <div
          class="p-4 rounded-lg"
          style="background: var(--neo-editorWidget-background); border: 1px solid var(--neo-editorWidget-border);"
        >
          <div class="flex items-center gap-2 mb-2">
            <Settings2 class="w-4 h-4 opacity-60" />
            <span class="text-xs uppercase tracking-wide opacity-60">Segmentation</span>
          </div>
          <div class="font-mono text-lg capitalize">
            {device().segmentation}
          </div>
        </div>
      </div>

      <!-- Objects Section -->
      <div
        class="rounded-lg"
        style="background: var(--neo-editorWidget-background); border: 1px solid var(--neo-editorWidget-border);"
      >
        <div class="flex items-center justify-between p-4 border-b" style="border-color: var(--neo-editorWidget-border);">
          <h2 class="font-medium">Object List ({objects().length} objects)</h2>
          <Button variant="outline" size="sm" onclick={refreshObjects} disabled={isLoadingObjects}>
            <span class={isLoadingObjects ? 'animate-spin' : ''}>
              <RefreshCw class="w-4 h-4 mr-2" />
            </span>
            {isLoadingObjects ? 'Loading...' : 'Read Objects'}
          </Button>
        </div>
        <div class="p-4">
          {#if objects().length === 0}
            <div class="text-center py-8 opacity-60">
              <Radio class="w-8 h-8 mx-auto mb-2 opacity-30" />
              <p class="text-sm">Click "Read Objects" to discover points on this device</p>
              <p class="text-xs mt-1 opacity-60">This will read the object list from the device</p>
            </div>
          {:else}
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b" style="border-color: var(--neo-editorWidget-border);">
                  <th class="text-left py-2 px-2 opacity-60">Object Type</th>
                  <th class="text-center py-2 px-2 opacity-60">Instance</th>
                  <th class="text-right py-2 px-2 opacity-60">Present Value</th>
                </tr>
              </thead>
              <tbody>
                {#each objects() as obj}
                  {@const objValue = getObjectValue(obj)}
                  <tr class="border-b hover:bg-[var(--neo-list-hoverBackground)]" style="border-color: var(--neo-editorWidget-border);">
                    <td class="py-2 px-2 font-mono">{obj.object_type}</td>
                    <td class="py-2 px-2 text-center font-mono" style="color: var(--neo-textLink-foreground);">
                      {obj.instance}
                    </td>
                    <td class="py-2 px-2 text-right font-mono">
                      {#if isReadableType(obj.object_type)}
                        {#if objValue}
                          <span class="inline-flex items-center gap-1">
                            <Activity class="w-3 h-3 text-green-500" />
                            <span style="color: var(--neo-editor-foreground);">{formatValue(objValue.value)}</span>
                          </span>
                        {:else}
                          <span class="opacity-40">polling...</span>
                        {/if}
                      {:else}
                        <span class="opacity-30">—</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      </div>

      <!-- Raw Data (collapsible) -->
      <details class="mt-6">
        <summary
          class="cursor-pointer text-sm opacity-60 hover:opacity-100 py-2"
        >
          Raw Device Data
        </summary>
        <pre
          class="mt-2 p-4 rounded-lg text-xs font-mono overflow-x-auto"
          style="background: var(--neo-editorWidget-background); border: 1px solid var(--neo-editorWidget-border);"
        >{content}</pre>
      </details>
    {:else}
      <!-- Error state -->
      <div class="text-center py-12 opacity-60">
        <Radio class="w-12 h-12 mx-auto mb-4 opacity-30" />
        <p>Failed to load device data</p>
      </div>
    {/if}
  </div>
</ScrollArea>

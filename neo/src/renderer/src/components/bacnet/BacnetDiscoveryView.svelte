<script lang="ts">
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import { Checkbox } from '$lib/components/ui/checkbox/index.js'
  import * as Select from '$lib/components/ui/select/index.js'
  import * as Table from '$lib/components/ui/table/index.js'
  import { Radio, Loader2, Square, CheckSquare, X, Plus, Trash2, Check } from '@lucide/svelte'
  import { serverStore, type DiscoveredDeviceWithStatus } from '$lib/stores/server.svelte'

  // Duration options for discovery scan
  const durationOptions = [
    { value: '5', label: '5 seconds' },
    { value: '10', label: '10 seconds' },
    { value: '30', label: '30 seconds' },
    { value: '60', label: '60 seconds' }
  ]

  let selectedDuration = $state('10')

  // Get label for current duration
  function getDurationLabel(value: string): string {
    return durationOptions.find(o => o.value === value)?.label ?? '10 seconds'
  }

  // Derived state from server store
  const discoveryState = $derived(serverStore.discoveryState)
  const discoveredDevices = $derived(serverStore.discoveredDevices)
  const selectedDeviceIds = $derived(serverStore.selectedDeviceIds)
  const isConnected = $derived(serverStore.isConnected)

  // Computed values
  const isScanning = $derived(discoveryState === 'scanning')
  const isStopping = $derived(discoveryState === 'stopping')
  const selectedCount = $derived(selectedDeviceIds.size)
  const selectableDevices = $derived(discoveredDevices.filter(d => !d.alreadyExists))
  const allSelectableSelected = $derived(
    selectableDevices.length > 0 && selectableDevices.every(d => selectedDeviceIds.has(d.device_id))
  )

  async function startScan() {
    const duration = parseInt(selectedDuration)
    await serverStore.startDiscovery({ duration })
  }

  async function stopScan() {
    await serverStore.stopDiscovery()
  }

  function clearResults() {
    serverStore.clearDiscoveredDevices()
  }

  function toggleDevice(deviceId: number) {
    serverStore.toggleDeviceSelection(deviceId)
  }

  function toggleSelectAll() {
    if (allSelectableSelected) {
      serverStore.deselectAllDevices()
    } else {
      serverStore.selectAllDevices()
    }
  }

  async function addSelectedDevices() {
    await serverStore.addSelectedDevices()
  }

  function getStatusBadge(device: DiscoveredDeviceWithStatus) {
    if (device.alreadyExists) {
      return { text: 'Added', class: 'bg-green-500/20 text-green-400' }
    }
    return null
  }
</script>

<div class="flex flex-col h-full">
  <!-- Scan Controls -->
  <div class="p-3 border-b border-[var(--neo-panel-border)]">
    <div class="flex items-center gap-2 mb-2">
      {#if !isScanning}
        <Button
          variant="default"
          size="sm"
          onclick={startScan}
          disabled={!isConnected}
          class="flex-1"
        >
          <Radio class="w-4 h-4 mr-2" />
          Scan Network
        </Button>
      {:else}
        <Button
          variant="destructive"
          size="sm"
          onclick={stopScan}
          disabled={isStopping}
          class="flex-1"
        >
          {#if isStopping}
            <Loader2 class="w-4 h-4 mr-2 animate-spin" />
            Stopping...
          {:else}
            <X class="w-4 h-4 mr-2" />
            Stop
          {/if}
        </Button>
      {/if}

      <Select.Root type="single" bind:value={selectedDuration}>
        <Select.Trigger class="w-32" disabled={isScanning}>
          {getDurationLabel(selectedDuration)}
        </Select.Trigger>
        <Select.Content>
          {#each durationOptions as option}
            <Select.Item value={option.value} label={option.label}>
              {option.label}
            </Select.Item>
          {/each}
        </Select.Content>
      </Select.Root>
    </div>

    <!-- Status line -->
    <div class="text-xs text-muted-foreground">
      {#if isScanning}
        <span class="flex items-center gap-2">
          <Loader2 class="w-3 h-3 animate-spin" />
          Scanning... ({discoveredDevices.length} devices found)
        </span>
      {:else if discoveredDevices.length > 0}
        Found {discoveredDevices.length} device{discoveredDevices.length !== 1 ? 's' : ''}
      {:else if !isConnected}
        Connect to a server to scan for devices
      {:else}
        Click "Scan Network" to discover BACnet devices
      {/if}
    </div>
  </div>

  <!-- Device List -->
  <ScrollArea class="flex-1">
    {#if discoveredDevices.length > 0}
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.Head class="w-10">
              <button
                class="flex items-center justify-center"
                onclick={toggleSelectAll}
                disabled={selectableDevices.length === 0}
              >
                {#if allSelectableSelected && selectableDevices.length > 0}
                  <CheckSquare class="w-4 h-4" />
                {:else}
                  <Square class="w-4 h-4" />
                {/if}
              </button>
            </Table.Head>
            <Table.Head>Device ID</Table.Head>
            <Table.Head>Address</Table.Head>
            <Table.Head>Vendor</Table.Head>
            <Table.Head class="w-16">Status</Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each discoveredDevices as device (device.device_id)}
            {@const isSelected = selectedDeviceIds.has(device.device_id)}
            {@const status = getStatusBadge(device)}
            <Table.Row
              class={device.alreadyExists ? 'opacity-60' : ''}
            >
              <Table.Cell>
                <Checkbox
                  checked={isSelected}
                  onCheckedChange={() => toggleDevice(device.device_id)}
                  disabled={device.alreadyExists}
                />
              </Table.Cell>
              <Table.Cell class="font-mono">{device.device_id}</Table.Cell>
              <Table.Cell class="font-mono text-xs">{device.address}</Table.Cell>
              <Table.Cell class="text-xs">
                {device.vendor_name || `Vendor ${device.vendor_id}`}
              </Table.Cell>
              <Table.Cell>
                {#if status}
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-xs {status.class}">
                    <Check class="w-3 h-3" />
                    {status.text}
                  </span>
                {/if}
              </Table.Cell>
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    {:else if !isScanning}
      <div class="flex flex-col items-center justify-center h-full p-6 text-center opacity-60">
        <Radio class="w-12 h-12 mb-4 opacity-30" />
        <p class="text-sm">No devices discovered yet</p>
        <p class="text-xs mt-1">Start a network scan to find BACnet devices</p>
      </div>
    {/if}
  </ScrollArea>

  <!-- Action Bar -->
  {#if discoveredDevices.length > 0}
    <div class="p-3 border-t border-[var(--neo-panel-border)] flex items-center justify-between">
      <Button
        variant="default"
        size="sm"
        disabled={selectedCount === 0}
        onclick={addSelectedDevices}
      >
        <Plus class="w-4 h-4 mr-2" />
        Add Selected ({selectedCount})
      </Button>
      <Button
        variant="ghost"
        size="sm"
        onclick={clearResults}
        disabled={isScanning}
      >
        <Trash2 class="w-4 h-4 mr-2" />
        Clear
      </Button>
    </div>
  {/if}
</div>

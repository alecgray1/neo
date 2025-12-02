<script lang="ts">
  import * as Dialog from '$lib/components/ui/dialog'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Wifi, WifiOff, Loader2, Server } from '@lucide/svelte'
  import { serverStore } from '$lib/stores/server.svelte'

  interface Props {
    open: boolean
  }

  let { open = $bindable() }: Props = $props()

  let host = $state(serverStore.config.host)
  let port = $state(serverStore.config.port)
  let isConnecting = $state(false)
  let error = $state<string | null>(null)

  // Sync with store config when dialog opens
  $effect(() => {
    if (open) {
      host = serverStore.config.host
      port = serverStore.config.port
      error = null
    }
  })

  async function handleConnect() {
    isConnecting = true
    error = null

    try {
      const success = await serverStore.connect({ host, port })
      if (success) {
        open = false
      } else {
        error = 'Failed to connect to server'
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Connection failed'
    } finally {
      isConnecting = false
    }
  }

  async function handleDisconnect() {
    await serverStore.disconnect()
  }

  const isConnected = $derived(serverStore.connection.state === 'connected')
</script>

<Dialog.Root bind:open>
  <Dialog.Content class="sm:max-w-[425px]">
    <Dialog.Header>
      <Dialog.Title class="flex items-center gap-2">
        <Server class="w-5 h-5" />
        Server Connection
      </Dialog.Title>
      <Dialog.Description>
        Connect to a Neo server to manage devices, schedules, and blueprints.
      </Dialog.Description>
    </Dialog.Header>

    <div class="grid gap-4 py-4">
      <div class="grid grid-cols-4 items-center gap-4">
        <Label class="text-right">Host</Label>
        <Input
          bind:value={host}
          placeholder="localhost"
          class="col-span-3"
          disabled={isConnected || isConnecting}
        />
      </div>
      <div class="grid grid-cols-4 items-center gap-4">
        <Label class="text-right">Port</Label>
        <Input
          type="number"
          bind:value={port}
          placeholder="9600"
          class="col-span-3"
          disabled={isConnected || isConnecting}
        />
      </div>

      {#if error}
        <div class="text-sm text-red-500 px-4">{error}</div>
      {/if}

      <div class="flex items-center gap-2 px-4 py-2 rounded-md bg-muted/50">
        {#if isConnected}
          <Wifi class="w-4 h-4 text-green-500" />
          <span class="text-sm">Connected to {serverStore.config.host}:{serverStore.config.port}</span>
        {:else if isConnecting || serverStore.connection.state === 'connecting'}
          <Loader2 class="w-4 h-4 animate-spin text-yellow-500" />
          <span class="text-sm">Connecting...</span>
        {:else if serverStore.connection.state === 'reconnecting'}
          <Loader2 class="w-4 h-4 animate-spin text-yellow-500" />
          <span class="text-sm">Reconnecting (attempt {serverStore.connection.reconnectAttempts})...</span>
        {:else}
          <WifiOff class="w-4 h-4 text-gray-500" />
          <span class="text-sm">Not connected</span>
        {/if}
      </div>
    </div>

    <Dialog.Footer>
      <Dialog.Close>
        <Button variant="outline">Cancel</Button>
      </Dialog.Close>
      {#if isConnected}
        <Button variant="destructive" onclick={handleDisconnect}>Disconnect</Button>
      {:else}
        <Button onclick={handleConnect} disabled={isConnecting || !host || !port}>
          {#if isConnecting}
            <Loader2 class="w-4 h-4 mr-2 animate-spin" />
          {/if}
          Connect
        </Button>
      {/if}
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

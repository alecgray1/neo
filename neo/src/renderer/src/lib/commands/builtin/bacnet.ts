// BACnet commands

import type { ICommand } from '../types'
import { serverStore } from '$lib/stores/server.svelte'
import { editorStore } from '$lib/stores/editor.svelte'

/** Context arg passed from explorer context menu for BACnet devices */
interface BacnetContextArg {
  resourcePath: string
  resourceName: string
  bacnetDeviceId?: number
}

function isBacnetContextArg(arg: unknown): arg is BacnetContextArg {
  return (
    typeof arg === 'object' &&
    arg !== null &&
    'bacnetDeviceId' in arg &&
    typeof (arg as BacnetContextArg).bacnetDeviceId === 'number'
  )
}

/**
 * BACnet commands for device management
 */
export const bacnetCommands: ICommand[] = [
  {
    id: 'neo.bacnet.removeDevice',
    title: 'Remove Device',
    category: 'BACnet',
    handler: async (_accessor, arg) => {
      if (!isBacnetContextArg(arg)) {
        console.warn('Remove device called without valid BACnet context')
        return
      }

      const deviceId = arg.bacnetDeviceId
      if (deviceId === undefined) return

      try {
        // Close any open editor for this device
        const uri = `neo://bacnet/devices/${deviceId}`
        editorStore.closeTabByUri(uri)

        // Remove the device
        await serverStore.removeBacnetDevice(deviceId)
        console.log('Removed BACnet device:', deviceId)
      } catch (e) {
        console.error('Failed to remove BACnet device:', e)
      }
    }
  }
]

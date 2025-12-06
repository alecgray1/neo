import type { ConnectionState, ServerConfig, ChangeEvent, DiscoveredDevice, DiscoveryOptions } from '../../../../preload/index.d'
import { SvelteMap, SvelteSet } from 'svelte/reactivity'
import { documentStore } from './documents.svelte'

export type DiscoveryState = 'idle' | 'scanning' | 'stopping'

// Re-export for convenience
export type { DiscoveredDevice, DiscoveryOptions } from '../../../../preload/index.d'

export interface ServerState {
  connection: ConnectionState
  config: ServerConfig
}

export interface Project {
  name: string
  version: string
  description?: string
}

export interface Device {
  id: string
  name: string
  device_type: string
  address?: string
  enabled: boolean
  tags: string[]
  points: Record<string, unknown>
  metadata: Record<string, unknown>
}

export interface Schedule {
  id: string
  name: string
  enabled: boolean
  schedule_type: string
  timezone: string
  entries: unknown[]
  exceptions: unknown[]
}

export interface BacnetDevice {
  device_id: number
  address: string
  max_apdu: number
  vendor_id: number
  segmentation: string
}

export interface BacnetObject {
  object_type: string
  instance: number
}

export interface BacnetObjectValue {
  device_id: number
  object_type: string
  instance: number
  property: string
  value: unknown
  timestamp: number
}

export interface Blueprint {
  id: string
  name: string
  description?: string
  nodes: unknown[]
  connections: unknown[]
}

const defaultConnectionState: ConnectionState = {
  state: 'disconnected',
  reconnectAttempts: 0
}

const defaultConfig: ServerConfig = {
  host: 'localhost',
  port: 9600
}

// Extended discovered device with alreadyExists flag
export interface DiscoveredDeviceWithStatus extends DiscoveredDevice {
  alreadyExists: boolean
}

function createServerStore() {
  let connection = $state<ConnectionState>({ ...defaultConnectionState })
  let config = $state<ServerConfig>({ ...defaultConfig })
  let project = $state<Project | null>(null)
  let devices = $state<Device[]>([])
  let blueprints = $state<Blueprint[]>([])
  let schedules = $state<Schedule[]>([])
  let bacnetDevices = $state<BacnetDevice[]>([])
  const bacnetDeviceObjects = new SvelteMap<number, BacnetObject[]>()
  // Key is "deviceId:objectType:instance" e.g., "201:analog-input:1"
  const bacnetObjectValues = new SvelteMap<string, BacnetObjectValue>()
  let error = $state<string | null>(null)
  let initialized = $state(false)

  // BACnet discovery state
  let discoveryState = $state<DiscoveryState>('idle')
  let discoveredDevices = $state<DiscoveredDeviceWithStatus[]>([])
  const selectedDeviceIds = new SvelteSet<number>()

  // Cleanup functions for event listeners
  let cleanupStateListener: (() => void) | null = null
  let cleanupChangeListener: (() => void) | null = null
  let cleanupBacnetDiscoveryStarted: (() => void) | null = null
  let cleanupBacnetDeviceFound: (() => void) | null = null
  let cleanupBacnetDiscoveryComplete: (() => void) | null = null
  let cleanupBacnetDeviceAdded: (() => void) | null = null
  let cleanupBacnetDeviceRemoved: (() => void) | null = null

  return {
    // Getters
    get connection() {
      return connection
    },
    get config() {
      return config
    },
    get project() {
      return project
    },
    get devices() {
      return devices
    },
    get blueprints() {
      return blueprints
    },
    get schedules() {
      return schedules
    },
    get bacnetDevices() {
      return bacnetDevices
    },
    get bacnetDeviceObjects() {
      return bacnetDeviceObjects
    },
    getBacnetDeviceObjects(deviceId: number): BacnetObject[] {
      return bacnetDeviceObjects.get(deviceId) ?? []
    },
    get bacnetObjectValues() {
      return bacnetObjectValues
    },
    getBacnetObjectValue(deviceId: number, objectType: string, instance: number): BacnetObjectValue | undefined {
      const key = `${deviceId}:${objectType}:${instance}`
      return bacnetObjectValues.get(key)
    },
    get error() {
      return error
    },
    get isConnected() {
      return connection.state === 'connected'
    },

    // Discovery state getters
    get discoveryState() {
      return discoveryState
    },
    get discoveredDevices() {
      return discoveredDevices
    },
    get selectedDeviceIds() {
      return selectedDeviceIds
    },
    get isScanning() {
      return discoveryState === 'scanning'
    },

    // Initialize the store - call this on app startup
    async init() {
      if (initialized) return

      // Load saved config
      try {
        config = await window.serverAPI.getConfig()
        connection = await window.serverAPI.getState()
      } catch (e) {
        console.error('Failed to load server config:', e)
      }

      // Listen for connection state changes
      cleanupStateListener = window.serverAPI.onStateChanged((state) => {
        connection = state
        if (state.state === 'disconnected') {
          // Clear data on disconnect
          project = null
          devices = []
          blueprints = []
          schedules = []
          bacnetDevices = []
          bacnetDeviceObjects.clear()
          bacnetObjectValues.clear()
        }
      })

      // Listen for data changes
      cleanupChangeListener = window.serverAPI.onChange((event) => {
        this.handleChange(event)
      })

      // Listen for BACnet discovery events
      cleanupBacnetDiscoveryStarted = window.bacnetAPI.onDiscoveryStarted(() => {
        discoveryState = 'scanning'
      })

      cleanupBacnetDeviceFound = window.bacnetAPI.onDeviceFound((device, alreadyExists) => {
        // Add to discovered devices if not already present
        const exists = discoveredDevices.some(d => d.device_id === device.device_id)
        if (!exists) {
          discoveredDevices = [...discoveredDevices, { ...device, alreadyExists }]
        }
      })

      cleanupBacnetDiscoveryComplete = window.bacnetAPI.onDiscoveryComplete(() => {
        discoveryState = 'idle'
      })

      cleanupBacnetDeviceAdded = window.bacnetAPI.onDeviceAdded((deviceId, _entityId) => {
        // Update the discovered device to show it's now in the system
        discoveredDevices = discoveredDevices.map(d =>
          d.device_id === deviceId ? { ...d, alreadyExists: true } : d
        )
        // Remove from selection
        selectedDeviceIds.delete(deviceId)
        // Refresh the bacnet devices list
        this.fetchBacnetDevices()
      })

      cleanupBacnetDeviceRemoved = window.bacnetAPI.onDeviceRemoved((deviceId) => {
        // Update the discovered device to show it's no longer in the system
        discoveredDevices = discoveredDevices.map(d =>
          d.device_id === deviceId ? { ...d, alreadyExists: false } : d
        )
        // Refresh the bacnet devices list
        this.fetchBacnetDevices()
      })

      initialized = true
    },

    // Cleanup listeners
    destroy() {
      cleanupStateListener?.()
      cleanupChangeListener?.()
      cleanupBacnetDiscoveryStarted?.()
      cleanupBacnetDeviceFound?.()
      cleanupBacnetDiscoveryComplete?.()
      cleanupBacnetDeviceAdded?.()
      cleanupBacnetDeviceRemoved?.()
      initialized = false
    },

    // Connection methods
    async connect(newConfig?: Partial<ServerConfig>) {
      error = null
      try {
        const success = await window.serverAPI.connect(newConfig)
        if (success) {
          // Update local config
          config = await window.serverAPI.getConfig()
          // Fetch initial data
          await this.fetchAll()
          // Subscribe to all changes
          await window.serverAPI.subscribe(['/devices/**', '/blueprints/**', '/schedules/**', '/bacnet/devices/**', '/project'])
        }
        return success
      } catch (e) {
        error = e instanceof Error ? e.message : 'Connection failed'
        return false
      }
    },

    async disconnect() {
      await window.serverAPI.disconnect()
      project = null
      devices = []
      blueprints = []
      schedules = []
      bacnetDevices = []
      bacnetDeviceObjects.clear()
      bacnetObjectValues.clear()
    },

    async setConfig(newConfig: Partial<ServerConfig>) {
      await window.serverAPI.setConfig(newConfig)
      config = { ...config, ...newConfig }
    },

    // Data fetching
    async fetchAll() {
      await Promise.all([this.fetchProject(), this.fetchDevices(), this.fetchBlueprints(), this.fetchSchedules(), this.fetchBacnetDevices()])
    },

    async fetchProject() {
      try {
        project = (await window.projectAPI.getProject()) as Project
      } catch (e) {
        console.error('Failed to fetch project:', e)
      }
    },

    async fetchDevices() {
      try {
        devices = (await window.projectAPI.getDevices()) as Device[]
      } catch (e) {
        console.error('Failed to fetch devices:', e)
      }
    },

    async fetchBlueprints() {
      try {
        blueprints = (await window.projectAPI.getBlueprints()) as Blueprint[]
      } catch (e) {
        console.error('Failed to fetch blueprints:', e)
      }
    },

    async fetchSchedules() {
      try {
        schedules = (await window.projectAPI.getSchedules()) as Schedule[]
      } catch (e) {
        console.error('Failed to fetch schedules:', e)
      }
    },

    async fetchBacnetDevices() {
      try {
        bacnetDevices = (await window.serverAPI.request('/bacnet/devices')) as BacnetDevice[]
      } catch (e) {
        console.error('Failed to fetch BACnet devices:', e)
        bacnetDevices = []
      }
    },

    // Handle real-time changes
    handleChange(event: ChangeEvent) {
      const { path, changeType, data } = event

      if (path === '/project') {
        if (changeType === 'updated' && data) {
          project = data as Project
        }
      } else if (path.startsWith('/devices/')) {
        const deviceId = path.split('/')[2]
        if (changeType === 'deleted') {
          devices = devices.filter((d) => d.id !== deviceId)
        } else if (changeType === 'created' || changeType === 'updated') {
          const device = data as Device
          const index = devices.findIndex((d) => d.id === deviceId)
          if (index >= 0) {
            devices[index] = device
          } else {
            devices = [...devices, device]
          }
        }
      } else if (path.startsWith('/blueprints/')) {
        const blueprintId = path.split('/')[2]
        console.log('handleChange: blueprint change received:', blueprintId, changeType)
        if (changeType === 'deleted') {
          blueprints = blueprints.filter((b) => b.id !== blueprintId)
        } else if (changeType === 'created' || changeType === 'updated') {
          const blueprint = data as Blueprint
          const index = blueprints.findIndex((b) => b.id === blueprintId)
          if (index >= 0) {
            blueprints[index] = blueprint
          } else {
            blueprints = [...blueprints, blueprint]
          }
          // Also update any open document for this blueprint
          console.log('handleChange: calling updateFromServer for', blueprintId)
          documentStore.updateFromServer(`neo://blueprints/${blueprintId}`, data)
        }
      } else if (path.startsWith('/schedules/')) {
        const scheduleId = path.split('/')[2]
        if (changeType === 'deleted') {
          schedules = schedules.filter((s) => s.id !== scheduleId)
        } else if (changeType === 'created' || changeType === 'updated') {
          const schedule = data as Schedule
          const index = schedules.findIndex((s) => s.id === scheduleId)
          if (index >= 0) {
            schedules[index] = schedule
          } else {
            schedules = [...schedules, schedule]
          }
        }
      } else if (path.startsWith('/bacnet/devices/')) {
        const parts = path.split('/')
        const deviceId = parseInt(parts[3], 10)

        // Check if this is an object list update: /bacnet/devices/{id}/objects
        if (parts[4] === 'objects' && parts.length === 5) {
          if (changeType === 'updated' && data) {
            const objectData = data as { device_id: number; objects: BacnetObject[] }
            bacnetDeviceObjects.set(objectData.device_id, objectData.objects)
          }
        }
        // Check if this is an object value update: /bacnet/devices/{id}/objects/{type}/{instance}/{property}
        else if (parts[4] === 'objects' && parts.length >= 8) {
          if (changeType === 'updated' && data) {
            const valueData = data as BacnetObjectValue
            const key = `${valueData.device_id}:${valueData.object_type}:${valueData.instance}`
            bacnetObjectValues.set(key, valueData)
          }
        } else {
          // This is a device update
          if (changeType === 'deleted') {
            bacnetDevices = bacnetDevices.filter((d) => d.device_id !== deviceId)
          } else if (changeType === 'created' || changeType === 'updated') {
            const device = data as BacnetDevice
            const index = bacnetDevices.findIndex((d) => d.device_id === deviceId)
            if (index >= 0) {
              bacnetDevices[index] = device
            } else {
              bacnetDevices = [...bacnetDevices, device]
            }
          }
        }
      }
    },

    // Device operations
    getDevice(id: string): Device | undefined {
      return devices.find((d) => d.id === id)
    },

    // Blueprint operations
    getBlueprint(id: string): Blueprint | undefined {
      return blueprints.find((b) => b.id === id)
    },

    // Schedule operations
    getSchedule(id: string): Schedule | undefined {
      return schedules.find((s) => s.id === id)
    },

    // BACnet operations
    async readBacnetDeviceObjects(deviceId: number): Promise<void> {
      try {
        await window.serverAPI.send({
          type: 'bacnet:readObjects',
          id: `bacnet-read-objects-${deviceId}-${Date.now()}`,
          deviceId
        })
      } catch (e) {
        console.error('Failed to request BACnet object list:', e)
        throw e
      }
    },

    // Request a single property read (for manual refresh)
    async readBacnetProperty(
      deviceId: number,
      objectType: string,
      instance: number,
      property: string = 'present-value'
    ): Promise<void> {
      try {
        await window.serverAPI.send({
          type: 'bacnet:readProperty',
          id: `bacnet-read-${deviceId}-${objectType}-${instance}-${Date.now()}`,
          deviceId,
          objectType,
          instance,
          property
        })
      } catch (e) {
        console.error('Failed to request BACnet property read:', e)
        throw e
      }
    },

    // BACnet Discovery operations
    async startDiscovery(options?: DiscoveryOptions): Promise<void> {
      // Clear previous results and selection
      discoveredDevices = []
      selectedDeviceIds.clear()
      discoveryState = 'scanning'

      try {
        await window.bacnetAPI.startDiscovery(options)
      } catch (e) {
        console.error('Failed to start BACnet discovery:', e)
        discoveryState = 'idle'
        throw e
      }
    },

    async stopDiscovery(): Promise<void> {
      if (discoveryState !== 'scanning') return

      discoveryState = 'stopping'
      try {
        await window.bacnetAPI.stopDiscovery()
      } catch (e) {
        console.error('Failed to stop BACnet discovery:', e)
      }
      discoveryState = 'idle'
    },

    clearDiscoveredDevices(): void {
      discoveredDevices = []
      selectedDeviceIds.clear()
    },

    // Device selection for batch add
    toggleDeviceSelection(deviceId: number): void {
      if (selectedDeviceIds.has(deviceId)) {
        selectedDeviceIds.delete(deviceId)
      } else {
        selectedDeviceIds.add(deviceId)
      }
    },

    selectAllDevices(): void {
      // Select only devices not already in the system
      discoveredDevices.forEach(d => {
        if (!d.alreadyExists) {
          selectedDeviceIds.add(d.device_id)
        }
      })
    },

    deselectAllDevices(): void {
      selectedDeviceIds.clear()
    },

    // Add a single device to the system
    async addBacnetDevice(device: DiscoveredDevice): Promise<void> {
      try {
        // Convert to plain object for IPC (Svelte proxy objects can't be cloned)
        const plainDevice: DiscoveredDevice = {
          device_id: device.device_id,
          address: device.address,
          max_apdu: device.max_apdu,
          vendor_id: device.vendor_id,
          segmentation: device.segmentation,
          vendor_name: device.vendor_name,
          model_name: device.model_name,
          object_name: device.object_name
        }
        await window.bacnetAPI.addDevice(plainDevice)
      } catch (e) {
        console.error('Failed to add BACnet device:', e)
        throw e
      }
    },

    // Add all selected devices
    async addSelectedDevices(): Promise<void> {
      const devicesToAdd = discoveredDevices.filter(
        d => selectedDeviceIds.has(d.device_id) && !d.alreadyExists
      )

      for (const device of devicesToAdd) {
        try {
          // Convert to plain object for IPC (Svelte proxy objects can't be cloned)
          const plainDevice: DiscoveredDevice = {
            device_id: device.device_id,
            address: device.address,
            max_apdu: device.max_apdu,
            vendor_id: device.vendor_id,
            segmentation: device.segmentation,
            vendor_name: device.vendor_name,
            model_name: device.model_name,
            object_name: device.object_name
          }
          await window.bacnetAPI.addDevice(plainDevice)
        } catch (e) {
          console.error(`Failed to add device ${device.device_id}:`, e)
        }
      }
    },

    // Remove a device from the system
    async removeBacnetDevice(deviceId: number): Promise<void> {
      try {
        await window.bacnetAPI.removeDevice(deviceId)
      } catch (e) {
        console.error('Failed to remove BACnet device:', e)
        throw e
      }
    }
  }
}

export const serverStore = createServerStore()

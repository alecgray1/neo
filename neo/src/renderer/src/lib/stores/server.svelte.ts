import type { ConnectionState, ServerConfig, ChangeEvent } from '../../../../preload/index.d'
import { documentStore } from './documents.svelte'

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

function createServerStore() {
  let connection = $state<ConnectionState>({ ...defaultConnectionState })
  let config = $state<ServerConfig>({ ...defaultConfig })
  let project = $state<Project | null>(null)
  let devices = $state<Device[]>([])
  let blueprints = $state<Blueprint[]>([])
  let schedules = $state<Schedule[]>([])
  let bacnetDevices = $state<BacnetDevice[]>([])
  let bacnetDeviceObjects = $state<Map<number, BacnetObject[]>>(new Map())
  let error = $state<string | null>(null)
  let initialized = $state(false)

  // Cleanup functions for event listeners
  let cleanupStateListener: (() => void) | null = null
  let cleanupChangeListener: (() => void) | null = null

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
    get error() {
      return error
    },
    get isConnected() {
      return connection.state === 'connected'
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
          bacnetDeviceObjects = new Map()
        }
      })

      // Listen for data changes
      cleanupChangeListener = window.serverAPI.onChange((event) => {
        this.handleChange(event)
      })

      initialized = true
    },

    // Cleanup listeners
    destroy() {
      cleanupStateListener?.()
      cleanupChangeListener?.()
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
      bacnetDeviceObjects = new Map()
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

        // Check if this is an object list update
        if (parts[4] === 'objects') {
          if (changeType === 'updated' && data) {
            const objectData = data as { device_id: number; objects: BacnetObject[] }
            const newMap = new Map(bacnetDeviceObjects)
            newMap.set(objectData.device_id, objectData.objects)
            bacnetDeviceObjects = newMap
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
    }
  }
}

export const serverStore = createServerStore()

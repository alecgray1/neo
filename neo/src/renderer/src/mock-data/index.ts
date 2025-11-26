// Mock data for Neo Building Automation System

// Device configurations
import vav101 from './devices/vav-101.json'
import ahu001 from './devices/ahu-001.json'
import boiler001 from './devices/boiler-001.json'

// Point data
import zoneTemps from './points/zone-temps.json'
import setpoints from './points/setpoints.json'
import alarms from './points/alarms.json'

// Schedules
import occupancy from './schedules/occupancy.json'
import holidays from './schedules/holidays.json'

// Configuration
import network from './config/network.json'
import system from './config/system.json'

export const mockData = {
  devices: {
    'vav-101': vav101,
    'ahu-001': ahu001,
    'boiler-001': boiler001
  },
  points: {
    'zone-temps': zoneTemps,
    setpoints: setpoints,
    alarms: alarms
  },
  schedules: {
    occupancy: occupancy,
    holidays: holidays
  },
  config: {
    network: network,
    system: system
  }
}

// File metadata for the file explorer
export interface MockFile {
  uri: string
  name: string
  path: string
  type: 'json'
  category: 'devices' | 'points' | 'schedules' | 'config'
}

export const mockFiles: MockFile[] = [
  // Devices
  { uri: 'mock://devices/vav-101.json', name: 'vav-101.json', path: 'devices/vav-101.json', type: 'json', category: 'devices' },
  { uri: 'mock://devices/ahu-001.json', name: 'ahu-001.json', path: 'devices/ahu-001.json', type: 'json', category: 'devices' },
  { uri: 'mock://devices/boiler-001.json', name: 'boiler-001.json', path: 'devices/boiler-001.json', type: 'json', category: 'devices' },
  // Points
  { uri: 'mock://points/zone-temps.json', name: 'zone-temps.json', path: 'points/zone-temps.json', type: 'json', category: 'points' },
  { uri: 'mock://points/setpoints.json', name: 'setpoints.json', path: 'points/setpoints.json', type: 'json', category: 'points' },
  { uri: 'mock://points/alarms.json', name: 'alarms.json', path: 'points/alarms.json', type: 'json', category: 'points' },
  // Schedules
  { uri: 'mock://schedules/occupancy.json', name: 'occupancy.json', path: 'schedules/occupancy.json', type: 'json', category: 'schedules' },
  { uri: 'mock://schedules/holidays.json', name: 'holidays.json', path: 'schedules/holidays.json', type: 'json', category: 'schedules' },
  // Config
  { uri: 'mock://config/network.json', name: 'network.json', path: 'config/network.json', type: 'json', category: 'config' },
  { uri: 'mock://config/system.json', name: 'system.json', path: 'config/system.json', type: 'json', category: 'config' }
]

// Helper to get content by URI
export function getMockContent(uri: string): unknown | null {
  const path = uri.replace('mock://', '')
  const [category, filename] = path.split('/')
  const key = filename.replace('.json', '')

  const categoryData = mockData[category as keyof typeof mockData]
  if (!categoryData) return null

  return categoryData[key as keyof typeof categoryData] ?? null
}

// Helper to get content as formatted JSON string
export function getMockContentString(uri: string): string | null {
  const content = getMockContent(uri)
  if (!content) return null
  return JSON.stringify(content, null, 2)
}

export default mockData

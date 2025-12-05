# Project Neo

This is a new BMS system, built on web technology

### Key Features

* Event Based
* Expression Visual Scripting Language
* Modern JS plugin system
  * Hot Module Reload and Build system using vite

## Multiplatform

### Electron App

Electron app that will work on both browser and desktop using DI

Buffer/Tabs/Pane workflow

Plugin system based on js and webviews

## Better Code Composition

### Devices

A **Device** is a typed data structure representing physical equipment. Devices are protocol-agnostic - they define *what* the equipment is, not *how* it communicates.

```typescript
interface Device {
  id: string;
  type: string;           // e.g., "VAV", "AHU", "Chiller"
  name: string;
  location?: string;
  tags: string[];
  points: Record<string, Point>;
  metadata: Record<string, any>;
}
```

### Points

A **Point** is a data value with metadata. Points belong to devices and can be bound to protocol sources.

```typescript
interface Point {
  id: string;
  name: string;
  type: "analog" | "binary" | "multistate" | "string";
  direction: "input" | "output" | "value";  // sensor, command, calculated
  unit?: string;
  value: any;
  quality: "good" | "stale" | "bad";
  timestamp: number;
  binding?: ProtocolBinding;
}
```

### Protocol Bindings

Points can be bound to various protocol sources:

```typescript
type ProtocolBinding =
  | BACnetBinding
  | ModbusBinding
  | MQTTBinding
  | VirtualBinding;

interface BACnetBinding {
  protocol: "bacnet";
  deviceInstance: number;
  objectType: string;      // "analogInput", "analogOutput", etc.
  objectInstance: number;
  property?: string;       // default: "presentValue"
}
```

---

## BACnet Device Modeling

How users go from raw BACnet discovery to typed device structs.

### Option 1: Manual Mapping

User explicitly maps discovered BACnet objects to device points.

```typescript
// User discovers BACnet device 12345 with objects:
// - AI:1 (Zone Temp)
// - AI:2 (Discharge Temp)
// - AO:1 (Damper Command)
// - BI:1 (Occupancy)

const vav = Device.create({
  type: "VAV",
  name: "VAV-101",
  points: {
    zoneTemp: {
      type: "analog",
      direction: "input",
      unit: "degF",
      binding: { protocol: "bacnet", deviceInstance: 12345, objectType: "analogInput", objectInstance: 1 }
    },
    dischargeTemp: {
      type: "analog",
      direction: "input",
      unit: "degF",
      binding: { protocol: "bacnet", deviceInstance: 12345, objectType: "analogInput", objectInstance: 2 }
    },
    damperCmd: {
      type: "analog",
      direction: "output",
      unit: "%",
      binding: { protocol: "bacnet", deviceInstance: 12345, objectType: "analogOutput", objectInstance: 1 }
    },
    occupied: {
      type: "binary",
      direction: "input",
      binding: { protocol: "bacnet", deviceInstance: 12345, objectType: "binaryInput", objectInstance: 1 }
    }
  }
});
```

**Pros:** Full control, works with any device
**Cons:** Tedious, error-prone, doesn't scale

---

### Option 2: Device Templates

Pre-defined templates that users apply to discovered devices.

```typescript
// Template definition
const VAVTemplate = {
  type: "VAV",
  points: {
    zoneTemp:      { type: "analog", direction: "input", unit: "degF", required: true },
    zoneTempSp:    { type: "analog", direction: "value", unit: "degF", required: true },
    dischargeTemp: { type: "analog", direction: "input", unit: "degF", required: false },
    damperCmd:     { type: "analog", direction: "output", unit: "%", required: true },
    damperFbk:     { type: "analog", direction: "input", unit: "%", required: false },
    occupied:      { type: "binary", direction: "input", required: false },
    heatingCmd:    { type: "analog", direction: "output", unit: "%", required: false },
  }
};

// User applies template and maps points
const vav = Device.fromTemplate(VAVTemplate, {
  name: "VAV-101",
  bindings: {
    zoneTemp:   { deviceInstance: 12345, objectType: "AI", objectInstance: 1 },
    damperCmd:  { deviceInstance: 12345, objectType: "AO", objectInstance: 1 },
    // ... user maps each point
  }
});
```

**Pros:** Consistent structure, validates required points, reusable
**Cons:** Still requires per-point mapping

---

### Option 3: Manufacturer Profiles

Pre-built mappings for known controllers. Auto-map based on device model.

```typescript
// Profile for Tridium JACE 8000 VAV
const profiles = {
  "Tridium/JACE8000/VAV": {
    match: (device) => device.modelName?.includes("JACE") && device.objectList.length < 20,
    template: "VAV",
    autoMap: {
      zoneTemp:      { objectType: "AI", objectInstance: 1 },
      zoneTempSp:    { objectType: "AV", objectInstance: 1 },
      damperCmd:     { objectType: "AO", objectInstance: 1 },
      occupied:      { objectType: "BI", objectInstance: 1 },
    }
  },
  "JCI/FEC/VAV": {
    match: (device) => device.vendorId === 5,  // Johnson Controls
    template: "VAV",
    autoMap: {
      zoneTemp:      { objectType: "AI", objectInstance: 0 },
      zoneTempSp:    { objectType: "AV", objectInstance: 0 },
      // JCI uses 0-based indexing
    }
  }
};

// Auto-create device from discovered BACnet device
const vav = Device.fromBACnet(discoveredDevice, { profile: "auto" });
// System matches profile, applies template, auto-maps points
```

**Pros:** Near-zero config for known devices, scalable
**Cons:** Need to build/maintain profile library, doesn't work for unknown devices

---

### Option 4: Semantic Discovery

Use BACnet object names/descriptions to auto-map via semantic matching.

```typescript
// Discovered BACnet objects:
// AI:1 "ZN-T" (Zone Temperature)
// AI:2 "DA-T" (Discharge Air Temp)
// AO:1 "DPR-O" (Damper Output)

const semanticRules = [
  { pattern: /zone.*temp|zn-t/i, maps_to: "zoneTemp" },
  { pattern: /discharge.*temp|da-t/i, maps_to: "dischargeTemp" },
  { pattern: /damper|dpr/i, maps_to: "damperCmd" },
  { pattern: /occupy|occ/i, maps_to: "occupied" },
];

// Auto-discover and map
const vav = Device.fromBACnet(discoveredDevice, {
  template: "VAV",
  autoMap: "semantic",  // Use object names to guess mappings
});
// System: "AI:1 'ZN-T' looks like zoneTemp, mapping..."
```

**Pros:** Works with unknown devices if they have good naming
**Cons:** Depends on naming conventions, may need manual corrections

---

### Option 5: Hybrid Approach (Recommended)

Combine all approaches with a priority chain:

```typescript
const deviceFactory = {
  async createFromBACnet(bacnetDevice: DiscoveredDevice): Promise<Device> {
    // 1. Try manufacturer profile first
    const profile = profiles.match(bacnetDevice);
    if (profile) {
      return Device.fromProfile(bacnetDevice, profile);
    }

    // 2. Try semantic auto-discovery
    const template = templates.guessType(bacnetDevice);
    if (template) {
      const autoMapped = semanticMapper.map(bacnetDevice, template);
      if (autoMapped.confidence > 0.8) {
        return Device.fromAutoMap(bacnetDevice, template, autoMapped);
      }
    }

    // 3. Fall back to manual - present UI for user to map
    return Device.createManual(bacnetDevice, {
      suggestedTemplate: template,
      suggestedMappings: autoMapped,
    });
  }
};
```

**Workflow:**

1. Discover BACnet devices on network
2. For each device:
   * Check if we have a manufacturer profile -> auto-create
   * Try semantic matching -> suggest mappings
   * Show UI for user to confirm/edit/manual map
3. User validates, device is created
4. Mappings can be saved as new profiles for future use

---

## Blueprints

A **Blueprint** is event-driven logic that can be attached to devices.

```typescript
interface Blueprint {
  id: string;
  name: string;
  // What triggers this blueprint
  attachment: AttachmentRule;
  // The logic (nodes + connections)
  nodes: Node[];
  connections: Connection[];
}
```

### Blueprint Execution

```typescript
// Blueprint attached to all VAVs
const comfortBlueprint = {
  name: "Comfort Monitor",
  attachment: { type: "type", deviceType: "VAV" },

  // Triggers
  triggers: [
    { event: "point.change", point: "zoneTemp" },
    { event: "schedule", cron: "*/5 * * * *" },
  ],

  // Logic nodes
  nodes: [
    { id: "1", type: "compare", config: { op: ">" } },
    { id: "2", type: "alarm", config: { priority: "low" } },
  ],
  connections: [
    { from: "device.zoneTemp", to: "1.a" },
    { from: "device.zoneTempSp", to: "1.b" },
    { from: "1.result", to: "2.condition" },
  ]
};

// When VAV-101.zoneTemp changes:
// 1. System finds all blueprints attached to VAV-101
// 2. Executes blueprint with device context bound
// 3. Blueprint can read device.zoneTemp, device.zoneTempSp, etc.
```

---

## Component System

Inspired by Unreal Engine's component architecture. Components are **reusable behaviors** that attach to device types.

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  DEVICE TYPE (class)          INSTANCE (runtime)           │
│  ┌─────────────────┐          ┌─────────────────┐          │
│  │ VAV             │          │ vav-101         │          │
│  │                 │ ──────▶  │                 │          │
│  │ points: {...}   │  create  │ zoneTemp: 72.5  │          │
│  │ components: []  │          │ damperCmd: 45   │          │
│  │ triggers: {...} │          │                 │          │
│  └─────────────────┘          └─────────────────┘          │
│         │                                                   │
│         │ composes                                          │
│         ▼                                                   │
│  ┌─────────────────┐                                       │
│  │ COMPONENTS      │                                       │
│  │ (reusable)      │                                       │
│  │                 │                                       │
│  │ ComfortMonitor  │                                       │
│  │ DamperControl   │                                       │
│  │ RuntimeTracker  │                                       │
│  └─────────────────┘                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Defining Components

Components are self-contained, reusable behaviors:

```typescript
defineComponent({
  name: "ComponentName",

  // What points the device MUST have
  requires: {
    pointName: "type:direction",
  },

  // What points are optional
  optional: {
    pointName: "type:direction",
  },

  // Configurable settings (overridable per-instance)
  config: {
    setting: defaultValue,
  },

  // Runtime state (per-instance)
  state: {
    variable: initialValue,
  },

  // Event handlers
  triggers: {
    "pointName.change": function(device) { },
    "schedule.event": function(device) { },
    "interval.5m": function(device) { },
  },

  // Lifecycle
  onAttach(device) { },
  onDetach(device) { },
});
```

**Example - DamperControl:**

```typescript
export const DamperControl = defineComponent({
  name: "DamperControl",

  requires: {
    damperCmd: "analog:output",
    damperFbk: "analog:input",
  },

  config: {
    tolerance: 10,      // % difference to alarm
    responseTime: 30,   // seconds to wait
  },

  state: {
    checkTimer: null,
    inAlarm: false,
  },

  triggers: {
    "damperCmd.change": function(device) {
      clearTimeout(this.state.checkTimer);

      this.state.checkTimer = setTimeout(() => {
        const diff = Math.abs(device.damperCmd - device.damperFbk);

        if (diff > this.config.tolerance && !this.state.inAlarm) {
          this.state.inAlarm = true;
          Neo.alarm(device, "Damper stuck");
        } else if (diff <= this.config.tolerance) {
          this.state.inAlarm = false;
        }
      }, this.config.responseTime * 1000);
    }
  }
});
```

### Defining Device Types

Device types compose points, components, and type-specific logic:

```typescript
defineDeviceType("TypeName", {
  // Point definitions
  points: {
    pointName: {
      type: "analog" | "binary" | "multistate",
      direction: "input" | "output" | "value",
      unit?: string,
      optional?: boolean,
    },
  },

  // Attached components
  components: [
    Component1,
    Component2,
  ],

  // Default config for components
  defaults: {
    ComponentName: { setting: value },
  },

  // Type-specific logic (not reusable)
  triggers: {
    "pointName.change": function(device) { },
  },
});
```

**Example - VAV Device Type:**

```typescript
export const VAV = defineDeviceType("VAV", {
  points: {
    zoneTemp:     { type: "analog", direction: "input", unit: "°F" },
    zoneTempSp:   { type: "analog", direction: "value", unit: "°F" },
    damperCmd:    { type: "analog", direction: "output", unit: "%" },
    damperFbk:    { type: "analog", direction: "input", unit: "%" },
    heatingCmd:   { type: "analog", direction: "output", unit: "%", optional: true },
    occupied:     { type: "binary", direction: "value" },
  },

  components: [
    ComfortMonitor,
    DamperControl,
    OccupancySchedule,
  ],

  defaults: {
    ComfortMonitor: { deadband: 2 },
    DamperControl: { tolerance: 10 },
    OccupancySchedule: { schedule: "weekdays 6:00-18:00" },
  },

  triggers: {
    // VAV-specific proportional control
    "zoneTemp.change": function(device) {
      const error = device.zoneTemp - device.zoneTempSp;
      device.damperCmd = clamp(20 + error * 10, 20, 100);
    }
  }
});
```

### Creating Instances

```typescript
// Create with defaults - components auto-attach
const vav101 = devices.create("VAV", {
  name: "VAV-101",
  bindings: { /* BACnet mappings */ },
});

// Create with config overrides
const vav102 = devices.create("VAV", {
  name: "VAV-102 Conference",
  config: {
    ComfortMonitor: { deadband: 1 },  // Tighter control
  },
});

// Create with extra component
const vav103 = devices.create("VAV", {
  name: "VAV-103 Server Room",
  addComponents: [CriticalTempShutdown],
});
```

### Component Data Access

Components access data from multiple sources:

```typescript
const MyComponent = defineComponent({
  requires: { zoneTemp: "analog:input" },
  config: { threshold: 75 },
  state: { alarmActive: false },

  triggers: {
    "zoneTemp.change": function(device) {
      // Access device's points
      const temp = device.zoneTemp;

      // Access component's config
      const limit = this.config.threshold;

      // Access/modify component's state
      if (temp > limit && !this.state.alarmActive) {
        this.state.alarmActive = true;
        Neo.alarm(device, "Over threshold");
      }

      // Access optional points (check first)
      if (device.occupied !== undefined && !device.occupied) {
        return; // Skip if unoccupied
      }

      // Find sibling component
      const scheduler = device.getComponent("OccupancySchedule");
      if (scheduler) {
        // Use scheduler data
      }
    }
  }
});
```

### Execution Flow

```
BACnet COV: zoneTemp changed to 76°F on device 12345
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Binding Manager                                            │
│  → Update vav-101.zoneTemp = 76                             │
│  → Emit "point.change" event                                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Event Router                                               │
│  → Find all triggers for "zoneTemp.change" on vav-101       │
│                                                             │
│  Matches:                                                   │
│  1. ComfortMonitor (component)                              │
│  2. VAV type logic                                          │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌──────────────────────────┐   ┌──────────────────────────┐
│ ComfortMonitor.trigger() │   │ VAV.trigger()            │
│                          │   │                          │
│ → Check if out of band   │   │ → Calculate new damper   │
│ → Start alarm timer      │   │ → Write damperCmd = 60   │
└──────────────────────────┘   └──────────────────────────┘
```

### Component vs Type Logic

| Put in **Component** | Put in **Device Type** |
|---------------------|------------------------|
| Reusable across device types | Only this type needs it |
| Self-contained behavior | Orchestrates multiple components |
| Could add/remove dynamically | Core to what this device IS |

**Examples:**

| Behavior | Where | Why |
|----------|-------|-----|
| Damper stuck detection | Component | VAV, AHU, UnitVent all have dampers |
| Comfort monitoring | Component | Any zone equipment needs this |
| VAV proportional control | Device Type | VAV-specific control math |
| Chiller staging | Device Type | Only chillers do this |
| Runtime tracking | Component | Any equipment with motors |

### Component Reuse Matrix

```
                    │ Comfort │ Damper  │ Runtime │ Occupied │
                    │ Monitor │ Control │ Tracker │ Schedule │
────────────────────┼─────────┼─────────┼─────────┼──────────│
VAV                 │    ✓    │    ✓    │         │    ✓     │
AHU                 │         │    ✓    │    ✓    │    ✓     │
FCU                 │    ✓    │         │    ✓    │    ✓     │
Chiller             │         │         │    ✓    │          │
Pump                │         │         │    ✓    │          │
────────────────────┴─────────┴─────────┴─────────┴──────────┘
```

### File Structure

```
project/
├── components/
│   ├── ComfortMonitor.ts
│   ├── DamperControl.ts
│   ├── RuntimeTracker.ts
│   ├── OccupancySchedule.ts
│   └── AlarmOnStale.ts
│
├── device-types/
│   ├── VAV.ts
│   ├── AHU.ts
│   ├── FCU.ts
│   ├── Chiller.ts
│   └── Pump.ts
│
└── profiles/
    ├── JCI-FEC.ts
    ├── Tridium-JACE.ts
    └── Honeywell-Spyder.ts
```

---

## Summary

| Concept | Description |
|---------|-------------|
| **Device** | Typed equipment struct with points |
| **Point** | Data value with metadata and protocol binding |
| **Component** | Reusable behavior that attaches to device types |
| **Device Type** | Blueprint class defining points, components, and logic |
| **Instance** | Runtime device created from a device type |
| **Template** | Reusable device structure definition |
| **Profile** | Manufacturer-specific auto-mapping |

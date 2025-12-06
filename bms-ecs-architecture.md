# Building Management System - ECS Architecture

## Overview

A building management system using ECS (Entity-Component-System) architecture with visual scripting and JavaScript as an escape hatch.

### Tech Stack

```
┌─────────────────────┐
│   JS (deno_core)    │  ← Expressive API / visual scripting target
├─────────────────────┤
│   Rust application  │  ← Core logic, BACnet integration
├─────────────────────┤
│   Flecs (C)         │  ← ECS engine
└─────────────────────┘
```

---

## Why ECS for BMS?

| Feature | Benefit |
|---------|---------|
| **Entities** | Natural fit for devices, points, zones |
| **Components** | Structured data (Temperature, Setpoint, Alarm) |
| **Queries** | "All VAVs on floor 3" is a first-class operation |
| **Relationships** | Building hierarchy (site → building → floor → zone → device) |
| **Composition** | Add component = add behavior |
| **Tags** | User-defined grouping without predefined structure |
| **Observers** | Event-driven, not frame-based |

### What We Use vs. Don't Use

| ECS Concept | Traditional Use | BMS Use | Needed? |
|-------------|-----------------|---------|---------|
| Entity | Game object ID | Device/point ID | ✓ |
| Component | Position, Health | Temperature, Damper | ✓ |
| System | Runs every frame | Event-triggered behaviors | ✓ (modified) |
| Observer | React to changes | React to BACnet COV | ✓ |
| Relationships | Scene graph | Building hierarchy | ✓ |
| Query | Find entities | Find entities | ✓ |
| Tags | Grouping | User-defined groups | ✓ |

---

## Entity Structure

### BACnet Device Discovery → ECS Entities

```
Building("main")
├── Floor-3 [Floor]
│   ├── VAV-3-01 [VAV_Box, BACnetDevice]
│   │   ├── ZoneTemp [ZoneTemperature, BACnetObjectRef, COVSubscription]
│   │   ├── DischargeTemp [DischargeAirTemp, BACnetObjectRef]
│   │   ├── DamperPos [DamperPosition, Commandable, BACnetObjectRef]
│   │   └── Airflow [Airflow, BACnetObjectRef]
│   ├── VAV-3-02 [VAV_Box, BACnetDevice]
│   │   └── ...
│   └── ...
```

### Component Examples

```js
// Data components
Temperature { value: 72.4, unit: "F" }
Setpoint { value: 70.0 }
DamperPosition { value: 45, min: 0, max: 100 }
BACnetObjectRef { type: "AnalogInput", instance: 1 }

// Behavior components (enable systems)
PIDControl { kp: 1.2, ki: 0.1, kd: 0.01 }
Alarmed { highLimit: 80, lowLimit: 55 }
Trended { interval: 300 }
Schedulable { calendar: "occupancy" }

// Tags (zero data, just markers)
NeedsService
Offline
Commissioning
```

---

## Querying

### By Components

```js
// All temperature sensors
world.query(ZoneTemperature)

// All commandable points in alarm
world.query(Commandable, Alarm)
```

### By Relationships (Hierarchy)

```js
// All VAVs on floor 3
world.query(VAV_Box).child_of(Floor3)

// All points in building
world.query(BACnetObjectRef).child_of(Building("main"), Wildcard)
```

### By Tags

```js
// User-defined grouping
entity.add("NeedsService")

world.query().with("#NeedsService").each(...)
```

### Single Entity (No Query Needed)

```js
// Direct by ID
world.entity(847291)

// By name
world.lookup("VAV-3-01")
```

---

## Systems as Composition

Systems are reusable behaviors that activate based on component presence.

### Pattern

```
Entity has components → System matches → Behavior applies
```

### Example Systems

```c
// PID Control System
// Activates for any entity with: Temperature + Setpoint + Output + PIDControl
ecs_observer(world, {
    .filter.terms = {
        { ecs_id(Temperature) },
        { ecs_id(Setpoint) },
        { ecs_id(PIDControl) }
    },
    .events = { EcsOnSet },
    .callback = pid_calculate
});

// Alarm System
// Activates for any entity with: Value + AlarmLimits + Alarmed
ecs_observer(world, {
    .filter.terms = {
        { ecs_id(Value) },
        { ecs_id(AlarmLimits) },
        { ecs_id(Alarmed) }
    },
    .events = { EcsOnSet },
    .callback = check_alarms
});

// Trend System (timer-based)
ecs_system(world, {
    .query.filter.terms = {
        { ecs_id(Value) },
        { ecs_id(Trended) }
    },
    .callback = record_trend
});
ecs_set_interval(world, trend_system, 300.0);  // every 5 min
```

### Composition in Action

```js
// Add PID control to a VAV
vav.add(PIDControl, { kp: 1.2, ki: 0.1, kd: 0.01 })
// → PIDSystem now automatically runs for this entity

// Add trending
vav.add(Trended, { interval: 300 })
// → TrendSystem now also activates for this entity

// Remove behavior
vav.remove(PIDControl)
// → PIDSystem no longer runs for this entity
```

---

## Event-Driven Model (Not Frame-Based)

### Trigger Types

| Trigger | Flecs Mechanism |
|---------|-----------------|
| Component changed | `ecs_observer` + `EcsOnSet` |
| Component added | `ecs_observer` + `EcsOnAdd` |
| Component removed | `ecs_observer` + `EcsOnRemove` |
| Custom event (BACnet COV) | `ecs_emit` + custom event tag |
| Timer | `ecs_set_interval` |
| On demand | `ecs_run` |

### Custom Events

```c
// Define custom event for BACnet
ECS_TAG(world, BACnetCOV);

// Observer listens
ecs_observer(world, {
    .filter.terms = {{ ecs_id(BACnetPoint) }},
    .events = { BACnetCOV },
    .callback = handle_cov
});

// Emit when COV arrives from network
ecs_emit(world, &(ecs_event_desc_t){
    .event = BACnetCOV,
    .entity = point_entity
});
```

### Main Loop

```c
while (running) {
    ecs_progress(world, 0);  // processes events, timers
    wait_for_next_event();   // sleep until BACnet/user input
}
```

---

## Visual Scripting Integration

### Query Node → Operations

```
┌──────────────────┐         ┌──────────────────┐
│ 📋 Query         │         │ Set Value        │
│                  │         │                  │
│ VAV_Box         ─┼────────▶│─ Damper Position │
│ child_of(Floor3) │  [5]    │  Value: 100%     │
└──────────────────┘         └──────────────────┘
```

### Pinned Entity (Single Device)

```
┌──────────────────┐                    ┌──────────────────┐
│ 📌 VAV-3-01      │                    │ Set Value        │
│                  │                    │                  │
│   ZoneTemp ○─────┼───────────────────▶│─ Setpoint        │
│   Damper ○       │                    │  Value: 74°F     │
└──────────────────┘                    └──────────────────┘

📌 = pinned entity (direct reference, not a query)
📋 = query (dynamic set of entities)
```

### ForEach for Per-Entity Logic

```
┌──────────────────┐     ┌─────────────────────────────────┐
│ 📋 Query         │     │ 🔁 ForEach                       │
│                  │     │ ┌─────────────────────────────┐ │
│ VAV_Box         ─┼────▶│ │  ┌─────────┐   ┌─────────┐  │ │
│ child_of(Floor3) │ [5] │ │  │ZoneTemp │──▶│ Compare │  │ │
│                  │     │ │  │ $.value │   │ > 74°F  │  │ │
└──────────────────┘     │ │  └─────────┘   └────┬────┘  │ │
                         │ │                     │       │ │
                         │ │  ┌─────────┐   ┌────▼────┐  │ │
                         │ │  │ Damper  │◀──│ If True │  │ │
                         │ │  │ += 10%  │   └─────────┘  │ │
                         │ │  └─────────┘                │ │
                         │ └─────────────────────────────┘ │
                         └─────────────────────────────────┘

$ = current entity in loop
```

### Aggregations

```
┌──────────────────┐     ┌───────────┐     ┌────────────────┐
│ 📋 Query         │     │ Average   │     │ Virtual Point  │
│                  │     │           │     │                │
│ ZoneTemperature ─┼────▶│   ○───────┼────▶│ Floor3_AvgTemp │
│ child_of(Floor3) │ [5] │           │     │ 73.2°F         │
└──────────────────┘     └───────────┘     └────────────────┘
```

### Compiles to JS

```js
// Visual graph compiles to:
world.observer(ZoneTemperature)
  .with(VAV_Box)
  .child_of(Floor3)
  .each((entity, temp) => {
    if (temp.value > 74) {
      const damper = entity.get(DamperPosition)
      damper.value = Math.min(100, damper.value + 10)
    }
  })
```

---

## UI Flows

### Device Discovery

```
1. Scan network → find BACnet devices
2. Match to templates (VAV_Box, AHU, etc.)
3. Auto-map BACnet objects to components
4. User confirms/adjusts
5. Assign to building hierarchy
6. Entities created with appropriate components
```

### Adding Behaviors

```
┌─────────────────────────────────────────────────────────────────┐
│  VAV-3-01 Behaviors                                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Active behaviors (components):                                 │
│                                                                 │
│  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐      │
│  │ ✓ PID Control  │ │ ✓ Alarmed      │ │ ✓ Trended      │      │
│  │   Kp: 1.2      │ │   Hi: 80°F     │ │   5 min        │      │
│  │   [Configure]  │ │   [Configure]  │ │   [Configure]  │      │
│  └────────────────┘ └────────────────┘ └────────────────┘      │
│                                                                 │
│  Available:                                                     │
│  [ ] Scheduled    [ ] Averaged    [ ] Rate Limited    [+ More] │
└─────────────────────────────────────────────────────────────────┘
```

### Tagging

```
┌─────────────────────────────────────────────────────────────────┐
│  VAV-3-01                                                       │
├─────────────────────────────────────────────────────────────────┤
│  Tags: [NeedsService] [Tenant-A] [+ Add Tag]                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## JS API Design

### Entity Operations

```js
// Get entity
const vav = world.lookup("VAV-3-01")
// or
const vav = world.entity(847291)

// Components
vav.set(Temperature, { value: 72.4 })
const temp = vav.get(Temperature)
vav.remove(PIDControl)

// Tags
vav.add("NeedsService")
vav.remove("NeedsService")

// Hierarchy
vav.child_of(floor3)
const parent = vav.parent()
const children = vav.children()
```

### Queries

```js
// Basic query
world.query(Temperature, VAV_Box).each((entity, temp, vav) => {
  // ...
})

// With relationships
world.query(VAV_Box).child_of(Floor3).each(...)

// With tags
world.query().with("#NeedsService").each(...)

// Cached query (reusable)
const floorSensors = world.query(Temperature).child_of(Floor3).cached()
floorSensors.each(...)
```

### Observers

```js
// On component change
world.onChanged(Temperature, (entity, temp) => {
  // ...
})

// On component added
world.onAdded(PIDControl, (entity, pid) => {
  // Initialize PID state
})
```

---

## Why Flecs?

- **Relationships** - First-class hierarchy support
- **Observers** - Event-driven, not just frame-based
- **Prefabs** - Device templates
- **Reflection** - Runtime component inspection for visual scripting
- **Query DSL** - Expressive queries
- **REST API** - Built-in HTTP access for debugging
- **Performance** - Handles hundreds of thousands of entities

### Flecs Resources

- Repo: https://github.com/SanderMertens/flecs
- Rust bindings: https://github.com/Indra-db/flecs-ecs-rs
- Documentation: https://www.flecs.dev/flecs/

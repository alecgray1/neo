# Service Abstraction + Deno Plugin Support Plan

## Overview

Create a unified service abstraction that allows services to be implemented in either Rust (native) or TypeScript/JavaScript (via Deno), while presenting a consistent interface to the rest of the system. Services are dynamically dispatched to support runtime plugin loading.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Service Registry                                   │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │  HashMap<String, Arc<dyn ServiceHandle>>                                ││
│  │  - Dynamic dispatch for runtime flexibility                             ││
│  │  - Services can be added/removed at runtime                             ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│         │                          │                          │             │
│         ▼                          ▼                          ▼             │
│  ┌─────────────┐          ┌─────────────────┐        ┌─────────────────┐   │
│  │   Native    │          │   Native        │        │   Deno Plugin   │   │
│  │   Service   │          │   Service       │        │   Service       │   │
│  │Arc<dyn Svc> │          │ Arc<dyn Svc>    │        │ Arc<dyn Svc>    │   │
│  ├─────────────┤          ├─────────────────┤        ├─────────────────┤   │
│  │HistoryService│         │  AlarmService   │        │ WeatherService  │   │
│  └─────────────┘          └─────────────────┘        └─────────────────┘   │
│         │                          │                          │             │
│         └──────────────────────────┴──────────────────────────┘             │
│                                    │                                         │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                          PubSub Broker                                   ││
│  │                   (Events flow to/from services)                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Core Service Abstraction (Dynamic Dispatch)

### 1.1 Service Trait

```rust
// src/services/traits.rs

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Core service trait - object safe for dynamic dispatch
#[async_trait]
pub trait Service: Send + Sync {
    /// Unique identifier for this service
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Service description
    fn description(&self) -> &str;

    /// Service type (native or plugin)
    fn service_type(&self) -> ServiceType;

    /// Current state
    fn state(&self) -> ServiceState;

    /// Configuration schema (JSON Schema)
    fn config_schema(&self) -> serde_json::Value;

    /// Start the service
    async fn start(&self) -> Result<()>;

    /// Stop the service
    async fn stop(&self) -> Result<()>;

    /// Handle an event from PubSub
    async fn on_event(&self, event: &Event) -> Result<()>;

    /// Handle a request and return a response
    async fn handle_request(&self, request: ServiceRequest) -> Result<ServiceResponse>;
}

/// Type alias for dynamic service references
pub type DynService = Arc<dyn Service>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceType {
    Native,
    Plugin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}
```

### 1.2 Service Messages (with ts-rs for Type Generation)

```rust
// src/services/messages.rs

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Request types that can be sent to any service
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
#[serde(tag = "type")]
pub enum ServiceRequest {
    // Common requests
    GetStatus,
    GetConfig,
    SetConfig { config: serde_json::Value },

    // History service
    QueryHistory {
        point: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        #[serde(default)]
        limit: Option<u32>,
    },

    // Alarm service
    GetActiveAlarms,
    AcknowledgeAlarm { alarm_id: Uuid },
    GetAlarmHistory { start: DateTime<Utc>, end: DateTime<Utc> },

    // Scheduler service
    GetSchedules,
    CreateSchedule { schedule: Schedule },
    UpdateSchedule { id: Uuid, schedule: Schedule },
    DeleteSchedule { id: Uuid },

    // Custom (for plugins)
    Custom {
        action: String,
        #[serde(default)]
        payload: serde_json::Value,
    },
}

/// Response types from services
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
#[serde(tag = "type")]
pub enum ServiceResponse {
    // Common responses
    Status {
        id: String,
        name: String,
        state: ServiceState,
        uptime_seconds: u64,
    },
    Config(serde_json::Value),
    ConfigUpdated,
    Ok,

    // History responses
    HistoryData { samples: Vec<HistorySample> },

    // Alarm responses
    ActiveAlarms { alarms: Vec<Alarm> },
    AlarmHistory { alarms: Vec<Alarm> },
    AlarmAcknowledged { alarm_id: Uuid },

    // Scheduler responses
    Schedules { schedules: Vec<Schedule> },
    ScheduleCreated { id: Uuid },
    ScheduleUpdated { id: Uuid },
    ScheduleDeleted { id: Uuid },

    // Custom (for plugins)
    Custom { payload: serde_json::Value },

    // Error
    Error { code: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct HistorySample {
    pub timestamp: DateTime<Utc>,
    pub value: PointValue,
    pub quality: PointQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct Alarm {
    pub id: Uuid,
    pub source: String,
    pub message: String,
    pub severity: AlarmSeverity,
    pub state: AlarmState,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub cleared_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub enum AlarmState {
    Active,
    Acknowledged,
    Cleared,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct Schedule {
    pub id: Option<Uuid>,
    pub name: String,
    pub target_point: String,
    pub entries: Vec<ScheduleEntry>,
    pub exceptions: Vec<ScheduleException>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct ScheduleEntry {
    pub days: Vec<Weekday>,
    pub time: String,  // "HH:MM:SS"
    pub value: PointValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub struct ScheduleException {
    pub date: String,  // "YYYY-MM-DD"
    pub entries: Vec<ScheduleEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export, export_to = "bindings/")]
pub enum Weekday {
    Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,
}
```

### 1.3 Generate TypeScript Types

```rust
// src/services/mod.rs

#[cfg(test)]
mod type_export {
    use super::*;

    #[test]
    fn export_typescript_types() {
        // Run with: cargo test export_typescript_types
        // Generates .ts files in bindings/

        use ts_rs::TS;

        ServiceRequest::export().expect("Failed to export ServiceRequest");
        ServiceResponse::export().expect("Failed to export ServiceResponse");
        HistorySample::export().expect("Failed to export HistorySample");
        Alarm::export().expect("Failed to export Alarm");
        Schedule::export().expect("Failed to export Schedule");
        PointValue::export().expect("Failed to export PointValue");
        // ... etc
    }
}
```

---

## Phase 2: Service Registry (Dynamic Dispatch)

### 2.1 Registry Implementation

```rust
// src/services/registry.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wildmatch::WildMatch;

/// Metadata about a registered service
#[derive(Debug, Clone)]
pub struct ServiceRegistration {
    pub service: DynService,
    pub subscriptions: Vec<String>,  // Event patterns like "PointValueChanged", "*"
}

/// Central registry for all services - supports dynamic add/remove
#[derive(kameo::Actor)]
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, ServiceRegistration>>>,
    pubsub: ActorRef<PubSubBroker>,
}

impl ServiceRegistry {
    pub fn new(pubsub: ActorRef<PubSubBroker>) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            pubsub,
        }
    }

    /// Register a service (native or plugin)
    pub async fn register(
        &self,
        service: DynService,
        subscriptions: Vec<String>,
    ) -> Result<()> {
        let id = service.id().to_string();

        let registration = ServiceRegistration {
            service,
            subscriptions,
        };

        self.services.write().await.insert(id.clone(), registration);
        tracing::info!("Registered service: {}", id);

        Ok(())
    }

    /// Unregister a service
    pub async fn unregister(&self, id: &str) -> Result<Option<DynService>> {
        let removed = self.services.write().await.remove(id);

        if let Some(reg) = &removed {
            // Stop the service
            let _ = reg.service.stop().await;
            tracing::info!("Unregistered service: {}", id);
            Ok(Some(reg.service.clone()))
        } else {
            Ok(None)
        }
    }

    /// Get a service by ID
    pub async fn get(&self, id: &str) -> Option<DynService> {
        self.services.read().await.get(id).map(|r| r.service.clone())
    }

    /// List all registered services
    pub async fn list(&self) -> Vec<ServiceInfo> {
        self.services
            .read()
            .await
            .values()
            .map(|reg| ServiceInfo {
                id: reg.service.id().to_string(),
                name: reg.service.name().to_string(),
                description: reg.service.description().to_string(),
                service_type: reg.service.service_type(),
                state: reg.service.state(),
            })
            .collect()
    }

    /// Start all services
    pub async fn start_all(&self) -> Result<()> {
        let services = self.services.read().await;

        for (id, reg) in services.iter() {
            if reg.service.state() == ServiceState::Stopped {
                tracing::info!("Starting service: {}", id);
                if let Err(e) = reg.service.start().await {
                    tracing::error!("Failed to start service {}: {}", id, e);
                }
            }
        }

        Ok(())
    }

    /// Stop all services
    pub async fn stop_all(&self) -> Result<()> {
        let services = self.services.read().await;

        for (id, reg) in services.iter() {
            if reg.service.state() == ServiceState::Running {
                tracing::info!("Stopping service: {}", id);
                if let Err(e) = reg.service.stop().await {
                    tracing::error!("Failed to stop service {}: {}", id, e);
                }
            }
        }

        Ok(())
    }

    /// Route an event to all subscribed services
    pub async fn route_event(&self, event: &Event) {
        let event_type = event.event_type();
        let services = self.services.read().await;

        for (id, reg) in services.iter() {
            // Check if service is subscribed to this event type
            let subscribed = reg.subscriptions.iter().any(|pattern| {
                pattern == "*" || WildMatch::new(pattern).matches(&event_type)
            });

            if subscribed {
                if let Err(e) = reg.service.on_event(event).await {
                    tracing::warn!("Service {} failed to handle event: {}", id, e);
                }
            }
        }
    }

    /// Send request to a specific service
    pub async fn request(
        &self,
        service_id: &str,
        request: ServiceRequest,
    ) -> Result<ServiceResponse> {
        let services = self.services.read().await;

        if let Some(reg) = services.get(service_id) {
            reg.service.handle_request(request).await
        } else {
            Ok(ServiceResponse::Error {
                code: "SERVICE_NOT_FOUND".to_string(),
                message: format!("Service '{}' not found", service_id),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub service_type: ServiceType,
    pub state: ServiceState,
}

/// Messages for ServiceRegistry actor
pub enum RegistryMsg {
    Register {
        service: DynService,
        subscriptions: Vec<String>,
    },
    Unregister {
        id: String,
    },
    Get {
        id: String,
    },
    List,
    StartAll,
    StopAll,
    RouteEvent {
        event: Event,
    },
    Request {
        service_id: String,
        request: ServiceRequest,
    },
}

#[derive(Debug, kameo::Reply)]
pub enum RegistryReply {
    Registered,
    Unregistered(Option<DynService>),
    Service(Option<DynService>),
    ServiceList(Vec<ServiceInfo>),
    Started,
    Stopped,
    EventRouted,
    Response(ServiceResponse),
    Error(String),
}

impl kameo::message::Message<RegistryMsg> for ServiceRegistry {
    type Reply = RegistryReply;

    async fn handle(
        &mut self,
        msg: RegistryMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            RegistryMsg::Register { service, subscriptions } => {
                match self.register(service, subscriptions).await {
                    Ok(_) => RegistryReply::Registered,
                    Err(e) => RegistryReply::Error(e.to_string()),
                }
            }
            RegistryMsg::Unregister { id } => {
                match self.unregister(&id).await {
                    Ok(svc) => RegistryReply::Unregistered(svc),
                    Err(e) => RegistryReply::Error(e.to_string()),
                }
            }
            RegistryMsg::Get { id } => {
                RegistryReply::Service(self.get(&id).await)
            }
            RegistryMsg::List => {
                RegistryReply::ServiceList(self.list().await)
            }
            RegistryMsg::StartAll => {
                match self.start_all().await {
                    Ok(_) => RegistryReply::Started,
                    Err(e) => RegistryReply::Error(e.to_string()),
                }
            }
            RegistryMsg::StopAll => {
                match self.stop_all().await {
                    Ok(_) => RegistryReply::Stopped,
                    Err(e) => RegistryReply::Error(e.to_string()),
                }
            }
            RegistryMsg::RouteEvent { event } => {
                self.route_event(&event).await;
                RegistryReply::EventRouted
            }
            RegistryMsg::Request { service_id, request } => {
                match self.request(&service_id, request).await {
                    Ok(response) => RegistryReply::Response(response),
                    Err(e) => RegistryReply::Error(e.to_string()),
                }
            }
        }
    }
}
```

---

## Phase 3: Native Service Implementations

### 3.1 Base Service Helper

```rust
// src/services/base.rs

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use parking_lot::RwLock;

/// Helper struct for common service state management
pub struct ServiceBase {
    id: String,
    name: String,
    description: String,
    state: RwLock<ServiceState>,
    started_at: RwLock<Option<Instant>>,
    config: RwLock<serde_json::Value>,
}

impl ServiceBase {
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            state: RwLock::new(ServiceState::Stopped),
            started_at: RwLock::new(None),
            config: RwLock::new(serde_json::Value::Null),
        }
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn name(&self) -> &str { &self.name }
    pub fn description(&self) -> &str { &self.description }

    pub fn state(&self) -> ServiceState {
        *self.state.read()
    }

    pub fn set_state(&self, state: ServiceState) {
        *self.state.write() = state;
        if state == ServiceState::Running {
            *self.started_at.write() = Some(Instant::now());
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at
            .read()
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }

    pub fn config(&self) -> serde_json::Value {
        self.config.read().clone()
    }

    pub fn set_config(&self, config: serde_json::Value) {
        *self.config.write() = config;
    }
}
```

### 3.2 History Service

```rust
// src/services/builtin/history.rs

use std::sync::Arc;
use async_trait::async_trait;
use parking_lot::RwLock;

pub struct HistoryService {
    base: ServiceBase,
    db: RwLock<Option<redb::Database>>,
    config: RwLock<HistoryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub db_path: String,
    pub retention_days: u32,
    pub sample_interval_ms: u64,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/history.redb".to_string(),
            retention_days: 365,
            sample_interval_ms: 1000,
        }
    }
}

impl HistoryService {
    pub fn new(config: HistoryConfig) -> Arc<Self> {
        Arc::new(Self {
            base: ServiceBase::new(
                "history",
                "History Service",
                "Stores time-series point data for trending and analysis",
            ),
            db: RwLock::new(None),
            config: RwLock::new(config),
        })
    }

    async fn store(&self, point: &str, value: PointValue, timestamp: DateTime<Utc>) -> Result<()> {
        // Store in redb
        if let Some(db) = self.db.read().as_ref() {
            // Implementation...
        }
        Ok(())
    }

    async fn query(
        &self,
        point: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: Option<u32>,
    ) -> Result<Vec<HistorySample>> {
        // Query from redb
        Ok(vec![])
    }
}

#[async_trait]
impl Service for HistoryService {
    fn id(&self) -> &str { self.base.id() }
    fn name(&self) -> &str { self.base.name() }
    fn description(&self) -> &str { self.base.description() }
    fn service_type(&self) -> ServiceType { ServiceType::Native }
    fn state(&self) -> ServiceState { self.base.state() }

    fn config_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "db_path": { "type": "string", "default": "./data/history.redb" },
                "retention_days": { "type": "integer", "minimum": 1, "default": 365 },
                "sample_interval_ms": { "type": "integer", "minimum": 100, "default": 1000 }
            }
        })
    }

    async fn start(&self) -> Result<()> {
        self.base.set_state(ServiceState::Starting);

        let config = self.config.read().clone();
        let db = redb::Database::create(&config.db_path)?;
        *self.db.write() = Some(db);

        self.base.set_state(ServiceState::Running);
        tracing::info!("History service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.base.set_state(ServiceState::Stopping);
        *self.db.write() = None;
        self.base.set_state(ServiceState::Stopped);
        tracing::info!("History service stopped");
        Ok(())
    }

    async fn on_event(&self, event: &Event) -> Result<()> {
        if let Event::PointValueChanged { point, value, timestamp_utc, .. } = event {
            self.store(point, value.clone(), *timestamp_utc).await?;
        }
        Ok(())
    }

    async fn handle_request(&self, request: ServiceRequest) -> Result<ServiceResponse> {
        match request {
            ServiceRequest::GetStatus => {
                Ok(ServiceResponse::Status {
                    id: self.id().to_string(),
                    name: self.name().to_string(),
                    state: self.state(),
                    uptime_seconds: self.base.uptime_seconds(),
                })
            }
            ServiceRequest::GetConfig => {
                Ok(ServiceResponse::Config(serde_json::to_value(&*self.config.read())?))
            }
            ServiceRequest::QueryHistory { point, start, end, limit } => {
                let samples = self.query(&point, start, end, limit).await?;
                Ok(ServiceResponse::HistoryData { samples })
            }
            _ => Ok(ServiceResponse::Error {
                code: "UNSUPPORTED".to_string(),
                message: "Request not supported by this service".to_string(),
            }),
        }
    }
}
```

### 3.3 Alarm Service

```rust
// src/services/builtin/alarm.rs

pub struct AlarmService {
    base: ServiceBase,
    configs: RwLock<Vec<AlarmConfig>>,
    active_alarms: RwLock<HashMap<Uuid, Alarm>>,
    pubsub: ActorRef<PubSubBroker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmConfig {
    pub id: Uuid,
    pub name: String,
    pub source_pattern: String,  // Glob pattern like "*/VAV-*/AI:1"
    pub condition: AlarmCondition,
    pub severity: AlarmSeverity,
    pub delay_seconds: u32,
    pub message_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlarmCondition {
    HighLimit { value: f32 },
    LowLimit { value: f32 },
    OutOfRange { low: f32, high: f32 },
    Equals { value: PointValue },
    NotEquals { value: PointValue },
    RateOfChange { threshold: f32, window_seconds: u32 },
    Stale { max_age_seconds: u32 },
}

#[async_trait]
impl Service for AlarmService {
    // ... implementation similar to HistoryService

    async fn on_event(&self, event: &Event) -> Result<()> {
        if let Event::PointValueChanged { point, value, .. } = event {
            let configs = self.configs.read().clone();

            for config in configs {
                if WildMatch::new(&config.source_pattern).matches(point) {
                    if let Some(alarm) = self.evaluate_condition(&config, value) {
                        self.raise_alarm(alarm).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_request(&self, request: ServiceRequest) -> Result<ServiceResponse> {
        match request {
            ServiceRequest::GetActiveAlarms => {
                let alarms: Vec<Alarm> = self.active_alarms.read().values().cloned().collect();
                Ok(ServiceResponse::ActiveAlarms { alarms })
            }
            ServiceRequest::AcknowledgeAlarm { alarm_id } => {
                if let Some(alarm) = self.active_alarms.write().get_mut(&alarm_id) {
                    alarm.state = AlarmState::Acknowledged;
                    alarm.acknowledged_at = Some(Utc::now());
                    Ok(ServiceResponse::AlarmAcknowledged { alarm_id })
                } else {
                    Ok(ServiceResponse::Error {
                        code: "NOT_FOUND".to_string(),
                        message: "Alarm not found".to_string(),
                    })
                }
            }
            _ => Ok(ServiceResponse::Error {
                code: "UNSUPPORTED".to_string(),
                message: "Request not supported".to_string(),
            }),
        }
    }
}
```

---

## Phase 4: Deno Plugin Runtime

### 4.1 Deno Ops Definition

```rust
// src/services/plugin/ops.rs

use deno_core::{op2, OpState, Extension};
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::mpsc;

/// Bridge to communicate between plugin and Rust
#[derive(Clone)]
pub struct PluginBridge {
    pub config: serde_json::Value,
    pub io_actor: ActorRef<BACnetIOActor>,
    pub pubsub: ActorRef<PubSubBroker>,
    pub event_tx: mpsc::Sender<Event>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Point Operations
// ─────────────────────────────────────────────────────────────────────────────

#[op2(async)]
#[serde]
pub async fn op_neo_point_read(
    state: Rc<RefCell<OpState>>,
    #[string] path: String,
) -> Result<PointValue, deno_core::error::AnyError> {
    let bridge = {
        let state = state.borrow();
        state.borrow::<PluginBridge>().clone()
    };

    // Parse path: "network/device/objectType:instance"
    let (device_id, object_type, instance) = parse_point_path(&path)?;

    let reply = bridge.io_actor
        .ask(BACnetIOMsg::ReadProperty {
            device_id,
            object_type,
            object_instance: instance,
            property_id: 85,  // present-value
            array_index: None,
            timeout_ms: Some(3000),
        })
        .await
        .map_err(|e| deno_core::error::generic_error(e.to_string()))?;

    match reply {
        BACnetIOReply::PropertyValue(value) => Ok(value),
        BACnetIOReply::IoError(e) => Err(deno_core::error::generic_error(e)),
        _ => Err(deno_core::error::generic_error("Unexpected reply")),
    }
}

#[op2(async)]
pub async fn op_neo_point_write(
    state: Rc<RefCell<OpState>>,
    #[string] path: String,
    #[serde] value: PointValue,
) -> Result<(), deno_core::error::AnyError> {
    let bridge = {
        let state = state.borrow();
        state.borrow::<PluginBridge>().clone()
    };

    let (device_id, object_type, instance) = parse_point_path(&path)?;

    let reply = bridge.io_actor
        .ask(BACnetIOMsg::WriteProperty {
            device_id,
            object_type,
            object_instance: instance,
            property_id: 85,
            value,
        })
        .await
        .map_err(|e| deno_core::error::generic_error(e.to_string()))?;

    match reply {
        BACnetIOReply::PropertyWritten => Ok(()),
        BACnetIOReply::IoError(e) => Err(deno_core::error::generic_error(e)),
        _ => Err(deno_core::error::generic_error("Unexpected reply")),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Event Operations
// ─────────────────────────────────────────────────────────────────────────────

#[op2(async)]
pub async fn op_neo_event_publish(
    state: Rc<RefCell<OpState>>,
    #[serde] event: Event,
) -> Result<(), deno_core::error::AnyError> {
    let bridge = {
        let state = state.borrow();
        state.borrow::<PluginBridge>().clone()
    };

    bridge.event_tx
        .send(event)
        .await
        .map_err(|e| deno_core::error::generic_error(e.to_string()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Logging
// ─────────────────────────────────────────────────────────────────────────────

#[op2(fast)]
pub fn op_neo_log(
    state: &OpState,
    #[string] level: &str,
    #[string] message: &str,
) {
    let plugin_id = state.borrow::<String>();

    match level {
        "info" => tracing::info!("[Plugin:{}] {}", plugin_id, message),
        "warn" => tracing::warn!("[Plugin:{}] {}", plugin_id, message),
        "error" => tracing::error!("[Plugin:{}] {}", plugin_id, message),
        _ => tracing::debug!("[Plugin:{}] {}", plugin_id, message),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

#[op2]
#[serde]
pub fn op_neo_get_config(state: &OpState) -> serde_json::Value {
    state.borrow::<PluginBridge>().config.clone()
}

// ─────────────────────────────────────────────────────────────────────────────
// Extension Definition
// ─────────────────────────────────────────────────────────────────────────────

deno_core::extension!(
    neo_plugin,
    ops = [
        op_neo_point_read,
        op_neo_point_write,
        op_neo_event_publish,
        op_neo_log,
        op_neo_get_config,
    ],
    esm_entry_point = "ext:neo_plugin/runtime.js",
    esm = ["runtime.js"],
    state = |state| {
        // State is populated when runtime is created
    },
);
```

### 4.2 JavaScript Runtime Bootstrap

```javascript
// src/services/plugin/runtime.js

((globalThis) => {
    const core = Deno.core;

    // ─────────────────────────────────────────────────────────────────────
    // Neo SDK - Clean API for plugins
    // ─────────────────────────────────────────────────────────────────────

    const Neo = {
        // Point operations
        points: {
            /**
             * Read a point value
             * @param {string} path - Point path like "MainNetwork/VAV-101/AI:1"
             * @returns {Promise<PointValue>}
             */
            read: (path) => core.ops.op_neo_point_read(path),

            /**
             * Write a point value
             * @param {string} path - Point path
             * @param {PointValue} value - Value to write
             */
            write: (path, value) => core.ops.op_neo_point_write(path, value),
        },

        // Event operations
        events: {
            /**
             * Publish an event to the system
             * @param {NeoEvent} event
             */
            publish: (event) => core.ops.op_neo_event_publish(event),
        },

        // Logging
        log: {
            info: (msg) => core.ops.op_neo_log("info", String(msg)),
            warn: (msg) => core.ops.op_neo_log("warn", String(msg)),
            error: (msg) => core.ops.op_neo_log("error", String(msg)),
            debug: (msg) => core.ops.op_neo_log("debug", String(msg)),
        },

        // Configuration (populated at runtime)
        get config() {
            return core.ops.op_neo_get_config();
        },
    };

    // ─────────────────────────────────────────────────────────────────────
    // Plugin lifecycle management
    // ─────────────────────────────────────────────────────────────────────

    globalThis.__neo_plugin_instance = null;

    globalThis.__neo_register_plugin = (plugin) => {
        globalThis.__neo_plugin_instance = plugin;
    };

    globalThis.__neo_call_start = async () => {
        if (globalThis.__neo_plugin_instance?.onStart) {
            await globalThis.__neo_plugin_instance.onStart();
        }
    };

    globalThis.__neo_call_stop = async () => {
        if (globalThis.__neo_plugin_instance?.onStop) {
            await globalThis.__neo_plugin_instance.onStop();
        }
    };

    globalThis.__neo_call_event = async (event) => {
        if (globalThis.__neo_plugin_instance?.onEvent) {
            await globalThis.__neo_plugin_instance.onEvent(event);
        }
    };

    globalThis.__neo_call_request = async (request) => {
        if (globalThis.__neo_plugin_instance?.onRequest) {
            return await globalThis.__neo_plugin_instance.onRequest(request);
        }
        return { type: "Error", code: "NOT_IMPLEMENTED", message: "onRequest not implemented" };
    };

    // ─────────────────────────────────────────────────────────────────────
    // Expose Neo globally
    // ─────────────────────────────────────────────────────────────────────

    globalThis.Neo = Neo;

    // Override console to route through our logger
    globalThis.console = {
        log: (...args) => Neo.log.info(args.map(String).join(" ")),
        info: (...args) => Neo.log.info(args.map(String).join(" ")),
        warn: (...args) => Neo.log.warn(args.map(String).join(" ")),
        error: (...args) => Neo.log.error(args.map(String).join(" ")),
        debug: (...args) => Neo.log.debug(args.map(String).join(" ")),
    };

})(globalThis);
```

### 4.3 Plugin SDK (TypeScript)

```typescript
// plugins/sdk/src/index.ts

// Re-export auto-generated types
export * from '../bindings/PointValue';
export * from '../bindings/ServiceRequest';
export * from '../bindings/ServiceResponse';
export * from '../bindings/Event';
export * from '../bindings/Alarm';
export * from '../bindings/Schedule';

// ─────────────────────────────────────────────────────────────────────────────
// Neo global type declaration
// ─────────────────────────────────────────────────────────────────────────────

import type { PointValue, ServiceRequest, ServiceResponse } from '../bindings';

declare global {
    const Neo: {
        points: {
            read(path: string): Promise<PointValue>;
            write(path: string, value: PointValue): Promise<void>;
        };
        events: {
            publish(event: NeoEvent): Promise<void>;
        };
        log: {
            info(msg: string): void;
            warn(msg: string): void;
            error(msg: string): void;
            debug(msg: string): void;
        };
        readonly config: Record<string, unknown>;
    };

    function __neo_register_plugin(plugin: ServicePlugin): void;
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin interface
// ─────────────────────────────────────────────────────────────────────────────

export interface NeoEvent {
    type: string;
    source?: string;
    timestamp?: string;
    data?: unknown;
}

export interface ServicePlugin {
    /** Called when the service starts */
    onStart?(): Promise<void>;

    /** Called when the service stops */
    onStop?(): Promise<void>;

    /** Called when a subscribed event is received */
    onEvent?(event: NeoEvent): Promise<void>;

    /** Called to handle a request */
    onRequest?(request: ServiceRequest): Promise<ServiceResponse>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper to define a service plugin
// ─────────────────────────────────────────────────────────────────────────────

export function defineService(plugin: ServicePlugin): void {
    __neo_register_plugin(plugin);
}

// ─────────────────────────────────────────────────────────────────────────────
// Typed config helper
// ─────────────────────────────────────────────────────────────────────────────

export function getConfig<T>(): T {
    return Neo.config as T;
}
```

### 4.4 Deno Plugin Service Wrapper

```rust
// src/services/plugin/service.rs

use std::sync::Arc;
use async_trait::async_trait;
use deno_core::{JsRuntime, RuntimeOptions, PollEventLoopOptions};
use parking_lot::RwLock;
use tokio::sync::mpsc;

use super::ops::{neo_plugin, PluginBridge};

/// Plugin manifest loaded from neo-plugin.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub main: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub subscriptions: Vec<String>,
}

/// Wraps a Deno plugin as a Service
pub struct DenoPluginService {
    manifest: PluginManifest,
    base_path: PathBuf,
    runtime: RwLock<Option<JsRuntime>>,
    state: RwLock<ServiceState>,
    bridge: RwLock<Option<PluginBridge>>,
    event_rx: RwLock<Option<mpsc::Receiver<Event>>>,
}

impl DenoPluginService {
    pub async fn load(
        manifest_path: PathBuf,
        io_actor: ActorRef<BACnetIOActor>,
        pubsub: ActorRef<PubSubBroker>,
    ) -> Result<Arc<Self>> {
        let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: PluginManifest = serde_json::from_str(&manifest_content)?;

        let base_path = manifest_path.parent().unwrap().to_path_buf();

        Ok(Arc::new(Self {
            manifest,
            base_path,
            runtime: RwLock::new(None),
            state: RwLock::new(ServiceState::Stopped),
            bridge: RwLock::new(None),
            event_rx: RwLock::new(None),
        }))
    }

    async fn call_js(&self, function: &str) -> Result<()> {
        if let Some(runtime) = self.runtime.write().as_mut() {
            runtime.execute_script(
                "<neo>",
                format!("(async () => {{ await {}(); }})()", function).into(),
            )?;
            runtime.run_event_loop(PollEventLoopOptions::default()).await?;
        }
        Ok(())
    }

    async fn call_js_with_arg<T: Serialize>(&self, function: &str, arg: &T) -> Result<()> {
        if let Some(runtime) = self.runtime.write().as_mut() {
            let arg_json = serde_json::to_string(arg)?;
            runtime.execute_script(
                "<neo>",
                format!("(async () => {{ await {}({}); }})()", function, arg_json).into(),
            )?;
            runtime.run_event_loop(PollEventLoopOptions::default()).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Service for DenoPluginService {
    fn id(&self) -> &str {
        &self.manifest.id
    }

    fn name(&self) -> &str {
        &self.manifest.name
    }

    fn description(&self) -> &str {
        &self.manifest.description
    }

    fn service_type(&self) -> ServiceType {
        ServiceType::Plugin
    }

    fn state(&self) -> ServiceState {
        *self.state.read()
    }

    fn config_schema(&self) -> serde_json::Value {
        self.manifest.config.clone()
    }

    async fn start(&self) -> Result<()> {
        *self.state.write() = ServiceState::Starting;

        // Create event channel
        let (event_tx, event_rx) = mpsc::channel(100);

        // Create bridge
        // Note: io_actor and pubsub would need to be stored or passed in
        let bridge = PluginBridge {
            config: self.manifest.config.clone(),
            io_actor: todo!("Need to pass io_actor"),
            pubsub: todo!("Need to pass pubsub"),
            event_tx,
        };

        // Create Deno runtime
        let mut runtime = JsRuntime::new(RuntimeOptions {
            extensions: vec![neo_plugin::init_ops_and_esm()],
            ..Default::default()
        });

        // Inject state
        {
            let op_state = runtime.op_state();
            let mut state = op_state.borrow_mut();
            state.put(bridge.clone());
            state.put(self.manifest.id.clone());
        }

        // Load and execute plugin main file
        let main_path = self.base_path.join(&self.manifest.main);
        let code = tokio::fs::read_to_string(&main_path).await?;
        runtime.execute_script(&self.manifest.main, code.into())?;
        runtime.run_event_loop(PollEventLoopOptions::default()).await?;

        // Call onStart
        runtime.execute_script("<neo>", "(async () => { await __neo_call_start(); })()".into())?;
        runtime.run_event_loop(PollEventLoopOptions::default()).await?;

        // Store runtime
        *self.runtime.write() = Some(runtime);
        *self.bridge.write() = Some(bridge);
        *self.event_rx.write() = Some(event_rx);
        *self.state.write() = ServiceState::Running;

        tracing::info!("Plugin service '{}' started", self.manifest.id);
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.state.write() = ServiceState::Stopping;

        // Call onStop
        self.call_js("__neo_call_stop").await?;

        // Drop runtime
        *self.runtime.write() = None;
        *self.bridge.write() = None;
        *self.event_rx.write() = None;
        *self.state.write() = ServiceState::Stopped;

        tracing::info!("Plugin service '{}' stopped", self.manifest.id);
        Ok(())
    }

    async fn on_event(&self, event: &Event) -> Result<()> {
        self.call_js_with_arg("__neo_call_event", event).await
    }

    async fn handle_request(&self, request: ServiceRequest) -> Result<ServiceResponse> {
        // TODO: Need to get return value from JS
        // This requires more complex handling with v8::Global<v8::Value>
        self.call_js_with_arg("__neo_call_request", &request).await?;

        Ok(ServiceResponse::Ok)
    }
}
```

---

## Phase 5: Example Plugin

### 5.1 Plugin Manifest

```json
// plugins/weather-service/neo-plugin.json
{
    "id": "weather-service",
    "name": "Weather Service",
    "description": "Fetches weather data and provides virtual weather points",
    "version": "1.0.0",
    "main": "src/index.ts",
    "config": {
        "api_key": "",
        "location": "New York, NY",
        "poll_interval_seconds": 900,
        "units": "imperial"
    },
    "subscriptions": []
}
```

### 5.2 Plugin Implementation

```typescript
// plugins/weather-service/src/index.ts

import { defineService, getConfig } from '@neo/plugin-sdk';

interface WeatherConfig {
    api_key: string;
    location: string;
    poll_interval_seconds: number;
    units: 'imperial' | 'metric';
}

let pollInterval: number | null = null;

defineService({
    async onStart() {
        const config = getConfig<WeatherConfig>();
        Neo.log.info(`Weather service starting for ${config.location}`);

        // Initial fetch
        await fetchAndUpdateWeather(config);

        // Start polling
        pollInterval = setInterval(
            () => fetchAndUpdateWeather(config),
            config.poll_interval_seconds * 1000
        );
    },

    async onStop() {
        if (pollInterval) {
            clearInterval(pollInterval);
            pollInterval = null;
        }
        Neo.log.info('Weather service stopped');
    },

    async onRequest(request) {
        if (request.type === 'Custom' && request.action === 'refresh') {
            const config = getConfig<WeatherConfig>();
            await fetchAndUpdateWeather(config);
            return { type: 'Ok' };
        }

        if (request.type === 'Custom' && request.action === 'getWeather') {
            // Return cached weather data
            return {
                type: 'Custom',
                payload: await fetchWeatherData(getConfig<WeatherConfig>()),
            };
        }

        return {
            type: 'Error',
            code: 'UNKNOWN_REQUEST',
            message: `Unknown request: ${request.type}`,
        };
    },
});

async function fetchAndUpdateWeather(config: WeatherConfig) {
    try {
        const weather = await fetchWeatherData(config);

        // Write to virtual points
        await Neo.points.write('virtual/weather/outdoor_temp', {
            type: 'Real',
            value: weather.temperature,
        });

        await Neo.points.write('virtual/weather/humidity', {
            type: 'Real',
            value: weather.humidity,
        });

        await Neo.points.write('virtual/weather/conditions', {
            type: 'Unsigned',
            value: weatherCodeFromConditions(weather.conditions),
        });

        // Publish event
        await Neo.events.publish({
            type: 'WeatherUpdated',
            source: 'weather-service',
            data: weather,
        });

        Neo.log.info(`Weather updated: ${weather.temperature}°, ${weather.conditions}`);

    } catch (error) {
        Neo.log.error(`Failed to fetch weather: ${error}`);
    }
}

async function fetchWeatherData(config: WeatherConfig) {
    const response = await fetch(
        `https://api.openweathermap.org/data/2.5/weather?q=${encodeURIComponent(config.location)}&appid=${config.api_key}&units=${config.units}`
    );

    if (!response.ok) {
        throw new Error(`Weather API error: ${response.status}`);
    }

    const data = await response.json();

    return {
        temperature: data.main.temp,
        humidity: data.main.humidity,
        conditions: data.weather[0].main,
        description: data.weather[0].description,
        wind_speed: data.wind.speed,
        pressure: data.main.pressure,
    };
}

function weatherCodeFromConditions(conditions: string): number {
    const codes: Record<string, number> = {
        'Clear': 0,
        'Clouds': 1,
        'Rain': 2,
        'Drizzle': 3,
        'Thunderstorm': 4,
        'Snow': 5,
        'Mist': 6,
        'Fog': 7,
    };
    return codes[conditions] ?? 255;
}
```

---

## File Structure

```
src/
├── actors/
│   ├── mod.rs
│   ├── bacnet/
│   └── modbus/
├── services/
│   ├── mod.rs                 # Module exports
│   ├── traits.rs              # Service trait (dyn dispatch)
│   ├── messages.rs            # Request/Response types + ts-rs
│   ├── base.rs                # ServiceBase helper
│   ├── registry.rs            # ServiceRegistry actor
│   ├── builtin/
│   │   ├── mod.rs
│   │   ├── history.rs
│   │   ├── alarm.rs
│   │   ├── scheduler.rs
│   │   └── notification.rs
│   └── plugin/
│       ├── mod.rs
│       ├── ops.rs             # Deno ops
│       ├── runtime.js         # JS bootstrap
│       └── service.rs         # DenoPluginService
├── lib.rs
└── main.rs

bindings/                      # Auto-generated by ts-rs
├── PointValue.ts
├── ServiceRequest.ts
├── ServiceResponse.ts
├── Alarm.ts
├── Schedule.ts
└── index.ts

plugins/
├── sdk/
│   ├── src/
│   │   └── index.ts           # Plugin SDK
│   ├── bindings/              # Symlink to ../bindings
│   └── package.json
└── weather-service/
    ├── neo-plugin.json
    ├── src/
    │   └── index.ts
    └── deno.json
```

---

## Implementation Order

1. **Phase 1.1** - Service trait with dynamic dispatch
2. **Phase 1.2** - Service messages with ts-rs annotations
3. **Phase 2** - ServiceRegistry with dynamic HashMap
4. **Phase 3.1** - ServiceBase helper
5. **Phase 3.2** - HistoryService (first native service)
6. **Phase 3.3** - AlarmService
7. **Phase 4.1** - Deno ops
8. **Phase 4.2** - JavaScript runtime bootstrap
9. **Phase 4.3** - TypeScript SDK
10. **Phase 4.4** - DenoPluginService wrapper
11. **Phase 5** - Example weather plugin
12. **Integration** - Wire into main.rs

---

## Open Questions

1. **Virtual points** - How do plugins create virtual points that other services/devices can read?
2. **Plugin permissions** - Should we restrict which points a plugin can read/write?
3. **Plugin hot reload** - Support updating plugins without restarting?
4. **Plugin dependencies** - Allow plugins to depend on npm packages?
5. **Inter-service requests** - Should services be able to call each other via registry?

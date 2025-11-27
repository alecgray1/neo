// Service Actor Infrastructure
//
// Common message types and type-erased actor references for service actors.
// All service actors (native and plugin) implement handlers for ServiceMsg.

use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

use kameo::actor::ActorRef;
use kameo::message::Message;
use kameo::Actor;
use tokio::sync::oneshot;

use crate::messages::Event;
use crate::types::{Result, ServiceState};

use super::messages::{ServiceRequest, ServiceResponse};

// ─────────────────────────────────────────────────────────────────────────────
// Common Service Messages
// ─────────────────────────────────────────────────────────────────────────────

/// Common messages that all service actors must handle.
/// These provide the standard lifecycle and routing interface.
#[derive(Debug)]
pub enum ServiceMsg {
    /// Start the service
    Start,
    /// Stop the service
    Stop,
    /// Get service status
    GetStatus,
    /// Get service configuration
    GetConfig,
    /// Update service configuration
    SetConfig { config: serde_json::Value },
    /// Handle an event from the event bus
    OnEvent { event: Event },
    /// Handle a service request (response sent via oneshot channel)
    HandleRequest {
        request: ServiceRequest,
        reply: oneshot::Sender<ServiceResponse>,
    },
}

/// Reply type for ServiceMsg
#[derive(Debug, kameo::Reply)]
pub enum ServiceReply {
    /// Service started successfully
    Started,
    /// Service stopped successfully
    Stopped,
    /// Service status information
    Status {
        id: String,
        name: String,
        state: ServiceState,
        uptime_secs: u64,
        extra: Option<serde_json::Value>,
    },
    /// Service configuration
    Config { config: serde_json::Value },
    /// Configuration updated
    ConfigSet,
    /// Event was handled
    EventHandled,
    /// Request was handled (response sent via oneshot)
    RequestHandled,
    /// Error occurred
    Failed(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Info (for listing/status)
// ─────────────────────────────────────────────────────────────────────────────

/// Metadata about a service (for listing)
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub service_type: ServiceType,
    pub state: ServiceState,
}

/// Type of service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    /// Native Rust service
    Native,
    /// JavaScript/TypeScript plugin
    Plugin,
}

// ─────────────────────────────────────────────────────────────────────────────
// Type-Erased Service Actor Reference
// ─────────────────────────────────────────────────────────────────────────────

/// Boxed future type for async operations
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Operations that can be performed on any service actor
trait ServiceActorOps: Send + Sync {
    /// Get the service ID
    fn id(&self) -> String;

    /// Get the service name
    fn name(&self) -> String;

    /// Get the service description
    fn description(&self) -> String;

    /// Get the service type
    fn service_type(&self) -> ServiceType;

    /// Send a message and wait for reply (ask pattern)
    fn ask_service(&self, msg: ServiceMsg) -> BoxFuture<'_, Result<ServiceReply>>;

    /// Send a message without waiting (tell pattern)
    fn tell_service(&self, msg: ServiceMsg) -> BoxFuture<'_, Result<()>>;

    /// Clone this reference
    fn clone_box(&self) -> Box<dyn ServiceActorOps>;
}

/// Type-erased reference to any service actor.
///
/// This allows the ServiceRegistry to hold references to different actor types
/// (HistoryActor, AlarmActor, PluginActor) in a uniform collection.
pub struct ServiceActorRef {
    inner: Box<dyn ServiceActorOps>,
}

impl ServiceActorRef {
    /// Create a new ServiceActorRef wrapping a kameo actor
    pub fn new<A>(actor_ref: ActorRef<A>, info: ServiceMetadata) -> Self
    where
        A: Actor + Message<ServiceMsg, Reply = ServiceReply> + 'static,
    {
        Self {
            inner: Box::new(ActorRefWrapper {
                actor_ref,
                info,
            }),
        }
    }

    /// Get the service ID
    pub fn id(&self) -> String {
        self.inner.id()
    }

    /// Get the service name
    pub fn name(&self) -> String {
        self.inner.name()
    }

    /// Get the service description
    pub fn description(&self) -> String {
        self.inner.description()
    }

    /// Get the service type
    pub fn service_type(&self) -> ServiceType {
        self.inner.service_type()
    }

    /// Send a message and wait for reply
    pub async fn ask(&self, msg: ServiceMsg) -> Result<ServiceReply> {
        self.inner.ask_service(msg).await
    }

    /// Send a message without waiting for reply
    pub async fn tell(&self, msg: ServiceMsg) -> Result<()> {
        self.inner.tell_service(msg).await
    }
}

impl Clone for ServiceActorRef {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone_box(),
        }
    }
}

impl std::fmt::Debug for ServiceActorRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceActorRef")
            .field("id", &self.inner.id())
            .field("name", &self.inner.name())
            .field("type", &self.inner.service_type())
            .finish()
    }
}

/// Metadata about a service (passed when creating ServiceActorRef)
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub service_type: ServiceType,
}

/// Internal wrapper that implements ServiceActorOps for any kameo actor
struct ActorRefWrapper<A>
where
    A: Actor + Message<ServiceMsg, Reply = ServiceReply>,
{
    actor_ref: ActorRef<A>,
    info: ServiceMetadata,
}

impl<A> ServiceActorOps for ActorRefWrapper<A>
where
    A: Actor + Message<ServiceMsg, Reply = ServiceReply> + 'static,
{
    fn id(&self) -> String {
        self.info.id.clone()
    }

    fn name(&self) -> String {
        self.info.name.clone()
    }

    fn description(&self) -> String {
        self.info.description.clone()
    }

    fn service_type(&self) -> ServiceType {
        self.info.service_type
    }

    fn ask_service(&self, msg: ServiceMsg) -> BoxFuture<'_, Result<ServiceReply>> {
        let actor_ref = self.actor_ref.clone();
        Box::pin(async move {
            actor_ref
                .ask(msg)
                .await
                .map_err(|e| crate::types::Error::Service(format!("Actor ask failed: {}", e)))
        })
    }

    fn tell_service(&self, msg: ServiceMsg) -> BoxFuture<'_, Result<()>> {
        let actor_ref = self.actor_ref.clone();
        Box::pin(async move {
            actor_ref
                .tell(msg)
                .await
                .map_err(|e| crate::types::Error::Service(format!("Actor tell failed: {}", e)))
        })
    }

    fn clone_box(&self) -> Box<dyn ServiceActorOps> {
        Box::new(ActorRefWrapper {
            actor_ref: self.actor_ref.clone(),
            info: self.info.clone(),
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service State Helper
// ─────────────────────────────────────────────────────────────────────────────

/// Helper struct for tracking service state within an actor
#[derive(Debug)]
pub struct ServiceStateTracker {
    state: ServiceState,
    started_at: Option<Instant>,
}

impl ServiceStateTracker {
    pub fn new() -> Self {
        Self {
            state: ServiceState::Stopped,
            started_at: None,
        }
    }

    pub fn state(&self) -> ServiceState {
        self.state
    }

    pub fn set_starting(&mut self) {
        self.state = ServiceState::Starting;
    }

    pub fn set_running(&mut self) {
        self.state = ServiceState::Running;
        self.started_at = Some(Instant::now());
    }

    pub fn set_stopping(&mut self) {
        self.state = ServiceState::Stopping;
    }

    pub fn set_stopped(&mut self) {
        self.state = ServiceState::Stopped;
        self.started_at = None;
    }

    pub fn set_failed(&mut self) {
        self.state = ServiceState::Failed;
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }
}

impl Default for ServiceStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

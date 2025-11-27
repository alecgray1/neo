// Service Registry - Central registration and routing for all services
//
// All services are actor-based and use ServiceActorRef for type-erased references.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wildmatch::WildMatch;

use crate::actors::PubSubBroker;
use crate::messages::Event;
use crate::types::{Error, Result, ServiceState};

use super::actor::{ServiceActorRef, ServiceMsg, ServiceReply, ServiceType};
use super::messages::{ServiceRequest, ServiceResponse};

/// Metadata about a registered service (for listing)
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub service_type: ServiceType,
    pub state: ServiceState,
}

/// A registered service with its event subscriptions
#[derive(Clone)]
pub struct ServiceRegistration {
    pub actor_ref: ServiceActorRef,
    /// Event patterns this service subscribes to (e.g., "PointValueChanged", "*")
    pub subscriptions: Vec<String>,
}

/// Central registry for all services - supports dynamic add/remove at runtime
///
/// The registry manages service lifecycle and event routing:
/// - Register/unregister services dynamically
/// - Start/stop all services
/// - Route events to subscribed services
/// - Forward requests to specific services
///
/// # Example
///
/// ```rust,ignore
/// let registry = ServiceRegistry::new(pubsub);
///
/// // Register a service
/// registry.register(service_ref, vec!["PointValueChanged"]).await?;
///
/// // Route an event
/// registry.route_event(&event).await;
///
/// // Send a request
/// let response = registry.request("history", ServiceRequest::GetStatus).await?;
/// ```
#[derive(kameo::Actor)]
pub struct ServiceRegistry {
    /// Registered services
    services: Arc<RwLock<HashMap<String, ServiceRegistration>>>,
    /// PubSub broker for event distribution (reserved for future use)
    #[allow(dead_code)]
    pubsub: kameo::actor::ActorRef<PubSubBroker>,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new(pubsub: kameo::actor::ActorRef<PubSubBroker>) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            pubsub,
        }
    }

    /// Register a service with optional event subscriptions
    ///
    /// # Arguments
    /// * `actor_ref` - The service actor reference
    /// * `subscriptions` - Event patterns to subscribe to ("*" for all events)
    pub async fn register(
        &self,
        actor_ref: ServiceActorRef,
        subscriptions: Vec<String>,
    ) -> Result<()> {
        let id = actor_ref.id();
        let name = actor_ref.name();

        let mut services = self.services.write().await;

        if services.contains_key(&id) {
            return Err(Error::Service(format!(
                "Service '{}' already registered",
                id
            )));
        }

        services.insert(
            id.clone(),
            ServiceRegistration {
                actor_ref,
                subscriptions,
            },
        );
        tracing::info!("Registered service: {} ({})", name, id);

        Ok(())
    }

    /// Unregister a service by ID
    ///
    /// Stops the service before removing it from the registry.
    pub async fn unregister(&self, id: &str) -> Result<Option<ServiceActorRef>> {
        let removed = self.services.write().await.remove(id);

        if let Some(reg) = removed {
            // Stop the service
            if let Err(e) = reg.actor_ref.tell(ServiceMsg::Stop).await {
                tracing::warn!("Error stopping service '{}': {}", id, e);
            }
            tracing::info!("Unregistered service: {}", id);
            Ok(Some(reg.actor_ref))
        } else {
            Ok(None)
        }
    }

    /// Get a service by ID
    pub async fn get(&self, id: &str) -> Option<ServiceActorRef> {
        self.services
            .read()
            .await
            .get(id)
            .map(|r| r.actor_ref.clone())
    }

    /// List all registered services
    pub async fn list(&self) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        let mut result = Vec::with_capacity(services.len());

        for reg in services.values() {
            // Get state from actor
            let state = match reg.actor_ref.ask(ServiceMsg::GetStatus).await {
                Ok(ServiceReply::Status { state, .. }) => state,
                _ => ServiceState::Stopped,
            };

            result.push(ServiceInfo {
                id: reg.actor_ref.id(),
                name: reg.actor_ref.name(),
                description: reg.actor_ref.description(),
                service_type: reg.actor_ref.service_type(),
                state,
            });
        }

        result
    }

    /// Start all registered services
    pub async fn start_all(&self) -> Result<()> {
        let services = self.services.read().await;

        for (id, reg) in services.iter() {
            tracing::info!("Starting service: {}", id);
            match reg.actor_ref.ask(ServiceMsg::Start).await {
                Ok(ServiceReply::Started) => {
                    tracing::info!("Service '{}' started successfully", id);
                }
                Ok(ServiceReply::Failed(reason)) => {
                    tracing::error!("Service '{}' failed to start: {}", id, reason);
                }
                Ok(other) => {
                    tracing::warn!("Service '{}' returned unexpected reply: {:?}", id, other);
                }
                Err(e) => {
                    tracing::error!("Failed to communicate with service '{}': {}", id, e);
                }
            }
        }

        Ok(())
    }

    /// Stop all registered services
    pub async fn stop_all(&self) -> Result<()> {
        let services = self.services.read().await;

        for (id, reg) in services.iter() {
            tracing::info!("Stopping service: {}", id);
            if let Err(e) = reg.actor_ref.ask(ServiceMsg::Stop).await {
                tracing::error!("Failed to stop service '{}': {}", id, e);
            }
        }

        Ok(())
    }

    /// Route an event to all subscribed services
    ///
    /// Checks each service's subscription patterns against the event type
    /// and sends the event to matching services.
    pub async fn route_event(&self, event: &Event) {
        let event_type = Self::event_type_name(event);
        let services = self.services.read().await;

        for (id, reg) in services.iter() {
            // Check if service is subscribed to this event type
            let subscribed = reg.subscriptions.iter().any(|pattern| {
                pattern == "*" || WildMatch::new(pattern).matches(&event_type)
            });

            if subscribed {
                if let Err(e) = reg
                    .actor_ref
                    .tell(ServiceMsg::OnEvent {
                        event: event.clone(),
                    })
                    .await
                {
                    tracing::warn!("Service '{}' failed to handle event: {}", id, e);
                }
            }
        }
    }

    /// Send a request to a specific service
    pub async fn request(
        &self,
        service_id: &str,
        request: ServiceRequest,
    ) -> Result<ServiceResponse> {
        let services = self.services.read().await;

        if let Some(reg) = services.get(service_id) {
            let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
            if let Err(e) = reg
                .actor_ref
                .tell(ServiceMsg::HandleRequest {
                    request,
                    reply: reply_tx,
                })
                .await
            {
                return Ok(ServiceResponse::Error {
                    code: "ACTOR_ERROR".to_string(),
                    message: format!("Failed to send to actor: {}", e),
                });
            }
            match reply_rx.await {
                Ok(response) => Ok(response),
                Err(_) => Ok(ServiceResponse::Error {
                    code: "NO_RESPONSE".to_string(),
                    message: "Service did not respond".to_string(),
                }),
            }
        } else {
            Ok(ServiceResponse::Error {
                code: "SERVICE_NOT_FOUND".to_string(),
                message: format!("Service '{}' not found", service_id),
            })
        }
    }

    /// Get the event type name for pattern matching
    fn event_type_name(event: &Event) -> String {
        match event {
            Event::PointValueChanged { .. } => "PointValueChanged".to_string(),
            Event::AlarmRaised { .. } => "AlarmRaised".to_string(),
            Event::AlarmCleared { .. } => "AlarmCleared".to_string(),
            Event::DeviceStatusChanged { .. } => "DeviceStatusChanged".to_string(),
            Event::DeviceDiscovered { .. } => "DeviceDiscovered".to_string(),
            Event::ServiceStateChanged { .. } => "ServiceStateChanged".to_string(),
            Event::Custom { event_type, .. } => event_type.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Actor Message Handling
// ─────────────────────────────────────────────────────────────────────────────

/// Messages for the ServiceRegistry actor
pub enum RegistryMsg {
    /// Register a new service
    Register {
        actor_ref: ServiceActorRef,
        subscriptions: Vec<String>,
    },
    /// Unregister a service by ID
    Unregister { id: String },
    /// Get a service by ID
    Get { id: String },
    /// List all services
    List,
    /// Start all services
    StartAll,
    /// Stop all services
    StopAll,
    /// Route an event to subscribed services
    RouteEvent { event: Event },
    /// Send a request to a specific service
    Request {
        service_id: String,
        request: ServiceRequest,
    },
}

/// Replies from the ServiceRegistry actor
#[derive(kameo::Reply)]
pub enum RegistryReply {
    /// Service was registered successfully
    Registered,
    /// Service was unregistered (true if found)
    Unregistered(bool),
    /// Service lookup result
    GotService(Option<ServiceActorRef>),
    /// List of all services
    ServiceList(Vec<ServiceInfo>),
    /// All services started
    Started,
    /// All services stopped
    Stopped,
    /// Event was routed
    EventRouted,
    /// Response from a service request
    Response(ServiceResponse),
    /// An error occurred
    Failed(String),
}

impl kameo::message::Message<RegistryMsg> for ServiceRegistry {
    type Reply = RegistryReply;

    async fn handle(
        &mut self,
        msg: RegistryMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            RegistryMsg::Register {
                actor_ref,
                subscriptions,
            } => match self.register(actor_ref, subscriptions).await {
                Ok(_) => RegistryReply::Registered,
                Err(e) => RegistryReply::Failed(e.to_string()),
            },

            RegistryMsg::Unregister { id } => match self.unregister(&id).await {
                Ok(Some(_)) => RegistryReply::Unregistered(true),
                Ok(None) => RegistryReply::Unregistered(false),
                Err(e) => RegistryReply::Failed(e.to_string()),
            },

            RegistryMsg::Get { id } => RegistryReply::GotService(self.get(&id).await),

            RegistryMsg::List => RegistryReply::ServiceList(self.list().await),

            RegistryMsg::StartAll => match self.start_all().await {
                Ok(_) => RegistryReply::Started,
                Err(e) => RegistryReply::Failed(e.to_string()),
            },

            RegistryMsg::StopAll => match self.stop_all().await {
                Ok(_) => RegistryReply::Stopped,
                Err(e) => RegistryReply::Failed(e.to_string()),
            },

            RegistryMsg::RouteEvent { event } => {
                self.route_event(&event).await;
                RegistryReply::EventRouted
            }

            RegistryMsg::Request {
                service_id,
                request,
            } => match self.request(&service_id, request).await {
                Ok(response) => RegistryReply::Response(response),
                Err(e) => RegistryReply::Failed(e.to_string()),
            },
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Debug implementations
// ─────────────────────────────────────────────────────────────────────────────

impl std::fmt::Debug for ServiceRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceRegistration")
            .field("service_id", &self.actor_ref.id())
            .field("subscriptions", &self.subscriptions)
            .finish()
    }
}

impl std::fmt::Debug for RegistryReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registered => write!(f, "Registered"),
            Self::Unregistered(found) => write!(f, "Unregistered({})", found),
            Self::GotService(s) => {
                write!(f, "GotService({:?})", s.as_ref().map(|s| s.id()))
            }
            Self::ServiceList(list) => write!(f, "ServiceList({} services)", list.len()),
            Self::Started => write!(f, "Started"),
            Self::Stopped => write!(f, "Stopped"),
            Self::EventRouted => write!(f, "EventRouted"),
            Self::Response(r) => write!(f, "Response({:?})", r),
            Self::Failed(e) => write!(f, "Failed({})", e),
        }
    }
}

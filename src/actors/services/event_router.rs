// Event Router - Bridges PubSub events to the ServiceRegistry
//
// This actor subscribes to the PubSub broker and forwards all events
// to the ServiceRegistry for routing to subscribed services.

use kameo::actor::ActorRef;
use kameo_actors::pubsub::Subscribe;

use crate::actors::PubSubBroker;
use crate::messages::Event;
use crate::services::{RegistryMsg, ServiceRegistry};

/// Actor that routes events from PubSub to the ServiceRegistry
#[derive(kameo::Actor)]
pub struct EventRouter {
    registry: ActorRef<ServiceRegistry>,
}

impl EventRouter {
    /// Create a new EventRouter
    pub fn new(registry: ActorRef<ServiceRegistry>) -> Self {
        Self { registry }
    }

    /// Subscribe this router to the PubSub broker
    pub async fn subscribe(
        actor_ref: ActorRef<Self>,
        pubsub: &ActorRef<PubSubBroker>,
    ) -> Result<(), kameo::error::SendError<Subscribe<Self>, kameo::error::Infallible>> {
        pubsub.tell(Subscribe(actor_ref)).await
    }
}

// Handle events from PubSub
impl kameo::message::Message<Event> for EventRouter {
    type Reply = ();

    async fn handle(
        &mut self,
        event: Event,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Forward event to service registry for routing
        if let Err(e) = self.registry.tell(RegistryMsg::RouteEvent { event }).await {
            tracing::warn!("Failed to route event to registry: {}", e);
        }
    }
}

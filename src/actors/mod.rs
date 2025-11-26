pub mod station;

pub mod bacnet;
pub mod modbus;
pub mod services;

// Re-exports
use crate::messages::Event;
pub use station::StationActor;
pub use services::EventRouter;

// Use Kameo's built-in PubSub for events
pub type PubSubBroker = kameo_actors::pubsub::PubSub<Event>;

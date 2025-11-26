// Neo Service System
//
// All services are actor-based using kameo actors. Services implement
// Message<ServiceMsg> for common lifecycle operations and can implement
// custom message types for service-specific functionality.

pub mod actor;
pub mod builtin;
pub mod messages;
pub mod plugin;
pub mod registry;

// Re-exports - Actor infrastructure
pub use actor::{
    ServiceActorRef, ServiceMetadata, ServiceMsg, ServiceReply, ServiceStateTracker,
    ServiceType as ActorServiceType,
};

// Re-exports - Common types
pub use crate::types::ServiceState;
pub use messages::{ServiceRequest, ServiceResponse};

// Re-exports - Registry
pub use registry::{RegistryMsg, RegistryReply, ServiceInfo, ServiceRegistration, ServiceRegistry};

// Re-exports - Built-in services
pub use builtin::{AlarmActor, AlarmCondition, AlarmConfig, AlarmMsg, AlarmReply};
pub use builtin::{HistoryActor, HistoryConfig, HistoryMsg, HistoryReply};

// Re-exports - Plugin services
pub use plugin::{
    discover_plugins, load_plugins, JsRuntimePoolActor, PluginActor, PluginManifest, PluginMsg,
    PluginReply, PoolMsg,
};

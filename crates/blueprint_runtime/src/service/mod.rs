//! Service Lifecycle Management
//!
//! This module provides the core service abstraction for Neo. Services are
//! long-running components that can be implemented in Rust, JavaScript, or Blueprints.
//!
//! # Lifecycle
//!
//! Services have the following lifecycle hooks:
//! - `on_start`: Called when the service starts
//! - `on_stop`: Called when the service stops
//! - `on_event`: Called when an event matching subscriptions arrives
//! - `on_tick`: Called periodically if tick_interval is set

mod event;
mod handle;
mod manager;

pub use event::*;
pub use handle::*;
pub use manager::*;

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Service Error
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during service lifecycle
#[derive(Debug, Clone, thiserror::Error)]
pub enum ServiceError {
    #[error("Service initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Service already running: {0}")]
    AlreadyRunning(String),

    #[error("Service not running: {0}")]
    NotRunning(String),

    #[error("Event handling failed: {0}")]
    EventError(String),

    #[error("Shutdown timeout")]
    ShutdownTimeout,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for service operations
pub type ServiceResult<T> = Result<T, ServiceError>;

// ─────────────────────────────────────────────────────────────────────────────
// Service Specification
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    /// Unique service identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Optional tick interval for periodic on_tick calls
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_interval: Option<Duration>,

    /// Event patterns this service subscribes to (e.g., "PointValueChanged", "Device/*")
    #[serde(default)]
    pub subscriptions: Vec<String>,

    /// Whether only one instance of this service can run
    #[serde(default = "default_singleton")]
    pub singleton: bool,

    /// Timeout for graceful shutdown
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout: Duration,

    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_singleton() -> bool {
    true
}

fn default_shutdown_timeout() -> Duration {
    Duration::from_secs(30)
}

impl Default for ServiceSpec {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            tick_interval: None,
            subscriptions: Vec::new(),
            singleton: true,
            shutdown_timeout: Duration::from_secs(30),
            description: None,
        }
    }
}

impl ServiceSpec {
    /// Create a new service spec with required fields
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set tick interval
    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = Some(interval);
        self
    }

    /// Add event subscriptions
    pub fn with_subscriptions(mut self, subscriptions: Vec<String>) -> Self {
        self.subscriptions = subscriptions;
        self
    }

    /// Add a single subscription
    pub fn subscribe(mut self, pattern: impl Into<String>) -> Self {
        self.subscriptions.push(pattern.into());
        self
    }

    /// Set singleton flag
    pub fn singleton(mut self, singleton: bool) -> Self {
        self.singleton = singleton;
        self
    }

    /// Set shutdown timeout
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Context
// ─────────────────────────────────────────────────────────────────────────────

/// Context passed to service lifecycle methods
pub struct ServiceContext {
    /// Service unique identifier
    pub service_id: String,

    /// Service configuration
    pub config: serde_json::Value,

    /// Handle to publish events
    event_publisher: EventPublisher,
}

impl ServiceContext {
    /// Create a new service context
    pub fn new(
        service_id: String,
        config: serde_json::Value,
        event_publisher: EventPublisher,
    ) -> Self {
        Self {
            service_id,
            config,
            event_publisher,
        }
    }

    /// Publish an event to all subscribed services
    pub fn publish(&self, event: Event) -> ServiceResult<()> {
        self.event_publisher.publish(event)
    }

    /// Create and publish an event
    pub fn emit(&self, event_type: impl Into<String>, data: serde_json::Value) -> ServiceResult<()> {
        let event = Event::new(event_type, &self.service_id, data);
        self.publish(event)
    }

    /// Get a config value as a specific type
    pub fn get_config<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.config.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get a config string
    pub fn get_config_string(&self, key: &str) -> Option<String> {
        self.config.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    /// Get a config number
    pub fn get_config_f64(&self, key: &str) -> Option<f64> {
        self.config.get(key).and_then(|v| v.as_f64())
    }

    /// Get a config boolean
    pub fn get_config_bool(&self, key: &str) -> Option<bool> {
        self.config.get(key).and_then(|v| v.as_bool())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Trait
// ─────────────────────────────────────────────────────────────────────────────

/// The core Service trait with lifecycle hooks
///
/// Implement this trait to create a service that can be managed by the ServiceManager.
/// Services can be implemented in Rust, JavaScript (via JsService), or Blueprints.
#[async_trait]
pub trait Service: Send + Sync + 'static {
    /// Returns the service specification
    fn spec(&self) -> ServiceSpec;

    /// Called when the service starts
    ///
    /// This is where you should initialize resources, start background tasks, etc.
    /// If this returns an error, the service will not be started.
    async fn on_start(&mut self, ctx: &ServiceContext) -> ServiceResult<()>;

    /// Called when the service stops
    ///
    /// This is where you should clean up resources, stop background tasks, etc.
    /// The service will be stopped even if this returns an error.
    async fn on_stop(&mut self, ctx: &ServiceContext) -> ServiceResult<()>;

    /// Called when an event matching subscriptions arrives
    ///
    /// Override this to handle events. The default implementation does nothing.
    async fn on_event(&mut self, _ctx: &ServiceContext, _event: Event) -> ServiceResult<()> {
        Ok(())
    }

    /// Called periodically if tick_interval is set in the spec
    ///
    /// Override this to perform periodic tasks. The default implementation does nothing.
    async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        started: bool,
        stopped: bool,
        events_received: usize,
        ticks: usize,
    }

    impl TestService {
        fn new() -> Self {
            Self {
                started: false,
                stopped: false,
                events_received: 0,
                ticks: 0,
            }
        }
    }

    #[async_trait]
    impl Service for TestService {
        fn spec(&self) -> ServiceSpec {
            ServiceSpec::new("test-service", "Test Service")
                .with_tick_interval(Duration::from_millis(100))
                .subscribe("TestEvent")
        }

        async fn on_start(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
            self.started = true;
            Ok(())
        }

        async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
            self.stopped = true;
            Ok(())
        }

        async fn on_event(&mut self, _ctx: &ServiceContext, _event: Event) -> ServiceResult<()> {
            self.events_received += 1;
            Ok(())
        }

        async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
            self.ticks += 1;
            Ok(())
        }
    }

    #[test]
    fn test_service_spec_builder() {
        let spec = ServiceSpec::new("my-service", "My Service")
            .with_tick_interval(Duration::from_secs(1))
            .subscribe("Event1")
            .subscribe("Event2")
            .singleton(true);

        assert_eq!(spec.id, "my-service");
        assert_eq!(spec.name, "My Service");
        assert_eq!(spec.tick_interval, Some(Duration::from_secs(1)));
        assert_eq!(spec.subscriptions, vec!["Event1", "Event2"]);
        assert!(spec.singleton);
    }
}

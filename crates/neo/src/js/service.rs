//! JavaScript Service Implementation
//!
//! Provides a Service implementation that runs JavaScript code via QuickJS.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use blueprint_runtime::service::{
    Event, Service, ServiceContext, ServiceError, ServiceResult, ServiceSpec,
};
use blueprint_types::TypeRegistry;

use super::runtime::{JsError, JsRuntime};

// ─────────────────────────────────────────────────────────────────────────────
// JavaScript Service Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Configuration for a JavaScript service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsServiceConfig {
    /// Unique service identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// JavaScript source code
    pub source: String,

    /// Optional tick interval in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_interval_ms: Option<u64>,

    /// Event patterns to subscribe to
    #[serde(default)]
    pub subscriptions: Vec<String>,

    /// Whether only one instance can run
    #[serde(default = "default_true")]
    pub singleton: bool,

    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

impl JsServiceConfig {
    /// Create a new JS service config
    pub fn new(id: impl Into<String>, name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            source: source.into(),
            tick_interval_ms: None,
            subscriptions: Vec::new(),
            singleton: true,
            description: None,
        }
    }

    /// Set tick interval
    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval_ms = Some(interval.as_millis() as u64);
        self
    }

    /// Add event subscriptions
    pub fn with_subscriptions(mut self, subscriptions: Vec<String>) -> Self {
        self.subscriptions = subscriptions;
        self
    }

    /// Subscribe to an event pattern
    pub fn subscribe(mut self, pattern: impl Into<String>) -> Self {
        self.subscriptions.push(pattern.into());
        self
    }

    /// Set singleton flag
    pub fn singleton(mut self, singleton: bool) -> Self {
        self.singleton = singleton;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// JavaScript Service
// ─────────────────────────────────────────────────────────────────────────────

/// A service implemented in JavaScript
///
/// This wraps a QuickJS runtime and calls JavaScript lifecycle functions:
/// - `onStart()` - Called when service starts
/// - `onStop()` - Called when service stops
/// - `onEvent(event)` - Called for each subscribed event
/// - `onTick()` - Called periodically if tick_interval is set
pub struct JsService {
    /// Service configuration
    config: JsServiceConfig,

    /// The QuickJS runtime (created on start)
    runtime: Option<JsRuntime>,

    /// Type registry for user-defined types
    type_registry: Option<Arc<TypeRegistry>>,
}

impl JsService {
    /// Create a new JavaScript service
    pub fn new(config: JsServiceConfig) -> Self {
        Self {
            config,
            runtime: None,
            type_registry: None,
        }
    }

    /// Create a new JavaScript service with a type registry
    pub fn with_type_registry(config: JsServiceConfig, type_registry: Arc<TypeRegistry>) -> Self {
        Self {
            config,
            runtime: None,
            type_registry: Some(type_registry),
        }
    }

    /// Load JavaScript service from source code
    pub fn from_source(
        id: impl Into<String>,
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::new(JsServiceConfig::new(id, name, source))
    }

    /// Check if a lifecycle function exists
    fn has_function(&self, name: &str) -> bool {
        self.runtime
            .as_ref()
            .map(|rt| rt.has_function(name))
            .unwrap_or(false)
    }

    /// Convert JsError to ServiceError
    fn map_js_error(err: JsError) -> ServiceError {
        ServiceError::Internal(format!("JavaScript error: {}", err))
    }
}

#[async_trait]
impl Service for JsService {
    fn spec(&self) -> ServiceSpec {
        let mut spec = ServiceSpec::new(&self.config.id, &self.config.name)
            .with_subscriptions(self.config.subscriptions.clone())
            .singleton(self.config.singleton);

        if let Some(ms) = self.config.tick_interval_ms {
            spec = spec.with_tick_interval(Duration::from_millis(ms));
        }

        if let Some(ref desc) = self.config.description {
            spec = spec.with_description(desc.clone());
        }

        spec
    }

    async fn on_start(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        // Create the JavaScript runtime
        let runtime = if let Some(ref registry) = self.type_registry {
            JsRuntime::with_type_registry(registry.clone())
                .map_err(Self::map_js_error)?
        } else {
            JsRuntime::new().map_err(Self::map_js_error)?
        };

        // Evaluate the service source code
        runtime
            .eval_file(&self.config.id, &self.config.source)
            .map_err(Self::map_js_error)?;

        self.runtime = Some(runtime);

        // Call onStart if it exists
        if self.has_function("onStart") {
            let runtime = self.runtime.as_ref().unwrap();

            // Pass config to onStart
            runtime
                .call_function_with_json("onStart", &ctx.config)
                .map_err(Self::map_js_error)?;
        }

        tracing::info!(service_id = %self.config.id, "JavaScript service started");
        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        // Call onStop if it exists
        if self.has_function("onStop") {
            if let Some(ref runtime) = self.runtime {
                runtime.call_function("onStop").map_err(Self::map_js_error)?;
            }
        }

        // Run garbage collection before dropping
        if let Some(ref runtime) = self.runtime {
            runtime.gc();
        }

        self.runtime = None;
        tracing::info!(service_id = %self.config.id, "JavaScript service stopped");
        Ok(())
    }

    async fn on_event(&mut self, _ctx: &ServiceContext, event: Event) -> ServiceResult<()> {
        if !self.has_function("onEvent") {
            return Ok(());
        }

        let runtime = self.runtime.as_ref().ok_or_else(|| {
            ServiceError::Internal("Runtime not initialized".to_string())
        })?;

        // Call onEvent with the event object
        runtime
            .call_with_event("onEvent", &event.event_type, &event.source, &event.data)
            .map_err(Self::map_js_error)?;

        Ok(())
    }

    async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        if !self.has_function("onTick") {
            return Ok(());
        }

        let runtime = self.runtime.as_ref().ok_or_else(|| {
            ServiceError::Internal("Runtime not initialized".to_string())
        })?;

        runtime.call_function("onTick").map_err(Self::map_js_error)?;

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_service_config() {
        let config = JsServiceConfig::new("my-service", "My Service", "function onStart() {}")
            .with_tick_interval(Duration::from_secs(1))
            .subscribe("TestEvent")
            .with_description("A test service");

        assert_eq!(config.id, "my-service");
        assert_eq!(config.name, "My Service");
        assert_eq!(config.tick_interval_ms, Some(1000));
        assert_eq!(config.subscriptions, vec!["TestEvent"]);
        assert_eq!(config.description, Some("A test service".to_string()));
    }

    #[test]
    fn test_js_service_spec() {
        let config = JsServiceConfig::new("test-js", "Test JS", "")
            .with_tick_interval(Duration::from_millis(500))
            .subscribe("Event1")
            .subscribe("Event2");

        let service = JsService::new(config);
        let spec = service.spec();

        assert_eq!(spec.id, "test-js");
        assert_eq!(spec.name, "Test JS");
        assert_eq!(spec.tick_interval, Some(Duration::from_millis(500)));
        assert_eq!(spec.subscriptions, vec!["Event1", "Event2"]);
    }

    #[tokio::test]
    async fn test_js_service_lifecycle() {
        let source = r#"
            let started = false;
            let stopped = false;
            let eventCount = 0;
            let tickCount = 0;

            function onStart(config) {
                started = true;
                neo.log("Service started with config: " + JSON.stringify(config));
            }

            function onStop() {
                stopped = true;
                neo.log("Service stopped");
            }

            function onEvent(event) {
                eventCount++;
                neo.log("Received event: " + event.type);
            }

            function onTick() {
                tickCount++;
            }
        "#;

        let config = JsServiceConfig::new("test-lifecycle", "Test Lifecycle", source)
            .subscribe("TestEvent");

        let mut service = JsService::new(config);

        // Create a mock context
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        let publisher = blueprint_runtime::service::EventPublisher::new(tx);
        let ctx = ServiceContext::new(
            "test-lifecycle".to_string(),
            serde_json::json!({"key": "value"}),
            publisher,
        );

        // Test lifecycle
        service.on_start(&ctx).await.unwrap();
        assert!(service.runtime.is_some());

        // Test event
        let event = Event::new("TestEvent", "test-source", serde_json::json!({}));
        service.on_event(&ctx, event).await.unwrap();

        // Test tick
        service.on_tick(&ctx).await.unwrap();

        // Stop
        service.on_stop(&ctx).await.unwrap();
        assert!(service.runtime.is_none());
    }
}

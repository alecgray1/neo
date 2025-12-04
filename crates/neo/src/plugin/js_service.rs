//! JavaScript Service
//!
//! A service implemented in JavaScript, running in a dedicated thread
//! using neo-js-runtime (Deno/V8).
//!
//! Each service runs in its own runtime with its own V8 isolate.
//! The JS runtime handles its own event loop - use setInterval/setTimeout
//! for periodic work instead of Rust-side ticking.

use std::sync::Arc;

use async_trait::async_trait;

use blueprint_runtime::service::{
    Event, Service, ServiceContext, ServiceError, ServiceResult, ServiceSpec,
};
use neo_js_runtime::{spawn_service_runtime, RuntimeHandle, RuntimeServices, ServiceMode};

/// Configuration for a JavaScript service
#[derive(Debug, Clone)]
pub struct JsServiceConfig {
    /// Service ID (e.g., "example/ticker")
    pub id: String,
    /// Service name
    pub name: String,
    /// JavaScript source code (the service chunk)
    pub code: String,
    /// Event patterns to subscribe to
    pub subscriptions: Vec<String>,
    /// Service configuration data
    pub config: serde_json::Value,
}

impl JsServiceConfig {
    /// Create a new JS service config
    pub fn new(id: impl Into<String>, name: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            code: code.into(),
            subscriptions: Vec::new(),
            config: serde_json::Value::Object(Default::default()),
        }
    }

    /// Add event subscriptions
    pub fn with_subscriptions(mut self, subs: Vec<String>) -> Self {
        self.subscriptions = subs;
        self
    }

    /// Set service config
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }
}

/// A service implemented in JavaScript
///
/// Each JsService runs in its own thread with its own V8 isolate.
/// The JS code should use `export default defineService({...})` with
/// lifecycle callbacks (onStart, onStop, onEvent).
/// For periodic work, use setInterval in JavaScript instead of on_tick.
pub struct JsService {
    config: JsServiceConfig,
    runtime: Option<Arc<RuntimeHandle<ServiceMode>>>,
}

impl JsService {
    /// Create a new JavaScript service
    pub fn new(config: JsServiceConfig) -> Self {
        Self {
            config,
            runtime: None,
        }
    }

    /// Get the service ID
    pub fn id(&self) -> &str {
        &self.config.id
    }
}

#[async_trait]
impl Service for JsService {
    fn spec(&self) -> ServiceSpec {
        ServiceSpec::new(&self.config.id, &self.config.name)
            .with_subscriptions(self.config.subscriptions.clone())
    }

    async fn on_start(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(service = %self.config.id, "Starting JS service");

        // Create runtime services (could inject event publisher, point store, etc.)
        let services = RuntimeServices::default();

        // Spawn the JS runtime with the service code
        // The runtime will load the service and assign it the ID
        let handle = spawn_service_runtime(
            format!("js:{}", self.config.id),
            self.config.code.clone(),
            self.config.id.clone(),
            services,
        )
        .map_err(|e| ServiceError::InitializationFailed(e.to_string()))?;

        let handle = Arc::new(handle);

        // Start the service (calls onStart in JS)
        handle
            .start_service()
            .await
            .map_err(|e| ServiceError::InitializationFailed(e.to_string()))?;

        self.runtime = Some(handle);

        tracing::info!(service = %self.config.id, "JS service started");
        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(service = %self.config.id, "Stopping JS service");

        if let Some(handle) = self.runtime.take() {
            // Stop JS service gracefully (calls onStop in JS)
            if let Err(e) = handle.stop_service().await {
                tracing::warn!(service = %self.config.id, error = %e, "Error stopping JS service");
            }

            handle.terminate();
        }

        tracing::info!(service = %self.config.id, "JS service stopped");
        Ok(())
    }

    async fn on_event(&mut self, _ctx: &ServiceContext, event: Event) -> ServiceResult<()> {
        if let Some(ref _runtime) = self.runtime {
            // TODO: Forward event to JS runtime
            tracing::debug!(
                service = %self.config.id,
                event_type = %event.event_type,
                "JS service received event (handler not yet implemented)"
            );
        }

        Ok(())
    }
}

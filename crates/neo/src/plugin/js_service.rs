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

        // Capture values for the blocking task
        let name = format!("js:{}", self.config.id);
        let code = self.config.code.clone();
        let id = self.config.id.clone();

        // Spawn the JS runtime in a blocking task to avoid blocking tokio's async workers.
        // spawn_service_runtime uses std::sync::mpsc which blocks while waiting for V8 init.
        let handle = tokio::task::spawn_blocking(move || {
            spawn_service_runtime(name, code, id, services)
        })
        .await
        .map_err(|e| ServiceError::InitializationFailed(format!("spawn task failed: {}", e)))?
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

//! JavaScript Service
//!
//! A service implemented in JavaScript, running in a dedicated thread
//! using neo-js-runtime (Deno/V8).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;

use blueprint_runtime::service::{
    Event, Service, ServiceContext, ServiceError, ServiceResult, ServiceSpec,
};
use neo_js_runtime::{spawn_runtime, RuntimeHandle, RuntimeServices};

/// Configuration for a JavaScript service
#[derive(Debug, Clone)]
pub struct JsServiceConfig {
    /// Service ID
    pub id: String,
    /// Service name
    pub name: String,
    /// JavaScript source code
    pub code: String,
    /// Target service ID in the JS code (for 1:1 model)
    /// If set, only this service's lifecycle methods are called
    pub target_service_id: Option<String>,
    /// Event patterns to subscribe to
    pub subscriptions: Vec<String>,
    /// Optional tick interval
    pub tick_interval: Option<Duration>,
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
            target_service_id: None,
            subscriptions: Vec::new(),
            tick_interval: None,
            config: serde_json::Value::Object(Default::default()),
        }
    }

    /// Set the target service ID (for 1:1 model)
    pub fn with_target_service_id(mut self, target_id: impl Into<String>) -> Self {
        self.target_service_id = Some(target_id.into());
        self
    }

    /// Add event subscriptions
    pub fn with_subscriptions(mut self, subs: Vec<String>) -> Self {
        self.subscriptions = subs;
        self
    }

    /// Set tick interval
    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = Some(interval);
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
/// The JS code should export lifecycle callbacks (onStart, onStop, onTick, onEvent).
pub struct JsService {
    config: JsServiceConfig,
    runtime: Option<Arc<Mutex<RuntimeHandle>>>,
    /// Pre-created runtime handle (used for first service after scan)
    pre_created_handle: Option<RuntimeHandle>,
}

impl JsService {
    /// Create a new JavaScript service
    pub fn new(config: JsServiceConfig) -> Self {
        Self {
            config,
            runtime: None,
            pre_created_handle: None,
        }
    }

    /// Create a JsService with an existing RuntimeHandle.
    ///
    /// This is used for the first service discovered during plugin scan,
    /// where we reuse the runtime that was already created for scanning.
    pub fn with_runtime(config: JsServiceConfig, handle: RuntimeHandle) -> Self {
        Self {
            config,
            runtime: None,
            pre_created_handle: Some(handle),
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
        let mut spec = ServiceSpec::new(&self.config.id, &self.config.name)
            .with_subscriptions(self.config.subscriptions.clone());

        if let Some(interval) = self.config.tick_interval {
            spec = spec.with_tick_interval(interval);
        }

        spec
    }

    async fn on_start(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(service = %self.config.id, "Starting JS service");

        // Use pre-created handle if available, otherwise spawn new runtime
        let handle = if let Some(h) = self.pre_created_handle.take() {
            tracing::debug!(service = %self.config.id, "Using pre-created runtime handle");
            h
        } else {
            tracing::debug!(service = %self.config.id, "Spawning new runtime");
            // Create runtime services (could inject event publisher, point store, etc.)
            let services = RuntimeServices::default();

            // Spawn the JS runtime
            spawn_runtime(
                format!("js:{}", self.config.id),
                self.config.code.clone(),
                services,
            )
            .map_err(|e| ServiceError::InitializationFailed(e.to_string()))?
        };

        // Start the JS service
        let target_id = self.config.target_service_id.as_ref().ok_or_else(|| {
            ServiceError::InitializationFailed("target_service_id is required".to_string())
        })?;
        handle
            .start_service(target_id)
            .await
            .map_err(|e| ServiceError::InitializationFailed(e.to_string()))?;

        self.runtime = Some(Arc::new(Mutex::new(handle)));

        tracing::info!(service = %self.config.id, "JS service started");
        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(service = %self.config.id, "Stopping JS service");

        if let Some(runtime) = self.runtime.take() {
            let handle = runtime.lock().await;

            // Stop JS service gracefully
            let stop_result = if let Some(ref target_id) = self.config.target_service_id {
                handle.stop_service(target_id).await
            } else {
                Ok(()) // No target_service_id means nothing to stop
            };

            if let Err(e) = stop_result {
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

    async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        if let Some(ref runtime) = self.runtime {
            let handle = runtime.lock().await;

            // Tick the target service
            let tick_result = if let Some(ref target_id) = self.config.target_service_id {
                handle.tick_service(target_id).await
            } else {
                Ok(()) // No target_service_id means nothing to tick
            };

            if let Err(e) = tick_result {
                tracing::warn!(service = %self.config.id, error = %e, "Error during JS service tick");
            }
        }

        Ok(())
    }
}

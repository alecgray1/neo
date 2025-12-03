//! JavaScript plugin service using in-process thread-based runtime
//!
//! Plugins run in dedicated threads using neo-js-runtime, providing
//! better performance and simpler communication than process isolation.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Mutex;

use blueprint_runtime::service::{
    Event, Service, ServiceContext, ServiceError, ServiceResult, ServiceSpec,
};
use neo_js_runtime::{spawn_runtime, RuntimeHandle, RuntimeServices};

/// Configuration for a JavaScript plugin service
#[derive(Debug, Clone)]
pub struct JsPluginConfig {
    /// Plugin ID
    pub id: String,
    /// Plugin name
    pub name: String,
    /// JavaScript source code
    pub code: String,
    /// Event patterns to subscribe to
    pub subscriptions: Vec<String>,
    /// Optional tick interval
    pub tick_interval: Option<Duration>,
    /// Plugin configuration data
    pub config: serde_json::Value,
}

impl JsPluginConfig {
    /// Create a new plugin config
    pub fn new(id: impl Into<String>, name: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            code: code.into(),
            subscriptions: Vec::new(),
            tick_interval: None,
            config: serde_json::Value::Object(Default::default()),
        }
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

    /// Set plugin config
    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }
}

/// A JavaScript plugin service that runs in a dedicated thread
pub struct JsPluginService {
    config: JsPluginConfig,
    runtime: Option<Arc<Mutex<RuntimeHandle>>>,
}

impl JsPluginService {
    /// Create a new JavaScript plugin service
    pub fn new(config: JsPluginConfig) -> Self {
        Self {
            config,
            runtime: None,
        }
    }

    /// Get the plugin ID
    pub fn id(&self) -> &str {
        &self.config.id
    }
}

#[async_trait]
impl Service for JsPluginService {
    fn spec(&self) -> ServiceSpec {
        let mut spec = ServiceSpec::new(&self.config.id, &self.config.name)
            .with_subscriptions(self.config.subscriptions.clone());

        if let Some(interval) = self.config.tick_interval {
            spec = spec.with_tick_interval(interval);
        }

        spec
    }

    async fn on_start(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(plugin = %self.config.id, "Starting JavaScript plugin");

        // Create runtime services (could inject event publisher, point store, etc.)
        let services = RuntimeServices::default();

        // Spawn the JS runtime
        let handle = spawn_runtime(
            format!("plugin:{}", self.config.id),
            self.config.code.clone(),
            services,
        )
        .map_err(|e| ServiceError::InitializationFailed(e.to_string()))?;

        self.runtime = Some(Arc::new(Mutex::new(handle)));

        tracing::info!(plugin = %self.config.id, "JavaScript plugin started");
        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(plugin = %self.config.id, "Stopping JavaScript plugin");

        if let Some(runtime) = self.runtime.take() {
            let handle = runtime.lock().await;
            handle.terminate();
        }

        tracing::info!(plugin = %self.config.id, "JavaScript plugin stopped");
        Ok(())
    }

    async fn on_event(&mut self, _ctx: &ServiceContext, event: Event) -> ServiceResult<()> {
        if let Some(ref runtime) = self.runtime {
            let handle = runtime.lock().await;

            // Call the plugin's onEvent handler if it exists
            let event_json = serde_json::to_string(&serde_json::json!({
                "type": event.event_type,
                "source": event.source,
                "data": event.data,
                "timestamp": event.timestamp,
            }))
            .map_err(|e| ServiceError::EventError(e.to_string()))?;

            // Execute __neo_internal.onEvent if registered
            let script = format!(
                r#"(async () => {{
                    if (globalThis.__neo_internal && globalThis.__neo_internal.onEvent) {{
                        await globalThis.__neo_internal.onEvent({});
                    }}
                }})()"#,
                event_json
            );

            // We can't easily run arbitrary scripts through the handle,
            // so for now we just log the event. Full event handling would
            // require extending the runtime command protocol.
            tracing::debug!(
                plugin = %self.config.id,
                event_type = %event.event_type,
                "Plugin received event (handler not yet implemented)"
            );
        }

        Ok(())
    }

    async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        if let Some(ref runtime) = self.runtime {
            let handle = runtime.lock().await;

            // Similar to on_event, we'd need to extend the runtime to support tick callbacks
            tracing::trace!(plugin = %self.config.id, "Plugin tick");
        }

        Ok(())
    }
}

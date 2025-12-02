//! Process-isolated plugin service
//!
//! Spawns a neo-plugin-host subprocess for each plugin, providing
//! true OS-level fault isolation.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use blueprint_runtime::service::{Event, Service, ServiceContext, ServiceError, ServiceResult, ServiceSpec};

use super::ipc::{
    EmitMessage, IpcChannel, IpcReader, IpcWriter, LogMessage, MessageType, PluginMessage,
    PointReadRequest, PointResponse, PointWriteRequest,
};
use super::supervisor::{RestartPolicy, Supervisor};

/// Configuration for a process-isolated plugin
#[derive(Debug, Clone)]
pub struct ProcessServiceConfig {
    /// Unique service identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Path to the plugin JavaScript file
    pub plugin_path: PathBuf,
    /// Plugin configuration (passed to onStart)
    pub config: serde_json::Value,
    /// Event subscriptions
    pub subscriptions: Vec<String>,
    /// Tick interval (if any)
    pub tick_interval: Option<Duration>,
    /// Whether only one instance can run
    pub singleton: bool,
    /// Restart policy for crash recovery
    pub restart_policy: RestartPolicy,
    /// Path to neo-plugin-host binary (defaults to searching PATH)
    pub host_binary: Option<PathBuf>,
}

impl ProcessServiceConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, plugin_path: impl Into<PathBuf>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            plugin_path: plugin_path.into(),
            config: serde_json::json!({}),
            subscriptions: Vec::new(),
            tick_interval: None,
            singleton: true,
            restart_policy: RestartPolicy::default(),
            host_binary: None,
        }
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = config;
        self
    }

    pub fn with_subscriptions(mut self, subscriptions: Vec<String>) -> Self {
        self.subscriptions = subscriptions;
        self
    }

    pub fn with_tick_interval(mut self, interval: Duration) -> Self {
        self.tick_interval = Some(interval);
        self
    }

    pub fn singleton(mut self, singleton: bool) -> Self {
        self.singleton = singleton;
        self
    }

    pub fn with_restart_policy(mut self, policy: RestartPolicy) -> Self {
        self.restart_policy = policy;
        self
    }

    pub fn with_host_binary(mut self, path: PathBuf) -> Self {
        self.host_binary = Some(path);
        self
    }
}

/// A service that runs a JS/TS plugin in a separate process
pub struct ProcessService {
    config: ProcessServiceConfig,
    child: Option<Child>,
    /// Writer for sending messages to the plugin
    writer: Option<IpcWriter>,
    supervisor: Supervisor,
    /// Handle to the background IPC reader task
    reader_handle: Option<tokio::task::JoinHandle<()>>,
    /// Sender for the response channel (used by background reader)
    response_tx: Option<mpsc::Sender<PluginMessage>>,
}

impl ProcessService {
    pub fn new(config: ProcessServiceConfig) -> Self {
        let supervisor = Supervisor::new(config.restart_policy.clone());
        Self {
            config,
            child: None,
            writer: None,
            supervisor,
            reader_handle: None,
            response_tx: None,
        }
    }

    /// Get the path to the neo-plugin-host binary
    fn host_binary_path(&self) -> PathBuf {
        self.config.host_binary.clone().unwrap_or_else(|| {
            // Try to find in same directory as current executable
            if let Ok(exe) = std::env::current_exe() {
                let host_path = exe.parent().unwrap().join("neo-plugin-host");
                if host_path.exists() {
                    return host_path;
                }
            }
            // Fall back to PATH
            PathBuf::from("neo-plugin-host")
        })
    }

    /// Spawn the plugin host process
    async fn spawn_process(&mut self) -> ServiceResult<()> {
        let host_binary = self.host_binary_path();
        let plugin_path = &self.config.plugin_path;

        info!(
            "Spawning plugin process: {} {}",
            host_binary.display(),
            plugin_path.display()
        );

        let mut cmd = Command::new(&host_binary);
        cmd.arg(plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()); // Plugin stderr goes to our stderr

        let mut child = cmd.spawn().map_err(|e| {
            ServiceError::InitializationFailed(format!(
                "Failed to spawn plugin host: {}",
                e
            ))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ServiceError::InitializationFailed("Failed to get stdin".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ServiceError::InitializationFailed("Failed to get stdout".into())
        })?;

        let ipc = IpcChannel::new(stdin, stdout);
        let (writer, mut reader) = ipc.split();

        // Wait for Ready message
        let msg = tokio::time::timeout(Duration::from_secs(10), reader.recv())
            .await
            .map_err(|_| ServiceError::InitializationFailed("Timeout waiting for Ready".into()))?
            .map_err(|e| ServiceError::InitializationFailed(format!("IPC error: {}", e)))?
            .ok_or_else(|| ServiceError::InitializationFailed("Plugin closed before Ready".into()))?;

        if msg.msg_type != MessageType::Ready {
            return Err(ServiceError::InitializationFailed(format!(
                "Expected Ready, got {:?}",
                msg.msg_type
            )));
        }

        debug!("Plugin {} is ready", self.config.id);

        // Spawn background reader task
        let plugin_id = self.config.id.clone();
        let handle = tokio::spawn(async move {
            loop {
                match reader.recv().await {
                    Ok(Some(msg)) => {
                        match msg.msg_type {
                            MessageType::Log => {
                                if let Ok(log) = msg.parse_json::<LogMessage>() {
                                    match log.level.as_str() {
                                        "trace" => tracing::trace!(target: "plugin", "[{}] {}", plugin_id, log.message),
                                        "debug" => tracing::debug!(target: "plugin", "[{}] {}", plugin_id, log.message),
                                        "info" => tracing::info!(target: "plugin", "[{}] {}", plugin_id, log.message),
                                        "warn" => tracing::warn!(target: "plugin", "[{}] {}", plugin_id, log.message),
                                        "error" => tracing::error!(target: "plugin", "[{}] {}", plugin_id, log.message),
                                        _ => tracing::info!(target: "plugin", "[{}] {}", plugin_id, log.message),
                                    }
                                }
                            }
                            MessageType::Emit => {
                                // TODO: Need access to ServiceContext to publish events
                                if let Ok(emit) = msg.parse_json::<EmitMessage>() {
                                    debug!("Plugin {} emitted event: {}", plugin_id, emit.event_type);
                                }
                            }
                            MessageType::PointReadRequest => {
                                // TODO: Implement actual point reading
                                if let Ok(req) = msg.parse_json::<PointReadRequest>() {
                                    warn!("Plugin {} requested point read for '{}' - not yet implemented", plugin_id, req.point_id);
                                }
                            }
                            MessageType::PointWriteRequest => {
                                // TODO: Implement actual point writing
                                if let Ok(req) = msg.parse_json::<PointWriteRequest>() {
                                    warn!("Plugin {} requested point write for '{}' - not yet implemented", plugin_id, req.point_id);
                                }
                            }
                            MessageType::Error => {
                                if let Ok(text) = String::from_utf8(msg.payload.clone()) {
                                    error!("Plugin {} error: {}", plugin_id, text);
                                }
                            }
                            _ => {
                                warn!("Unexpected message type from plugin: {:?}", msg.msg_type);
                            }
                        }
                    }
                    Ok(None) => {
                        info!("Plugin {} closed connection", plugin_id);
                        break;
                    }
                    Err(e) => {
                        error!("Error reading from plugin {}: {}", plugin_id, e);
                        break;
                    }
                }
            }
        });

        self.child = Some(child);
        self.writer = Some(writer);
        self.reader_handle = Some(handle);
        self.supervisor.on_start();

        Ok(())
    }

    /// Send a message to the plugin
    async fn send(&mut self, msg: &PluginMessage) -> ServiceResult<()> {
        if let Some(ref mut writer) = self.writer {
            writer
                .send(msg)
                .await
                .map_err(|e| ServiceError::Internal(format!("IPC send error: {}", e)))
        } else {
            Err(ServiceError::NotRunning(self.config.id.clone()))
        }
    }
}

#[async_trait]
impl Service for ProcessService {
    fn spec(&self) -> ServiceSpec {
        ServiceSpec::new(&self.config.id, &self.config.name)
            .with_subscriptions(self.config.subscriptions.clone())
            .singleton(self.config.singleton)
            .with_tick_interval_opt(self.config.tick_interval)
    }

    async fn on_start(&mut self, ctx: &ServiceContext) -> ServiceResult<()> {
        // Spawn the plugin process
        self.spawn_process().await?;

        // Send Start message with config
        let start_payload = serde_json::json!({
            "config": self.config.config
        });
        let payload = serde_json::to_vec(&start_payload)
            .map_err(|e| ServiceError::Internal(format!("JSON error: {}", e)))?;

        self.send(&PluginMessage::new(MessageType::Start, payload)).await?;

        info!("Plugin {} started", self.config.id);
        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        // Send Stop message
        if self.writer.is_some() {
            let _ = self.send(&PluginMessage::empty(MessageType::Stop)).await;
        }

        // Abort the reader task
        if let Some(handle) = self.reader_handle.take() {
            handle.abort();
        }

        // Wait for process to exit gracefully
        if let Some(ref mut child) = self.child {
            match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
                Ok(Ok(status)) => {
                    info!("Plugin {} exited with status: {}", self.config.id, status);
                }
                Ok(Err(e)) => {
                    error!("Error waiting for plugin {}: {}", self.config.id, e);
                }
                Err(_) => {
                    warn!("Plugin {} did not exit in time, killing", self.config.id);
                    let _ = child.kill().await;
                }
            }
        }

        self.child = None;
        self.writer = None;
        self.response_tx = None;

        info!("Plugin {} stopped", self.config.id);
        Ok(())
    }

    async fn on_event(&mut self, _ctx: &ServiceContext, event: Event) -> ServiceResult<()> {
        let payload = serde_json::to_vec(&event)
            .map_err(|e| ServiceError::Internal(format!("JSON error: {}", e)))?;

        self.send(&PluginMessage::new(MessageType::Event, payload)).await
    }

    async fn on_tick(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        self.send(&PluginMessage::empty(MessageType::Tick)).await
    }
}

// Extension trait for ServiceSpec to handle optional tick interval
trait ServiceSpecExt {
    fn with_tick_interval_opt(self, interval: Option<Duration>) -> Self;
}

impl ServiceSpecExt for ServiceSpec {
    fn with_tick_interval_opt(self, interval: Option<Duration>) -> Self {
        match interval {
            Some(i) => self.with_tick_interval(i),
            None => self,
        }
    }
}

// PluginActor - Actor wrapper for JavaScript/TypeScript plugins
//
// This actor provides the ServiceMsg interface for plugins and delegates
// execution to the JsRuntimePool.

use std::path::PathBuf;

use kameo::actor::ActorRef;
use kameo::message::{Context, Message};
use tokio::sync::oneshot;

use crate::services::actor::{ServiceMsg, ServiceReply, ServiceStateTracker};
use crate::services::messages::{ServiceRequest, ServiceResponse};
use crate::types::ServiceState;

use super::pool::{JsRuntimePoolActor, PoolMsg};
use super::PluginManifest;

// ─────────────────────────────────────────────────────────────────────────────
// Plugin-specific Messages
// ─────────────────────────────────────────────────────────────────────────────

/// Plugin-specific messages (beyond common ServiceMsg)
#[derive(Debug)]
pub enum PluginMsg {
    /// Get plugin subscriptions (event patterns)
    GetSubscriptions {
        reply: oneshot::Sender<Vec<String>>,
    },
    /// Get plugin version
    GetVersion {
        reply: oneshot::Sender<String>,
    },
}

/// Reply type for PluginMsg
#[derive(Debug, kameo::Reply)]
pub enum PluginReply {
    /// Operation completed
    Done,
}

// ─────────────────────────────────────────────────────────────────────────────
// Plugin Actor
// ─────────────────────────────────────────────────────────────────────────────

/// Actor wrapper for a JavaScript/TypeScript plugin
#[derive(kameo::Actor)]
pub struct PluginActor {
    /// Plugin manifest
    manifest: PluginManifest,
    /// Base path containing the plugin files
    base_path: PathBuf,
    /// Service state tracker
    state: ServiceStateTracker,
    /// Reference to the runtime pool
    pool: ActorRef<JsRuntimePoolActor>,
    /// Worker ID this plugin is assigned to (set after loading)
    worker_id: Option<usize>,
}

impl PluginActor {
    /// Create a new PluginActor
    pub fn new(
        manifest: PluginManifest,
        base_path: PathBuf,
        pool: ActorRef<JsRuntimePoolActor>,
    ) -> Self {
        Self {
            manifest,
            base_path,
            state: ServiceStateTracker::new(),
            pool,
            worker_id: None,
        }
    }

    /// Get the plugin ID
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    /// Get the plugin name
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// Get the plugin description
    pub fn description(&self) -> &str {
        &self.manifest.description
    }

    /// Get the plugin version
    pub fn version(&self) -> &str {
        &self.manifest.version
    }

    /// Get the plugin's event subscriptions
    pub fn subscriptions(&self) -> &[String] {
        &self.manifest.subscriptions
    }
}

// Handle common ServiceMsg
impl Message<ServiceMsg> for PluginActor {
    type Reply = ServiceReply;

    async fn handle(
        &mut self,
        msg: ServiceMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            ServiceMsg::Start => {
                self.state.set_starting();

                // Send load request to pool
                let (reply_tx, reply_rx) = oneshot::channel();
                if let Err(e) = self
                    .pool
                    .tell(PoolMsg::LoadPlugin {
                        manifest: self.manifest.clone(),
                        base_path: self.base_path.clone(),
                        reply: reply_tx,
                    })
                    .await
                {
                    self.state.set_failed();
                    return ServiceReply::Failed(format!("Failed to send to pool: {}", e));
                }

                // Wait for load result
                match reply_rx.await {
                    Ok(Ok(worker_id)) => {
                        self.worker_id = Some(worker_id);
                        self.state.set_running();
                        tracing::info!(
                            "Plugin '{}' v{} started on worker {}",
                            self.manifest.name,
                            self.manifest.version,
                            worker_id
                        );
                        ServiceReply::Started
                    }
                    Ok(Err(e)) => {
                        self.state.set_failed();
                        ServiceReply::Failed(format!("Plugin load failed: {}", e))
                    }
                    Err(_) => {
                        self.state.set_failed();
                        ServiceReply::Failed("Pool did not respond".to_string())
                    }
                }
            }

            ServiceMsg::Stop => {
                self.state.set_stopping();

                // Send stop request to pool
                let (reply_tx, reply_rx) = oneshot::channel();
                if let Err(e) = self
                    .pool
                    .tell(PoolMsg::StopPlugin {
                        plugin_id: self.manifest.id.clone(),
                        reply: reply_tx,
                    })
                    .await
                {
                    tracing::warn!("Failed to send stop to pool: {}", e);
                }

                // Wait for stop (with timeout)
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    reply_rx,
                )
                .await;

                self.worker_id = None;
                self.state.set_stopped();
                tracing::info!("Plugin '{}' stopped", self.manifest.name);
                ServiceReply::Stopped
            }

            ServiceMsg::GetStatus => ServiceReply::Status {
                id: self.manifest.id.clone(),
                name: self.manifest.name.clone(),
                state: self.state.state(),
                uptime_secs: self.state.uptime_secs(),
                extra: Some(serde_json::json!({
                    "version": self.manifest.version,
                    "worker_id": self.worker_id,
                    "subscriptions": self.manifest.subscriptions,
                })),
            },

            ServiceMsg::GetConfig => ServiceReply::Config {
                config: self.manifest.config.clone(),
            },

            ServiceMsg::SetConfig { config } => {
                // Plugins typically don't support runtime config changes
                // but we can update the cached config
                self.manifest.config = config;
                ServiceReply::ConfigSet
            }

            ServiceMsg::OnEvent { event } => {
                if self.state.state() != ServiceState::Running {
                    return ServiceReply::EventHandled;
                }

                // Fire and forget - send event to pool
                let _ = self
                    .pool
                    .tell(PoolMsg::SendEvent {
                        plugin_id: self.manifest.id.clone(),
                        event,
                    })
                    .await;

                ServiceReply::EventHandled
            }

            ServiceMsg::HandleRequest { request, reply } => {
                if self.state.state() != ServiceState::Running {
                    let _ = reply.send(ServiceResponse::Error {
                        code: "NOT_RUNNING".to_string(),
                        message: format!("Plugin '{}' is not running", self.manifest.id),
                    });
                    return ServiceReply::RequestHandled;
                }

                // Handle common requests locally
                let local_response = match &request {
                    ServiceRequest::GetStatus => Some(ServiceResponse::Status {
                        id: self.manifest.id.clone(),
                        name: self.manifest.name.clone(),
                        state: self.state.state(),
                        uptime_seconds: self.state.uptime_secs(),
                        extra: Some(serde_json::json!({
                            "version": self.manifest.version,
                            "worker_id": self.worker_id,
                        })),
                    }),
                    ServiceRequest::GetConfig => Some(ServiceResponse::Config {
                        config: self.manifest.config.clone(),
                    }),
                    _ => None,
                };

                if let Some(response) = local_response {
                    let _ = reply.send(response);
                    return ServiceReply::RequestHandled;
                }

                // Forward to pool
                let (pool_reply_tx, pool_reply_rx) = oneshot::channel();
                if let Err(e) = self
                    .pool
                    .tell(PoolMsg::HandleRequest {
                        plugin_id: self.manifest.id.clone(),
                        request,
                        reply: pool_reply_tx,
                    })
                    .await
                {
                    let _ = reply.send(ServiceResponse::Error {
                        code: "POOL_ERROR".to_string(),
                        message: format!("Failed to forward to pool: {}", e),
                    });
                    return ServiceReply::RequestHandled;
                }

                // Forward response from pool to original requester
                tokio::spawn(async move {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        pool_reply_rx,
                    )
                    .await
                    {
                        Ok(Ok(Ok(response))) => {
                            let _ = reply.send(response);
                        }
                        Ok(Ok(Err(e))) => {
                            let _ = reply.send(ServiceResponse::Error {
                                code: "PLUGIN_ERROR".to_string(),
                                message: e.to_string(),
                            });
                        }
                        Ok(Err(_)) => {
                            let _ = reply.send(ServiceResponse::Error {
                                code: "POOL_NO_RESPONSE".to_string(),
                                message: "Pool did not respond".to_string(),
                            });
                        }
                        Err(_) => {
                            let _ = reply.send(ServiceResponse::Error {
                                code: "TIMEOUT".to_string(),
                                message: "Request timed out".to_string(),
                            });
                        }
                    }
                });

                ServiceReply::RequestHandled
            }
        }
    }
}

// Handle plugin-specific messages
impl Message<PluginMsg> for PluginActor {
    type Reply = PluginReply;

    async fn handle(
        &mut self,
        msg: PluginMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            PluginMsg::GetSubscriptions { reply } => {
                let _ = reply.send(self.manifest.subscriptions.clone());
                PluginReply::Done
            }
            PluginMsg::GetVersion { reply } => {
                let _ = reply.send(self.manifest.version.clone());
                PluginReply::Done
            }
        }
    }
}

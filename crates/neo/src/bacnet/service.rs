//! BACnet Network Service
//!
//! Implements the Service trait to integrate BACnet/IP into Neo's service architecture.

use std::sync::mpsc;
use std::thread;

use async_trait::async_trait;

use blueprint_runtime::service::{Event, Service, ServiceContext, ServiceResult, ServiceSpec, ServiceError};

use crate::server::AppState;

use super::types::{WorkerCommand, WorkerResponse};
use super::worker::BacnetWorker;

/// Configuration for the BACnet service
#[derive(Debug, Clone)]
pub struct BacnetConfig {
    /// Interface to bind to (e.g., "0.0.0.0" for all interfaces)
    pub interface: String,
    /// UDP port (default: 47808 / 0xBAC0)
    pub port: u16,
    /// Broadcast address (e.g., "10.0.1.255" for subnet broadcast)
    pub broadcast: Option<String>,
    /// Seconds between auto-discovery broadcasts (0 = disabled)
    pub discovery_interval: u64,
}

impl Default for BacnetConfig {
    fn default() -> Self {
        Self {
            interface: "0.0.0.0".to_string(),
            port: 47808,
            broadcast: None,
            discovery_interval: 60,
        }
    }
}

impl BacnetConfig {
    /// Create config from environment variables
    ///
    /// Reads:
    /// - `BACNET_IP` or `NEO_BACNET_IP`: Bind address (default: 0.0.0.0)
    /// - `BACNET_PORT`: UDP port (default: 47808)
    /// - `BACNET_BROADCAST`: Broadcast address (default: 255.255.255.255)
    pub fn from_env() -> Self {
        let interface = std::env::var("BACNET_IP")
            .or_else(|_| std::env::var("NEO_BACNET_IP"))
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let port = std::env::var("BACNET_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(47808);

        let broadcast = std::env::var("BACNET_BROADCAST").ok();

        Self {
            interface,
            port,
            broadcast,
            discovery_interval: 60,
        }
    }
}

/// BACnet Network Service
///
/// Provides device discovery and point reading over BACnet/IP.
/// Runs a blocking worker thread for UDP I/O and bridges to async via channels.
pub struct BacnetService {
    config: BacnetConfig,
    /// Application state for storing discovered devices
    state: AppState,
    /// Channel to send commands to the worker
    worker_cmd_tx: Option<mpsc::Sender<WorkerCommand>>,
    /// Handle to the worker thread
    worker_handle: Option<thread::JoinHandle<()>>,
}

impl BacnetService {
    /// Create a new BACnet service
    pub fn new(config: BacnetConfig, state: AppState) -> Self {
        Self {
            config,
            state,
            worker_cmd_tx: None,
            worker_handle: None,
        }
    }
}

#[async_trait]
impl Service for BacnetService {
    fn spec(&self) -> ServiceSpec {
        ServiceSpec::new("bacnet/network", "BACnet Network Service")
            .with_subscriptions(vec![
                "bacnet/discover".to_string(),
                "bacnet/discover-session".to_string(),
                "bacnet/stop-discovery-session".to_string(),
                "bacnet/read".to_string(),
                "bacnet/read-objects".to_string(),
                "bacnet/device-added".to_string(),
                "bacnet/device-removed".to_string(),
            ])
            .with_description("BACnet/IP device discovery and point reading")
    }

    async fn on_start(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!(
            "Starting BACnet service on {}:{} (broadcast: {})",
            self.config.interface,
            self.config.port,
            self.config.broadcast.as_deref().unwrap_or("255.255.255.255")
        );

        // Create channels for worker communication
        let (cmd_tx, cmd_rx) = mpsc::channel::<WorkerCommand>();
        let (resp_tx, resp_rx) = mpsc::channel::<WorkerResponse>();

        let bind_addr = self.config.interface.clone();
        let port = self.config.port;
        let broadcast = self.config.broadcast.clone();

        // Spawn the blocking worker thread
        let handle = thread::Builder::new()
            .name("bacnet-worker".to_string())
            .spawn(move || {
                match BacnetWorker::new(&bind_addr, port, broadcast.as_deref(), cmd_rx, resp_tx) {
                    Ok(mut worker) => worker.run(),
                    Err(e) => {
                        tracing::error!("BACnet worker failed to start: {}", e);
                    }
                }
            })
            .map_err(|e| ServiceError::InitializationFailed(format!("Failed to spawn worker: {}", e)))?;

        // Clone the command sender for use in the response bridge (before moving to self)
        let worker_tx = cmd_tx.clone();

        self.worker_cmd_tx = Some(cmd_tx);
        self.worker_handle = Some(handle);

        // Bridge responses from worker to async world
        let state = self.state.clone();
        tokio::task::spawn_blocking(move || {
            // Create a mini runtime for calling async methods from blocking context
            let rt = tokio::runtime::Handle::current();

            while let Ok(response) = resp_rx.recv() {
                match response {
                    WorkerResponse::DeviceDiscovered(device) => {
                        // Legacy discovery path - just log, don't auto-store
                        // New flow uses SessionDeviceDiscovered instead
                        tracing::info!(
                            "BACnet device discovered (legacy): {} at {} (vendor={})",
                            device.device_id,
                            device.address,
                            device.vendor_id
                        );
                    }
                    WorkerResponse::PropertyRead(result) => {
                        tracing::debug!(
                            "BACnet property read: device={} {}.{}.{} = {:?}",
                            result.device_id,
                            result.object_type,
                            result.instance,
                            result.property,
                            result.value
                        );

                        // Emit event for ECS to update entity component
                        let event_data = serde_json::json!({
                            "device_id": result.device_id,
                            "object_type": result.object_type,
                            "instance": result.instance,
                            "property": result.property,
                            "value": result.value,
                            "timestamp": result.timestamp,
                        });
                        let event = blueprint_runtime::service::Event::new(
                            "bacnet/property-read",
                            "bacnet/network",
                            event_data.clone(),
                        );
                        state.service_manager().publish_event(event);

                        // Broadcast to WebSocket subscribers
                        let path = format!(
                            "/bacnet/devices/{}/objects/{}/{}/{}",
                            result.device_id, result.object_type, result.instance, result.property
                        );
                        let state = state.clone();
                        rt.spawn(async move {
                            state.broadcast(
                                &path,
                                crate::server::ServerMessage::change(
                                    path.clone(),
                                    crate::server::ChangeType::Updated,
                                    Some(event_data),
                                ),
                            ).await;
                        });
                    }
                    WorkerResponse::ObjectListRead(result) => {
                        tracing::info!(
                            "BACnet object list read: device={} ({} objects)",
                            result.device_id,
                            result.objects.len()
                        );

                        // Filter to readable object types for polling
                        let readable_types = [
                            "analoginput", "analogoutput", "analogvalue",
                            "binaryinput", "binaryoutput", "binaryvalue",
                            "multistateinput", "multistateoutput", "multistatevalue",
                        ];
                        let poll_objects: Vec<(String, u32)> = result.objects.iter()
                            .filter(|obj| readable_types.contains(&obj.object_type.as_str()))
                            .map(|obj| (obj.object_type.clone(), obj.instance))
                            .collect();

                        // Start polling if there are readable objects
                        if !poll_objects.is_empty() {
                            let device_id = result.device_id;
                            let num_objects = poll_objects.len();
                            let _ = worker_tx.send(WorkerCommand::StartPolling {
                                device_id,
                                objects: poll_objects,
                                interval_ms: 3000,
                            });
                            tracing::info!(
                                "Started polling {} objects for device {} (full cycle: {}s)",
                                num_objects,
                                device_id,
                                num_objects * 3
                            );
                        }

                        // Emit event for ECS to create child entities
                        let event_data = serde_json::json!({
                            "device_id": result.device_id,
                            "objects": result.objects,
                        });
                        let event = blueprint_runtime::service::Event::new(
                            "bacnet/object-list",
                            "bacnet/network",
                            event_data.clone(),
                        );
                        state.service_manager().publish_event(event);

                        // Broadcast to WebSocket subscribers
                        let path = format!("/bacnet/devices/{}/objects", result.device_id);
                        let state = state.clone();
                        rt.spawn(async move {
                            state.broadcast(
                                &path,
                                crate::server::ServerMessage::change(
                                    path.clone(),
                                    crate::server::ChangeType::Updated,
                                    Some(event_data),
                                ),
                            ).await;
                        });
                    }
                    WorkerResponse::SessionDeviceDiscovered { client_id, request_id, device } => {
                        tracing::debug!(
                            "Session device discovered: client={} device={}",
                            client_id, device.device_id
                        );

                        // Check if device already exists in ECS
                        let device_id = device.device_id;
                        let state = state.clone();
                        rt.spawn(async move {
                            let already_exists = state.device_exists_in_ecs(device_id).await;

                            let message = crate::server::ServerMessage::BacnetDeviceFound {
                                id: request_id,
                                device,
                                already_exists,
                            };
                            state.send_to_client(client_id, message).await;
                        });
                    }
                    WorkerResponse::SessionComplete { client_id, request_id, devices_found } => {
                        tracing::debug!(
                            "Session complete: client={} devices_found={}",
                            client_id, devices_found
                        );

                        let state = state.clone();
                        rt.spawn(async move {
                            let message = crate::server::ServerMessage::BacnetDiscoveryComplete {
                                id: request_id,
                                devices_found,
                            };
                            state.send_to_client(client_id, message).await;
                        });
                    }
                    WorkerResponse::Error(e) => {
                        tracing::warn!("BACnet error: {}", e);
                    }
                }
            }
            tracing::debug!("Response bridge task finished");
        });

        // Trigger initial discovery
        if let Some(ref tx) = self.worker_cmd_tx {
            let _ = tx.send(WorkerCommand::Discover {
                low_limit: None,
                high_limit: None,
            });
        }

        tracing::info!("BACnet service started");
        Ok(())
    }

    async fn on_event(&mut self, _ctx: &ServiceContext, event: Event) -> ServiceResult<()> {
        let Some(ref tx) = self.worker_cmd_tx else {
            return Ok(());
        };

        match event.event_type.as_str() {
            "bacnet/discover" => {
                let low = event
                    .data
                    .get("low_limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let high = event
                    .data
                    .get("high_limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                tracing::debug!("Discovery requested (low={:?}, high={:?})", low, high);
                let _ = tx.send(WorkerCommand::Discover {
                    low_limit: low,
                    high_limit: high,
                });
            }
            "bacnet/read" => {
                if let (Some(device_id), Some(object_type), Some(instance)) = (
                    event.data.get("device_id").and_then(|v| v.as_u64()),
                    event.data.get("object_type").and_then(|v| v.as_str()),
                    event.data.get("instance").and_then(|v| v.as_u64()),
                ) {
                    let property = event
                        .data
                        .get("property")
                        .and_then(|v| v.as_str())
                        .unwrap_or("present-value")
                        .to_string();

                    tracing::debug!(
                        "Read requested: device={}, {}.{}.{}",
                        device_id,
                        object_type,
                        instance,
                        property
                    );

                    let _ = tx.send(WorkerCommand::ReadProperty {
                        device_id: device_id as u32,
                        object_type: object_type.to_string(),
                        instance: instance as u32,
                        property,
                    });
                } else {
                    tracing::warn!("Invalid bacnet/read event: {:?}", event.data);
                }
            }
            "bacnet/read-objects" => {
                if let Some(device_id) = event.data.get("device_id").and_then(|v| v.as_u64()) {
                    tracing::info!("Read object list requested for device {}", device_id);
                    let _ = tx.send(WorkerCommand::ReadObjectList {
                        device_id: device_id as u32,
                    });
                } else {
                    tracing::warn!("Invalid bacnet/read-objects event: {:?}", event.data);
                }
            }
            "bacnet/discover-session" => {
                // Start a streaming discovery session for a specific client
                let session_id = event.data.get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
                    .unwrap_or_else(uuid::Uuid::new_v4);
                let client_id = event.data.get("client_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok());
                let request_id = event.data.get("request_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let low_limit = event.data.get("low_limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let high_limit = event.data.get("high_limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let duration_secs = event.data.get("duration")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10);

                if let Some(client_id) = client_id {
                    tracing::info!("Starting discovery session {} for client {}", session_id, client_id);
                    let _ = tx.send(WorkerCommand::DiscoverSession {
                        session_id,
                        client_id,
                        request_id,
                        low_limit,
                        high_limit,
                        duration_secs,
                    });
                } else {
                    tracing::warn!("Invalid bacnet/discover-session event: missing client_id");
                }
            }
            "bacnet/stop-discovery-session" => {
                if let Some(session_id) = event.data.get("session_id")
                    .and_then(|v| v.as_str())
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
                {
                    tracing::info!("Stopping discovery session {}", session_id);
                    let _ = tx.send(WorkerCommand::StopDiscoverySession { session_id });
                } else {
                    tracing::warn!("Invalid bacnet/stop-discovery-session event: missing session_id");
                }
            }
            "bacnet/device-added" => {
                // Device was added to ECS - read its object list and start polling
                if let Some(device_id) = event.data.get("device_id").and_then(|v| v.as_u64()) {
                    tracing::info!("Device {} added to system, reading object list", device_id);
                    let _ = tx.send(WorkerCommand::ReadObjectList {
                        device_id: device_id as u32,
                    });
                }
            }
            "bacnet/device-removed" => {
                // Device was removed from ECS - stop polling
                if let Some(device_id) = event.data.get("device_id").and_then(|v| v.as_u64()) {
                    tracing::info!("Device {} removed from system, stopping polling", device_id);
                    let _ = tx.send(WorkerCommand::StopPolling {
                        device_id: device_id as u32,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        tracing::info!("Stopping BACnet service");

        // Send shutdown command to worker
        if let Some(tx) = self.worker_cmd_tx.take() {
            let _ = tx.send(WorkerCommand::Shutdown);
        }

        // Wait for worker thread to finish
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }

        tracing::info!("BACnet service stopped");
        Ok(())
    }
}

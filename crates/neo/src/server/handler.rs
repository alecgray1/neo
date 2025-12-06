//! WebSocket Connection Handler
//!
//! Handles individual WebSocket connections and message processing.

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::protocol::{ClientMessage, ErrorCode, PluginRegistration, ServerMessage};
use super::state::AppState;
use crate::project::{BlueprintConfig, ProjectLoader};

/// Handle a WebSocket connection
pub async fn handle_websocket(socket: WebSocket, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for sending messages to this client
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(64);

    // Register client and get session ID
    let session_id = state.register_client(tx).await;

    // Send connected message
    let connected_msg = ServerMessage::connected(session_id.to_string());
    if let Ok(json) = serde_json::to_string(&connected_msg) {
        let _ = ws_sender.send(Message::Text(json.into())).await;
    }

    // Spawn task to forward messages from channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Process incoming messages
    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Text(text) => {
                    handle_client_message(&state_clone, session_id, &text).await;
                }
                Message::Close(_) => {
                    break;
                }
                Message::Ping(_data) => {
                    // Pong is handled automatically by axum
                    tracing::trace!("Received ping from {}", session_id);
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // Cleanup
    state.remove_client(session_id).await;
}

/// Handle a client message
async fn handle_client_message(state: &AppState, session_id: Uuid, text: &str) {
    // Parse the message
    let msg: ClientMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Failed to parse client message: {}", e);
            send_error(state, session_id, None, ErrorCode::InvalidRequest, "Invalid JSON").await;
            return;
        }
    };

    // Handle based on message type
    match msg {
        ClientMessage::Subscribe { id, paths } => {
            handle_subscribe(state, session_id, &id, paths).await;
        }
        ClientMessage::Unsubscribe { id, paths } => {
            handle_unsubscribe(state, session_id, &id, paths).await;
        }
        ClientMessage::Get { id, path } => {
            handle_get(state, session_id, &id, &path).await;
        }
        ClientMessage::Update { id, path, data } => {
            handle_update(state, session_id, &id, &path, data).await;
        }
        ClientMessage::Create { id, path, data } => {
            handle_create(state, session_id, &id, &path, data).await;
        }
        ClientMessage::Delete { id, path } => {
            handle_delete(state, session_id, &id, &path).await;
        }
        ClientMessage::Ping { id } => {
            send_to_client(state, session_id, ServerMessage::pong(id)).await;
        }
        ClientMessage::PluginRegister { plugin } => {
            handle_plugin_register(state, session_id, plugin).await;
        }
        ClientMessage::PluginRebuilt { plugin_id, entry_path } => {
            handle_plugin_rebuilt(state, session_id, &plugin_id, &entry_path).await;
        }
        ClientMessage::BacnetReadObjects { id, device_id } => {
            handle_bacnet_read_objects(state, session_id, &id, device_id).await;
        }
        ClientMessage::BacnetReadProperty { id, device_id, object_type, instance, property } => {
            handle_bacnet_read_property(state, session_id, &id, device_id, &object_type, instance, &property).await;
        }
        ClientMessage::BacnetDiscover { id, low_limit, high_limit, duration } => {
            handle_bacnet_discover(state, session_id, &id, low_limit, high_limit, duration).await;
        }
        ClientMessage::BacnetStopDiscovery { id } => {
            handle_bacnet_stop_discovery(state, session_id, &id).await;
        }
        ClientMessage::BacnetAddDevice { id, device } => {
            handle_bacnet_add_device(state, session_id, &id, device).await;
        }
        ClientMessage::BacnetRemoveDevice { id, device_id } => {
            handle_bacnet_remove_device(state, session_id, &id, device_id).await;
        }
    }
}

/// Handle subscribe request
async fn handle_subscribe(state: &AppState, session_id: Uuid, id: &str, paths: Vec<String>) {
    state.subscribe(session_id, paths.clone()).await;

    // Send initial data for subscribed paths
    let mut initial_data = serde_json::Map::new();

    if let Some(project) = state.project().await {
        for path in &paths {
            if path == "/devices" || path == "/devices/*" || path == "/devices/**" {
                let devices: Vec<_> = project.devices.values().cloned().collect();
                initial_data.insert(
                    "/devices".to_string(),
                    serde_json::to_value(&devices).unwrap_or(Value::Null),
                );
            } else if path.starts_with("/devices/") && !path.contains('*') {
                let device_id = path.trim_start_matches("/devices/");
                if let Some(device) = project.devices.get(device_id) {
                    initial_data.insert(
                        path.clone(),
                        serde_json::to_value(device).unwrap_or(Value::Null),
                    );
                }
            } else if path == "/schedules" || path == "/schedules/*" || path == "/schedules/**" {
                let schedules: Vec<_> = project.schedules.values().cloned().collect();
                initial_data.insert(
                    "/schedules".to_string(),
                    serde_json::to_value(&schedules).unwrap_or(Value::Null),
                );
            }
        }
    }

    // BACnet devices are now stored in ECS
    // TODO: Query ECS for initial data in Phase 6
    for path in &paths {
        if path == "/bacnet/devices" || path == "/bacnet/devices/*" || path == "/bacnet/devices/**" {
            // Return empty array until ECS query is implemented
            initial_data.insert(
                "/bacnet/devices".to_string(),
                serde_json::json!([]),
            );
        }
    }

    let response = ServerMessage::success(
        id,
        Some(serde_json::json!({
            "subscribed": paths,
            "data": initial_data
        })),
    );

    send_to_client(state, session_id, response).await;
}

/// Handle unsubscribe request
async fn handle_unsubscribe(state: &AppState, session_id: Uuid, id: &str, paths: Vec<String>) {
    state.unsubscribe(session_id, paths.clone()).await;

    let response = ServerMessage::success(
        id,
        Some(serde_json::json!({ "unsubscribed": paths })),
    );

    send_to_client(state, session_id, response).await;
}

/// Handle get request
async fn handle_get(state: &AppState, session_id: Uuid, id: &str, path: &str) {
    // Handle BACnet devices - now stored in ECS
    // TODO: Implement ECS query in Phase 6
    if path == "/bacnet/devices" {
        // Return empty array until ECS query is implemented
        send_to_client(
            state,
            session_id,
            ServerMessage::success(id, Some(serde_json::json!([]))),
        )
        .await;
        return;
    } else if path.starts_with("/bacnet/devices/") {
        // Device lookup not implemented yet
        send_error(
            state,
            session_id,
            Some(id),
            ErrorCode::NotFound,
            format!("BACnet device queries not yet implemented: {}", path),
        )
        .await;
        return;
    }

    let project = match state.project().await {
        Some(p) => p,
        None => {
            send_error(state, session_id, Some(id), ErrorCode::NotFound, "No project loaded")
                .await;
            return;
        }
    };

    let data = match path {
        "/project" => Some(serde_json::json!({
            "id": project.manifest.project.id,
            "name": project.manifest.project.name,
            "version": project.manifest.project.version,
            "description": project.manifest.project.description,
        })),
        "/devices" => Some(serde_json::to_value(
            project.devices.values().cloned().collect::<Vec<_>>(),
        ).unwrap_or(Value::Null)),
        "/schedules" => Some(serde_json::to_value(
            project.schedules.values().cloned().collect::<Vec<_>>(),
        ).unwrap_or(Value::Null)),
        "/blueprints" => Some(serde_json::to_value(
            project.blueprints.values().cloned().collect::<Vec<_>>(),
        ).unwrap_or(Value::Null)),
        p if p.starts_with("/devices/") => {
            let device_id = p.trim_start_matches("/devices/");
            project
                .devices
                .get(device_id)
                .map(|d| serde_json::to_value(d).unwrap_or(Value::Null))
        }
        p if p.starts_with("/schedules/") => {
            let schedule_id = p.trim_start_matches("/schedules/");
            project
                .schedules
                .get(schedule_id)
                .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
        }
        p if p.starts_with("/blueprints/") => {
            let blueprint_id = p.trim_start_matches("/blueprints/");
            project
                .blueprints
                .get(blueprint_id)
                .map(|b| serde_json::to_value(b).unwrap_or(Value::Null))
        }
        _ => None,
    };

    match data {
        Some(d) => {
            send_to_client(state, session_id, ServerMessage::success(id, Some(d))).await;
        }
        None => {
            send_error(
                state,
                session_id,
                Some(id),
                ErrorCode::NotFound,
                format!("Path not found: {}", path),
            )
            .await;
        }
    }
}

/// Handle update request
async fn handle_update(
    state: &AppState,
    session_id: Uuid,
    id: &str,
    path: &str,
    data: Value,
) {
    // Get project path
    let project_path = match state.project_path().await {
        Some(p) => p,
        None => {
            send_error(state, session_id, Some(id), ErrorCode::NotFound, "No project loaded").await;
            return;
        }
    };

    // Handle based on path
    if path.starts_with("/blueprints/") {
        let blueprint_id = path.trim_start_matches("/blueprints/");

        // Deserialize the blueprint data
        let blueprint: BlueprintConfig = match serde_json::from_value(data) {
            Ok(bp) => bp,
            Err(e) => {
                send_error(
                    state,
                    session_id,
                    Some(id),
                    ErrorCode::InvalidRequest,
                    format!("Invalid blueprint data: {}", e),
                )
                .await;
                return;
            }
        };

        // Verify the ID matches
        if blueprint.id != blueprint_id {
            send_error(
                state,
                session_id,
                Some(id),
                ErrorCode::InvalidRequest,
                "Blueprint ID in path doesn't match data",
            )
            .await;
            return;
        }

        // Save to disk
        if let Err(e) = ProjectLoader::save_blueprint(&project_path, &blueprint).await {
            send_error(
                state,
                session_id,
                Some(id),
                ErrorCode::InternalError,
                format!("Failed to save blueprint: {}", e),
            )
            .await;
            return;
        }

        // Update in-memory project state immediately
        // (file watcher will also do this, but we want it to be immediate)
        state.update_blueprint(blueprint).await;

        tracing::info!(blueprint_id = %blueprint_id, "Blueprint updated via WebSocket");

        // Send success response
        // Note: The file watcher will detect the change and broadcast to all clients
        send_to_client(state, session_id, ServerMessage::success(id, None)).await;
    } else {
        send_error(
            state,
            session_id,
            Some(id),
            ErrorCode::InvalidRequest,
            format!("Update not supported for path: {}", path),
        )
        .await;
    }
}

/// Handle create request
async fn handle_create(
    state: &AppState,
    session_id: Uuid,
    id: &str,
    _path: &str,
    _data: Value,
) {
    // TODO: Implement create logic - write to disk, reload, broadcast
    send_error(
        state,
        session_id,
        Some(id),
        ErrorCode::InvalidRequest,
        "Create not yet implemented",
    )
    .await;
}

/// Handle delete request
async fn handle_delete(state: &AppState, session_id: Uuid, id: &str, _path: &str) {
    // TODO: Implement delete logic - delete from disk, reload, broadcast
    send_error(
        state,
        session_id,
        Some(id),
        ErrorCode::InvalidRequest,
        "Delete not yet implemented",
    )
    .await;
}

/// Send a message to a specific client
async fn send_to_client(state: &AppState, session_id: Uuid, message: ServerMessage) {
    state.send_to_client(session_id, message).await;
}

/// Send an error to a specific client
async fn send_error(
    state: &AppState,
    session_id: Uuid,
    id: Option<&str>,
    code: ErrorCode,
    message: impl Into<String>,
) {
    let error = ServerMessage::Error {
        id: id.map(String::from),
        code,
        message: message.into(),
    };
    send_to_client(state, session_id, error).await;
}

/// Handle plugin registration from Vite dev server
async fn handle_plugin_register(state: &AppState, session_id: Uuid, plugin: PluginRegistration) {
    let plugin_id = plugin.id.clone();
    tracing::info!(
        plugin_id = %plugin_id,
        name = %plugin.name,
        entry_path = %plugin.entry_path,
        "Plugin registration request"
    );

    match state.register_dev_plugin(plugin).await {
        Ok(()) => {
            let response = ServerMessage::PluginRegistered {
                plugin_id: plugin_id.clone(),
            };
            send_to_client(state, session_id, response).await;
            tracing::info!(plugin_id = %plugin_id, "Plugin registered successfully");
        }
        Err(e) => {
            send_error(
                state,
                session_id,
                None,
                ErrorCode::InternalError,
                format!("Failed to register plugin: {}", e),
            )
            .await;
        }
    }
}

/// Handle plugin rebuilt notification from Vite dev server
async fn handle_plugin_rebuilt(state: &AppState, session_id: Uuid, plugin_id: &str, entry_path: &str) {
    tracing::info!(
        plugin_id = %plugin_id,
        entry_path = %entry_path,
        "Plugin rebuilt notification"
    );

    match state.restart_dev_plugin(plugin_id, entry_path).await {
        Ok(()) => {
            let response = ServerMessage::PluginRestarted {
                plugin_id: plugin_id.to_string(),
            };
            send_to_client(state, session_id, response).await;
            tracing::info!(plugin_id = %plugin_id, "Plugin restarted successfully");
        }
        Err(e) => {
            send_error(
                state,
                session_id,
                None,
                ErrorCode::InternalError,
                format!("Failed to restart plugin: {}", e),
            )
            .await;
        }
    }
}

/// Handle BACnet read objects request
async fn handle_bacnet_read_objects(state: &AppState, session_id: Uuid, id: &str, device_id: u32) {
    use blueprint_runtime::service::Event;

    tracing::info!(device_id = device_id, "BACnet read objects request");

    // Publish event to the BACnet service
    let event = Event::new(
        "bacnet/read-objects",
        "websocket",
        serde_json::json!({ "device_id": device_id }),
    );

    state.service_manager().publish_event(event);

    // Send acknowledgment - actual data will come via subscription
    send_to_client(
        state,
        session_id,
        ServerMessage::success(id, Some(serde_json::json!({
            "status": "pending",
            "device_id": device_id,
            "message": "Object list read in progress"
        }))),
    )
    .await;
}

/// Handle BACnet read property request
async fn handle_bacnet_read_property(
    state: &AppState,
    session_id: Uuid,
    id: &str,
    device_id: u32,
    object_type: &str,
    instance: u32,
    property: &str,
) {
    use blueprint_runtime::service::Event;

    tracing::info!(
        device_id = device_id,
        object_type = object_type,
        instance = instance,
        property = property,
        "BACnet read property request"
    );

    // Publish event to the BACnet service
    let event = Event::new(
        "bacnet/read",
        "websocket",
        serde_json::json!({
            "device_id": device_id,
            "object_type": object_type,
            "instance": instance,
            "property": property,
        }),
    );

    state.service_manager().publish_event(event);

    // Send acknowledgment - actual data will come via subscription
    send_to_client(
        state,
        session_id,
        ServerMessage::success(id, Some(serde_json::json!({
            "status": "pending",
            "device_id": device_id,
            "object_type": object_type,
            "instance": instance,
            "property": property,
            "message": "Property read in progress"
        }))),
    )
    .await;
}

/// Create a deterministic discovery session UUID from client session and request ID
fn discovery_session_uuid(client_session: Uuid, request_id: &str) -> Uuid {
    // Create a deterministic UUID by hashing client session + request id
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    client_session.hash(&mut hasher);
    request_id.hash(&mut hasher);
    let hash = hasher.finish();

    // Use the hash to create a UUID v4-like value
    Uuid::from_u64_pair(hash, hash.rotate_left(32))
}

/// Handle BACnet discovery request - starts streaming discovery session
async fn handle_bacnet_discover(
    state: &AppState,
    session_id: Uuid,
    id: &str,
    low_limit: Option<u32>,
    high_limit: Option<u32>,
    duration: u32,
) {
    use blueprint_runtime::service::Event;

    // Create deterministic session ID so we can stop it later
    let discovery_session_id = discovery_session_uuid(session_id, id);

    tracing::info!(
        "Starting BACnet discovery: client={} session={} low={:?} high={:?} duration={}",
        session_id, discovery_session_id, low_limit, high_limit, duration
    );

    // Emit event to BacnetService with client_id for streaming responses back
    let event = Event::new(
        "bacnet/discover-session",
        "websocket",
        serde_json::json!({
            "session_id": discovery_session_id.to_string(),
            "client_id": session_id.to_string(),
            "request_id": id,
            "low_limit": low_limit,
            "high_limit": high_limit,
            "duration": duration as u64,
        }),
    );
    state.service_manager().publish_event(event);

    // Send acknowledgment - results will stream back via BacnetDeviceFound messages
    send_to_client(
        state,
        session_id,
        ServerMessage::BacnetDiscoveryStarted { id: id.to_string() },
    )
    .await;
}

/// Handle request to stop an active discovery session
async fn handle_bacnet_stop_discovery(
    state: &AppState,
    session_id: Uuid,
    id: &str,
) {
    use blueprint_runtime::service::Event;

    // Recreate the same discovery session ID
    let discovery_session_id = discovery_session_uuid(session_id, id);

    tracing::info!(
        "Stopping BACnet discovery: client={} session={}",
        session_id, discovery_session_id
    );

    // Emit event to stop the discovery session
    let event = Event::new(
        "bacnet/stop-discovery-session",
        "websocket",
        serde_json::json!({
            "session_id": discovery_session_id.to_string(),
        }),
    );
    state.service_manager().publish_event(event);

    // Send success - SessionComplete will be sent when worker stops
    send_to_client(
        state,
        session_id,
        ServerMessage::success(id, Some(serde_json::json!({
            "status": "stopping"
        }))),
    )
    .await;
}

/// Handle request to add a discovered device to the system
async fn handle_bacnet_add_device(
    state: &AppState,
    session_id: Uuid,
    id: &str,
    device: crate::bacnet::DiscoveredDevice,
) {
    use blueprint_runtime::service::Event;

    let device_id = device.device_id;

    tracing::info!(
        "Adding BACnet device: client={} device_id={} address={}",
        session_id, device_id, device.address
    );

    // Get ECS handle
    let ecs_handle = match state.ecs_handle().await {
        Some(h) => h,
        None => {
            send_to_client(
                state,
                session_id,
                ServerMessage::error_response(id, ErrorCode::InternalError, "ECS not initialized"),
            )
            .await;
            return;
        }
    };

    // Check if device already exists
    let entity_name = format!("bacnet-device-{}", device_id);
    if let Ok(Some(_)) = ecs_handle.lookup(&entity_name).await {
        send_to_client(
            state,
            session_id,
            ServerMessage::error_response(id, ErrorCode::AlreadyExists, format!("Device {} already exists", device_id)),
        )
        .await;
        return;
    }

    // Create ECS entity with BacnetDevice component
    let component_data = serde_json::json!({
        "device_id": device_id,
        "address": device.address,
        "vendor_id": device.vendor_id,
        "max_apdu": device.max_apdu,
        "segmentation": device.segmentation,
    });

    let entity_id = match ecs_handle.create_entity(
        Some(entity_name),
        None,  // No parent
        vec![("BacnetDevice".to_string(), component_data)],
        vec!["Device".to_string()],  // Tag as a Device
    ).await {
        Ok(id) => id,
        Err(e) => {
            send_to_client(
                state,
                session_id,
                ServerMessage::error_response(id, ErrorCode::InternalError, format!("Failed to create entity: {}", e)),
            )
            .await;
            return;
        }
    };

    tracing::info!(
        "Created ECS entity for BACnet device {}: entity_id={:?}",
        device_id, entity_id
    );

    // Emit event to trigger object list read and polling
    let event = Event::new(
        "bacnet/device-added",
        "websocket",
        serde_json::json!({
            "device_id": device_id,
            "entity_id": entity_id.0,
            "address": device.address,
        }),
    );
    state.service_manager().publish_event(event);

    // Send success response
    send_to_client(
        state,
        session_id,
        ServerMessage::BacnetDeviceAdded {
            id: id.to_string(),
            device_id,
            entity_id: entity_id.0,
        },
    )
    .await;
}

/// Handle request to remove a device from the system
async fn handle_bacnet_remove_device(
    state: &AppState,
    session_id: Uuid,
    id: &str,
    device_id: u32,
) {
    use blueprint_runtime::service::Event;

    tracing::info!(
        "Removing BACnet device: client={} device_id={}",
        session_id, device_id
    );

    // Get ECS handle
    let ecs_handle = match state.ecs_handle().await {
        Some(h) => h,
        None => {
            send_to_client(
                state,
                session_id,
                ServerMessage::error_response(id, ErrorCode::InternalError, "ECS not initialized"),
            )
            .await;
            return;
        }
    };

    // Find and delete the entity
    let entity_name = format!("bacnet-device-{}", device_id);
    let entity_id = match ecs_handle.lookup(&entity_name).await {
        Ok(Some(eid)) => eid,
        Ok(None) => {
            send_to_client(
                state,
                session_id,
                ServerMessage::error_response(id, ErrorCode::NotFound, format!("Device {} not found", device_id)),
            )
            .await;
            return;
        }
        Err(e) => {
            send_to_client(
                state,
                session_id,
                ServerMessage::error_response(id, ErrorCode::InternalError, format!("Failed to lookup entity: {}", e)),
            )
            .await;
            return;
        }
    };

    // Delete the entity
    if let Err(e) = ecs_handle.delete_entity(entity_id).await {
        send_to_client(
            state,
            session_id,
            ServerMessage::error_response(id, ErrorCode::InternalError, format!("Failed to delete entity: {}", e)),
        )
        .await;
        return;
    }

    tracing::info!(
        "Deleted ECS entity for BACnet device {}: entity_id={:?}",
        device_id, entity_id
    );

    // Emit event to stop polling
    let event = Event::new(
        "bacnet/device-removed",
        "websocket",
        serde_json::json!({
            "device_id": device_id,
            "entity_id": entity_id.0,
        }),
    );
    state.service_manager().publish_event(event);

    // Send success response
    send_to_client(
        state,
        session_id,
        ServerMessage::BacnetDeviceRemoved {
            id: id.to_string(),
            device_id,
        },
    )
    .await;
}

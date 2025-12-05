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

    // BACnet devices are stored in AppState, not in project
    for path in &paths {
        if path == "/bacnet/devices" || path == "/bacnet/devices/*" || path == "/bacnet/devices/**" {
            let devices = state.get_bacnet_devices();
            initial_data.insert(
                "/bacnet/devices".to_string(),
                serde_json::to_value(&devices).unwrap_or(Value::Null),
            );
        } else if path.starts_with("/bacnet/devices/") && !path.contains('*') {
            let device_id_str = path.trim_start_matches("/bacnet/devices/");
            if let Ok(device_id) = device_id_str.parse::<u32>() {
                if let Some(device) = state.get_bacnet_device(device_id) {
                    initial_data.insert(
                        path.clone(),
                        serde_json::to_value(&device).unwrap_or(Value::Null),
                    );
                }
            }
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
    // Handle BACnet devices separately (stored in AppState, not project)
    if path == "/bacnet/devices" {
        let devices = state.get_bacnet_devices();
        send_to_client(
            state,
            session_id,
            ServerMessage::success(id, Some(serde_json::to_value(devices).unwrap_or(Value::Null))),
        )
        .await;
        return;
    } else if path.starts_with("/bacnet/devices/") {
        let device_id_str = path.trim_start_matches("/bacnet/devices/");
        if let Ok(device_id) = device_id_str.parse::<u32>() {
            if let Some(device) = state.get_bacnet_device(device_id) {
                send_to_client(
                    state,
                    session_id,
                    ServerMessage::success(id, Some(serde_json::to_value(device).unwrap_or(Value::Null))),
                )
                .await;
                return;
            }
        }
        send_error(
            state,
            session_id,
            Some(id),
            ErrorCode::NotFound,
            format!("BACnet device not found: {}", path),
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

//! Server Application State
//!
//! Shared state accessible by all WebSocket handlers.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use wildmatch::WildMatch;

use blueprint_runtime::service::ServiceManager;
use blueprint_types::TypeRegistry;

use crate::project::{BlueprintConfig, Project};

use super::protocol::ServerMessage;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    /// Currently loaded project
    project: RwLock<Option<Arc<Project>>>,

    /// Project path (for reloading)
    project_path: RwLock<Option<PathBuf>>,

    /// Service manager
    service_manager: Arc<ServiceManager>,

    /// Type registry
    type_registry: Arc<TypeRegistry>,

    /// Connected clients
    clients: RwLock<HashMap<Uuid, ClientState>>,

    /// Broadcast channel for server-wide notifications
    broadcast_tx: broadcast::Sender<ServerMessage>,
}

/// Per-client state
#[derive(Debug, Clone)]
pub struct ClientState {
    /// Client session ID
    pub session_id: Uuid,

    /// Subscribed paths (with potential wildcards)
    pub subscriptions: HashSet<String>,

    /// Channel to send messages to this client
    pub tx: tokio::sync::mpsc::Sender<ServerMessage>,
}

impl AppState {
    /// Create new application state
    pub fn new(service_manager: Arc<ServiceManager>, type_registry: Arc<TypeRegistry>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(256);

        Self {
            inner: Arc::new(AppStateInner {
                project: RwLock::new(None),
                project_path: RwLock::new(None),
                service_manager,
                type_registry,
                clients: RwLock::new(HashMap::new()),
                broadcast_tx,
            }),
        }
    }

    /// Get the current project
    pub async fn project(&self) -> Option<Arc<Project>> {
        self.inner.project.read().await.clone()
    }

    /// Set the current project
    pub async fn set_project(&self, project: Project, path: PathBuf) {
        *self.inner.project.write().await = Some(Arc::new(project));
        *self.inner.project_path.write().await = Some(path);
    }

    /// Get the project path
    pub async fn project_path(&self) -> Option<PathBuf> {
        self.inner.project_path.read().await.clone()
    }

    /// Update a blueprint in the project state
    pub async fn update_blueprint(&self, blueprint: BlueprintConfig) {
        let mut project_guard = self.inner.project.write().await;
        if let Some(project_arc) = project_guard.take() {
            // Clone the project and update the blueprint
            let mut project = (*project_arc).clone();
            project.blueprints.insert(blueprint.id.clone(), blueprint);
            *project_guard = Some(Arc::new(project));
            tracing::debug!("Updated blueprint in project state");
        }
    }

    /// Remove a blueprint from the project state
    pub async fn remove_blueprint(&self, blueprint_id: &str) {
        let mut project_guard = self.inner.project.write().await;
        if let Some(project_arc) = project_guard.take() {
            let mut project = (*project_arc).clone();
            project.blueprints.remove(blueprint_id);
            *project_guard = Some(Arc::new(project));
            tracing::debug!("Removed blueprint from project state");
        }
    }

    /// Get the service manager
    pub fn service_manager(&self) -> &Arc<ServiceManager> {
        &self.inner.service_manager
    }

    /// Get the type registry
    pub fn type_registry(&self) -> &Arc<TypeRegistry> {
        &self.inner.type_registry
    }

    /// Register a new client connection
    pub async fn register_client(&self, tx: tokio::sync::mpsc::Sender<ServerMessage>) -> Uuid {
        let session_id = Uuid::new_v4();
        let client = ClientState {
            session_id,
            subscriptions: HashSet::new(),
            tx,
        };

        self.inner.clients.write().await.insert(session_id, client);
        tracing::info!("Client connected: {}", session_id);

        session_id
    }

    /// Remove a client connection
    pub async fn remove_client(&self, session_id: Uuid) {
        self.inner.clients.write().await.remove(&session_id);
        tracing::info!("Client disconnected: {}", session_id);
    }

    /// Add subscriptions for a client
    pub async fn subscribe(&self, session_id: Uuid, paths: Vec<String>) {
        let mut clients = self.inner.clients.write().await;
        if let Some(client) = clients.get_mut(&session_id) {
            for path in paths {
                tracing::debug!("Client {} subscribed to: {}", session_id, path);
                client.subscriptions.insert(path);
            }
        }
    }

    /// Remove subscriptions for a client
    pub async fn unsubscribe(&self, session_id: Uuid, paths: Vec<String>) {
        let mut clients = self.inner.clients.write().await;
        if let Some(client) = clients.get_mut(&session_id) {
            for path in &paths {
                client.subscriptions.remove(path);
            }
        }
    }

    /// Get subscriptions for a client
    pub async fn get_subscriptions(&self, session_id: Uuid) -> HashSet<String> {
        let clients = self.inner.clients.read().await;
        clients
            .get(&session_id)
            .map(|c| c.subscriptions.clone())
            .unwrap_or_default()
    }

    /// Broadcast a message to all clients subscribed to a path
    pub async fn broadcast(&self, path: &str, message: ServerMessage) {
        let clients = self.inner.clients.read().await;

        tracing::info!("Broadcasting to path: {} ({} clients connected)", path, clients.len());

        for client in clients.values() {
            let matches = Self::matches_any_subscription(&client.subscriptions, path);
            tracing::debug!(
                "Client {} subscriptions: {:?}, matches {}: {}",
                client.session_id,
                client.subscriptions,
                path,
                matches
            );
            if matches {
                if let Err(e) = client.tx.try_send(message.clone()) {
                    tracing::warn!(
                        "Failed to send message to client {}: {}",
                        client.session_id,
                        e
                    );
                } else {
                    tracing::info!("Sent change to client {}", client.session_id);
                }
            }
        }
    }

    /// Broadcast a message to all connected clients
    pub async fn broadcast_all(&self, message: ServerMessage) {
        let clients = self.inner.clients.read().await;

        for client in clients.values() {
            if let Err(e) = client.tx.try_send(message.clone()) {
                tracing::warn!(
                    "Failed to send message to client {}: {}",
                    client.session_id,
                    e
                );
            }
        }
    }

    /// Get the global broadcast sender
    pub fn broadcast_tx(&self) -> broadcast::Sender<ServerMessage> {
        self.inner.broadcast_tx.clone()
    }

    /// Subscribe to global broadcasts
    pub fn subscribe_broadcasts(&self) -> broadcast::Receiver<ServerMessage> {
        self.inner.broadcast_tx.subscribe()
    }

    /// Check if a path matches any subscription pattern
    fn matches_any_subscription(subscriptions: &HashSet<String>, path: &str) -> bool {
        for pattern in subscriptions {
            // Convert our patterns to wildmatch format
            // /devices/* -> /devices/*
            // /devices/** -> /devices/**
            // Exact match
            if pattern == path {
                return true;
            }

            // Wildcard match
            let wildcard_pattern = if pattern.ends_with("/**") {
                // Recursive: /devices/** matches /devices/foo and /devices/foo/bar
                let prefix = &pattern[..pattern.len() - 3];
                if path.starts_with(prefix) {
                    return true;
                }
                continue;
            } else if pattern.ends_with("/*") {
                // Single level: /devices/* matches /devices/foo but not /devices/foo/bar
                let prefix = &pattern[..pattern.len() - 2];
                if path.starts_with(prefix) {
                    let remainder = &path[prefix.len()..];
                    // Should have exactly one more path segment
                    if remainder.starts_with('/') && !remainder[1..].contains('/') {
                        return true;
                    }
                    // Also match the exact prefix path
                    if remainder.is_empty() {
                        return true;
                    }
                }
                continue;
            } else {
                pattern.clone()
            };

            // Use wildmatch for more complex patterns
            let matcher = WildMatch::new(&wildcard_pattern);
            if matcher.matches(path) {
                return true;
            }
        }

        false
    }

    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.inner.clients.read().await.len()
    }

    /// Send a message to a specific client
    pub async fn send_to_client(&self, session_id: Uuid, message: ServerMessage) {
        let clients = self.inner.clients.read().await;
        if let Some(client) = clients.get(&session_id) {
            let _ = client.tx.try_send(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_subscriptions(patterns: &[&str]) -> HashSet<String> {
        patterns.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_exact_match() {
        let subs = create_subscriptions(&["/devices/vav-101"]);
        assert!(AppState::matches_any_subscription(&subs, "/devices/vav-101"));
        assert!(!AppState::matches_any_subscription(&subs, "/devices/vav-102"));
    }

    #[test]
    fn test_single_wildcard() {
        let subs = create_subscriptions(&["/devices/*"]);
        assert!(AppState::matches_any_subscription(&subs, "/devices/vav-101"));
        assert!(AppState::matches_any_subscription(&subs, "/devices/ahu-001"));
        assert!(!AppState::matches_any_subscription(
            &subs,
            "/devices/vav-101/points"
        ));
    }

    #[test]
    fn test_recursive_wildcard() {
        let subs = create_subscriptions(&["/devices/**"]);
        assert!(AppState::matches_any_subscription(&subs, "/devices/vav-101"));
        assert!(AppState::matches_any_subscription(
            &subs,
            "/devices/vav-101/points"
        ));
        assert!(AppState::matches_any_subscription(
            &subs,
            "/devices/vav-101/points/zone-temp"
        ));
        assert!(!AppState::matches_any_subscription(&subs, "/schedules/foo"));
    }

    #[test]
    fn test_multiple_subscriptions() {
        let subs = create_subscriptions(&["/devices/*", "/schedules/*"]);
        assert!(AppState::matches_any_subscription(&subs, "/devices/vav-101"));
        assert!(AppState::matches_any_subscription(
            &subs,
            "/schedules/occupancy"
        ));
        assert!(!AppState::matches_any_subscription(&subs, "/alarms/foo"));
    }
}

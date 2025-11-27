// Blueprint Service - Actor that manages blueprint execution with hot reload
//
// The BlueprintService handles:
// - Loading blueprints from JSON files
// - Hot reloading when files change
// - Dispatching events to blueprints
// - Managing latent node state

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use kameo::actor::{ActorRef, Spawn};
use kameo::message::{Context, Message};
use kameo_actors::pubsub::Publish;
use notify::{Event as NotifyEvent, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::actors::PubSubBroker;
use crate::messages::Event;
use crate::services::actor::{ServiceActorRef, ServiceMetadata, ServiceType};
use crate::services::registry::{RegistryMsg, RegistryReply, ServiceRegistry};

use super::executor::{BlueprintExecutor, ExecutionContext};
use super::registry::NodeRegistry;
use super::service_adapter::BlueprintServiceAdapter;
use super::types::{Blueprint, ExecutionResult, ExecutionTrigger, LatentState, WakeCondition};

// ─────────────────────────────────────────────────────────────────────────────
// Messages
// ─────────────────────────────────────────────────────────────────────────────

/// Load a blueprint from a JSON file
#[derive(Debug)]
pub struct LoadBlueprint {
    pub path: PathBuf,
}

/// Unload a blueprint by ID
#[derive(Debug)]
pub struct UnloadBlueprint {
    pub blueprint_id: String,
}

/// Reload all blueprints from disk
#[derive(Debug)]
pub struct ReloadAll;

/// Trigger an event that may start blueprint execution
#[derive(Debug, Clone)]
pub struct TriggerEvent {
    pub event_type: String,
    pub data: Value,
}

/// Execute a blueprint by request (manual trigger)
#[derive(Debug)]
pub struct ExecuteBlueprint {
    pub blueprint_id: String,
    pub event_node: String,
    pub inputs: Value,
}

/// Get list of loaded blueprints
#[derive(Debug)]
pub struct ListBlueprints;

/// Get details of a specific blueprint
#[derive(Debug)]
pub struct GetBlueprint {
    pub blueprint_id: String,
}

/// Register a custom node from a plugin
pub struct RegisterCustomNode {
    pub definition: super::types::NodeDef,
    pub executor: Arc<dyn super::registry::NodeExecutor>,
}

/// Internal message for file change notifications
#[derive(Debug)]
pub struct FileChanged {
    pub path: PathBuf,
    pub kind: FileChangeKind,
}

#[derive(Debug, Clone)]
pub enum FileChangeKind {
    Created,
    Modified,
    Removed,
}

/// Check and resume any pending latent executions
#[derive(Debug)]
pub struct TickLatent;

/// Register all blueprints with service.enabled as services
#[derive(Debug)]
pub struct RegisterServiceBlueprints;

/// Handle an event from PubSub (for blueprints to react to system events)
#[derive(Debug, Clone)]
pub struct HandleEvent {
    pub event: Event,
}

/// Set the PubSub and ServiceRegistry references
#[derive(Debug)]
pub struct SetServiceRefs {
    pub pubsub: ActorRef<PubSubBroker>,
    pub service_registry: ActorRef<ServiceRegistry>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Response Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub node_count: usize,
    pub connection_count: usize,
    pub file_path: Option<String>,
}

impl BlueprintInfo {
    fn from_blueprint(bp: &Blueprint, path: Option<&Path>) -> Self {
        Self {
            id: bp.id.clone(),
            name: bp.name.clone(),
            version: bp.version.clone(),
            description: bp.description.clone(),
            node_count: bp.nodes.len(),
            connection_count: bp.connections.len(),
            file_path: path.map(|p| p.display().to_string()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint Service Actor
// ─────────────────────────────────────────────────────────────────────────────

/// Actor that manages blueprint execution with hot reload
#[derive(kameo::Actor)]
pub struct BlueprintService {
    /// Node registry with built-in and custom nodes
    registry: Arc<NodeRegistry>,
    /// Loaded blueprints (id -> blueprint)
    blueprints: HashMap<String, Arc<Blueprint>>,
    /// Map from file path to blueprint ID (for hot reload)
    path_to_id: HashMap<PathBuf, String>,
    /// Blueprint executor
    executor: BlueprintExecutor,
    /// Suspended executions waiting for conditions
    suspended: HashMap<String, SuspendedExecution>,
    /// Directory to watch for blueprint files
    blueprints_dir: PathBuf,
    /// File watcher (kept alive)
    #[allow(dead_code)]
    watcher: Option<RecommendedWatcher>,
    /// Channel receiver for file events
    file_event_rx: Option<mpsc::UnboundedReceiver<FileChanged>>,
    /// PubSub broker for event publishing (optional)
    pubsub: Option<ActorRef<PubSubBroker>>,
    /// Service registry for blueprint-as-service registration (optional)
    service_registry: Option<ActorRef<ServiceRegistry>>,
    /// Registered blueprint service adapters (blueprint_id -> adapter actor)
    service_adapters: HashMap<String, ActorRef<BlueprintServiceAdapter>>,
}

struct SuspendedExecution {
    blueprint_id: String,
    context: ExecutionContext,
    state: LatentState,
}

impl BlueprintService {
    /// Create a new BlueprintService
    pub fn new(blueprints_dir: impl AsRef<Path>) -> Self {
        let registry = Arc::new(NodeRegistry::with_builtins());
        let executor = BlueprintExecutor::new(Arc::clone(&registry));

        Self {
            registry,
            blueprints: HashMap::new(),
            path_to_id: HashMap::new(),
            executor,
            suspended: HashMap::new(),
            blueprints_dir: blueprints_dir.as_ref().to_path_buf(),
            watcher: None,
            file_event_rx: None,
            pubsub: None,
            service_registry: None,
            service_adapters: HashMap::new(),
        }
    }

    /// Create a new BlueprintService with PubSub and ServiceRegistry
    pub fn with_services(
        blueprints_dir: impl AsRef<Path>,
        pubsub: ActorRef<PubSubBroker>,
        service_registry: ActorRef<ServiceRegistry>,
    ) -> Self {
        let mut service = Self::new(blueprints_dir);
        service.pubsub = Some(pubsub);
        service.service_registry = Some(service_registry);
        service
    }

    /// Set the PubSub broker
    pub fn set_pubsub(&mut self, pubsub: ActorRef<PubSubBroker>) {
        self.pubsub = Some(pubsub);
    }

    /// Set the ServiceRegistry
    pub fn set_service_registry(&mut self, registry: ActorRef<ServiceRegistry>) {
        self.service_registry = Some(registry);
    }

    /// Publish an event to PubSub
    pub async fn publish_event(&self, event: Event) {
        if let Some(pubsub) = &self.pubsub {
            if let Err(e) = pubsub.tell(Publish(event)).await {
                warn!("Failed to publish event: {}", e);
            }
        }
    }

    /// Start watching the blueprints directory for changes
    pub fn start_watching(&mut self) -> Result<(), String> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.file_event_rx = Some(rx);

        let blueprints_dir = self.blueprints_dir.clone();

        // Create file watcher
        let mut watcher = notify::recommended_watcher(move |res: Result<NotifyEvent, _>| {
            if let Ok(event) = res {
                let kind = match event.kind {
                    EventKind::Create(_) => Some(FileChangeKind::Created),
                    EventKind::Modify(_) => Some(FileChangeKind::Modified),
                    EventKind::Remove(_) => Some(FileChangeKind::Removed),
                    _ => None,
                };

                if let Some(kind) = kind {
                    for path in event.paths {
                        // Only care about .json files
                        if path.extension().and_then(|s| s.to_str()) == Some("json") {
                            let _ = tx.send(FileChanged {
                                path: path.clone(),
                                kind: kind.clone(),
                            });
                        }
                    }
                }
            }
        })
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

        // Start watching
        watcher
            .watch(&blueprints_dir, RecursiveMode::NonRecursive)
            .map_err(|e| format!("Failed to watch directory: {}", e))?;

        self.watcher = Some(watcher);

        info!(
            dir = %blueprints_dir.display(),
            "Started watching blueprints directory"
        );

        Ok(())
    }

    /// Process any pending file change events
    fn process_file_events(&mut self) {
        // Collect all events first to avoid borrow issues
        let events: Vec<FileChanged> = if let Some(rx) = &mut self.file_event_rx {
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            events
        } else {
            return;
        };

        // Now process the collected events
        for event in events {
            match event.kind {
                FileChangeKind::Created | FileChangeKind::Modified => {
                    info!(path = %event.path.display(), "Blueprint file changed, reloading...");
                    if let Err(e) = self.load_blueprint_file(&event.path) {
                        warn!(
                            path = %event.path.display(),
                            error = %e,
                            "Failed to reload blueprint"
                        );
                    }
                }
                FileChangeKind::Removed => {
                    if let Some(id) = self.path_to_id.remove(&event.path) {
                        info!(
                            blueprint_id = %id,
                            path = %event.path.display(),
                            "Blueprint file removed, unloading..."
                        );
                        self.blueprints.remove(&id);
                    }
                }
            }
        }
    }

    /// Load all blueprints from the blueprints directory
    pub fn load_all(&mut self) -> Result<usize, std::io::Error> {
        let dir = &self.blueprints_dir;
        if !dir.exists() {
            info!(path = %dir.display(), "Blueprints directory does not exist, creating");
            std::fs::create_dir_all(dir)?;
            return Ok(0);
        }

        let mut loaded = 0;
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_blueprint_file(&path) {
                    Ok(id) => {
                        info!(blueprint_id = %id, path = %path.display(), "Loaded blueprint");
                        loaded += 1;
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Failed to load blueprint");
                    }
                }
            }
        }

        info!(count = loaded, "Loaded blueprints");
        Ok(loaded)
    }

    /// Load a single blueprint file
    fn load_blueprint_file(&mut self, path: &Path) -> Result<String, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        let blueprint: Blueprint = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse blueprint JSON: {}", e))?;

        // Validate the blueprint
        self.validate_blueprint(&blueprint)?;

        let id = blueprint.id.clone();

        // Remove old mapping if this file was previously loaded with a different ID
        if let Some(old_id) = self.path_to_id.get(path) {
            if old_id != &id {
                self.blueprints.remove(old_id);
            }
        }

        // Update mappings
        self.path_to_id.insert(path.to_path_buf(), id.clone());
        self.blueprints.insert(id.clone(), Arc::new(blueprint));

        Ok(id)
    }

    /// Validate a blueprint's structure
    fn validate_blueprint(&self, blueprint: &Blueprint) -> Result<(), String> {
        // Check that all node types exist in the registry
        for node in &blueprint.nodes {
            // Skip event nodes - they're handled specially
            if node.node_type.contains("/On") || node.node_type.ends_with("Event") {
                continue;
            }

            if self.registry.get_definition(&node.node_type).is_none() {
                return Err(format!(
                    "Unknown node type '{}' in node '{}'",
                    node.node_type, node.id
                ));
            }
        }

        // Check that all connections reference valid nodes
        for conn in &blueprint.connections {
            if let Some((from_node, _)) = conn.from_parts() {
                if blueprint.get_node(from_node).is_none() {
                    return Err(format!(
                        "Connection references unknown source node '{}'",
                        from_node
                    ));
                }
            }

            if let Some((to_node, _)) = conn.to_parts() {
                if blueprint.get_node(to_node).is_none() {
                    return Err(format!(
                        "Connection references unknown target node '{}'",
                        to_node
                    ));
                }
            }
        }

        Ok(())
    }

    /// Find blueprints that handle a specific event type
    fn find_event_handlers(&self, event_type: &str) -> Vec<(Arc<Blueprint>, String)> {
        let mut handlers = Vec::new();

        for blueprint in self.blueprints.values() {
            for node in blueprint.event_nodes() {
                // Check if this event node handles this event type
                let matches = if let Some(config_event) = node.config.get("event_type") {
                    config_event.as_str() == Some(event_type)
                } else {
                    // Check if the node type matches (e.g., "neo/OnPointChanged" for "point_changed")
                    node.node_type
                        .to_lowercase()
                        .contains(&event_type.to_lowercase().replace('_', ""))
                };

                if matches {
                    handlers.push((Arc::clone(blueprint), node.id.clone()));
                }
            }
        }

        handlers
    }

    /// Process pending latent executions
    async fn tick_latent(&mut self) {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Find one-shot delays ready to resume
        let ready_delays: Vec<String> = self
            .suspended
            .iter()
            .filter(|(_, exec)| match &exec.state.wake_condition {
                WakeCondition::Delay { until_ms } => now_ms >= *until_ms,
                _ => false,
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Find interval timers ready to tick
        let ready_intervals: Vec<String> = self
            .suspended
            .iter()
            .filter(|(_, exec)| match &exec.state.wake_condition {
                WakeCondition::Interval { next_tick_ms, .. } => now_ms >= *next_tick_ms,
                _ => false,
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Resume one-shot delays (remove from suspended)
        for id in ready_delays {
            if let Some(mut exec) = self.suspended.remove(&id) {
                debug!(
                    blueprint_id = %exec.blueprint_id,
                    node_id = %exec.state.node_id,
                    "Resuming delay execution"
                );

                let result = self.executor.resume(&mut exec.context, &exec.state).await;

                // Handle result
                match result {
                    ExecutionResult::Completed { .. } => {
                        debug!(blueprint_id = %exec.blueprint_id, "Latent execution completed");
                    }
                    ExecutionResult::Suspended { state } => {
                        // Re-suspend with new state
                        let new_id = format!("{}-{}", exec.blueprint_id, state.node_id);
                        self.suspended.insert(
                            new_id,
                            SuspendedExecution {
                                blueprint_id: exec.blueprint_id,
                                context: exec.context,
                                state,
                            },
                        );
                    }
                    ExecutionResult::Failed { error } => {
                        error!(
                            blueprint_id = %exec.blueprint_id,
                            error = %error,
                            "Latent execution failed"
                        );
                    }
                }
            }
        }

        // Handle interval timers (keep in suspended, update next_tick)
        for id in ready_intervals {
            if let Some(mut exec) = self.suspended.remove(&id) {
                // Extract interval info before execution
                let (interval_ms, timer_id, tick_count) = match &exec.state.wake_condition {
                    WakeCondition::Interval { interval_ms, timer_id, tick_count, .. } => {
                        (*interval_ms, timer_id.clone(), *tick_count)
                    }
                    _ => continue,
                };

                debug!(
                    blueprint_id = %exec.blueprint_id,
                    node_id = %exec.state.node_id,
                    tick_count = tick_count,
                    "Timer tick"
                );

                // Set tick_count in context for the execution
                exec.context.set_node_output(
                    &exec.state.node_id,
                    "tick_count",
                    serde_json::Value::from(tick_count as i64),
                );

                // Execute the tick
                let result = self.executor.resume(&mut exec.context, &exec.state).await;

                // Schedule next tick regardless of result (timer keeps running)
                let next_tick_ms = now_ms + interval_ms;
                let new_tick_count = tick_count + 1;

                // Update state for next tick
                exec.state.wake_condition = WakeCondition::Interval {
                    interval_ms,
                    next_tick_ms,
                    timer_id: timer_id.clone(),
                    tick_count: new_tick_count,
                };

                // Handle execution result
                match result {
                    ExecutionResult::Completed { .. } => {
                        debug!(blueprint_id = %exec.blueprint_id, "Timer tick completed");
                    }
                    ExecutionResult::Suspended { state: _ } => {
                        // Timer tick produced another suspension - this is unusual but handle it
                        debug!(blueprint_id = %exec.blueprint_id, "Timer tick suspended (unusual)");
                    }
                    ExecutionResult::Failed { error } => {
                        error!(
                            blueprint_id = %exec.blueprint_id,
                            error = %error,
                            "Timer tick failed"
                        );
                    }
                }

                // Re-insert for next tick
                self.suspended.insert(id, exec);
            }
        }
    }

    /// Register a custom node type
    fn register_custom_node(
        &mut self,
        definition: super::types::NodeDef,
        executor_fn: Arc<dyn super::registry::NodeExecutor>,
    ) {
        let mut new_registry = NodeRegistry::with_builtins();
        let node_id = definition.id.clone();
        new_registry.register(definition, executor_fn);

        self.registry = Arc::new(new_registry);
        self.executor = BlueprintExecutor::new(Arc::clone(&self.registry));

        info!(node_id = %node_id, "Registered custom node");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Message Handlers
// ─────────────────────────────────────────────────────────────────────────────

impl Message<LoadBlueprint> for BlueprintService {
    type Reply = Result<String, String>;

    async fn handle(
        &mut self,
        msg: LoadBlueprint,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.load_blueprint_file(&msg.path)
    }
}

impl Message<UnloadBlueprint> for BlueprintService {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: UnloadBlueprint,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.blueprints.remove(&msg.blueprint_id).is_some() {
            // Also remove from path mapping
            self.path_to_id.retain(|_, id| id != &msg.blueprint_id);
            info!(blueprint_id = %msg.blueprint_id, "Unloaded blueprint");
            true
        } else {
            false
        }
    }
}

impl Message<ReloadAll> for BlueprintService {
    type Reply = Result<usize, String>;

    async fn handle(
        &mut self,
        _msg: ReloadAll,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Clear existing blueprints
        self.blueprints.clear();
        self.path_to_id.clear();

        // Reload all
        self.load_all().map_err(|e| e.to_string())
    }
}

impl Message<TriggerEvent> for BlueprintService {
    type Reply = Vec<ExecutionResult>;

    async fn handle(
        &mut self,
        msg: TriggerEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Process any pending file changes first
        self.process_file_events();

        debug!(
            event_type = %msg.event_type,
            "Triggering event for blueprints"
        );

        let handlers = self.find_event_handlers(&msg.event_type);
        let mut results = Vec::new();

        for (blueprint, event_node_id) in handlers {
            let trigger = ExecutionTrigger::Event {
                event_type: msg.event_type.clone(),
                data: msg.data.clone(),
            };

            let result = self
                .executor
                .execute(Arc::clone(&blueprint), &event_node_id, trigger.clone())
                .await;

            // Handle suspended executions
            if let ExecutionResult::Suspended { state } = &result {
                let ctx = ExecutionContext::new(Arc::clone(&blueprint), trigger);
                let suspension_id = format!("{}-{}", blueprint.id, state.node_id);
                self.suspended.insert(
                    suspension_id,
                    SuspendedExecution {
                        blueprint_id: blueprint.id.clone(),
                        context: ctx,
                        state: state.clone(),
                    },
                );
            }

            results.push(result);
        }

        // Also check if any suspended executions are waiting for this event
        let waiting_for_event: Vec<String> = self
            .suspended
            .iter()
            .filter(|(_, exec)| matches!(
                &exec.state.wake_condition,
                WakeCondition::Event { event_type, .. } if event_type == &msg.event_type
            ))
            .map(|(id, _)| id.clone())
            .collect();

        for id in waiting_for_event {
            if let Some(mut exec) = self.suspended.remove(&id) {
                // Set the event data as output
                exec.context
                    .set_node_output(&exec.state.node_id, "event_data", msg.data.clone());

                let result = self.executor.resume(&mut exec.context, &exec.state).await;

                if let ExecutionResult::Suspended { state } = result {
                    let new_id = format!("{}-{}", exec.blueprint_id, state.node_id);
                    self.suspended.insert(
                        new_id,
                        SuspendedExecution {
                            blueprint_id: exec.blueprint_id,
                            context: exec.context,
                            state,
                        },
                    );
                }
            }
        }

        results
    }
}

impl Message<ExecuteBlueprint> for BlueprintService {
    type Reply = Result<ExecutionResult, String>;

    async fn handle(
        &mut self,
        msg: ExecuteBlueprint,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Process any pending file changes first
        self.process_file_events();

        let blueprint = self
            .blueprints
            .get(&msg.blueprint_id)
            .ok_or_else(|| format!("Blueprint '{}' not found", msg.blueprint_id))?;

        let trigger = ExecutionTrigger::Request {
            inputs: msg.inputs,
        };

        let result = self
            .executor
            .execute(Arc::clone(blueprint), &msg.event_node, trigger)
            .await;

        Ok(result)
    }
}

impl Message<ListBlueprints> for BlueprintService {
    type Reply = Vec<BlueprintInfo>;

    async fn handle(
        &mut self,
        _msg: ListBlueprints,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.blueprints
            .iter()
            .map(|(id, bp)| {
                let path = self
                    .path_to_id
                    .iter()
                    .find(|(_, bp_id)| *bp_id == id)
                    .map(|(p, _)| p.as_path());
                BlueprintInfo::from_blueprint(bp, path)
            })
            .collect()
    }
}

impl Message<GetBlueprint> for BlueprintService {
    type Reply = Option<Arc<Blueprint>>;

    async fn handle(
        &mut self,
        msg: GetBlueprint,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.blueprints.get(&msg.blueprint_id).cloned()
    }
}

impl Message<RegisterCustomNode> for BlueprintService {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RegisterCustomNode,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.register_custom_node(msg.definition, msg.executor);
    }
}

impl Message<FileChanged> for BlueprintService {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: FileChanged,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg.kind {
            FileChangeKind::Created | FileChangeKind::Modified => {
                info!(path = %msg.path.display(), "Blueprint file changed, reloading...");
                if let Err(e) = self.load_blueprint_file(&msg.path) {
                    warn!(
                        path = %msg.path.display(),
                        error = %e,
                        "Failed to reload blueprint"
                    );
                }
            }
            FileChangeKind::Removed => {
                if let Some(id) = self.path_to_id.remove(&msg.path) {
                    info!(
                        blueprint_id = %id,
                        path = %msg.path.display(),
                        "Blueprint file removed, unloading..."
                    );
                    self.blueprints.remove(&id);
                }
            }
        }
    }
}

impl Message<TickLatent> for BlueprintService {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: TickLatent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.tick_latent().await;
    }
}

impl Message<SetServiceRefs> for BlueprintService {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetServiceRefs,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.pubsub = Some(msg.pubsub);
        self.service_registry = Some(msg.service_registry);
        info!("BlueprintService connected to PubSub and ServiceRegistry");
    }
}

impl Message<RegisterServiceBlueprints> for BlueprintService {
    type Reply = usize;

    async fn handle(
        &mut self,
        _msg: RegisterServiceBlueprints,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(registry) = &self.service_registry else {
            warn!("Cannot register blueprint services: ServiceRegistry not set");
            return 0;
        };

        let registry = registry.clone();
        let self_ref = ctx.actor_ref().clone();
        let mut registered = 0;

        // Find all blueprints with service.enabled = true
        let service_blueprints: Vec<Arc<Blueprint>> = self
            .blueprints
            .values()
            .filter(|bp| bp.is_service())
            .cloned()
            .collect();

        for blueprint in service_blueprints {
            let blueprint_id = blueprint.id.clone();

            // Skip if already registered
            if self.service_adapters.contains_key(&blueprint_id) {
                continue;
            }

            // Create the adapter actor
            let adapter = BlueprintServiceAdapter::new(
                Arc::clone(&blueprint),
                self_ref.clone(),
            );
            let adapter_ref = BlueprintServiceAdapter::spawn(adapter);

            // Create ServiceActorRef
            let service_ref = ServiceActorRef::new(
                adapter_ref.clone(),
                ServiceMetadata {
                    id: format!("blueprint:{}", blueprint_id),
                    name: blueprint.name.clone(),
                    description: blueprint
                        .service
                        .as_ref()
                        .and_then(|s| s.description.clone())
                        .unwrap_or_else(|| blueprint.description.clone().unwrap_or_default()),
                    service_type: ServiceType::Native,
                },
            );

            // Register with ServiceRegistry
            let subscriptions = blueprint.service_subscriptions();
            match registry
                .ask(RegistryMsg::Register {
                    actor_ref: service_ref,
                    subscriptions: subscriptions.clone(),
                })
                .await
            {
                Ok(RegistryReply::Registered) => {
                    info!(
                        blueprint_id = %blueprint_id,
                        subscriptions = ?subscriptions,
                        "Registered blueprint as service"
                    );
                    self.service_adapters.insert(blueprint_id, adapter_ref);
                    registered += 1;
                }
                Ok(RegistryReply::Failed(e)) => {
                    error!(
                        blueprint_id = %blueprint_id,
                        error = %e,
                        "Failed to register blueprint as service"
                    );
                }
                Err(e) => {
                    error!(
                        blueprint_id = %blueprint_id,
                        error = %e,
                        "Failed to communicate with ServiceRegistry"
                    );
                }
                _ => {}
            }
        }

        registered
    }
}

impl Message<HandleEvent> for BlueprintService {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: HandleEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // Convert Event to TriggerEvent for blueprint execution
        let (event_type, data) = match &msg.event {
            Event::ServiceStateChanged { service, state, .. } => {
                ("ServiceStateChanged".to_string(), serde_json::json!({
                    "service": service,
                    "state": format!("{:?}", state),
                }))
            }
            Event::PointValueChanged { point, value, .. } => {
                ("PointValueChanged".to_string(), serde_json::json!({
                    "point": point,
                    "value": value,
                }))
            }
            Event::AlarmRaised { source, message, severity, .. } => {
                ("AlarmRaised".to_string(), serde_json::json!({
                    "source": source,
                    "message": message,
                    "severity": format!("{:?}", severity),
                }))
            }
            Event::AlarmCleared { source, .. } => {
                ("AlarmCleared".to_string(), serde_json::json!({
                    "source": source,
                }))
            }
            Event::DeviceStatusChanged { device, network, status, .. } => {
                ("DeviceStatusChanged".to_string(), serde_json::json!({
                    "device": device,
                    "network": network,
                    "status": format!("{:?}", status),
                }))
            }
            Event::DeviceDiscovered { network, device, instance, address, .. } => {
                ("DeviceDiscovered".to_string(), serde_json::json!({
                    "network": network,
                    "device": device,
                    "instance": instance,
                    "address": address.to_string(),
                }))
            }
            Event::Custom { event_type, source, data, .. } => {
                (event_type.clone(), serde_json::json!({
                    "source": source,
                    "data": data,
                }))
            }
        };

        // Trigger blueprints that listen for this event
        let handlers = self.find_event_handlers(&event_type);

        for (blueprint, event_node_id) in handlers {
            let trigger = ExecutionTrigger::Event {
                event_type: event_type.clone(),
                data: data.clone(),
            };

            let result = self
                .executor
                .execute(Arc::clone(&blueprint), &event_node_id, trigger.clone())
                .await;

            // Handle suspended executions
            if let ExecutionResult::Suspended { state } = &result {
                let ctx = ExecutionContext::new(Arc::clone(&blueprint), trigger);
                let suspension_id = format!("{}-{}", blueprint.id, state.node_id);
                self.suspended.insert(
                    suspension_id,
                    SuspendedExecution {
                        blueprint_id: blueprint.id.clone(),
                        context: ctx,
                        state: state.clone(),
                    },
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper to start the blueprint service with watching
// ─────────────────────────────────────────────────────────────────────────────

/// Start the blueprint service background tasks (file watching, latent ticking)
pub fn start_background_tasks(
    actor_ref: ActorRef<BlueprintService>,
) -> tokio::task::JoinHandle<()> {
    let actor = actor_ref.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            // Tick latent executions
            let _ = actor.tell(TickLatent).await;
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_blueprint_service_creation() {
        let temp_dir = tempdir().unwrap();
        let service = BlueprintService::new(temp_dir.path());

        assert!(service.blueprints.is_empty());
    }

    #[tokio::test]
    async fn test_load_blueprint_file() {
        let temp_dir = tempdir().unwrap();
        let blueprint_path = temp_dir.path().join("test.json");

        // Create a test blueprint file
        let blueprint_json = r#"{
            "id": "test-blueprint",
            "name": "Test Blueprint",
            "nodes": [
                {"id": "event", "type": "neo/OnEvent", "config": {"event_type": "test"}},
                {"id": "log", "type": "neo/Log", "config": {"defaults": {"message": "Hello!"}}}
            ],
            "connections": [
                {"from": "event.exec", "to": "log.exec"}
            ]
        }"#;

        std::fs::write(&blueprint_path, blueprint_json).unwrap();

        let mut service = BlueprintService::new(temp_dir.path());
        let result = service.load_blueprint_file(&blueprint_path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-blueprint");
        assert!(service.blueprints.contains_key("test-blueprint"));
        assert!(service.path_to_id.contains_key(&blueprint_path));
    }

    #[tokio::test]
    async fn test_find_event_handlers() {
        let temp_dir = tempdir().unwrap();
        let mut service = BlueprintService::new(temp_dir.path());

        // Create and load a test blueprint
        let blueprint_path = temp_dir.path().join("test.json");
        let blueprint_json = r#"{
            "id": "event-test",
            "name": "Event Test",
            "nodes": [
                {"id": "event", "type": "neo/OnEvent", "config": {"event_type": "point_changed"}}
            ],
            "connections": []
        }"#;

        std::fs::write(&blueprint_path, blueprint_json).unwrap();
        service.load_blueprint_file(&blueprint_path).unwrap();

        let handlers = service.find_event_handlers("point_changed");
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].1, "event");
    }
}

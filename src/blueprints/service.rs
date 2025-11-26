// Blueprint Service - Actor that manages blueprint execution
//
// The BlueprintService handles:
// - Loading blueprints from JSON files
// - Managing blueprint lifecycle
// - Dispatching events to blueprints
// - Managing latent node state

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kameo::message::{Context, Message};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, warn};

use super::executor::{BlueprintExecutor, ExecutionContext};
use super::registry::NodeRegistry;
use super::types::{Blueprint, ExecutionResult, ExecutionTrigger, LatentState};

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
}

impl From<&Blueprint> for BlueprintInfo {
    fn from(bp: &Blueprint) -> Self {
        Self {
            id: bp.id.clone(),
            name: bp.name.clone(),
            version: bp.version.clone(),
            description: bp.description.clone(),
            node_count: bp.nodes.len(),
            connection_count: bp.connections.len(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint Service Actor
// ─────────────────────────────────────────────────────────────────────────────

/// Actor that manages blueprint execution
#[derive(kameo::Actor)]
pub struct BlueprintService {
    /// Node registry with built-in and custom nodes
    registry: Arc<NodeRegistry>,
    /// Loaded blueprints
    blueprints: HashMap<String, Arc<Blueprint>>,
    /// Blueprint executor
    executor: BlueprintExecutor,
    /// Suspended executions waiting for conditions
    suspended: HashMap<String, SuspendedExecution>,
    /// Directory to watch for blueprint files
    blueprints_dir: PathBuf,
}

struct SuspendedExecution {
    #[allow(dead_code)]
    blueprint_id: String,
    #[allow(dead_code)]
    context: ExecutionContext,
    #[allow(dead_code)]
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
            executor,
            suspended: HashMap::new(),
            blueprints_dir: blueprints_dir.as_ref().to_path_buf(),
        }
    }

    /// Load all blueprints from the blueprints directory
    pub async fn load_all(&mut self) -> Result<usize, std::io::Error> {
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
                // Event nodes have config.event_type or match by node type
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

    /// Register a custom node type
    fn register_custom_node(
        &mut self,
        definition: super::types::NodeDef,
        executor: Arc<dyn super::registry::NodeExecutor>,
    ) {
        // We need to get mutable access to the registry
        // Create a new registry with the custom node added
        let mut new_registry = NodeRegistry::with_builtins();
        let node_id = definition.id.clone();
        new_registry.register(definition, executor);

        // Copy existing custom nodes (this is a simplified approach)
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
            info!(blueprint_id = %msg.blueprint_id, "Unloaded blueprint");
            true
        } else {
            false
        }
    }
}

impl Message<TriggerEvent> for BlueprintService {
    type Reply = Vec<ExecutionResult>;

    async fn handle(
        &mut self,
        msg: TriggerEvent,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
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
                .execute(Arc::clone(&blueprint), &event_node_id, trigger)
                .await;

            // Handle suspended executions
            if let ExecutionResult::Suspended { state } = &result {
                let ctx = ExecutionContext::new(
                    Arc::clone(&blueprint),
                    ExecutionTrigger::Event {
                        event_type: msg.event_type.clone(),
                        data: msg.data.clone(),
                    },
                );
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
            .values()
            .map(|bp| BlueprintInfo::from(bp.as_ref()))
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

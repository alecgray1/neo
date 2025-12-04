// Node Registry - Stores node definitions and their executors
//
// The registry holds all available node types (built-in and plugin-provided).
// Each node type has a definition (pins, category, etc.) and an executor function.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use blueprint_types::NodeDef;

use super::executor::{NodeContext, NodeOutput};

// ─────────────────────────────────────────────────────────────────────────────
// Node Executor Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait for node execution
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Execute the node with the given context
    async fn execute(&self, ctx: &mut NodeContext) -> NodeOutput;
}

/// Function-based node executor (for simple nodes)
pub struct FnNodeExecutor<F>
where
    F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync,
{
    func: F,
}

impl<F> FnNodeExecutor<F>
where
    F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[async_trait]
impl<F> NodeExecutor for FnNodeExecutor<F>
where
    F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync,
{
    async fn execute(&self, ctx: &mut NodeContext) -> NodeOutput {
        (self.func)(ctx)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Node Registry
// ─────────────────────────────────────────────────────────────────────────────

/// Entry in the node registry
struct NodeEntry {
    definition: NodeDef,
    executor: Arc<dyn NodeExecutor>,
}

/// Registry of all available node types
pub struct NodeRegistry {
    nodes: HashMap<String, NodeEntry>,
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Register a node type with its executor
    pub fn register(&mut self, definition: NodeDef, executor: Arc<dyn NodeExecutor>) {
        let id = definition.id.clone();
        self.nodes.insert(id, NodeEntry {
            definition,
            executor,
        });
    }

    /// Register a node with a sync function executor
    pub fn register_fn<F>(&mut self, definition: NodeDef, func: F)
    where
        F: Fn(&mut NodeContext) -> NodeOutput + Send + Sync + 'static,
    {
        self.register(definition, Arc::new(FnNodeExecutor::new(func)));
    }

    /// Register a plugin node executor with a minimal definition
    ///
    /// Plugin nodes define their metadata in JavaScript, so we create a placeholder
    /// definition on the Rust side. The actual pins/category come from the JS.
    pub fn register_plugin(&mut self, id: &str, executor: Arc<dyn NodeExecutor>) {
        let definition = NodeDef {
            id: id.to_string(),
            name: id.split('/').last().unwrap_or(id).to_string(),
            category: "Plugin".to_string(),
            pure: false,
            latent: false,
            description: Some(format!("Plugin node: {}", id)),
            pins: vec![], // Pins are defined in JS, not visible here
        };
        self.register(definition, executor);
    }

    /// Get a node definition by ID
    pub fn get_definition(&self, id: &str) -> Option<&NodeDef> {
        self.nodes.get(id).map(|e| &e.definition)
    }

    /// Get a node executor by ID
    pub fn get_executor(&self, id: &str) -> Option<Arc<dyn NodeExecutor>> {
        self.nodes.get(id).map(|e| Arc::clone(&e.executor))
    }

    /// Get all registered node IDs
    pub fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.nodes.keys().map(|s| s.as_str())
    }

    /// Get all node definitions
    pub fn definitions(&self) -> impl Iterator<Item = &NodeDef> {
        self.nodes.values().map(|e| &e.definition)
    }

    /// Get nodes by category
    pub fn nodes_in_category(&self, category: &str) -> Vec<&NodeDef> {
        self.nodes
            .values()
            .filter(|e| e.definition.category == category)
            .map(|e| &e.definition)
            .collect()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<_> = self
            .nodes
            .values()
            .map(|e| e.definition.category.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// Check if a node is registered
    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// Get node count
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_empty_registry() {
        let registry = NodeRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_node() {
        let mut registry = NodeRegistry::new();

        let def = NodeDef {
            id: "test/Node".to_string(),
            name: "Test Node".to_string(),
            category: "Test".to_string(),
            pure: true,
            latent: false,
            pins: vec![],
            description: Some("A test node".to_string()),
        };

        registry.register_fn(def, |_ctx| NodeOutput::end(HashMap::new()));

        assert!(registry.contains("test/Node"));
        assert_eq!(registry.len(), 1);

        let retrieved = registry.get_definition("test/Node").unwrap();
        assert_eq!(retrieved.name, "Test Node");
    }

    #[test]
    fn test_categories() {
        let mut registry = NodeRegistry::new();

        registry.register_fn(
            NodeDef {
                id: "math/Add".to_string(),
                name: "Add".to_string(),
                category: "Math".to_string(),
                pure: true,
                latent: false,
                pins: vec![],
                description: None,
            },
            |_ctx| NodeOutput::end(HashMap::new()),
        );

        registry.register_fn(
            NodeDef {
                id: "logic/And".to_string(),
                name: "And".to_string(),
                category: "Logic".to_string(),
                pure: true,
                latent: false,
                pins: vec![],
                description: None,
            },
            |_ctx| NodeOutput::end(HashMap::new()),
        );

        let categories = registry.categories();
        assert!(categories.contains(&"Math".to_string()));
        assert!(categories.contains(&"Logic".to_string()));
    }
}

//! JavaScript Node Library
//!
//! Holds JS node definitions (code + metadata) without creating runtimes.
//! Used to store plugin node code that can be loaded into blueprint runtimes on demand.

use std::collections::HashMap;

/// A JavaScript node definition.
#[derive(Debug, Clone)]
pub struct JsNodeDef {
    /// Node type ID (e.g., "example/add")
    pub id: String,
    /// The JavaScript code for this node
    pub code: String,
}

/// Library of JavaScript node definitions.
///
/// This stores the code for all available JS nodes without creating any V8 runtimes.
/// When a blueprint needs a JS node, the code is retrieved from here and loaded
/// into the blueprint's shared runtime.
#[derive(Default)]
pub struct JsNodeLibrary {
    nodes: HashMap<String, JsNodeDef>,
}

impl JsNodeLibrary {
    /// Create a new empty library.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Register a node definition.
    ///
    /// The code should use `export default defineNode({...})`.
    pub fn register(&mut self, id: String, code: String) {
        tracing::debug!("Registered JS node definition: {}", id);
        self.nodes.insert(id.clone(), JsNodeDef { id, code });
    }

    /// Get a node definition by ID.
    pub fn get(&self, id: &str) -> Option<&JsNodeDef> {
        self.nodes.get(id)
    }

    /// Check if a node is registered.
    pub fn contains(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// Get all registered node IDs.
    pub fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.nodes.keys().map(|s| s.as_str())
    }

    /// Get the number of registered nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the library is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let mut lib = JsNodeLibrary::new();
        lib.register(
            "test/add".to_string(),
            "export default defineNode({...})".to_string(),
        );

        assert!(lib.contains("test/add"));
        assert!(!lib.contains("test/missing"));

        let def = lib.get("test/add").unwrap();
        assert_eq!(def.id, "test/add");
    }

    #[test]
    fn test_node_ids() {
        let mut lib = JsNodeLibrary::new();
        lib.register("a/node".to_string(), "code".to_string());
        lib.register("b/node".to_string(), "code".to_string());

        let ids: Vec<_> = lib.node_ids().collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"a/node"));
        assert!(ids.contains(&"b/node"));
    }
}

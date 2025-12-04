//! JavaScript Node Executor
//!
//! Executes plugin-defined blueprint nodes using the neo-js-runtime.
//! Each node runs in its own V8 runtime with `export default defineNode({...})`.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use neo_js_runtime::{spawn_runtime, RuntimeHandle, RuntimeServices};

use crate::executor::{NodeContext, NodeOutput};
use crate::registry::NodeExecutor;

/// A JavaScript runtime for a single node type.
///
/// Each PluginNodeRuntime runs one node definition in its own V8 isolate.
/// The node code should use `export default defineNode({...})`.
pub struct PluginNodeRuntime {
    /// The runtime handle for communicating with the JS thread
    handle: Arc<RuntimeHandle>,
    /// The node ID (e.g., "example/add")
    node_id: String,
}

impl PluginNodeRuntime {
    /// Create a new plugin node runtime by loading the node code.
    ///
    /// The code should use `export default defineNode({...})`.
    /// The node_id is assigned to the loaded definition.
    pub fn new(node_id: &str, node_code: &str) -> Result<Self, PluginRuntimeError> {
        let handle = spawn_runtime(
            format!("node:{}", node_id),
            node_code.to_string(),
            node_id.to_string(),
            RuntimeServices::default(),
        )
        .map_err(|e| PluginRuntimeError::SpawnFailed(e.to_string()))?;

        Ok(Self {
            handle: Arc::new(handle),
            node_id: node_id.to_string(),
        })
    }

    /// Execute the node with the given context.
    pub async fn execute(&self, ctx: &NodeContext) -> Result<NodeOutput, PluginRuntimeError> {
        if self.handle.is_terminated() {
            return Err(PluginRuntimeError::RuntimeTerminated);
        }

        // Serialize the context to send to JS
        let ctx_json = serde_json::json!({
            "nodeId": ctx.node_id,
            "config": ctx.config,
            "inputs": ctx.inputs,
            "variables": ctx.variables,
        });

        let context_str = serde_json::to_string(&ctx_json)
            .map_err(|e| PluginRuntimeError::SerializationError(e.to_string()))?;

        // Execute the node via the runtime handle (now async)
        let result = self.handle.execute_node(&context_str)
            .await
            .map_err(|e| PluginRuntimeError::ExecutionError(e.to_string()))?;

        // Extract values from the JS response
        let values: HashMap<String, serde_json::Value> = result
            .get("values")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // Check for error in the result
        if let Some(error) = result.get("error").and_then(|e| e.as_str()) {
            return Err(PluginRuntimeError::ExecutionError(error.to_string()));
        }

        Ok(NodeOutput::pure(values))
    }

    /// Get the node ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Terminate the runtime
    pub fn terminate(&self) {
        self.handle.terminate();
    }
}

impl Drop for PluginNodeRuntime {
    fn drop(&mut self) {
        self.terminate();
    }
}

/// Errors that can occur in the plugin runtime
#[derive(Debug, thiserror::Error)]
pub enum PluginRuntimeError {
    #[error("Failed to spawn runtime: {0}")]
    SpawnFailed(String),
    #[error("Runtime has terminated")]
    RuntimeTerminated,
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
}

/// Node executor that runs JavaScript plugin nodes.
///
/// This wraps a PluginNodeRuntime and implements NodeExecutor so it can
/// be registered in the NodeRegistry like any other node.
pub struct JsNodeExecutor {
    /// The node runtime
    runtime: Arc<PluginNodeRuntime>,
}

impl JsNodeExecutor {
    /// Create a new JS node executor.
    pub fn new(runtime: Arc<PluginNodeRuntime>) -> Self {
        Self { runtime }
    }

    /// Create a new JS node executor from code.
    pub fn from_code(node_id: &str, code: &str) -> Result<Self, PluginRuntimeError> {
        let runtime = PluginNodeRuntime::new(node_id, code)?;
        Ok(Self {
            runtime: Arc::new(runtime),
        })
    }
}

#[async_trait]
impl NodeExecutor for JsNodeExecutor {
    async fn execute(&self, ctx: &mut NodeContext) -> NodeOutput {
        match self.runtime.execute(ctx).await {
            Ok(output) => output,
            Err(e) => {
                tracing::error!(
                    node = %self.runtime.node_id(),
                    error = %e,
                    "JS node execution failed"
                );
                NodeOutput::error(e.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "SIGSEGV when running multiple V8 isolates in test harness"]
    async fn test_plugin_node_runtime_creation() {
        // Initialize the V8 platform
        neo_js_runtime::init_platform();

        let code = r#"
            export default defineNode({
                name: "My Test Node",
                inputs: [],
                outputs: [{ name: "result", type: "number" }],
                execute: async (ctx) => {
                    return { result: 42 };
                }
            });
        "#;

        let runtime = PluginNodeRuntime::new("test/MyNode", code);
        assert!(runtime.is_ok());

        let runtime = runtime.unwrap();
        assert_eq!(runtime.node_id(), "test/MyNode");

        runtime.terminate();
        // Give V8 time to clean up
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    #[tokio::test]
    async fn test_execute_node() {
        neo_js_runtime::init_platform();

        let code = r#"
            export default defineNode({
                name: "Add Numbers",
                inputs: [
                    { name: "a", type: "number" },
                    { name: "b", type: "number" }
                ],
                outputs: [{ name: "sum", type: "number" }],
                execute: async (ctx) => {
                    const a = ctx.getInput("a") || 0;
                    const b = ctx.getInput("b") || 0;
                    return { sum: a + b };
                }
            });
        "#;

        let runtime = PluginNodeRuntime::new("test/Add", code).unwrap();

        // Create a mock context
        let ctx = NodeContext {
            node_id: "node1".to_string(),
            config: serde_json::json!({}),
            inputs: {
                let mut m = HashMap::new();
                m.insert("a".to_string(), serde_json::json!(5));
                m.insert("b".to_string(), serde_json::json!(3));
                m
            },
            variables: HashMap::new(),
        };

        let result = runtime.execute(&ctx).await;
        assert!(result.is_ok(), "Execute failed: {:?}", result.err());

        let output = result.unwrap();
        assert_eq!(output.values.get("sum"), Some(&serde_json::json!(8)));

        runtime.terminate();
        // Give V8 time to clean up
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}

//! JavaScript Node Executor
//!
//! Executes plugin-defined blueprint nodes using the neo-js-runtime.
//! Plugin nodes are defined in JavaScript using Neo.nodes.register().

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use neo_js_runtime::{spawn_runtime, RuntimeHandle, RuntimeServices};
use tokio::sync::Mutex;

use crate::executor::{NodeContext, NodeOutput};
use crate::registry::NodeExecutor;

/// A JavaScript runtime that can execute plugin nodes.
///
/// One PluginRuntime is created per plugin bundle. It loads the plugin's
/// JavaScript code and can execute any nodes that plugin registers.
pub struct PluginRuntime {
    /// The runtime handle for communicating with the JS thread
    handle: RuntimeHandle,
    /// Plugin ID (e.g., "myPlugin")
    plugin_id: String,
}

impl PluginRuntime {
    /// Create a new plugin runtime by loading the plugin code.
    ///
    /// The code should register nodes using Neo.nodes.register().
    pub fn new(plugin_id: &str, plugin_code: &str) -> Result<Self, PluginRuntimeError> {
        let handle = spawn_runtime(
            format!("plugin:{}", plugin_id),
            plugin_code.to_string(),
            RuntimeServices::default(),
        )
        .map_err(|e| PluginRuntimeError::SpawnFailed(e.to_string()))?;

        Ok(Self {
            handle,
            plugin_id: plugin_id.to_string(),
        })
    }

    /// Execute a node that was registered by this plugin.
    ///
    /// The node_id should be the full node ID (e.g., "myPlugin/HttpGet").
    pub async fn execute_node(
        &self,
        node_id: &str,
        ctx: &NodeContext,
    ) -> Result<NodeOutput, PluginRuntimeError> {
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

        // Execute the node via the runtime handle
        let result_json = self
            .handle
            .execute_node(node_id, &context_str)
            .await
            .map_err(|e| PluginRuntimeError::ExecutionError(e.to_string()))?;

        // Parse the result
        let result: serde_json::Value = serde_json::from_str(&result_json)
            .map_err(|e| PluginRuntimeError::SerializationError(e.to_string()))?;

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

    /// Get the plugin ID
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Terminate the runtime
    pub fn terminate(&self) {
        self.handle.terminate();
    }
}

impl Drop for PluginRuntime {
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
/// This wraps a PluginRuntime and implements NodeExecutor so it can
/// be registered in the NodeRegistry like any other node.
pub struct JsNodeExecutor {
    /// The plugin runtime (shared across all nodes from this plugin)
    runtime: Arc<Mutex<PluginRuntime>>,
    /// The specific node ID this executor handles
    node_id: String,
}

impl JsNodeExecutor {
    /// Create a new JS node executor.
    ///
    /// The runtime is shared across all nodes from the same plugin.
    pub fn new(runtime: Arc<Mutex<PluginRuntime>>, node_id: String) -> Self {
        Self { runtime, node_id }
    }
}

#[async_trait]
impl NodeExecutor for JsNodeExecutor {
    async fn execute(&self, ctx: &mut NodeContext) -> NodeOutput {
        let runtime = self.runtime.lock().await;

        match runtime.execute_node(&self.node_id, ctx).await {
            Ok(output) => output,
            Err(e) => {
                tracing::error!(
                    node = %self.node_id,
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
    async fn test_plugin_runtime_creation() {
        // Initialize the V8 platform
        neo_js_runtime::init_platform();

        let code = r#"
            Neo.nodes.register({
                id: "test/MyNode",
                name: "My Test Node",
                execute: async (ctx) => {
                    return { result: 42 };
                }
            });
            Neo.log.info("Test plugin loaded");
        "#;

        let runtime = PluginRuntime::new("test", code);
        assert!(runtime.is_ok());

        let runtime = runtime.unwrap();
        assert_eq!(runtime.plugin_id(), "test");

        runtime.terminate();
    }

    #[tokio::test]
    async fn test_execute_node() {
        neo_js_runtime::init_platform();

        let code = r#"
            Neo.nodes.register({
                id: "test/Add",
                name: "Add Numbers",
                execute: async (ctx) => {
                    const a = ctx.getInput("a") || 0;
                    const b = ctx.getInput("b") || 0;
                    return { sum: a + b };
                }
            });
        "#;

        let runtime = PluginRuntime::new("test", code).unwrap();

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

        let result = runtime.execute_node("test/Add", &ctx).await;
        assert!(result.is_ok(), "Execute failed: {:?}", result.err());

        let output = result.unwrap();
        assert_eq!(output.values.get("sum"), Some(&serde_json::json!(8)));

        runtime.terminate();
    }
}

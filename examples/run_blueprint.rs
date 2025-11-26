//! Example: Run a blueprint from a JSON file
//!
//! Usage: cargo run --example run_blueprint

use std::sync::Arc;

use neo::blueprints::{
    Blueprint, BlueprintExecutor, ExecutionResult, ExecutionTrigger, NodeRegistry,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,blueprint=debug")
        .init();

    println!("Neo Blueprint Runner");
    println!("====================\n");

    // Create registry with built-in nodes
    let registry = Arc::new(NodeRegistry::with_builtins());

    // Print available nodes
    println!("Available Nodes:");
    for category in registry.categories() {
        println!("  {}:", category);
        for node in registry.nodes_in_category(&category) {
            let pure_marker = if node.pure { " (pure)" } else { "" };
            println!("    - {}: {}{}", node.id, node.name, pure_marker);
        }
    }
    println!();

    // Load the example blueprint
    let blueprint_path = "data/blueprints/example-temperature-alert.json";
    println!("Loading blueprint: {}", blueprint_path);

    let content = std::fs::read_to_string(blueprint_path)?;
    let blueprint: Blueprint = serde_json::from_str(&content)?;

    println!("  ID: {}", blueprint.id);
    println!("  Name: {}", blueprint.name);
    println!("  Nodes: {}", blueprint.nodes.len());
    println!("  Connections: {}", blueprint.connections.len());
    println!();

    // Create executor
    let executor = BlueprintExecutor::new(registry);
    let blueprint = Arc::new(blueprint);

    // Test 1: Temperature above threshold (should trigger alert)
    println!("Test 1: Temperature = 85 (above threshold of 80)");
    println!("─────────────────────────────────────────────────");

    let trigger = ExecutionTrigger::Event {
        event_type: "temperature_changed".to_string(),
        data: serde_json::json!({
            "value": 85.0,
            "source": "sensor-1"
        }),
    };

    match executor
        .execute(Arc::clone(&blueprint), "event", trigger)
        .await
    {
        ExecutionResult::Completed { outputs } => {
            println!("✓ Execution completed");
            if !outputs.is_empty() {
                println!("  Outputs: {:?}", outputs);
            }
        }
        ExecutionResult::Failed { error } => {
            println!("✗ Execution failed: {}", error);
        }
        ExecutionResult::Suspended { state } => {
            println!("⏸ Execution suspended at node: {}", state.node_id);
        }
    }
    println!();

    // Test 2: Temperature below threshold (normal)
    println!("Test 2: Temperature = 72 (below threshold of 80)");
    println!("─────────────────────────────────────────────────");

    let trigger = ExecutionTrigger::Event {
        event_type: "temperature_changed".to_string(),
        data: serde_json::json!({
            "value": 72.0,
            "source": "sensor-1"
        }),
    };

    match executor
        .execute(Arc::clone(&blueprint), "event", trigger)
        .await
    {
        ExecutionResult::Completed { outputs } => {
            println!("✓ Execution completed");
            if !outputs.is_empty() {
                println!("  Outputs: {:?}", outputs);
            }
        }
        ExecutionResult::Failed { error } => {
            println!("✗ Execution failed: {}", error);
        }
        ExecutionResult::Suspended { state } => {
            println!("⏸ Execution suspended at node: {}", state.node_id);
        }
    }
    println!();

    println!("Done!");

    Ok(())
}

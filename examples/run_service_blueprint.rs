//! Example: Run a service blueprint with periodic timer
//!
//! This example demonstrates running a blueprint that acts as a service,
//! printing a random 3-letter code every 5 seconds.
//!
//! Usage: cargo run --example run_service_blueprint

use std::sync::Arc;

use neo::blueprints::{
    Blueprint, BlueprintExecutor, ExecutionResult, ExecutionTrigger, NodeRegistry,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,neo=debug,blueprint=info")
        .init();

    println!("Neo Service Blueprint Runner");
    println!("============================\n");

    // Create registry with built-in nodes
    let registry = Arc::new(NodeRegistry::with_builtins());

    // Show available nodes in relevant categories
    println!("Available Nodes:");
    for category in ["Flow Control", "Utilities"] {
        println!("  {}:", category);
        for node in registry.nodes_in_category(category) {
            let marker = if node.latent { " (latent)" } else if node.pure { " (pure)" } else { "" };
            println!("    - {}: {}{}", node.id, node.name, marker);
        }
    }
    println!();

    // Load the service blueprint
    let blueprint_path = "data/blueprints/example-service-monitor.json";
    println!("Loading blueprint: {}", blueprint_path);

    let content = std::fs::read_to_string(blueprint_path)?;
    let blueprint: Blueprint = serde_json::from_str(&content)?;

    println!("  ID: {}", blueprint.id);
    println!("  Name: {}", blueprint.name);
    println!("  Nodes: {}", blueprint.nodes.len());
    println!("  Connections: {}", blueprint.connections.len());

    // Show service configuration
    if let Some(ref service) = blueprint.service {
        println!("\n  Service Configuration:");
        println!("    Enabled: {}", service.enabled);
        println!("    Singleton: {}", service.singleton);
        if let Some(ref desc) = service.description {
            println!("    Description: {}", desc);
        }
    }
    println!();

    // Create executor
    let executor = BlueprintExecutor::new(registry);
    let blueprint = Arc::new(blueprint);

    // Test 1: Service Start (this will start the timer)
    println!("Test 1: Service Start");
    println!("─────────────────────");

    let trigger = ExecutionTrigger::Event {
        event_type: "service_start".to_string(),
        data: serde_json::json!({}),
    };

    match executor
        .execute(Arc::clone(&blueprint), "on-start", trigger)
        .await
    {
        ExecutionResult::Completed { outputs } => {
            println!("✓ OnServiceStart completed");
            if !outputs.is_empty() {
                println!("  Outputs: {:?}", outputs);
            }
        }
        ExecutionResult::Failed { error } => {
            println!("✗ Execution failed: {}", error);
        }
        ExecutionResult::Suspended { state } => {
            println!("⏸ Timer started - suspended at node: {}", state.node_id);
            println!("  Wake condition: {:?}", state.wake_condition);
        }
    }
    println!();

    // Test 2: Simulate timer ticks manually
    println!("Test 2: Simulating Timer Ticks");
    println!("──────────────────────────────");
    println!("(In the real service, these would fire automatically every 5 seconds)");
    println!();

    for tick in 0..3 {
        println!("Tick #{}", tick + 1);

        // Execute just the random-code node to show different random values each tick
        let random_ctx_trigger = ExecutionTrigger::Request {
            inputs: serde_json::json!({ "length": 3 }),
        };

        match executor
            .execute(Arc::clone(&blueprint), "random-code", random_ctx_trigger)
            .await
        {
            ExecutionResult::Completed { outputs } => {
                if let Some(result) = outputs.get("result") {
                    println!("  Random code: {}", result);
                }
            }
            _ => {}
        }

        // Small delay between simulated ticks
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    println!();

    // Test 3: Service Stop
    println!("Test 3: Service Stop");
    println!("────────────────────");

    let trigger = ExecutionTrigger::Event {
        event_type: "service_stop".to_string(),
        data: serde_json::json!({}),
    };

    match executor
        .execute(Arc::clone(&blueprint), "on-stop", trigger)
        .await
    {
        ExecutionResult::Completed { outputs } => {
            println!("✓ OnServiceStop completed");
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
    println!();
    println!("To see the timer running in real-time, run the main application:");
    println!("  cargo run");
    println!();
    println!("The service-monitor blueprint will automatically start and print");
    println!("random 3-letter codes every 5 seconds.");

    Ok(())
}

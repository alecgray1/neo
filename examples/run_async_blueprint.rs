//! Example: Run an async/latent blueprint with delays
//!
//! Usage: cargo run --example run_async_blueprint

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use neo::blueprints::{
    Blueprint, BlueprintExecutor, ExecutionContext, ExecutionResult, ExecutionTrigger,
    NodeRegistry, WakeCondition,
};

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,blueprint=debug")
        .init();

    println!("Neo Async Blueprint Runner");
    println!("==========================\n");

    // Create registry with built-in nodes
    let registry = Arc::new(NodeRegistry::with_builtins());

    // Show available latent nodes
    println!("Available Latent Nodes:");
    for node in registry.definitions() {
        if node.latent {
            println!("  - {}: {}", node.id, node.name);
        }
    }
    println!();

    // Load the async blueprint
    let blueprint_path = "data/blueprints/example-async-sequence.json";
    println!("Loading blueprint: {}", blueprint_path);

    let content = std::fs::read_to_string(blueprint_path)?;
    let blueprint: Blueprint = serde_json::from_str(&content)?;

    println!("  ID: {}", blueprint.id);
    println!("  Name: {}", blueprint.name);
    println!("  Nodes: {}", blueprint.nodes.len());
    println!();

    // Create executor
    let executor = BlueprintExecutor::new(Arc::clone(&registry));
    let blueprint = Arc::new(blueprint);

    // Start execution
    println!("Starting async execution...");
    println!("─────────────────────────────────────────────────\n");

    let trigger = ExecutionTrigger::Event {
        event_type: "start_sequence".to_string(),
        data: serde_json::json!({}),
    };

    // Run with latent handling
    let mut ctx = ExecutionContext::new(Arc::clone(&blueprint), trigger.clone());
    let mut result = executor
        .execute(Arc::clone(&blueprint), "event", trigger)
        .await;

    // Handle suspended states (latent nodes)
    loop {
        match result {
            ExecutionResult::Completed { outputs } => {
                println!("\n✓ Execution completed");
                if !outputs.is_empty() {
                    println!("  Outputs: {:?}", outputs);
                }
                break;
            }
            ExecutionResult::Failed { error } => {
                println!("\n✗ Execution failed: {}", error);
                break;
            }
            ExecutionResult::Suspended { state } => {
                println!(
                    "⏸ Suspended at node '{}', waiting for: {:?}",
                    state.node_id, state.wake_condition
                );

                // Handle the wake condition
                match &state.wake_condition {
                    WakeCondition::Delay { until_ms } => {
                        let now = now_ms();
                        if *until_ms > now {
                            let wait_ms = until_ms - now;
                            println!("  Sleeping for {}ms...", wait_ms);
                            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                        }
                        println!("  Resuming execution...\n");
                    }
                    WakeCondition::Event { event_type, .. } => {
                        println!("  (Would wait for event: {})", event_type);
                        println!("  Simulating event arrival...\n");
                        // In a real system, you'd wait for the event
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    WakeCondition::PointChanged { point_path, .. } => {
                        println!("  (Would wait for point change: {})", point_path);
                        println!("  Simulating point change...\n");
                        // In a real system, you'd wait for the point to change
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }

                // Resume execution
                result = executor.resume(&mut ctx, &state).await;
            }
        }
    }

    println!("\nDone!");

    Ok(())
}

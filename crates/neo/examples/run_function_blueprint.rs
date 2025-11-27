//! Example: Run a blueprint with functions
//!
//! Usage: cargo run --example run_function_blueprint

use std::collections::HashMap;
use std::sync::Arc;

use neo::blueprints::{Blueprint, BlueprintExecutor, NodeRegistry, NodeRegistryExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,neo::blueprints=debug")
        .init();

    println!("Neo Blueprint Function Runner");
    println!("==============================\n");

    // Create registry with built-in nodes
    let registry = Arc::new(NodeRegistry::with_builtins());

    // Load the example blueprint with functions
    let blueprint_path = "data/blueprints/example-with-functions.json";
    println!("Loading blueprint: {}", blueprint_path);

    let content = std::fs::read_to_string(blueprint_path)?;
    let blueprint: Blueprint = serde_json::from_str(&content)?;

    println!("  ID: {}", blueprint.id);
    println!("  Name: {}", blueprint.name);
    println!("  Functions: {}", blueprint.functions.len());
    println!("  Exports: {:?}", blueprint.exports);
    println!();

    // List available functions
    println!("Available Functions:");
    for (name, func) in &blueprint.functions {
        let pure_marker = if func.pure { " (pure)" } else { "" };
        println!("  {}{}:", name, pure_marker);
        println!("    Inputs:");
        for input in &func.inputs {
            let default = input
                .default
                .as_ref()
                .map(|d| format!(" = {}", d))
                .unwrap_or_default();
            println!("      - {}: {:?}{}", input.name, input.param_type, default);
        }
        println!("    Outputs:");
        for output in &func.outputs {
            println!("      - {}: {:?}", output.name, output.param_type);
        }
        println!();
    }

    // Create executor
    let executor = BlueprintExecutor::new(registry);
    let blueprint = Arc::new(blueprint);

    // Test the calculate_error function
    println!("Testing calculate_error function");
    println!("─────────────────────────────────\n");

    // Get the function
    let func = blueprint
        .functions
        .get("calculate_error")
        .expect("Function not found");

    // Test case 1: Temperature within deadband
    println!("Test 1: current=70.5, setpoint=70.0, deadband=1.0");
    let mut inputs = HashMap::new();
    inputs.insert("current".to_string(), serde_json::json!(70.5));
    inputs.insert("setpoint".to_string(), serde_json::json!(70.0));
    inputs.insert("deadband".to_string(), serde_json::json!(1.0));

    match executor
        .execute_function(Arc::clone(&blueprint), func, inputs)
        .await
    {
        Ok(outputs) => {
            let error = outputs.get("error").map(|v| v.as_f64().unwrap_or(0.0));
            let in_deadband = outputs.get("in_deadband").map(|v| v.as_bool().unwrap_or(false));
            println!("  error: {:?}", error);
            println!("  in_deadband: {:?}", in_deadband);
            if in_deadband == Some(true) {
                println!("  -> Temperature is within deadband (no control action needed)");
            }
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
    println!();

    // Test case 2: Temperature outside deadband
    println!("Test 2: current=75.0, setpoint=70.0, deadband=1.0");
    let mut inputs = HashMap::new();
    inputs.insert("current".to_string(), serde_json::json!(75.0));
    inputs.insert("setpoint".to_string(), serde_json::json!(70.0));
    inputs.insert("deadband".to_string(), serde_json::json!(1.0));

    match executor
        .execute_function(Arc::clone(&blueprint), func, inputs)
        .await
    {
        Ok(outputs) => {
            let error = outputs.get("error").map(|v| v.as_f64().unwrap_or(0.0));
            let in_deadband = outputs.get("in_deadband").map(|v| v.as_bool().unwrap_or(false));
            println!("  error: {:?}", error);
            println!("  in_deadband: {:?}", in_deadband);
            if in_deadband == Some(false) {
                println!("  -> Temperature is outside deadband (control action needed!)");
            }
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
    println!();

    // Test case 3: Cooling needed (negative error)
    println!("Test 3: current=65.0, setpoint=70.0, deadband=1.0");
    let mut inputs = HashMap::new();
    inputs.insert("current".to_string(), serde_json::json!(65.0));
    inputs.insert("setpoint".to_string(), serde_json::json!(70.0));
    inputs.insert("deadband".to_string(), serde_json::json!(1.0));

    match executor
        .execute_function(Arc::clone(&blueprint), func, inputs)
        .await
    {
        Ok(outputs) => {
            let error = outputs.get("error").map(|v| v.as_f64().unwrap_or(0.0));
            let in_deadband = outputs.get("in_deadband").map(|v| v.as_bool().unwrap_or(false));
            println!("  error: {:?}", error);
            println!("  in_deadband: {:?}", in_deadband);
            if let Some(e) = error {
                if e < 0.0 {
                    println!("  -> Heating needed (temperature below setpoint)");
                } else {
                    println!("  -> Cooling needed (temperature above setpoint)");
                }
            }
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
    println!();

    println!("Done!");

    Ok(())
}

//! Example: Working with Blueprint Behaviours
//!
//! Demonstrates loading behaviour definitions and validating blueprints against them.
//!
//! Usage: cargo run --example run_behaviour_example

use std::collections::HashMap;
use std::path::Path;

use neo::blueprints::{BehaviourRegistry, Blueprint, FunctionDef, FunctionParam, PinType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Neo Blueprint Behaviours Example");
    println!("=================================\n");

    // Create a behaviour registry and load definitions from the data directory
    let mut registry = BehaviourRegistry::new();

    let behaviours_dir = Path::new("data/behaviours");
    println!("Loading behaviours from: {}\n", behaviours_dir.display());

    let loaded = registry.load_from_directory(behaviours_dir)?;
    println!("Loaded {} behaviour(s): {:?}\n", loaded.len(), loaded);

    // List all loaded behaviours
    println!("Available Behaviours:");
    println!("─────────────────────");
    for behaviour_id in registry.behaviour_ids() {
        if let Some(def) = registry.get(behaviour_id) {
            println!("\n  {} ({})", def.name, def.id);
            if let Some(desc) = &def.description {
                println!("  {}", desc);
            }

            println!("  Required Callbacks:");
            for callback in &def.callbacks {
                let inputs: Vec<String> = callback
                    .inputs
                    .iter()
                    .map(|p| format!("{}: {:?}", p.name, p.param_type))
                    .collect();
                let outputs: Vec<String> = callback
                    .outputs
                    .iter()
                    .map(|p| format!("{}: {:?}", p.name, p.param_type))
                    .collect();
                println!(
                    "    - {}({}) -> ({})",
                    callback.name,
                    inputs.join(", "),
                    outputs.join(", ")
                );
            }

            if !def.optional_callbacks.is_empty() {
                println!("  Optional Callbacks:");
                for callback in &def.optional_callbacks {
                    println!("    - {} (optional)", callback.name);
                }
            }
        }
    }
    println!();

    // Create a compliant blueprint
    println!("Testing Behaviour Compliance:");
    println!("─────────────────────────────\n");

    let compliant_blueprint = create_compliant_blueprint();
    println!("Blueprint: {} (implements: {:?})", compliant_blueprint.id, compliant_blueprint.implements);
    println!("  Exports: {:?}", compliant_blueprint.exports);

    match registry.validate_blueprint(&compliant_blueprint) {
        Ok(()) => println!("  Result: COMPLIANT - implements all required callbacks\n"),
        Err(violations) => {
            println!("  Result: NON-COMPLIANT");
            for v in violations {
                println!("    - {}", v);
            }
            println!();
        }
    }

    // Create a non-compliant blueprint (missing a callback)
    let mut missing_callback = create_compliant_blueprint();
    missing_callback.id = "missing-callback-controller".to_string();
    missing_callback.functions.remove("set_mode");
    missing_callback.exports.retain(|e| e != "set_mode");

    println!("Blueprint: {} (implements: {:?})", missing_callback.id, missing_callback.implements);
    println!("  Exports: {:?}", missing_callback.exports);

    match registry.validate_blueprint(&missing_callback) {
        Ok(()) => println!("  Result: COMPLIANT (unexpected!)\n"),
        Err(violations) => {
            println!("  Result: NON-COMPLIANT");
            for v in violations {
                println!("    - {}", v);
            }
            println!();
        }
    }

    // Create a blueprint with wrong signature
    let mut wrong_signature = create_compliant_blueprint();
    wrong_signature.id = "wrong-signature-controller".to_string();
    // Change set_mode to return Integer instead of Boolean
    if let Some(func) = wrong_signature.functions.get_mut("set_mode") {
        func.outputs[0].param_type = PinType::Integer;
    }

    println!("Blueprint: {} (implements: {:?})", wrong_signature.id, wrong_signature.implements);
    println!("  Exports: {:?}", wrong_signature.exports);
    println!("  Note: set_mode returns Integer instead of Boolean");

    match registry.validate_blueprint(&wrong_signature) {
        Ok(()) => println!("  Result: COMPLIANT (unexpected!)\n"),
        Err(violations) => {
            println!("  Result: NON-COMPLIANT");
            for v in violations {
                println!("    - {}", v);
            }
            println!();
        }
    }

    // Create a blueprint that doesn't export the function
    let mut not_exported = create_compliant_blueprint();
    not_exported.id = "not-exported-controller".to_string();
    not_exported.exports.retain(|e| e != "get_status");

    println!("Blueprint: {} (implements: {:?})", not_exported.id, not_exported.implements);
    println!("  Exports: {:?}", not_exported.exports);
    println!("  Note: get_status function exists but is not exported");

    match registry.validate_blueprint(&not_exported) {
        Ok(()) => println!("  Result: COMPLIANT (unexpected!)\n"),
        Err(violations) => {
            println!("  Result: NON-COMPLIANT");
            for v in violations {
                println!("    - {}", v);
            }
            println!();
        }
    }

    println!("Done!");

    Ok(())
}

/// Create a blueprint that properly implements the Controllable behaviour
fn create_compliant_blueprint() -> Blueprint {
    let mut blueprint = Blueprint {
        id: "compliant-controller".to_string(),
        name: "Compliant Controller".to_string(),
        version: "1.0.0".to_string(),
        description: Some("A controller that implements Controllable".to_string()),
        service: None,
        variables: HashMap::new(),
        nodes: vec![],
        connections: vec![],
        functions: HashMap::new(),
        imports: vec![],
        exports: vec!["get_status".to_string(), "set_mode".to_string()],
        implements: vec!["controllable".to_string()],
    };

    // Add get_status function: () -> (mode: String, is_enabled: Boolean)
    blueprint.functions.insert(
        "get_status".to_string(),
        FunctionDef {
            name: Some("Get Status".to_string()),
            description: Some("Return current equipment status".to_string()),
            inputs: vec![],
            outputs: vec![
                FunctionParam {
                    name: "mode".to_string(),
                    param_type: PinType::String,
                    default: None,
                    description: None,
                },
                FunctionParam {
                    name: "is_enabled".to_string(),
                    param_type: PinType::Boolean,
                    default: None,
                    description: None,
                },
            ],
            pure: true,
            nodes: vec![],
            connections: vec![],
        },
    );

    // Add set_mode function: (mode: String) -> (success: Boolean)
    blueprint.functions.insert(
        "set_mode".to_string(),
        FunctionDef {
            name: Some("Set Mode".to_string()),
            description: Some("Set the operating mode".to_string()),
            inputs: vec![FunctionParam {
                name: "mode".to_string(),
                param_type: PinType::String,
                default: None,
                description: None,
            }],
            outputs: vec![FunctionParam {
                name: "success".to_string(),
                param_type: PinType::Boolean,
                default: None,
                description: None,
            }],
            pure: false,
            nodes: vec![],
            connections: vec![],
        },
    );

    blueprint
}

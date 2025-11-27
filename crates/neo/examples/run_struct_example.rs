//! Example: Working with Blueprint Structs
//!
//! Demonstrates loading struct definitions and creating/validating struct instances.
//!
//! Usage: cargo run --example run_struct_example

use std::path::Path;

use neo::blueprints::{StructRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Neo Blueprint Structs Example");
    println!("==============================\n");

    // Create a struct registry and load definitions from the data directory
    let mut registry = StructRegistry::new();

    let structs_dir = Path::new("data/structs");
    println!("Loading structs from: {}\n", structs_dir.display());

    let loaded = registry.load_from_directory(structs_dir)?;
    println!("Loaded {} struct(s): {:?}\n", loaded.len(), loaded);

    // List all loaded structs
    println!("Available Structs:");
    println!("──────────────────");
    for struct_id in registry.struct_ids() {
        if let Some(def) = registry.get(struct_id) {
            println!("\n  {} ({})", def.name, def.id);
            if let Some(desc) = &def.description {
                println!("  {}", desc);
            }
            println!("  Fields:");
            for field in &def.fields {
                let units = field.units.as_ref().map(|u| format!(" [{}]", u)).unwrap_or_default();
                let default = field.default.as_ref().map(|d| format!(" = {}", d)).unwrap_or_default();
                println!("    - {}: {:?}{}{}", field.name, field.field_type, units, default);
            }
        }
    }
    println!();

    // Create a default instance of the VAV device struct
    println!("Creating Struct Instances:");
    println!("──────────────────────────\n");

    if let Some(vav_default) = registry.create_default_instance("vav-device") {
        println!("Default VAV Device instance:");
        println!("{}\n", serde_json::to_string_pretty(&vav_default)?);
    }

    // Create a custom instance and validate it
    let custom_vav = serde_json::json!({
        "zone_temp": 74.5,
        "setpoint": 72.0,
        "damper_cmd": 45.0,
        "heating_cmd": 0.0,
        "occupied": true
    });

    println!("Custom VAV Device instance:");
    println!("{}\n", serde_json::to_string_pretty(&custom_vav)?);

    // Validate the custom instance
    println!("Validating Instances:");
    println!("─────────────────────\n");

    match registry.validate_instance("vav-device", &custom_vav) {
        Ok(()) => println!("  Custom VAV instance is valid"),
        Err(errors) => {
            println!("  Custom VAV instance is invalid:");
            for err in errors {
                println!("    - {}", err);
            }
        }
    }

    // Try an invalid instance (wrong type for zone_temp)
    let invalid_vav = serde_json::json!({
        "zone_temp": "not a number",
        "setpoint": 72.0,
        "damper_cmd": 45.0,
        "heating_cmd": 0.0,
        "occupied": true
    });

    println!();
    match registry.validate_instance("vav-device", &invalid_vav) {
        Ok(()) => println!("  Invalid VAV instance passed (unexpected!)"),
        Err(errors) => {
            println!("  Invalid VAV instance correctly rejected:");
            for err in errors {
                println!("    - {}", err);
            }
        }
    }

    // Try a partial instance (missing fields with defaults is okay)
    let partial_vav = serde_json::json!({
        "zone_temp": 75.0
    });

    println!();
    match registry.validate_instance("vav-device", &partial_vav) {
        Ok(()) => println!("  Partial VAV instance is valid (fields have defaults)"),
        Err(errors) => {
            println!("  Partial VAV instance rejected:");
            for err in errors {
                println!("    - {}", err);
            }
        }
    }

    println!("\nDone!");

    Ok(())
}

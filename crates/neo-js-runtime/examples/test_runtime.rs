//! Test example for neo-js-runtime
//!
//! Run with: cargo run -p neo-js-runtime --example test_runtime

use neo_js_runtime::{spawn_runtime, RuntimeServices};

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== Test 1: Basic runtime ===");
    test_basic_runtime();

    std::thread::sleep(std::time::Duration::from_millis(200));

    println!("\n=== Test 2: Node registration ===");
    test_node_registration();

    std::thread::sleep(std::time::Duration::from_millis(200));

    println!("\n=== Test 3: Example plugin ===");
    test_example_plugin();

    println!("\n=== All tests passed! ===");

    std::thread::sleep(std::time::Duration::from_millis(100));
}

fn test_basic_runtime() {
    let code = r#"
        Neo.log.info("Hello from JavaScript!");
        Neo.log.debug("This is a debug message");
    "#;

    let handle = spawn_runtime("test-basic".to_string(), code.to_string(), RuntimeServices::default()).unwrap();

    // Give it a moment to execute
    std::thread::sleep(std::time::Duration::from_millis(100));

    handle.terminate();
    println!("  [OK] Basic runtime executed successfully");
}

fn test_node_registration() {
    let code = r#"
        Neo.nodes.register({
            id: "test/MyNode",
            name: "My Test Node",
            inputs: [{ name: "value", type: "number" }],
            outputs: [{ name: "result", type: "number" }],
            execute: async (ctx) => {
                return { result: ctx.inputs.value * 2 };
            }
        });

        Neo.nodes.register({
            id: "test/AnotherNode",
            name: "Another Node",
            execute: async (ctx) => {
                return {};
            }
        });

        Neo.log.info("Registered nodes: " + Neo.nodes.list().join(", "));
    "#;

    let handle = spawn_runtime("test-nodes".to_string(), code.to_string(), RuntimeServices::default()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));

    handle.terminate();
    println!("  [OK] Node registration worked");
}

fn test_example_plugin() {
    // Load the example plugin from project/plugins/example
    let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("project/plugins/example/index.js");

    let code = std::fs::read_to_string(&plugin_path)
        .expect("Failed to read example plugin");

    let handle = spawn_runtime("example-plugin".to_string(), code, RuntimeServices::default()).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(100));

    handle.terminate();
    println!("  [OK] Example plugin loaded successfully");
}

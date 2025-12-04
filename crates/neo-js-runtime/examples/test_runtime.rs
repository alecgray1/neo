//! Test example for neo-js-runtime
//!
//! Run with: cargo run -p neo-js-runtime --example test_runtime

use neo_js_runtime::{spawn_runtime, spawn_runtime_empty, RuntimeServices};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== Test 1: Basic runtime ===");
    test_basic_runtime().await;

    std::thread::sleep(std::time::Duration::from_millis(200));

    println!("\n=== Test 2: Service lifecycle ===");
    test_service_lifecycle().await;

    std::thread::sleep(std::time::Duration::from_millis(200));

    println!("\n=== Test 3: Empty runtime for blueprints ===");
    test_empty_runtime().await;

    println!("\n=== All tests passed! ===");

    std::thread::sleep(std::time::Duration::from_millis(100));
}

async fn test_basic_runtime() {
    let code = r#"
        export default defineService({
            name: "Test Service",
            onStart: async () => {
                Neo.log.info("Hello from JavaScript!");
                Neo.log.debug("This is a debug message");
            }
        });
    "#;

    let handle = spawn_runtime(
        "test-basic".to_string(),
        code.to_string(),
        "test/basic".to_string(),
        RuntimeServices::default(),
    ).unwrap();

    // Start the service
    handle.start_service().await.expect("Failed to start service");

    // Give it a moment to execute
    std::thread::sleep(std::time::Duration::from_millis(100));

    handle.terminate();
    println!("  [OK] Basic runtime executed successfully");
}

async fn test_service_lifecycle() {
    // Simple service without interval - just onStart/onStop
    let code = r#"
        export default defineService({
            name: "Counter Service",
            onStart: async (ctx) => {
                ctx.state.count = 0;
                Neo.log.info("Counter started");
            },
            onStop: async (ctx) => {
                Neo.log.info("Counter stopped");
            },
        });
    "#;

    let handle = spawn_runtime(
        "test-counter".to_string(),
        code.to_string(),
        "test/counter".to_string(),
        RuntimeServices::default(),
    ).expect("Failed to spawn runtime");

    // Start the service
    handle.start_service().await.expect("Failed to start service");

    // Stop the service
    handle.stop_service().await.expect("Failed to stop service");

    handle.terminate();
    println!("  [OK] Service lifecycle completed");
}

async fn test_empty_runtime() {
    // Create an empty runtime for blueprint execution
    let handle = spawn_runtime_empty(
        "test-empty".to_string(),
        RuntimeServices::default(),
    ).expect("Failed to spawn empty runtime");

    // Empty runtimes can have node definitions loaded into them
    // This is used for blueprint execution where all built-in nodes are in JS

    std::thread::sleep(std::time::Duration::from_millis(50));

    handle.terminate();
    println!("  [OK] Empty runtime created successfully");
}

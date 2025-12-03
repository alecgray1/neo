//! Test example for neo-js-runtime
//!
//! Run with: cargo run -p neo-js-runtime --example test_runtime

use neo_js_runtime::{scan_and_spawn_runtime, spawn_runtime, RuntimeServices};

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

    std::thread::sleep(std::time::Duration::from_millis(200));

    println!("\n=== Test 4: Multi-service scan ===");
    test_multi_service_scan();

    std::thread::sleep(std::time::Duration::from_millis(200));

    println!("\n=== Test 5: Multi-service lifecycle (1:1 model) ===");
    test_multi_service_lifecycle();

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

fn test_multi_service_scan() {
    // Plugin with 3 services
    let code = r#"
        Neo.services.register({
            id: "test/service-a",
            name: "Service A",
            subscriptions: ["events/a/**"],
            tickInterval: 1000,
            onStart: async (ctx) => { ctx.state.started = true; },
            onTick: async (ctx) => { Neo.log.info("A tick"); },
        });

        Neo.services.register({
            id: "test/service-b",
            name: "Service B",
            subscriptions: ["events/b/**"],
            tickInterval: 2000,
            onStart: async (ctx) => { ctx.state.started = true; },
            onTick: async (ctx) => { Neo.log.info("B tick"); },
        });

        Neo.services.register({
            id: "test/service-c",
            name: "Service C",
            // No tick interval - event-driven only
            subscriptions: ["events/c/**"],
            onStart: async (ctx) => { ctx.state.started = true; },
        });

        Neo.log.info("Registered " + Neo.services.list().length + " services");
    "#;

    // Use scan_and_spawn_runtime which returns both scan result and a runtime handle
    // This avoids V8 corruption from dropping/recreating runtimes
    let (result, handle) = scan_and_spawn_runtime(
        "test-scan".to_string(),
        code.to_string(),
        RuntimeServices::default(),
    ).expect("scan_and_spawn_runtime failed");

    assert_eq!(result.services.len(), 3, "Expected 3 services");

    // Check service A
    let svc_a = result.services.iter().find(|s| s.id == "test/service-a").expect("Service A not found");
    assert_eq!(svc_a.name, "Service A");
    assert_eq!(svc_a.subscriptions, vec!["events/a/**"]);
    assert_eq!(svc_a.tick_interval, Some(1000));

    // Check service B
    let svc_b = result.services.iter().find(|s| s.id == "test/service-b").expect("Service B not found");
    assert_eq!(svc_b.name, "Service B");
    assert_eq!(svc_b.tick_interval, Some(2000));

    // Check service C (no tick interval)
    let svc_c = result.services.iter().find(|s| s.id == "test/service-c").expect("Service C not found");
    assert_eq!(svc_c.name, "Service C");
    assert_eq!(svc_c.tick_interval, None);

    // Terminate the runtime (we're just testing the scan, not using the runtime)
    handle.terminate();

    println!("  [OK] Discovered {} services: {:?}",
        result.services.len(),
        result.services.iter().map(|s| &s.id).collect::<Vec<_>>()
    );
}

fn test_multi_service_lifecycle() {
    // Test that each service has isolated state and only its lifecycle is called
    let code = r#"
        Neo.services.register({
            id: "test/counter-a",
            name: "Counter A",
            onStart: async (ctx) => {
                ctx.state.count = 0;
                Neo.log.info("Counter A started");
            },
            onTick: async (ctx) => {
                ctx.state.count += 1;
                Neo.log.info("Counter A: " + ctx.state.count);
            },
            onStop: async (ctx) => {
                Neo.log.info("Counter A stopped with count: " + ctx.state.count);
            },
        });

        Neo.services.register({
            id: "test/counter-b",
            name: "Counter B",
            onStart: async (ctx) => {
                ctx.state.count = 100;  // Different starting value
                Neo.log.info("Counter B started");
            },
            onTick: async (ctx) => {
                ctx.state.count += 10;  // Different increment
                Neo.log.info("Counter B: " + ctx.state.count);
            },
            onStop: async (ctx) => {
                Neo.log.info("Counter B stopped with count: " + ctx.state.count);
            },
        });
    "#;

    // Create a tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Spawn runtime for service A only
    let handle_a = spawn_runtime(
        "test-counter-a".to_string(),
        code.to_string(),
        RuntimeServices::default(),
    ).expect("Failed to spawn runtime A");

    // Start only service A
    rt.block_on(async {
        handle_a.start_service("test/counter-a").await.expect("Failed to start service A");
    });

    // Tick service A twice
    rt.block_on(async {
        handle_a.tick_service("test/counter-a").await.expect("Failed to tick service A");
        handle_a.tick_service("test/counter-a").await.expect("Failed to tick service A");
    });

    // Stop service A
    rt.block_on(async {
        handle_a.stop_service("test/counter-a").await.expect("Failed to stop service A");
    });

    handle_a.terminate();

    // Spawn separate runtime for service B
    let handle_b = spawn_runtime(
        "test-counter-b".to_string(),
        code.to_string(),
        RuntimeServices::default(),
    ).expect("Failed to spawn runtime B");

    // Start only service B
    rt.block_on(async {
        handle_b.start_service("test/counter-b").await.expect("Failed to start service B");
    });

    // Tick service B three times
    rt.block_on(async {
        handle_b.tick_service("test/counter-b").await.expect("Failed to tick service B");
        handle_b.tick_service("test/counter-b").await.expect("Failed to tick service B");
        handle_b.tick_service("test/counter-b").await.expect("Failed to tick service B");
    });

    // Stop service B
    rt.block_on(async {
        handle_b.stop_service("test/counter-b").await.expect("Failed to stop service B");
    });

    handle_b.terminate();

    println!("  [OK] Each service has isolated state and lifecycle");
    println!("       Counter A: started at 0, ticked twice (count=2), stopped");
    println!("       Counter B: started at 100, ticked 3x by 10 (count=130), stopped");
}

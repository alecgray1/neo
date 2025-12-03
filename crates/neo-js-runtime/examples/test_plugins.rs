//! Test loading multiple plugins - one working, one throwing
//!
//! Run with: cargo run -p neo-js-runtime --example test_plugins

use neo_js_runtime::{spawn_runtime, RuntimeServices};

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== Test: Loading Multiple Plugins ===\n");

    // Good plugin - registers nodes and services properly
    let good_plugin = r#"
        Neo.log.info("Good plugin loading...");

        // Register a working service
        Neo.services.register({
            id: "good-plugin/main",
            name: "Good Plugin Service",
            onStart: async () => {
                Neo.log.info("Good plugin service started!");
            },
            onStop: async () => {
                Neo.log.info("Good plugin service stopped");
            },
            onTick: async () => {
                Neo.log.debug("Good plugin tick");
            },
        });

        // Register a working node
        Neo.nodes.register({
            id: "good-plugin/Add",
            name: "Add Numbers",
            category: "Math",
            inputs: [
                { name: "a", type: "number" },
                { name: "b", type: "number" },
            ],
            outputs: [
                { name: "sum", type: "number" },
            ],
            pure: true,
            execute: async (ctx) => {
                const a = (ctx.getInput("a")) || 0;
                const b = (ctx.getInput("b")) || 0;
                return { sum: a + b };
            },
        });

        Neo.log.info("Good plugin loaded successfully!");
    "#;

    // Bad plugin - throws during service onStart
    let bad_plugin = r#"
        Neo.log.info("Bad plugin loading...");

        // Register a service that throws on start
        Neo.services.register({
            id: "bad-plugin/crasher",
            name: "Crasher Service",
            onStart: async () => {
                Neo.log.info("Bad plugin about to throw...");
                throw new Error("Intentional crash in onStart!");
            },
            onStop: async () => {
                Neo.log.info("Bad plugin service stopped");
            },
        });

        Neo.log.info("Bad plugin registered (but onStart will throw)");
    "#;

    // Plugin that throws during load (not in service callback)
    let throw_on_load_plugin = r#"
        Neo.log.info("Throw-on-load plugin starting...");
        throw new Error("This plugin throws immediately during load!");
    "#;

    // Create a tokio runtime for async tests
    let rt = tokio::runtime::Runtime::new().unwrap();

    println!("--- Test 1: Good plugin with full lifecycle ---");
    match spawn_runtime("good-plugin".to_string(), good_plugin.to_string(), RuntimeServices::default()) {
        Ok(handle) => {
            println!("  [OK] Good plugin loaded");

            rt.block_on(async {
                // Start services
                match handle.start_services().await {
                    Ok(()) => println!("  [OK] Services started"),
                    Err(e) => println!("  [FAIL] Start failed: {}", e),
                }

                // Execute a node
                let context = serde_json::json!({
                    "nodeId": "instance-1",
                    "inputs": { "a": 5, "b": 3 },
                    "config": {},
                    "variables": {}
                });
                match handle.execute_node("good-plugin/Add", &context.to_string()).await {
                    Ok(output) => println!("  [OK] Node executed: {}", output),
                    Err(e) => println!("  [FAIL] Node execution failed: {}", e),
                }

                // Tick
                match handle.tick().await {
                    Ok(()) => println!("  [OK] Tick completed"),
                    Err(e) => println!("  [FAIL] Tick failed: {}", e),
                }

                // Stop services
                match handle.stop_services().await {
                    Ok(()) => println!("  [OK] Services stopped"),
                    Err(e) => println!("  [FAIL] Stop failed: {}", e),
                }
            });

            handle.terminate();
            println!("  [OK] Good plugin terminated\n");
        }
        Err(e) => {
            println!("  [FAIL] Good plugin failed to load: {}\n", e);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("--- Test 2: Bad plugin (throws in onStart) ---");
    match spawn_runtime("bad-plugin".to_string(), bad_plugin.to_string(), RuntimeServices::default()) {
        Ok(handle) => {
            println!("  [OK] Bad plugin loaded (registration phase passed)");

            rt.block_on(async {
                // Try to start services - this should fail
                match handle.start_services().await {
                    Ok(()) => println!("  [UNEXPECTED] Services started without error!"),
                    Err(e) => println!("  [OK] Start correctly failed: {}", e),
                }
            });

            handle.terminate();
            println!("  [OK] Bad plugin terminated\n");
        }
        Err(e) => {
            println!("  [INFO] Bad plugin failed to load: {}\n", e);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("--- Test 3: Plugin that throws during load ---");
    match spawn_runtime("throw-on-load".to_string(), throw_on_load_plugin.to_string(), RuntimeServices::default()) {
        Ok(handle) => {
            println!("  [UNEXPECTED] Plugin loaded but shouldn't have!");
            handle.terminate();
        }
        Err(e) => {
            println!("  [OK] Plugin correctly failed to load: {}\n", e);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("--- Test 4: Isolation - bad plugin doesn't affect good plugin ---");
    let good_handle = spawn_runtime("good-isolated".to_string(), good_plugin.to_string(), RuntimeServices::default());
    let bad_handle = spawn_runtime("bad-isolated".to_string(), bad_plugin.to_string(), RuntimeServices::default());

    match (&good_handle, &bad_handle) {
        (Ok(good), Ok(bad)) => {
            println!("  [OK] Both plugins loaded");

            rt.block_on(async {
                // Start bad plugin - should fail
                match bad.start_services().await {
                    Ok(()) => println!("  [UNEXPECTED] Bad services started!"),
                    Err(e) => println!("  [OK] Bad plugin start failed: {}", e),
                }

                // Good plugin should still work
                match good.start_services().await {
                    Ok(()) => println!("  [OK] Good plugin still works after bad plugin failure"),
                    Err(e) => println!("  [FAIL] Good plugin affected by bad: {}", e),
                }

                // Execute node in good plugin
                let context = serde_json::json!({
                    "nodeId": "instance-1",
                    "inputs": { "a": 10, "b": 20 },
                    "config": {},
                    "variables": {}
                });
                match good.execute_node("good-plugin/Add", &context.to_string()).await {
                    Ok(output) => println!("  [OK] Good plugin node works: {}", output),
                    Err(e) => println!("  [FAIL] Good plugin node failed: {}", e),
                }
            });
        }
        (Err(e), _) => println!("  [FAIL] Good plugin failed: {}", e),
        (_, Err(e)) => println!("  [INFO] Bad plugin failed early: {}", e),
    }

    // Cleanup
    if let Ok(h) = good_handle { h.terminate(); }
    if let Ok(h) = bad_handle { h.terminate(); }

    println!("\n=== All tests complete ===");
}

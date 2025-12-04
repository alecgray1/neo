//! Test loading plugins with different behaviors
//!
//! Run with: cargo run -p neo-js-runtime --example test_plugins

use neo_js_runtime::{spawn_runtime, RuntimeServices};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("=== Test: Loading Plugins ===\n");

    // Good plugin - registers as a service
    let good_plugin = r#"
        export default defineService({
            name: "Good Plugin Service",
            onStart: async () => {
                Neo.log.info("Good plugin service started!");
            },
            onStop: async () => {
                Neo.log.info("Good plugin service stopped");
            },
        });
    "#;

    // Bad plugin - throws during onStart
    let bad_plugin = r#"
        export default defineService({
            name: "Bad Plugin Service",
            onStart: async () => {
                Neo.log.info("Bad plugin about to throw...");
                throw new Error("Intentional crash in onStart!");
            },
            onStop: async () => {
                Neo.log.info("Bad plugin service stopped");
            },
        });
    "#;

    // Plugin that throws during load (not in service callback)
    let throw_on_load_plugin = r#"
        Neo.log.info("Throw-on-load plugin starting...");
        throw new Error("This plugin throws immediately during load!");
    "#;

    println!("--- Test 1: Good plugin with full lifecycle ---");
    match spawn_runtime(
        "good-plugin".to_string(),
        good_plugin.to_string(),
        "good-plugin/main".to_string(),
        RuntimeServices::default(),
    ) {
        Ok(handle) => {
            println!("  [OK] Good plugin loaded");

            // Start service
            match handle.start_service().await {
                Ok(()) => println!("  [OK] Service started"),
                Err(e) => println!("  [FAIL] Start failed: {}", e),
            }

            // Stop service
            match handle.stop_service().await {
                Ok(()) => println!("  [OK] Service stopped"),
                Err(e) => println!("  [FAIL] Stop failed: {}", e),
            }

            handle.terminate();
            println!("  [OK] Good plugin terminated\n");
        }
        Err(e) => {
            println!("  [FAIL] Good plugin failed to load: {}\n", e);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("--- Test 2: Bad plugin (throws in onStart) ---");
    match spawn_runtime(
        "bad-plugin".to_string(),
        bad_plugin.to_string(),
        "bad-plugin/crasher".to_string(),
        RuntimeServices::default(),
    ) {
        Ok(handle) => {
            println!("  [OK] Bad plugin loaded (registration phase passed)");

            // Try to start service - this should fail
            match handle.start_service().await {
                Ok(()) => println!("  [UNEXPECTED] Service started without error!"),
                Err(e) => println!("  [OK] Start correctly failed: {}", e),
            }

            handle.terminate();
            println!("  [OK] Bad plugin terminated\n");
        }
        Err(e) => {
            println!("  [INFO] Bad plugin failed to load: {}\n", e);
        }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    println!("--- Test 3: Plugin that throws during load ---");
    match spawn_runtime(
        "throw-on-load".to_string(),
        throw_on_load_plugin.to_string(),
        "throw-on-load/main".to_string(),
        RuntimeServices::default(),
    ) {
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
    let good_handle = spawn_runtime(
        "good-isolated".to_string(),
        good_plugin.to_string(),
        "good-isolated/main".to_string(),
        RuntimeServices::default(),
    );
    let bad_handle = spawn_runtime(
        "bad-isolated".to_string(),
        bad_plugin.to_string(),
        "bad-isolated/main".to_string(),
        RuntimeServices::default(),
    );

    match (&good_handle, &bad_handle) {
        (Ok(good), Ok(bad)) => {
            println!("  [OK] Both plugins loaded");

            // Start bad plugin - should fail
            match bad.start_service().await {
                Ok(()) => println!("  [UNEXPECTED] Bad service started!"),
                Err(e) => println!("  [OK] Bad plugin start failed: {}", e),
            }

            // Good plugin should still work
            match good.start_service().await {
                Ok(()) => println!("  [OK] Good plugin still works after bad plugin failure"),
                Err(e) => println!("  [FAIL] Good plugin affected by bad: {}", e),
            }
        }
        (Err(e), _) => println!("  [FAIL] Good plugin failed: {}", e),
        (_, Err(e)) => println!("  [INFO] Bad plugin failed early: {}", e),
    }

    // Cleanup
    if let Ok(h) = good_handle { h.terminate(); }
    if let Ok(h) = bad_handle { h.terminate(); }

    println!("\n=== All tests complete ===");
}

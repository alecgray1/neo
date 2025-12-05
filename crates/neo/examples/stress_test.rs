//! Stress test for JS runtime scalability
//!
//! Spawns N service instances to measure memory and thread usage.
//!
//! Run with: cargo run --example stress_test -- --count 300

use std::sync::Arc;
use std::time::{Duration, Instant};

use neo::plugin::{JsService, JsServiceConfig};
use blueprint_runtime::service::ServiceManager;

// Service code that creates 100 JS objects to test memory usage
const SERVICE_CODE: &str = r#"
defineService({
    name: "StressTest",
    onStart: async (ctx) => {
        ctx.state.id = ctx.config.id || "unknown";

        // Create 10000 objects with 25 fields each to simulate real workload
        ctx.state.objects = [];
        for (let i = 0; i < 10000; i++) {
            ctx.state.objects.push({
                id: `obj-${ctx.state.id}-${i}`,
                value: Math.random() * 1000,
                timestamp: Date.now(),
                field4: Math.random(),
                field5: Math.random(),
                field6: Math.random(),
                field7: Math.random(),
                field8: Math.random(),
                field9: Math.random(),
                field10: Math.random(),
                field11: `string-${i}-a`,
                field12: `string-${i}-b`,
                field13: `string-${i}-c`,
                field14: `string-${i}-d`,
                field15: `string-${i}-e`,
                field16: i * 2,
                field17: i * 3,
                field18: i * 4,
                field19: i * 5,
                field20: i * 6,
                field21: true,
                field22: false,
                field23: null,
                field24: [1, 2, 3],
                field25: { nested: i },
            });
        }

        Neo.log.info(`[Service ${ctx.state.id}] Started with ${ctx.state.objects.length} objects`);
    },
    onStop: async (ctx) => {
        Neo.log.info(`[Service ${ctx.state.id}] Stopped`);
    },
});
"#;

fn get_memory_usage() -> (usize, usize) {
    // Read from /proc/self/status on Linux
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        let mut vm_rss = 0;
        let mut vm_size = 0;
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                vm_rss = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
            if line.starts_with("VmSize:") {
                vm_size = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }
        (vm_rss, vm_size) // in KB
    } else {
        (0, 0)
    }
}

fn get_thread_count() -> usize {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("Threads:") {
                return line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }
    }
    0
}

fn main() -> anyhow::Result<()> {
    // Parse args
    let args: Vec<String> = std::env::args().collect();
    let count: usize = args.iter()
        .position(|a| a == "--count")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    // Initialize V8
    neo_js_runtime::init_platform();

    // Build tokio runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        println!("=== JS Runtime Stress Test ===\n");
        println!("Target: {} service instances\n", count);

        // Baseline measurements
        let (baseline_rss, baseline_vsize) = get_memory_usage();
        let baseline_threads = get_thread_count();

        println!("Baseline:");
        println!("  RSS: {} MB", baseline_rss / 1024);
        println!("  Virtual: {} MB", baseline_vsize / 1024);
        println!("  Threads: {}", baseline_threads);
        println!();

        let service_manager = Arc::new(ServiceManager::new());
        let mut handles = Vec::new();

        println!("Spawning {} services...", count);
        let start = Instant::now();

        for i in 0..count {
            let config = JsServiceConfig::new(
                format!("stress-test-{}", i),
                format!("Stress Test {}", i),
                SERVICE_CODE,
            )
            .with_config(serde_json::json!({ "id": i }));

            let service = JsService::new(config);

            match service_manager.spawn(service).await {
                Ok(handle) => {
                    handles.push(handle);
                    if (i + 1) % 50 == 0 {
                        let (rss, _) = get_memory_usage();
                        let threads = get_thread_count();
                        println!(
                            "  [{}/{}] RSS: {} MB, Threads: {}",
                            i + 1,
                            count,
                            rss / 1024,
                            threads
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Failed to spawn service {}: {:?}", i, e);
                    // Don't break - continue to see how many fail
                }
            }
        }

        let spawn_duration = start.elapsed();
        println!("\nSpawn completed in {:.2}s\n", spawn_duration.as_secs_f64());

        // Final measurements
        let (final_rss, final_vsize) = get_memory_usage();
        let final_threads = get_thread_count();

        println!("Final State:");
        println!("  RSS: {} MB (+ {} MB)", final_rss / 1024, (final_rss - baseline_rss) / 1024);
        println!("  Virtual: {} MB (+ {} MB)", final_vsize / 1024, (final_vsize - baseline_vsize) / 1024);
        println!("  Threads: {} (+ {})", final_threads, final_threads - baseline_threads);
        println!("  Services running: {}", handles.len());
        println!();

        // Per-service stats
        let services_spawned = handles.len();
        if services_spawned > 0 {
            let mem_per_service = (final_rss - baseline_rss) / services_spawned;
            let threads_per_service = (final_threads - baseline_threads) as f64 / services_spawned as f64;

            println!("Per-Service Average:");
            println!("  Memory: {} KB ({:.2} MB)", mem_per_service, mem_per_service as f64 / 1024.0);
            println!("  Threads: {:.2}", threads_per_service);
            println!();
        }

        // Wait a bit to let services stabilize
        println!("Waiting 10 seconds for services to stabilize...");
        tokio::time::sleep(Duration::from_secs(10)).await;

        let (stable_rss, _) = get_memory_usage();
        let stable_threads = get_thread_count();
        println!("After stabilization: RSS {} MB, Threads {}\n", stable_rss / 1024, stable_threads);

        // Shutdown
        println!("Shutting down...");
        let shutdown_start = Instant::now();

        if let Err(e) = service_manager.shutdown_all().await {
            eprintln!("Shutdown error: {}", e);
        }

        println!("Shutdown completed in {:.2}s\n", shutdown_start.elapsed().as_secs_f64());

        // Post-shutdown
        tokio::time::sleep(Duration::from_millis(500)).await;
        let (post_rss, _) = get_memory_usage();
        let post_threads = get_thread_count();
        println!("After shutdown: RSS {} MB, Threads {}", post_rss / 1024, post_threads);

        Ok(())
    })
}

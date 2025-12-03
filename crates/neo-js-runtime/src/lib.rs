//! Neo JavaScript Runtime
//!
//! This crate provides the JavaScript runtime infrastructure for Neo.
//! It follows Deno's worker pattern: each runtime runs in its own OS thread
//! with its own V8 isolate.
//!
//! # Architecture
//!
//! - Each service runs in a dedicated runtime thread
//! - One service per runtime (no multi-service files)
//! - Services use `export default defineService({...})` pattern
//! - Commands are executed via V8CrossThreadTaskSpawner (Deno's pattern)

mod ops;

pub use ops::neo_runtime;

use std::future::poll_fn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::thread;

use deno_core::v8;
use deno_core::JsRuntime;
use deno_core::PollEventLoopOptions;
use deno_core::RuntimeOptions;
use deno_core::V8CrossThreadTaskSpawner;

/// Ensure V8 platform is initialized exactly once.
static V8_INIT: Once = Once::new();

/// Mutex to serialize V8 isolate creation.
/// Creating multiple isolates concurrently can cause crashes in V8.
static ISOLATE_CREATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Initialize the V8 platform. Call this before spawning any runtimes.
/// Safe to call multiple times - will only initialize once.
pub fn init_platform() {
    V8_INIT.call_once(|| {
        JsRuntime::init_platform(None, false);
    });
}

/// Handle to a spawned JavaScript runtime.
///
/// Each runtime contains exactly one service or node definition.
/// Commands are executed via the V8CrossThreadTaskSpawner.
pub struct RuntimeHandle {
    /// Spawner for executing tasks on the V8 event loop from this thread.
    spawner: V8CrossThreadTaskSpawner,
    /// Whether the runtime has terminated.
    terminated: Arc<AtomicBool>,
    /// V8 isolate handle for forced termination.
    isolate_handle: v8::IsolateHandle,
    /// Thread join handle.
    thread_handle: std::sync::Mutex<Option<thread::JoinHandle<Result<(), RuntimeError>>>>,
}

impl RuntimeHandle {
    /// Execute the loaded node and wait for the result.
    pub fn execute_node(&self, context_json: &str) -> Result<String, RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let context_json = context_json.to_string();

        // Use spawn_blocking to execute on the V8 thread and wait for result
        let result = self.spawner.spawn_blocking(move |scope| {
            execute_node_inner(scope, &context_json)
        });

        result.map_err(RuntimeError::JavaScript)
    }

    /// Start the loaded service (calls onStart).
    pub fn start_service(&self) -> Result<(), RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let result = self.spawner.spawn_blocking(|scope| {
            call_service_lifecycle_inner(scope, "startService")
        });

        result.map_err(RuntimeError::JavaScript)
    }

    /// Stop the loaded service (calls onStop).
    pub fn stop_service(&self) -> Result<(), RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let result = self.spawner.spawn_blocking(|scope| {
            call_service_lifecycle_inner(scope, "stopService")
        });

        result.map_err(RuntimeError::JavaScript)
    }

    /// Tick the loaded service (calls onTick).
    pub fn tick_service(&self) -> Result<(), RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let result = self.spawner.spawn_blocking(|scope| {
            call_service_lifecycle_inner(scope, "tickService")
        });

        result.map_err(RuntimeError::JavaScript)
    }

    /// Terminate the runtime.
    pub fn terminate(&self) {
        if self.terminated.swap(true, Ordering::SeqCst) {
            return; // Already terminated
        }
        // Force terminate V8 execution if it's stuck
        self.isolate_handle.terminate_execution();
    }

    /// Check if the runtime has terminated.
    pub fn is_terminated(&self) -> bool {
        self.terminated.load(Ordering::SeqCst)
    }

    /// Wait for the runtime thread to finish.
    pub fn join(self) -> Result<(), RuntimeError> {
        if let Some(handle) = self.thread_handle.lock().unwrap().take() {
            handle.join().map_err(|_| RuntimeError::ThreadPanic)??;
        }
        Ok(())
    }
}

impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        self.terminate();
    }
}

// SAFETY: RuntimeHandle is designed to be used from multiple threads.
// - V8CrossThreadTaskSpawner has an unsafe impl Send
// - v8::IsolateHandle is documented as Send + Sync
// - Arc<AtomicBool> is Send + Sync
// - Mutex<Option<JoinHandle>> is Send + Sync
unsafe impl Send for RuntimeHandle {}
unsafe impl Sync for RuntimeHandle {}

/// Errors that can occur in the runtime.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Runtime has terminated")]
    Terminated,
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Runtime thread panicked")]
    ThreadPanic,
    #[error("JavaScript error: {0}")]
    JavaScript(String),
    #[error("Failed to spawn thread: {0}")]
    SpawnFailed(#[from] std::io::Error),
}

/// Services that can be accessed from JavaScript.
///
/// These are passed as Arc into the runtime and stored in OpState.
#[derive(Clone, Default)]
pub struct RuntimeServices {
    /// Event publisher for emitting events from JS
    pub events: Option<EventPublisher>,
    /// Point store for reading/writing point values from JS
    pub points: Option<Arc<dyn PointStore>>,
}

/// Trait for point value storage.
#[async_trait::async_trait]
pub trait PointStore: Send + Sync + 'static {
    /// Read the current value of a point.
    async fn read(&self, point_id: &str) -> Result<Option<serde_json::Value>, PointError>;

    /// Write a value to a point.
    async fn write(&self, point_id: &str, value: serde_json::Value) -> Result<(), PointError>;
}

impl Clone for Box<dyn PointStore> {
    fn clone(&self) -> Self {
        panic!("Cannot clone Box<dyn PointStore> - use Arc instead")
    }
}

/// Errors from point operations.
#[derive(Debug, thiserror::Error)]
pub enum PointError {
    #[error("Point not found: {0}")]
    NotFound(String),
    #[error("Write failed: {0}")]
    WriteFailed(String),
    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

/// Event publisher handle for emitting events.
#[derive(Clone)]
pub struct EventPublisher {
    tx: tokio::sync::broadcast::Sender<crate::Event>,
}

impl EventPublisher {
    /// Create a new event publisher.
    pub fn new(tx: tokio::sync::broadcast::Sender<crate::Event>) -> Self {
        Self { tx }
    }

    /// Emit an event.
    pub fn emit(&self, event: crate::Event) -> Result<(), RuntimeError> {
        self.tx
            .send(event)
            .map(|_| ())
            .map_err(|_| RuntimeError::ChannelClosed)
    }
}

/// An event that can be published.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    /// Event type identifier
    pub event_type: String,
    /// Source that generated the event
    pub source: String,
    /// Event payload
    pub data: serde_json::Value,
    /// Timestamp (Unix milliseconds)
    pub timestamp: u64,
}

impl Event {
    /// Create a new event with current timestamp.
    pub fn new(event_type: impl Into<String>, source: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.into(),
            source: source.into(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Spawn a new JavaScript runtime for a single service/node.
///
/// The `code` should use `export default defineService({...})` or `defineNode({...})`.
/// The `service_id` is the full ID (e.g., "example/ticker") that will be assigned
/// to the loaded definition.
pub fn spawn_runtime(
    name: String,
    code: String,
    service_id: String,
    services: RuntimeServices,
) -> Result<RuntimeHandle, RuntimeError> {
    tracing::debug!("[spawn_runtime] Starting for {}", name);
    init_platform();

    let terminated = Arc::new(AtomicBool::new(false));
    let terminated_clone = terminated.clone();

    // Channel to receive spawner and isolate handle from the worker thread
    let (init_tx, init_rx) = std::sync::mpsc::sync_channel::<Result<(V8CrossThreadTaskSpawner, v8::IsolateHandle), String>>(1);

    // Channel to signal that the event loop is ready to process tasks
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<()>(1);

    let name_clone = name.clone();
    let thread_handle = thread::Builder::new()
        .name(name.clone())
        .spawn(move || -> Result<(), RuntimeError> {
            tracing::debug!("[spawn_runtime:{}] Thread started", name_clone);

            // Create tokio runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(RuntimeError::SpawnFailed)?;

            // Run the worker
            let result = rt.block_on(run_worker(
                name_clone.clone(),
                code,
                service_id,
                services,
                terminated_clone,
                init_tx,
                ready_tx,
            ));

            // Force shutdown any lingering tasks
            rt.shutdown_background();

            tracing::debug!("[spawn_runtime:{}] Thread exiting", name_clone);
            result
        })?;

    // Wait for initialization (spawner and isolate handle)
    let (spawner, isolate_handle) = init_rx
        .recv()
        .map_err(|_| RuntimeError::ChannelClosed)?
        .map_err(RuntimeError::JavaScript)?;

    // Wait for the event loop to be ready to process tasks
    // This ensures start_service() won't be called before the event loop is polling
    let _ = ready_rx.recv();

    tracing::debug!("[spawn_runtime] {} is ready", name);

    Ok(RuntimeHandle {
        spawner,
        terminated,
        isolate_handle,
        thread_handle: std::sync::Mutex::new(Some(thread_handle)),
    })
}

/// The main worker loop that runs inside the spawned thread.
async fn run_worker(
    name: String,
    code: String,
    service_id: String,
    services: RuntimeServices,
    terminated: Arc<AtomicBool>,
    init_tx: std::sync::mpsc::SyncSender<Result<(V8CrossThreadTaskSpawner, v8::IsolateHandle), String>>,
    ready_tx: std::sync::mpsc::SyncSender<()>,
) -> Result<(), RuntimeError> {
    // Create the JsRuntime (serialize creation to avoid V8 crashes)
    let mut js_runtime = {
        let _lock = ISOLATE_CREATE_LOCK.lock().unwrap();
        tracing::debug!("[run_worker:{}] Creating JsRuntime", name);
        JsRuntime::new(RuntimeOptions {
            extensions: vec![neo_runtime::init_ops_and_esm()],
            ..Default::default()
        })
    };

    // Get the cross-thread spawner and isolate handle
    let spawner = js_runtime
        .op_state()
        .borrow()
        .borrow::<V8CrossThreadTaskSpawner>()
        .clone();
    let isolate_handle = js_runtime.v8_isolate().thread_safe_handle();

    // Store services in OpState for ops to access
    {
        let op_state = js_runtime.op_state();
        let mut state = op_state.borrow_mut();
        state.put(services);
    }

    // Execute plugin code (the chunk with defineService/defineNode)
    let script_name: &'static str = Box::leak(format!("<service:{}>", name).into_boxed_str());
    tracing::debug!("[run_worker:{}] Executing service code", name);

    // Wrap the code to capture the default export and register it
    let wrapped_code = format!(
        r#"
        const __module = (() => {{
            {}
        }})();

        // Try to get the definition from either:
        // 1. The return value (if export default was present)
        // 2. The __getLastDefinition() fallback (if Vite stripped export default)
        const __def = __module || globalThis.__getLastDefinition();

        if (__def && typeof __def === 'object') {{
            globalThis.__neo_internal.setLoadedDefinition(__def, {});
        }} else {{
            throw new Error("Service chunk must call defineService({{...}}) or defineNode({{...}})");
        }}
        "#,
        code.replace("export default", "return"),
        serde_json::to_string(&service_id).unwrap()
    );

    if let Err(e) = js_runtime.execute_script(script_name, wrapped_code) {
        let _ = init_tx.send(Err(e.to_string()));
        return Err(RuntimeError::JavaScript(e.to_string()));
    }

    // Run event loop to complete any async initialization
    if let Err(e) = js_runtime.run_event_loop(Default::default()).await {
        let _ = init_tx.send(Err(e.to_string()));
        return Err(RuntimeError::JavaScript(e.to_string()));
    }

    tracing::debug!("[run_worker:{}] Service loaded successfully", name);

    // Send the spawner and isolate handle to the host
    let _ = init_tx.send(Ok((spawner, isolate_handle)));

    // Signal that the event loop is about to start polling
    // This ensures the host won't call start_service() before we're ready
    let _ = ready_tx.send(());

    // Main event loop - just keep polling until terminated
    // Commands will be injected via the V8CrossThreadTaskSpawner
    while !terminated.load(Ordering::SeqCst) {
        // Poll the event loop - this will process any spawned tasks
        match poll_fn(|cx| js_runtime.poll_event_loop(cx, PollEventLoopOptions::default())).await {
            Ok(()) => {
                // Event loop completed (no pending work)
                // Sleep briefly to avoid busy-spinning when idle
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            Err(e) => {
                tracing::error!("[run_worker:{}] Event loop error: {}", name, e);
                // Don't exit on errors - the runtime might still be usable
            }
        }
    }

    tracing::debug!("[run_worker:{}] Event loop finished", name);
    Ok(())
}

/// Execute a node within a V8 scope (called from spawn_blocking).
fn execute_node_inner(scope: &mut v8::HandleScope, context_json: &str) -> Result<String, String> {
    let script = format!(
        r#"(async () => {{
            const result = await globalThis.__neo_internal.executeNode({});
            return JSON.stringify(result);
        }})()"#,
        context_json
    );

    let source = v8::String::new(scope, &script).unwrap();
    let script = v8::Script::compile(scope, source, None)
        .ok_or_else(|| "Failed to compile script".to_string())?;

    let result = script.run(scope)
        .ok_or_else(|| "Script execution failed".to_string())?;

    // If it's a promise, we need to handle it
    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(result) {
        match promise.state() {
            v8::PromiseState::Fulfilled => {
                let value = promise.result(scope);
                Ok(value.to_rust_string_lossy(scope))
            }
            v8::PromiseState::Rejected => {
                let value = promise.result(scope);
                Err(value.to_rust_string_lossy(scope))
            }
            v8::PromiseState::Pending => {
                // Promise is pending - this is a problem since spawn_blocking is sync
                Err("Promise is pending - async operations not supported in spawn_blocking".to_string())
            }
        }
    } else {
        Ok(result.to_rust_string_lossy(scope))
    }
}

/// Call a service lifecycle method within a V8 scope (called from spawn_blocking).
fn call_service_lifecycle_inner(scope: &mut v8::HandleScope, method: &str) -> Result<(), String> {
    let script = format!(
        r#"(async () => {{
            await globalThis.__neo_internal.{}();
        }})()"#,
        method
    );

    let source = v8::String::new(scope, &script).unwrap();
    let script = v8::Script::compile(scope, source, None)
        .ok_or_else(|| "Failed to compile script".to_string())?;

    let result = script.run(scope)
        .ok_or_else(|| "Script execution failed".to_string())?;

    // If it's a promise, check for rejection
    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(result) {
        match promise.state() {
            v8::PromiseState::Fulfilled => Ok(()),
            v8::PromiseState::Rejected => {
                let value = promise.result(scope);
                Err(value.to_rust_string_lossy(scope))
            }
            v8::PromiseState::Pending => {
                // Promise is pending - this is a problem since spawn_blocking is sync
                Err("Promise is pending - async operations not supported in spawn_blocking".to_string())
            }
        }
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // Tests are in examples/test_runtime.rs to avoid V8 multi-initialization issues
    // Run with: cargo run -p neo-js-runtime --example test_runtime
}

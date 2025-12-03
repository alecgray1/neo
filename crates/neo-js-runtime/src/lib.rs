//! Neo JavaScript Runtime
//!
//! This crate provides the JavaScript runtime infrastructure for Neo.
//! It follows Deno's worker pattern: each runtime runs in its own OS thread
//! with its own V8 isolate and LocalSet (no work-stealing).
//!
//! # Architecture
//!
//! - Each plugin runtime runs in a dedicated thread
//! - Services (PointStore, EventBus, etc.) are passed as Arc into the runtime
//! - Ops call services directly - no RPC back to host needed
//! - Communication with host is just: execute_node request → response

mod ops;

pub use ops::neo_runtime;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Once;
use std::thread;

use deno_core::v8;
use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// Ensure V8 platform is initialized exactly once.
static V8_INIT: Once = Once::new();

/// Initialize the V8 platform. Call this before spawning any runtimes.
/// Safe to call multiple times - will only initialize once.
pub fn init_platform() {
    V8_INIT.call_once(|| {
        JsRuntime::init_platform(None, false);
    });
}

/// Request to execute a node in the runtime.
struct ExecuteNodeRequest {
    node_id: String,
    context_json: String,
    response_tx: oneshot::Sender<Result<String, String>>,
}

/// Request to call a service lifecycle method.
struct ServiceLifecycleRequest {
    response_tx: oneshot::Sender<Result<(), String>>,
}

/// Messages sent to the runtime thread.
enum RuntimeCommand {
    ExecuteNode(ExecuteNodeRequest),
    StartServices(ServiceLifecycleRequest),
    StopServices(ServiceLifecycleRequest),
    Tick(ServiceLifecycleRequest),
    Shutdown,
}

/// Handle to a spawned JavaScript runtime.
///
/// This is a simple, lightweight handle. All the complexity lives
/// in the runtime thread. Communication is request/response only.
pub struct RuntimeHandle {
    /// Send commands to the runtime thread.
    cmd_tx: mpsc::UnboundedSender<RuntimeCommand>,
    /// Whether the runtime has terminated.
    terminated: Arc<AtomicBool>,
    /// V8 isolate handle for forced termination.
    isolate_handle: v8::IsolateHandle,
    /// Thread join handle.
    thread_handle: std::sync::Mutex<Option<thread::JoinHandle<Result<(), RuntimeError>>>>,
}

// RuntimeHandle is Send + Sync because:
// - cmd_tx: UnboundedSender is Send + Sync
// - terminated: Arc<AtomicBool> is Send + Sync
// - isolate_handle: v8::IsolateHandle is Send + Sync
// - thread_handle: Mutex<Option<JoinHandle>> is Send + Sync

impl RuntimeHandle {
    /// Execute a node in the JS runtime and wait for the result.
    ///
    /// This sends a request to the runtime thread and awaits the response.
    pub async fn execute_node(
        &self,
        node_id: &str,
        context_json: &str,
    ) -> Result<String, RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let (response_tx, response_rx) = oneshot::channel();

        self.cmd_tx
            .send(RuntimeCommand::ExecuteNode(ExecuteNodeRequest {
                node_id: node_id.to_string(),
                context_json: context_json.to_string(),
                response_tx,
            }))
            .map_err(|_| RuntimeError::ChannelClosed)?;

        response_rx
            .await
            .map_err(|_| RuntimeError::ChannelClosed)?
            .map_err(RuntimeError::JavaScript)
    }

    /// Start all registered services (calls onStart on each).
    pub async fn start_services(&self) -> Result<(), RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let (response_tx, response_rx) = oneshot::channel();

        self.cmd_tx
            .send(RuntimeCommand::StartServices(ServiceLifecycleRequest {
                response_tx,
            }))
            .map_err(|_| RuntimeError::ChannelClosed)?;

        response_rx
            .await
            .map_err(|_| RuntimeError::ChannelClosed)?
            .map_err(RuntimeError::JavaScript)
    }

    /// Stop all registered services (calls onStop on each).
    pub async fn stop_services(&self) -> Result<(), RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let (response_tx, response_rx) = oneshot::channel();

        self.cmd_tx
            .send(RuntimeCommand::StopServices(ServiceLifecycleRequest {
                response_tx,
            }))
            .map_err(|_| RuntimeError::ChannelClosed)?;

        response_rx
            .await
            .map_err(|_| RuntimeError::ChannelClosed)?
            .map_err(RuntimeError::JavaScript)
    }

    /// Tick all registered services (calls onTick on each).
    pub async fn tick(&self) -> Result<(), RuntimeError> {
        if self.terminated.load(Ordering::SeqCst) {
            return Err(RuntimeError::Terminated);
        }

        let (response_tx, response_rx) = oneshot::channel();

        self.cmd_tx
            .send(RuntimeCommand::Tick(ServiceLifecycleRequest {
                response_tx,
            }))
            .map_err(|_| RuntimeError::ChannelClosed)?;

        response_rx
            .await
            .map_err(|_| RuntimeError::ChannelClosed)?
            .map_err(RuntimeError::JavaScript)
    }

    /// Terminate the runtime.
    pub fn terminate(&self) {
        if self.terminated.swap(true, Ordering::SeqCst) {
            return; // Already terminated
        }
        self.isolate_handle.terminate_execution();
        let _ = self.cmd_tx.send(RuntimeCommand::Shutdown);
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
/// Ops can then access them directly without RPC.
#[derive(Clone, Default)]
pub struct RuntimeServices {
    /// Event publisher for emitting events from JS
    pub events: Option<EventPublisher>,
    /// Point store for reading/writing point values from JS
    pub points: Option<Arc<dyn PointStore>>,
}

/// Trait for point value storage.
///
/// Implementations provide read/write access to point values.
/// Points are identified by string IDs and hold JSON values.
#[async_trait::async_trait]
pub trait PointStore: Send + Sync + 'static {
    /// Read the current value of a point.
    async fn read(&self, point_id: &str) -> Result<Option<serde_json::Value>, PointError>;

    /// Write a value to a point.
    async fn write(&self, point_id: &str, value: serde_json::Value) -> Result<(), PointError>;
}

// Make PointStore dyn-compatible by manually implementing Clone for Arc<dyn PointStore>
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
///
/// This is a simple wrapper that can be cloned and passed to runtimes.
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

/// Spawn a new JavaScript runtime in its own thread.
///
/// The `code` should register nodes using `Neo.nodes.register()`.
pub fn spawn_runtime(
    name: String,
    code: String,
    services: RuntimeServices,
) -> Result<RuntimeHandle, RuntimeError> {
    init_platform();

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<RuntimeCommand>();

    let terminated = Arc::new(AtomicBool::new(false));
    let terminated_clone = terminated.clone();

    // Channel to receive isolate handle from the spawned thread
    let (isolate_tx, isolate_rx) = std::sync::mpsc::sync_channel::<v8::IsolateHandle>(1);

    // Channel to receive ready signal
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<Result<(), String>>(1);

    let thread_handle = thread::Builder::new()
        .name(name.clone())
        .spawn(move || -> Result<(), RuntimeError> {
            // Single-threaded tokio runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(RuntimeError::SpawnFailed)?;

            // LocalSet ensures no work-stealing - all tasks stay on this thread
            let local = tokio::task::LocalSet::new();

            local.block_on(&rt, async {
                let mut js_runtime = JsRuntime::new(RuntimeOptions {
                    extensions: vec![neo_runtime::init_ops_and_esm()],
                    ..Default::default()
                });

                // Send isolate handle to host
                let isolate_handle = js_runtime.v8_isolate().thread_safe_handle();
                let _ = isolate_tx.send(isolate_handle);

                // Store services in OpState for ops to access
                {
                    let op_state = js_runtime.op_state();
                    let mut state = op_state.borrow_mut();
                    state.put(services);
                }

                // Execute plugin code
                if let Err(e) = js_runtime.execute_script("<plugin>", code) {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return Err(RuntimeError::JavaScript(e.to_string()));
                }

                // Run initial event loop to let plugin register nodes
                if let Err(e) = js_runtime.run_event_loop(Default::default()).await {
                    let _ = ready_tx.send(Err(e.to_string()));
                    return Err(RuntimeError::JavaScript(e.to_string()));
                }

                // Signal ready
                let _ = ready_tx.send(Ok(()));

                // Main command loop
                loop {
                    if terminated_clone.load(Ordering::SeqCst) {
                        break;
                    }

                    tokio::select! {
                        cmd = cmd_rx.recv() => {
                            match cmd {
                                Some(RuntimeCommand::ExecuteNode(req)) => {
                                    let result = execute_node_in_js(&mut js_runtime, &req.node_id, &req.context_json).await;
                                    let _ = req.response_tx.send(result);
                                }
                                Some(RuntimeCommand::StartServices(req)) => {
                                    let result = call_service_lifecycle(&mut js_runtime, "start").await;
                                    let _ = req.response_tx.send(result);
                                }
                                Some(RuntimeCommand::StopServices(req)) => {
                                    let result = call_service_lifecycle(&mut js_runtime, "stop").await;
                                    let _ = req.response_tx.send(result);
                                }
                                Some(RuntimeCommand::Tick(req)) => {
                                    let result = call_service_lifecycle(&mut js_runtime, "tick").await;
                                    let _ = req.response_tx.send(result);
                                }
                                Some(RuntimeCommand::Shutdown) | None => {
                                    break;
                                }
                            }
                        }
                        // Also poll the JS event loop for any pending async ops
                        _ = js_runtime.run_event_loop(deno_core::PollEventLoopOptions {
                            wait_for_inspector: false,
                            pump_v8_message_loop: true,
                        }) => {
                            // Event loop tick complete
                        }
                    }
                }

                Ok(())
            })
        })?;

    // Wait for isolate handle
    let isolate_handle = isolate_rx
        .recv()
        .map_err(|_| RuntimeError::ChannelClosed)?;

    // Wait for ready signal
    ready_rx
        .recv()
        .map_err(|_| RuntimeError::ChannelClosed)?
        .map_err(RuntimeError::JavaScript)?;

    Ok(RuntimeHandle {
        cmd_tx,
        terminated,
        isolate_handle,
        thread_handle: std::sync::Mutex::new(Some(thread_handle)),
    })
}

/// Execute a node in the JS runtime and return the result.
async fn execute_node_in_js(
    js_runtime: &mut JsRuntime,
    node_id: &str,
    context_json: &str,
) -> Result<String, String> {
    // Call __neo_internal.executeNode in JS
    let script = format!(
        r#"(async () => {{
            const result = await globalThis.__neo_internal.executeNode({}, {});
            return JSON.stringify(result);
        }})()"#,
        serde_json::to_string(&node_id).unwrap(),
        context_json
    );

    let result = js_runtime
        .execute_script("<execute_node>", script)
        .map_err(|e| e.to_string())?;

    // Run event loop to resolve the promise
    js_runtime
        .run_event_loop(Default::default())
        .await
        .map_err(|e| e.to_string())?;

    // Get the resolved value
    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, result);

    // Extract string result from promise
    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(local) {
        if promise.state() == v8::PromiseState::Fulfilled {
            let value = promise.result(scope);
            Ok(value.to_rust_string_lossy(scope))
        } else if promise.state() == v8::PromiseState::Rejected {
            let value = promise.result(scope);
            Err(value.to_rust_string_lossy(scope))
        } else {
            Err("Promise still pending".to_string())
        }
    } else {
        Ok(local.to_rust_string_lossy(scope))
    }
}

/// Call a service lifecycle method (start, stop, or tick) on all registered services.
async fn call_service_lifecycle(
    js_runtime: &mut JsRuntime,
    method: &str,
) -> Result<(), String> {
    let script = format!(
        r#"(async () => {{
            const method = "{}";
            const errors = [];

            for (const [id, service] of Object.entries(Neo.services.getAll())) {{
                try {{
                    if (method === "start" && service.onStart) {{
                        await service.onStart();
                    }} else if (method === "stop" && service.onStop) {{
                        await service.onStop();
                    }} else if (method === "tick" && service.onTick) {{
                        await service.onTick();
                    }}
                }} catch (err) {{
                    const msg = `Service ${{id}} ${{method}} failed: ${{err.message}}`;
                    Neo.log.error(msg);
                    errors.push(msg);
                }}
            }}

            if (errors.length > 0) {{
                throw new Error(errors.join("; "));
            }}
        }})()"#,
        method
    );

    let result = js_runtime
        .execute_script("<service_lifecycle>", script)
        .map_err(|e| e.to_string())?;

    // Run event loop to resolve the promise
    js_runtime
        .run_event_loop(Default::default())
        .await
        .map_err(|e| e.to_string())?;

    // Check if promise was rejected
    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, result);

    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(local) {
        if promise.state() == v8::PromiseState::Rejected {
            let value = promise.result(scope);
            return Err(value.to_rust_string_lossy(scope));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Tests are in examples/test_runtime.rs to avoid V8 multi-initialization issues
    // Run with: cargo run -p neo-js-runtime --example test_runtime
}

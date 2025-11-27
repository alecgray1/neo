// JsRuntimePool - Worker thread pool for JavaScript plugin execution
//
// Since JsRuntime is !Send + !Sync, each worker runs in a dedicated thread.
// The pool actor manages worker assignment using round-robin distribution.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};

// Global mutex to serialize JsRuntime creation (deno_core has global state issues)
static JS_RUNTIME_CREATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn runtime_creation_lock() -> &'static Mutex<()> {
    JS_RUNTIME_CREATION_LOCK.get_or_init(|| Mutex::new(()))
}

use kameo::message::{Context, Message};
use tokio::sync::{mpsc, oneshot};

use crate::messages::Event;
use crate::services::messages::{ServiceRequest, ServiceResponse};
use crate::types::{Error, Result};

use super::ops::{PluginBridge, PointReadRequest, PointWriteRequest, RUNTIME_JS};
use super::PluginManifest;

// ─────────────────────────────────────────────────────────────────────────────
// Worker Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Commands sent to a worker thread
#[derive(Debug)]
pub enum WorkerCommand {
    /// Load and start a plugin
    LoadPlugin {
        plugin_id: String,
        manifest: PluginManifest,
        base_path: PathBuf,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Stop a plugin
    StopPlugin {
        plugin_id: String,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Send an event to a plugin
    SendEvent {
        plugin_id: String,
        event: Event,
    },
    /// Handle a request for a plugin
    HandleRequest {
        plugin_id: String,
        request: ServiceRequest,
        reply: oneshot::Sender<Result<ServiceResponse>>,
    },
    /// Shutdown the worker
    Shutdown,
}

/// Handle to a worker thread
pub struct WorkerHandle {
    /// Channel to send commands to the worker
    pub command_tx: mpsc::UnboundedSender<WorkerCommand>,
    /// Thread join handle
    pub thread_handle: Option<JoinHandle<()>>,
    /// Worker ID
    pub id: usize,
    /// Number of plugins assigned to this worker
    pub plugin_count: AtomicUsize,
}

impl WorkerHandle {
    /// Send a command to the worker
    pub fn send(&self, cmd: WorkerCommand) -> Result<()> {
        self.command_tx
            .send(cmd)
            .map_err(|_| Error::Service("Worker not responding".to_string()))
    }

    /// Increment plugin count
    pub fn add_plugin(&self) {
        self.plugin_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement plugin count
    pub fn remove_plugin(&self) {
        self.plugin_count.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get current plugin count
    pub fn get_plugin_count(&self) -> usize {
        self.plugin_count.load(Ordering::SeqCst)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pool Actor
// ─────────────────────────────────────────────────────────────────────────────

/// Messages for the JsRuntimePool actor
#[derive(Debug)]
pub enum PoolMsg {
    /// Load a plugin (assigns to a worker using round-robin)
    LoadPlugin {
        manifest: PluginManifest,
        base_path: PathBuf,
        reply: oneshot::Sender<Result<usize>>, // Returns worker ID
    },
    /// Stop a plugin
    StopPlugin {
        plugin_id: String,
        reply: oneshot::Sender<Result<()>>,
    },
    /// Send event to a plugin
    SendEvent {
        plugin_id: String,
        event: Event,
    },
    /// Handle a request for a plugin
    HandleRequest {
        plugin_id: String,
        request: ServiceRequest,
        reply: oneshot::Sender<Result<ServiceResponse>>,
    },
    /// Get pool status
    GetStatus {
        reply: oneshot::Sender<PoolStatus>,
    },
    /// Shutdown the pool
    Shutdown,
}

/// Reply type for PoolMsg
#[derive(Debug, kameo::Reply)]
pub enum PoolReply {
    /// Operation completed
    Done,
}

/// Status of the runtime pool
#[derive(Debug, Clone)]
pub struct PoolStatus {
    pub worker_count: usize,
    pub total_plugins: usize,
    pub plugins_per_worker: Vec<usize>,
}

/// Actor that manages a pool of JS runtime workers
#[derive(kameo::Actor)]
pub struct JsRuntimePoolActor {
    /// Worker handles
    workers: Vec<Arc<WorkerHandle>>,
    /// Map of plugin_id -> worker_id
    plugin_assignments: std::collections::HashMap<String, usize>,
}

impl JsRuntimePoolActor {
    /// Create a new pool with the specified number of workers
    pub fn new(worker_count: usize) -> Self {
        let workers = (0..worker_count)
            .map(|id| Arc::new(spawn_worker(id)))
            .collect();

        tracing::info!("JsRuntimePool created with {} workers", worker_count);

        Self {
            workers,
            plugin_assignments: std::collections::HashMap::new(),
        }
    }

    /// Create a pool with default size
    /// Note: Currently limited to 1 worker due to deno_core/V8 threading issues
    /// with multiple JsRuntime instances across threads
    pub fn with_default_size() -> Self {
        // TODO: Investigate V8 threading issues and potentially increase worker count
        Self::new(1)
    }

    /// Get the next available worker (least-loaded strategy)
    ///
    /// Multiple plugins can run on the same worker, each with its own JsRuntime.
    /// All runtimes on a worker run on the same thread to avoid V8 threading issues.
    fn next_worker(&mut self) -> Option<&Arc<WorkerHandle>> {
        // Find the least loaded worker
        self.workers.iter().min_by_key(|w| w.get_plugin_count())
    }

    /// Get the worker assigned to a plugin
    fn get_worker(&self, plugin_id: &str) -> Option<&Arc<WorkerHandle>> {
        self.plugin_assignments
            .get(plugin_id)
            .and_then(|&id| self.workers.get(id))
    }

    /// Shutdown all workers
    fn shutdown_workers(&mut self) {
        for worker in &self.workers {
            let _ = worker.send(WorkerCommand::Shutdown);
        }
    }
}

impl Message<PoolMsg> for JsRuntimePoolActor {
    type Reply = PoolReply;

    async fn handle(
        &mut self,
        msg: PoolMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            PoolMsg::LoadPlugin {
                manifest,
                base_path,
                reply,
            } => {
                let plugin_id = manifest.id.clone();

                // Assign to an available worker
                let Some(worker) = self.next_worker().cloned() else {
                    let _ = reply.send(Err(Error::Service(
                        "No workers available in the pool".to_string()
                    )));
                    return PoolReply::Done;
                };
                let worker_id = worker.id;

                // Reserve this worker immediately (before async load)
                // This prevents another plugin from being assigned to the same worker
                worker.add_plugin();
                self.plugin_assignments.insert(plugin_id.clone(), worker_id);

                // Send load command to worker
                let (load_reply_tx, load_reply_rx) = oneshot::channel();
                if let Err(e) = worker.send(WorkerCommand::LoadPlugin {
                    plugin_id: plugin_id.clone(),
                    manifest,
                    base_path,
                    reply: load_reply_tx,
                }) {
                    // Undo reservation on send failure
                    worker.remove_plugin();
                    self.plugin_assignments.remove(&plugin_id);
                    let _ = reply.send(Err(e));
                    return PoolReply::Done;
                }

                // Wait for load result
                match load_reply_rx.await {
                    Ok(Ok(())) => {
                        tracing::info!(
                            "Plugin '{}' assigned to worker {}",
                            plugin_id,
                            worker_id
                        );
                        let _ = reply.send(Ok(worker_id));
                    }
                    Ok(Err(e)) => {
                        // Undo reservation on load failure
                        worker.remove_plugin();
                        self.plugin_assignments.remove(&plugin_id);
                        let _ = reply.send(Err(e));
                    }
                    Err(_) => {
                        // Undo reservation on communication failure
                        worker.remove_plugin();
                        self.plugin_assignments.remove(&plugin_id);
                        let _ = reply.send(Err(Error::Service(
                            "Worker did not respond".to_string(),
                        )));
                    }
                }

                PoolReply::Done
            }

            PoolMsg::StopPlugin { plugin_id, reply } => {
                if let Some(worker) = self.get_worker(&plugin_id).cloned() {
                    let (stop_reply_tx, stop_reply_rx) = oneshot::channel();
                    if let Err(e) = worker.send(WorkerCommand::StopPlugin {
                        plugin_id: plugin_id.clone(),
                        reply: stop_reply_tx,
                    }) {
                        let _ = reply.send(Err(e));
                        return PoolReply::Done;
                    }

                    match stop_reply_rx.await {
                        Ok(result) => {
                            if result.is_ok() {
                                worker.remove_plugin();
                                self.plugin_assignments.remove(&plugin_id);
                            }
                            let _ = reply.send(result);
                        }
                        Err(_) => {
                            let _ = reply.send(Err(Error::Service(
                                "Worker did not respond".to_string(),
                            )));
                        }
                    }
                } else {
                    let _ = reply.send(Err(Error::Service(format!(
                        "Plugin '{}' not found",
                        plugin_id
                    ))));
                }

                PoolReply::Done
            }

            PoolMsg::SendEvent { plugin_id, event } => {
                if let Some(worker) = self.get_worker(&plugin_id) {
                    let _ = worker.send(WorkerCommand::SendEvent { plugin_id, event });
                }
                PoolReply::Done
            }

            PoolMsg::HandleRequest {
                plugin_id,
                request,
                reply,
            } => {
                if let Some(worker) = self.get_worker(&plugin_id).cloned() {
                    let (req_reply_tx, req_reply_rx) = oneshot::channel();
                    if let Err(e) = worker.send(WorkerCommand::HandleRequest {
                        plugin_id,
                        request,
                        reply: req_reply_tx,
                    }) {
                        let _ = reply.send(Err(e));
                        return PoolReply::Done;
                    }

                    // Forward response
                    tokio::spawn(async move {
                        match req_reply_rx.await {
                            Ok(result) => {
                                let _ = reply.send(result);
                            }
                            Err(_) => {
                                let _ = reply.send(Err(Error::Service(
                                    "Worker did not respond".to_string(),
                                )));
                            }
                        }
                    });
                } else {
                    let _ = reply.send(Err(Error::Service(format!(
                        "Plugin '{}' not found",
                        plugin_id
                    ))));
                }

                PoolReply::Done
            }

            PoolMsg::GetStatus { reply } => {
                let status = PoolStatus {
                    worker_count: self.workers.len(),
                    total_plugins: self.plugin_assignments.len(),
                    plugins_per_worker: self
                        .workers
                        .iter()
                        .map(|w| w.get_plugin_count())
                        .collect(),
                };
                let _ = reply.send(status);
                PoolReply::Done
            }

            PoolMsg::Shutdown => {
                self.shutdown_workers();
                PoolReply::Done
            }
        }
    }
}

impl Drop for JsRuntimePoolActor {
    fn drop(&mut self) {
        self.shutdown_workers();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Worker Thread
// ─────────────────────────────────────────────────────────────────────────────

/// Spawn a new worker thread
fn spawn_worker(id: usize) -> WorkerHandle {
    let (command_tx, command_rx) = mpsc::unbounded_channel::<WorkerCommand>();

    let thread_handle = thread::spawn(move || {
        // Create a new tokio runtime for this thread
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        rt.block_on(async {
            run_worker(id, command_rx).await;
        });
    });

    WorkerHandle {
        command_tx,
        thread_handle: Some(thread_handle),
        id,
        plugin_count: AtomicUsize::new(0),
    }
}

/// Run a worker - manages multiple plugins, each with its own JsRuntime
///
/// Each plugin gets its own JsRuntime to isolate module loaders and state.
/// All runtimes run on the same thread to avoid V8 threading issues.
async fn run_worker(id: usize, mut command_rx: mpsc::UnboundedReceiver<WorkerCommand>) {
    use deno_core::{JsRuntime, PollEventLoopOptions};
    use std::collections::HashMap;

    tracing::debug!("Worker {} started", id);

    // Map of plugin_id -> JsRuntime (each plugin gets its own runtime)
    let mut runtimes: HashMap<String, JsRuntime> = HashMap::new();

    loop {
        // Tick timers for all runtimes
        let cmd = tokio::select! {
            cmd = command_rx.recv() => cmd,
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                for (plugin_id, rt) in runtimes.iter_mut() {
                    let _ = rt.execute_script("<neo:tick>", "__neo_tick_timers()");
                    if let Err(e) = tokio::time::timeout(
                        std::time::Duration::from_millis(50),
                        rt.run_event_loop(PollEventLoopOptions::default())
                    ).await {
                        tracing::trace!("Plugin '{}' event loop timeout (normal): {:?}", plugin_id, e);
                    }
                }
                continue;
            }
        };

        let Some(cmd) = cmd else { break };

        match cmd {
            WorkerCommand::LoadPlugin {
                plugin_id,
                manifest,
                base_path,
                reply,
            } => {
                let result = create_plugin_runtime(
                    &plugin_id,
                    &manifest,
                    &base_path,
                )
                .await;

                match result {
                    Ok(rt) => {
                        runtimes.insert(plugin_id.clone(), rt);
                        tracing::info!("Worker {}: Plugin '{}' loaded", id, plugin_id);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }

            WorkerCommand::StopPlugin { plugin_id, reply } => {
                if let Some(mut rt) = runtimes.remove(&plugin_id) {
                    // Call plugin's onStop
                    let code = format!(
                        "(async () => {{ await __neo_call_stop_plugin('{}'); }})()",
                        plugin_id
                    );
                    let _ = rt.execute_script("<neo:stop>", code);
                    let _ = rt.run_event_loop(PollEventLoopOptions::default()).await;
                    tracing::info!("Worker {}: Plugin '{}' stopped", id, plugin_id);
                    let _ = reply.send(Ok(()));
                } else {
                    let _ = reply.send(Err(Error::Service(format!(
                        "Plugin '{}' not found",
                        plugin_id
                    ))));
                }
            }

            WorkerCommand::SendEvent { plugin_id, event } => {
                if let Some(rt) = runtimes.get_mut(&plugin_id) {
                    let event_json = serde_json::to_string(&event).unwrap_or_default();
                    let code = format!(
                        "(async () => {{ await __neo_call_event_for_plugin('{}', {}); }})()",
                        plugin_id, event_json
                    );
                    let _ = rt.execute_script("<neo:event>", code);
                    let _ = rt.run_event_loop(PollEventLoopOptions::default()).await;
                }
            }

            WorkerCommand::HandleRequest {
                plugin_id,
                request,
                reply,
            } => {
                let result = if let Some(rt) = runtimes.get_mut(&plugin_id) {
                    let request_json = serde_json::to_string(&request).unwrap_or_default();
                    let code = format!(
                        "(async () => {{ return await __neo_call_request_for_plugin('{}', {}); }})()",
                        plugin_id, request_json
                    );
                    match rt.execute_script("<neo:request>", code) {
                        Ok(_) => {
                            let _ = rt.run_event_loop(PollEventLoopOptions::default()).await;
                            Ok(ServiceResponse::Ok)
                        }
                        Err(e) => Ok(ServiceResponse::Error {
                            code: "PLUGIN_ERROR".to_string(),
                            message: e.to_string(),
                        }),
                    }
                } else {
                    Ok(ServiceResponse::Error {
                        code: "PLUGIN_NOT_FOUND".to_string(),
                        message: format!("Plugin '{}' not loaded", plugin_id),
                    })
                };
                let _ = reply.send(result);
            }

            WorkerCommand::Shutdown => {
                tracing::info!("Worker {} shutting down", id);
                // Stop all plugins
                for (plugin_id, mut rt) in runtimes.drain() {
                    let code = format!(
                        "(async () => {{ await __neo_call_stop_plugin('{}'); }})()",
                        plugin_id
                    );
                    let _ = rt.execute_script("<neo:stop>", code);
                    let _ = rt.run_event_loop(PollEventLoopOptions::default()).await;
                }
                break;
            }
        }
    }

    tracing::debug!("Worker {} stopped", id);
}

/// Create a new JsRuntime for a plugin
///
/// Each plugin gets its own JsRuntime to isolate module loaders and state.
/// All runtimes run on the same worker thread to avoid V8 threading issues.
async fn create_plugin_runtime(
    plugin_id: &str,
    manifest: &PluginManifest,
    base_path: &PathBuf,
) -> Result<deno_core::JsRuntime> {
    use deno_core::{JsRuntime, ModuleSpecifier, PollEventLoopOptions, RuntimeOptions};
    use super::ops::neo_plugin;
    use super::service::module_loader;

    // Create channels for plugin communication
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
    let (point_read_tx, point_read_rx) = tokio::sync::mpsc::unbounded_channel::<PointReadRequest>();
    let (_response_tx, response_rx) = tokio::sync::mpsc::unbounded_channel();
    let (point_write_tx, point_write_rx) = tokio::sync::mpsc::unbounded_channel::<PointWriteRequest>();

    // Keep channel receivers alive by leaking them
    // (In a full implementation, these would be connected to actual handlers)
    // We use Box::leak to prevent the receivers from being dropped
    Box::leak(Box::new(event_rx));
    Box::leak(Box::new(point_read_rx));
    Box::leak(Box::new(point_write_rx));

    // Create bridge
    let bridge = PluginBridge {
        plugin_id: plugin_id.to_string(),
        config: manifest.config.clone(),
        event_tx,
        point_read_tx,
        point_read_rx: Arc::new(tokio::sync::Mutex::new(response_rx)),
        point_write_tx,
    };

    // Create module loader
    let module_loader = module_loader::FsModuleLoader {
        base_path: base_path.clone(),
    };

    // Create runtime (with global lock to prevent deno_core threading issues)
    let mut js_runtime = {
        let _guard = runtime_creation_lock().lock().unwrap();
        JsRuntime::new(RuntimeOptions {
            extensions: vec![neo_plugin::init_ops()],
            module_loader: Some(std::rc::Rc::new(module_loader)),
            ..Default::default()
        })
    };

    // Inject bridge state
    {
        let op_state = js_runtime.op_state();
        let mut state = op_state.borrow_mut();
        state.put(bridge);
    }

    // Initialize runtime
    js_runtime
        .execute_script("<neo:runtime>", RUNTIME_JS)
        .map_err(|e| Error::Service(format!("Failed to initialize runtime: {}", e)))?;

    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| Error::Service(format!("Runtime init failed: {}", e)))?;

    // Load plugin as ES module
    let main_path = base_path.join(&manifest.main);
    let main_path_abs = main_path
        .canonicalize()
        .map_err(|e| Error::Service(format!("Failed to resolve plugin path: {}", e)))?;

    let main_specifier = ModuleSpecifier::from_file_path(&main_path_abs)
        .map_err(|_| Error::Service(format!("Invalid plugin path: {}", main_path_abs.display())))?;

    let mod_id = js_runtime
        .load_main_es_module(&main_specifier)
        .await
        .map_err(|e| Error::Service(format!("Failed to load plugin module: {}", e)))?;

    // Evaluate the module
    let result = js_runtime.mod_evaluate(mod_id);

    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| Error::Service(format!("Plugin module evaluation failed: {}", e)))?;

    result
        .await
        .map_err(|e| Error::Service(format!("Plugin module error: {}", e)))?;

    // Call plugin's onStart
    let code = format!(
        "(async () => {{ await __neo_call_start_plugin('{}'); }})()",
        plugin_id
    );
    js_runtime
        .execute_script("<neo:start>", code)
        .map_err(|e| Error::Service(format!("Plugin onStart failed: {}", e)))?;

    let _ = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        js_runtime.run_event_loop(PollEventLoopOptions::default()),
    )
    .await;

    Ok(js_runtime)
}

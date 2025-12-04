//! Worker thread implementation for the JavaScript runtime.
//!
//! This module contains the main event loop that runs in a dedicated thread
//! for each runtime, handling commands and managing the V8 isolate.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Once;

use deno_core::serde_v8;
use deno_core::v8;
use deno_core::JsRuntime;
use deno_core::PollEventLoopOptions;
use deno_core::RuntimeOptions;
use tokio::sync::{mpsc, watch};

use crate::command::{BlueprintCommand, RuntimeCommand, ServiceCommand};
use crate::error::RuntimeError;
use crate::ops::neo_runtime;
use crate::services::RuntimeServices;
use crate::types::{BlueprintJs, ExecutionResultJs, ExecutionTrigger};

/// Ensure V8 platform is initialized exactly once.
static V8_INIT: Once = Once::new();

/// Mutex to serialize V8 isolate creation.
/// Creating multiple isolates concurrently can cause crashes in V8.
pub(crate) static ISOLATE_CREATE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Initialize the V8 platform. Call this before spawning any runtimes.
/// Safe to call multiple times - will only initialize once.
pub fn init_platform() {
    V8_INIT.call_once(|| {
        JsRuntime::init_platform(None, false);
    });
}

/// The main worker loop that runs inside the spawned thread.
pub(crate) async fn run_worker(
    name: String,
    initial_code: Option<(String, String)>,
    services: RuntimeServices,
    terminated: Arc<AtomicBool>,
    mut cmd_rx: mpsc::Receiver<RuntimeCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
    init_tx: std::sync::mpsc::SyncSender<Result<v8::IsolateHandle, String>>,
) -> Result<(), RuntimeError> {
    // Create the JsRuntime
    let mut js_runtime = {
        let _lock = ISOLATE_CREATE_LOCK.lock().unwrap();
        tracing::debug!("[run_worker:{}] Creating JsRuntime", name);
        JsRuntime::new(RuntimeOptions {
            extensions: vec![neo_runtime::init_ops_and_esm()],
            ..Default::default()
        })
    };

    let isolate_handle = js_runtime.v8_isolate().thread_safe_handle();

    // Store services in OpState
    {
        let op_state = js_runtime.op_state();
        let mut state = op_state.borrow_mut();
        state.put(services);
    }

    // Load initial code if provided
    if let Some((code, service_id)) = initial_code {
        let script_name: &'static str = Box::leak(format!("<service:{}>", name).into_boxed_str());
        tracing::debug!("[run_worker:{}] Executing service code", name);

        let wrapped_code = format!(
            r#"
            const __module = (() => {{
                {}
            }})();
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

        // Run event loop to complete initialization
        if let Err(e) = js_runtime
            .run_event_loop(PollEventLoopOptions::default())
            .await
        {
            let _ = init_tx.send(Err(e.to_string()));
            return Err(RuntimeError::JavaScript(e.to_string()));
        }

        tracing::debug!("[run_worker:{}] Service loaded successfully", name);
    } else {
        tracing::debug!("[run_worker:{}] Empty runtime ready", name);
    }

    // Send isolate handle to host
    let _ = init_tx.send(Ok(isolate_handle));

    // Track if we're in "service mode" (running event loop continuously)
    let mut service_running = false;

    // Command loop - use select! to wait for commands OR shutdown
    loop {
        // Check shutdown first
        if *shutdown_rx.borrow() || terminated.load(Ordering::SeqCst) {
            tracing::debug!("[run_worker:{}] Shutdown signal received", name);
            break;
        }

        if service_running {
            // Service mode: poll event loop while checking for commands/shutdown
            tokio::select! {
                biased;  // Check shutdown first

                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::debug!("[run_worker:{}] Received shutdown signal during service", name);
                        break;
                    }
                }

                // Check for commands (non-blocking via try_recv style with timeout)
                cmd = cmd_rx.recv() => {
                    if let Some(cmd) = cmd {
                        match cmd {
                            RuntimeCommand::Service(ServiceCommand::Stop { reply }) => {
                                let result = call_service_stop(&mut js_runtime).await;
                                let _ = reply.send(result);
                                service_running = false;
                            }
                            _ => {
                                tracing::warn!("[run_worker:{}] Unexpected command during service run", name);
                            }
                        }
                    } else {
                        break; // Channel closed
                    }
                }

                // Run event loop for a short time to process timers
                _ = run_event_loop_tick(&mut js_runtime) => {
                    // Event loop tick completed, continue loop
                }
            }
        } else {
            // Normal mode: wait for commands
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::debug!("[run_worker:{}] Received shutdown signal", name);
                        break;
                    }
                }

                cmd = cmd_rx.recv() => {
                    let cmd = match cmd {
                        Some(cmd) => cmd,
                        None => {
                            tracing::debug!("[run_worker:{}] Command channel closed", name);
                            break;
                        }
                    };

                    match cmd {
                        // Blueprint commands
                        RuntimeCommand::Blueprint(bp_cmd) => match bp_cmd {
                            BlueprintCommand::ExecuteBlueprint { trigger, reply } => {
                                let result = execute_blueprint(&mut js_runtime, trigger).await;
                                let _ = reply.send(result);
                            }

                            BlueprintCommand::SetBlueprint { blueprint, reply } => {
                                let result = set_blueprint(&mut js_runtime, blueprint);
                                let _ = reply.send(result);
                            }

                            BlueprintCommand::LoadNode { node_id, code, reply } => {
                                let result = load_node(&mut js_runtime, &node_id, &code).await;
                                let _ = reply.send(result);
                            }

                            BlueprintCommand::ExecuteNode { context_json, reply } => {
                                let result = execute_node(&mut js_runtime, &context_json).await;
                                let _ = reply.send(result);
                            }

                            BlueprintCommand::ExecuteNodeById { node_id, context_json, reply } => {
                                let result = execute_node_by_id(&mut js_runtime, &node_id, &context_json).await;
                                let _ = reply.send(result);
                            }

                            BlueprintCommand::HasNode { node_id, reply } => {
                                let result = has_node(&mut js_runtime, &node_id);
                                let _ = reply.send(result);
                            }
                        },

                        // Service commands
                        RuntimeCommand::Service(svc_cmd) => match svc_cmd {
                            ServiceCommand::Start { reply } => {
                                let result = call_service_start(&mut js_runtime).await;
                                let ok = result.is_ok();
                                let _ = reply.send(result);
                                if ok {
                                    service_running = true;
                                }
                            }

                            ServiceCommand::Stop { reply } => {
                                let result = call_service_stop(&mut js_runtime).await;
                                let _ = reply.send(result);
                            }
                        },
                    }
                }
            }
        }
    }

    tracing::debug!("[run_worker:{}] Worker finished", name);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// JS Execution Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Execute a blueprint with the event loop running until completion.
async fn execute_blueprint(
    js_runtime: &mut JsRuntime,
    trigger: ExecutionTrigger,
) -> Result<ExecutionResultJs, String> {
    let trigger_json =
        serde_json::to_string(&trigger).map_err(|e| format!("Failed to serialize trigger: {}", e))?;

    let script = format!(
        r#"(async () => {{
            const trigger = {};
            return await globalThis.__neo_internal.executeBlueprint(trigger);
        }})()"#,
        trigger_json
    );

    // Execute the script
    let result = js_runtime
        .execute_script("<blueprint>", script)
        .map_err(|e| e.to_string())?;

    // Run event loop until the promise resolves
    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| e.to_string())?;

    // Now get the result from the resolved promise
    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, result);

    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(local) {
        match promise.state() {
            v8::PromiseState::Fulfilled => {
                let value = promise.result(scope);
                serde_v8::from_v8(scope, value)
                    .map_err(|e| format!("Failed to deserialize result: {}", e))
            }
            v8::PromiseState::Rejected => {
                let value = promise.result(scope);
                let err_str: String =
                    serde_v8::from_v8(scope, value).unwrap_or_else(|_| "Unknown error".to_string());
                Err(err_str)
            }
            v8::PromiseState::Pending => Err("Promise still pending after event loop".to_string()),
        }
    } else {
        serde_v8::from_v8(scope, local)
            .map_err(|e| format!("Failed to deserialize result: {}", e))
    }
}

/// Set the blueprint for execution.
fn set_blueprint(js_runtime: &mut JsRuntime, blueprint: BlueprintJs) -> Result<(), String> {
    let script = format!(
        r#"globalThis.__neo_current_blueprint = {};"#,
        serde_json::to_string(&blueprint).map_err(|e| e.to_string())?
    );

    js_runtime
        .execute_script("<set_blueprint>", script)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Execute the loaded node (single-node mode).
async fn execute_node(
    js_runtime: &mut JsRuntime,
    context_json: &str,
) -> Result<serde_json::Value, String> {
    let script = format!(
        r#"(async () => {{
            return await globalThis.__neo_internal.executeNode({});
        }})()"#,
        context_json
    );

    let result = js_runtime
        .execute_script("<execute_node>", script)
        .map_err(|e| e.to_string())?;

    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| e.to_string())?;

    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, result);

    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(local) {
        match promise.state() {
            v8::PromiseState::Fulfilled => {
                let value = promise.result(scope);
                serde_v8::from_v8(scope, value)
                    .map_err(|e| format!("Failed to deserialize result: {}", e))
            }
            v8::PromiseState::Rejected => {
                let value = promise.result(scope);
                let err_str: String =
                    serde_v8::from_v8(scope, value).unwrap_or_else(|_| "Unknown error".to_string());
                Err(err_str)
            }
            v8::PromiseState::Pending => Err("Promise still pending after event loop".to_string()),
        }
    } else {
        serde_v8::from_v8(scope, local)
            .map_err(|e| format!("Failed to deserialize result: {}", e))
    }
}

/// Load a node definition into the runtime's node registry.
async fn load_node(js_runtime: &mut JsRuntime, node_id: &str, code: &str) -> Result<(), String> {
    let script = format!(
        r#"(async () => {{
            const __module = (() => {{
                {}
            }})();
            const __def = __module || globalThis.__getLastDefinition();
            if (__def && typeof __def === 'object') {{
                globalThis.__neo_internal.registerNode({}, __def);
            }} else {{
                throw new Error("Node code must call defineNode({{...}})");
            }}
        }})()"#,
        code.replace("export default", "return"),
        serde_json::to_string(node_id).unwrap()
    );

    js_runtime
        .execute_script("<load_node>", script)
        .map_err(|e| e.to_string())?;

    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Execute a node by ID from the registry.
async fn execute_node_by_id(
    js_runtime: &mut JsRuntime,
    node_id: &str,
    context_json: &str,
) -> Result<serde_json::Value, String> {
    let script = format!(
        r#"(async () => {{
            return await globalThis.__neo_internal.executeNodeById({}, {});
        }})()"#,
        serde_json::to_string(node_id).unwrap(),
        context_json
    );

    let result = js_runtime
        .execute_script("<execute_node_by_id>", script)
        .map_err(|e| e.to_string())?;

    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| e.to_string())?;

    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, result);

    if let Ok(promise) = v8::Local::<v8::Promise>::try_from(local) {
        match promise.state() {
            v8::PromiseState::Fulfilled => {
                let value = promise.result(scope);
                serde_v8::from_v8(scope, value)
                    .map_err(|e| format!("Failed to deserialize result: {}", e))
            }
            v8::PromiseState::Rejected => {
                let value = promise.result(scope);
                let err_str: String =
                    serde_v8::from_v8(scope, value).unwrap_or_else(|_| "Unknown error".to_string());
                Err(err_str)
            }
            v8::PromiseState::Pending => Err("Promise still pending after event loop".to_string()),
        }
    } else {
        serde_v8::from_v8(scope, local)
            .map_err(|e| format!("Failed to deserialize result: {}", e))
    }
}

/// Check if a node is registered.
fn has_node(js_runtime: &mut JsRuntime, node_id: &str) -> Result<bool, String> {
    let script = format!(
        r#"globalThis.__neo_internal.hasNode({})"#,
        serde_json::to_string(node_id).unwrap()
    );

    let result = js_runtime
        .execute_script("<has_node>", script)
        .map_err(|e| e.to_string())?;

    let scope = &mut js_runtime.handle_scope();
    let local = v8::Local::new(scope, result);

    serde_v8::from_v8(scope, local).map_err(|e| format!("Failed to deserialize result: {}", e))
}

/// Run the event loop for a single tick (process pending work without blocking).
async fn run_event_loop_tick(js_runtime: &mut JsRuntime) {
    // Run with wait_for_inspector=false to not block
    let _ = js_runtime
        .run_event_loop(PollEventLoopOptions {
            wait_for_inspector: false,
            pump_v8_message_loop: true,
        })
        .await;

    // Small yield to allow other tasks to run
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
}

/// Start a service (calls onStart, but doesn't wait for event loop to drain).
async fn call_service_start(js_runtime: &mut JsRuntime) -> Result<(), String> {
    let script = r#"(async () => {
        await globalThis.__neo_internal.startService();
    })()"#;

    let promise_global = js_runtime
        .execute_script("<startService>", script)
        .map_err(|e| e.to_string())?;

    // Poll until the onStart promise resolves (but not waiting for timers like setInterval)
    loop {
        // Check promise state
        let state = {
            let scope = &mut js_runtime.handle_scope();
            let local = v8::Local::new(scope, &promise_global);
            if let Ok(promise) = v8::Local::<v8::Promise>::try_from(local) {
                match promise.state() {
                    v8::PromiseState::Fulfilled => Some(Ok(())),
                    v8::PromiseState::Rejected => {
                        let value = promise.result(scope);
                        let err_str: String = serde_v8::from_v8(scope, value)
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        Some(Err(err_str))
                    }
                    v8::PromiseState::Pending => None,
                }
            } else {
                // Not a promise, return OK
                Some(Ok(()))
            }
        };

        if let Some(result) = state {
            return result;
        }

        // Run one tick of the event loop
        let _ = js_runtime
            .run_event_loop(PollEventLoopOptions {
                wait_for_inspector: false,
                pump_v8_message_loop: true,
            })
            .await;

        tokio::task::yield_now().await;
    }
}

/// Stop a service (calls onStop and waits for it to complete).
async fn call_service_stop(js_runtime: &mut JsRuntime) -> Result<(), String> {
    let script = r#"(async () => {
        await globalThis.__neo_internal.stopService();
    })()"#;

    js_runtime
        .execute_script("<stopService>", script)
        .map_err(|e| e.to_string())?;

    // For stop, we DO want to wait for it to complete fully
    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

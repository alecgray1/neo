//! Deno ops for the Neo JavaScript runtime.
//!
//! Ops are the bridge between JavaScript and Rust. They're called from JS
//! and can access services stored in OpState.
//!
//! Following Deno's pattern, services are injected into OpState and ops
//! access them directly - no RPC back to a host thread needed.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use deno_core::op2;
use deno_core::OpState;

use crate::types::BlueprintJs;
use crate::{Event, RuntimeServices};

/// State for blueprint execution.
///
/// Stored in OpState and accessed by ops during execution.
#[derive(Default)]
pub struct BlueprintExecutionState {
    /// The current blueprint being executed (set before calling executeBlueprint)
    pub current_blueprint: Option<BlueprintJs>,
    /// Current variable values (mutable during execution)
    pub variables: HashMap<String, serde_json::Value>,
}

/// Synchronous logging op - writes to the Rust tracing system.
#[op2(fast)]
pub fn op_log(#[string] level: &str, #[string] msg: &str) {
    match level {
        "error" => tracing::error!("{}", msg),
        "warn" => tracing::warn!("{}", msg),
        "info" => tracing::info!("{}", msg),
        "debug" => tracing::debug!("{}", msg),
        "trace" => tracing::trace!("{}", msg),
        _ => tracing::info!("{}", msg),
    }
}

/// Get the current timestamp in milliseconds.
#[op2(fast)]
#[bigint]
pub fn op_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Read a point value.
#[op2(async)]
#[serde]
pub async fn op_point_read(
    state: Rc<RefCell<OpState>>,
    #[string] point_id: String,
) -> Result<Option<serde_json::Value>, deno_core::error::AnyError> {
    let points = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().points.clone()
    };

    match points {
        Some(store) => store
            .read(&point_id)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error(
            "Point store not available",
        )),
    }
}

/// Write a point value.
#[op2(async)]
pub async fn op_point_write(
    state: Rc<RefCell<OpState>>,
    #[string] point_id: String,
    #[serde] value: serde_json::Value,
) -> Result<(), deno_core::error::AnyError> {
    let points = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().points.clone()
    };

    match points {
        Some(store) => store
            .write(&point_id, value)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error(
            "Point store not available",
        )),
    }
}

/// Emit an event.
#[op2]
pub fn op_event_emit(
    state: &mut OpState,
    #[string] event_type: String,
    #[string] source: String,
    #[serde] data: serde_json::Value,
) -> Result<(), deno_core::error::AnyError> {
    let events = state.borrow::<RuntimeServices>().events.clone();

    match events {
        Some(publisher) => {
            let event = Event::new(event_type, source, data);
            publisher
                .emit(event)
                .map_err(|e| deno_core::error::generic_error(e.to_string()))
        }
        None => Err(deno_core::error::generic_error(
            "Event publisher not available",
        )),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Blueprint Execution Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Get the current blueprint for execution.
/// Returns the blueprint that was set via `set_blueprint_for_execution`.
#[op2]
#[serde]
pub fn op_get_blueprint(state: &mut OpState) -> Option<BlueprintJs> {
    state
        .try_borrow::<BlueprintExecutionState>()
        .and_then(|s| s.current_blueprint.clone())
}

/// Get a variable value.
#[op2]
#[serde]
pub fn op_get_variable(
    state: &mut OpState,
    #[string] name: String,
) -> Option<serde_json::Value> {
    state
        .try_borrow::<BlueprintExecutionState>()
        .and_then(|s| s.variables.get(&name).cloned())
}

/// Set a variable value.
#[op2]
pub fn op_set_variable(
    state: &mut OpState,
    #[string] name: String,
    #[serde] value: serde_json::Value,
) -> Result<(), deno_core::error::AnyError> {
    if let Some(exec_state) = state.try_borrow_mut::<BlueprintExecutionState>() {
        exec_state.variables.insert(name, value);
        Ok(())
    } else {
        Err(deno_core::error::generic_error(
            "Blueprint execution state not available",
        ))
    }
}

/// Get all current variable values.
#[op2]
#[serde]
pub fn op_get_all_variables(state: &mut OpState) -> HashMap<String, serde_json::Value> {
    state
        .try_borrow::<BlueprintExecutionState>()
        .map(|s| s.variables.clone())
        .unwrap_or_default()
}

deno_core::extension!(
    neo_runtime,
    ops = [
        op_log,
        op_now,
        op_point_read,
        op_point_write,
        op_event_emit,
        // Blueprint execution ops
        op_get_blueprint,
        op_get_variable,
        op_set_variable,
        op_get_all_variables,
    ],
    esm_entry_point = "ext:neo_runtime/bootstrap.js",
    esm = [dir "src", "bootstrap.js"],
    state = |state| {
        state.put(BlueprintExecutionState::default());
    },
);

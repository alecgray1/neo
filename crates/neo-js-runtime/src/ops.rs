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

// ─────────────────────────────────────────────────────────────────────────────
// ECS Ops
// ─────────────────────────────────────────────────────────────────────────────

/// Create a new ECS entity.
#[op2(async)]
#[bigint]
pub async fn op_ecs_create_entity(
    state: Rc<RefCell<OpState>>,
    #[string] name: Option<String>,
    #[bigint] parent: Option<u64>,
    #[serde] components: Vec<(String, serde_json::Value)>,
    #[serde] tags: Vec<String>,
) -> Result<u64, deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .create_entity(name, parent, components, tags)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Delete an ECS entity.
#[op2(async)]
pub async fn op_ecs_delete_entity(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
) -> Result<(), deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .delete_entity(entity_id)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Get a component from an entity.
#[op2(async)]
#[serde]
pub async fn op_ecs_get_component(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
    #[string] component: String,
) -> Result<Option<serde_json::Value>, deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .get_component(entity_id, &component)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Set a component on an entity.
#[op2(async)]
pub async fn op_ecs_set_component(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
    #[string] component: String,
    #[serde] data: serde_json::Value,
) -> Result<(), deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .set_component(entity_id, &component, data)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Add a tag to an entity.
#[op2(async)]
pub async fn op_ecs_add_tag(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
    #[string] tag: String,
) -> Result<(), deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .add_tag(entity_id, &tag)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Remove a tag from an entity.
#[op2(async)]
pub async fn op_ecs_remove_tag(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
    #[string] tag: String,
) -> Result<(), deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .remove_tag(entity_id, &tag)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Check if an entity has a tag.
#[op2(async)]
pub async fn op_ecs_has_tag(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
    #[string] tag: String,
) -> Result<bool, deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .has_tag(entity_id, &tag)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Look up an entity by name.
#[op2(async)]
#[serde]
pub async fn op_ecs_lookup(
    state: Rc<RefCell<OpState>>,
    #[string] name: String,
) -> Result<Option<u64>, deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .lookup(&name)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Get children of an entity.
#[op2(async)]
#[serde]
pub async fn op_ecs_get_children(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
) -> Result<Vec<u64>, deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .get_children(entity_id)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Get parent of an entity.
#[op2(async)]
#[serde]
pub async fn op_ecs_get_parent(
    state: Rc<RefCell<OpState>>,
    #[bigint] entity_id: u64,
) -> Result<Option<u64>, deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .get_parent(entity_id)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
}

/// Query entities with specific components.
#[op2(async)]
#[serde]
pub async fn op_ecs_query(
    state: Rc<RefCell<OpState>>,
    #[serde] components: Vec<String>,
) -> Result<Vec<serde_json::Value>, deno_core::error::AnyError> {
    let ecs = {
        let state = state.borrow();
        state.borrow::<RuntimeServices>().ecs.clone()
    };

    match ecs {
        Some(store) => store
            .query(components)
            .await
            .map_err(|e| deno_core::error::generic_error(e.to_string())),
        None => Err(deno_core::error::generic_error("ECS store not available")),
    }
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
        // ECS ops
        op_ecs_create_entity,
        op_ecs_delete_entity,
        op_ecs_get_component,
        op_ecs_set_component,
        op_ecs_add_tag,
        op_ecs_remove_tag,
        op_ecs_has_tag,
        op_ecs_lookup,
        op_ecs_get_children,
        op_ecs_get_parent,
        op_ecs_query,
    ],
    esm_entry_point = "ext:neo_runtime/bootstrap.js",
    esm = [dir "src", "bootstrap.js"],
    state = |state| {
        state.put(BlueprintExecutionState::default());
    },
);

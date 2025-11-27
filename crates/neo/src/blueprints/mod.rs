// Blueprint Visual Scripting Engine
//
// A server-side visual scripting execution engine inspired by Unreal Engine Blueprints.
// Blueprints are stored as JSON files and executed by the runtime.
//
// Types and runtime are provided by the blueprint_* crates.
// Neo provides the service integration and built-in node implementations.

mod executor;
mod registry;
mod service;
mod service_adapter;

// Re-export from blueprint_types crate
pub use blueprint_types::{
    // Core types
    Blueprint, BlueprintNode, Connection, Position, VariableDef,
    // Pin types
    PinDef, PinDirection, PinType,
    // Node types
    NodeDef, NodeResult,
    // Function types
    FunctionDef, FunctionParam, FUNCTION_ENTRY_NODE, FUNCTION_EXIT_NODE,
    // Execution types
    ExecutionResult, ExecutionTrigger, LatentState, PointCondition, WakeCondition,
    // Service config
    ServiceConfig,
    // Struct types
    StructDef, StructField, StructRegistry,
    // Behaviour types
    BehaviourDef, BehaviourRegistry, BehaviourViolation, CallbackDef, CallbackParam, SignatureMismatch,
    // Function validation
    FunctionValidationError, validate_function, validate_all_functions,
    // Abstract types for neo integration
    DynEvent, DynRequest,
};

// Re-export from blueprint_runtime crate
pub use blueprint_runtime::{
    NodeContext, NodeOutput, NodeExecutor, NodeRegistry,
    FnNodeExecutor, AsyncFnNodeExecutor,
};

// Re-export neo-specific modules
pub use executor::{BlueprintExecutor, ExecutionContext};
pub use registry::{register_builtin_nodes, NodeRegistryExt};
pub use service::{
    start_background_tasks, BlueprintInfo, BlueprintService, ExecuteBlueprint, GetBlueprint,
    HandleEvent, ListBlueprints, LoadBlueprint, RegisterCustomNode, RegisterServiceBlueprints,
    SetServiceRefs, TriggerEvent, UnloadBlueprint,
};
pub use service_adapter::BlueprintServiceAdapter;

// Blueprint Visual Scripting Engine
//
// A server-side visual scripting execution engine inspired by Unreal Engine Blueprints.
// Blueprints are stored as JSON files and executed by the runtime.

mod executor;
mod registry;
mod service;
mod service_adapter;
mod types;

pub use executor::{BlueprintExecutor, ExecutionContext};
pub use registry::{NodeExecutor, NodeRegistry};
pub use service::{
    start_background_tasks, BlueprintInfo, BlueprintService, ExecuteBlueprint, GetBlueprint,
    HandleEvent, ListBlueprints, LoadBlueprint, RegisterCustomNode, RegisterServiceBlueprints,
    SetServiceRefs, TriggerEvent, UnloadBlueprint,
};
pub use service_adapter::BlueprintServiceAdapter;
pub use types::*;

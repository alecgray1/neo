//! ECS World wrapper with worker thread for async access.
//!
//! The Flecs World is not Send+Sync, so we run it in a dedicated thread
//! and communicate via channels (same pattern as BacnetService).

use std::collections::HashMap;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

use flecs_ecs::prelude::*;
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

use crate::commands::{EcsCommand, EcsResponse, EntityId, QueryRelationship, QueryResult};
use crate::components::*;
use crate::hierarchy::*;
use crate::registry::{ComponentRegistry, ComponentSchema, FieldType};

/// Handle for sending commands to the ECS worker.
#[derive(Clone)]
pub struct EcsHandle {
    cmd_tx: mpsc::Sender<EcsCommand>,
}

impl EcsHandle {
    /// Create a new entity.
    pub async fn create_entity(
        &self,
        name: Option<String>,
        parent: Option<EntityId>,
        components: Vec<(String, Value)>,
        tags: Vec<String>,
    ) -> Result<EntityId, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::CreateEntity {
                name,
                parent,
                components,
                tags,
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::EntityCreated(id)) => Ok(id),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Delete an entity.
    pub async fn delete_entity(&self, entity: EntityId) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::DeleteEntity {
                entity,
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::Ok) => Ok(()),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Look up an entity by name.
    pub async fn lookup(&self, name: &str) -> Result<Option<EntityId>, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::LookupEntity {
                name: name.to_string(),
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::EntityFound(id)) => Ok(id),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Get a component from an entity.
    pub async fn get_component(
        &self,
        entity: EntityId,
        component: &str,
    ) -> Result<Option<Value>, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::GetComponent {
                entity,
                component: component.to_string(),
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::ComponentData(data)) => Ok(data),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Set a component on an entity.
    pub async fn set_component(
        &self,
        entity: EntityId,
        component: &str,
        data: Value,
    ) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::SetComponent {
                entity,
                component: component.to_string(),
                data,
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::Ok) => Ok(()),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Add a tag to an entity.
    pub async fn add_tag(&self, entity: EntityId, tag: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::AddTag {
                entity,
                tag: tag.to_string(),
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::Ok) => Ok(()),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Query entities.
    pub async fn query(
        &self,
        with_components: Vec<String>,
        with_tags: Vec<String>,
        relationship: Option<QueryRelationship>,
        include_data: bool,
    ) -> Result<Vec<QueryResult>, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::Query {
                with_components,
                with_tags,
                relationship,
                include_data,
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::QueryResults(results)) => Ok(results),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Register a dynamic component.
    pub async fn register_component(&self, schema: ComponentSchema) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::RegisterComponent { schema, response: tx })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::Ok) => Ok(()),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Save world state to JSON.
    pub async fn save(&self) -> Result<String, String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::SaveWorld { response: tx })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::WorldJson(json)) => Ok(json),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Load world state from JSON.
    pub async fn load(&self, json: &str) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx
            .send(EcsCommand::LoadWorld {
                json: json.to_string(),
                response: tx,
            })
            .map_err(|e| e.to_string())?;

        match rx.await {
            Ok(EcsResponse::Ok) => Ok(()),
            Ok(EcsResponse::Error(e)) => Err(e),
            Ok(_) => Err("Unexpected response".to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Shutdown the ECS worker.
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(EcsCommand::Shutdown);
    }
}

/// The ECS World manager.
///
/// Spawns a worker thread that owns the Flecs World.
pub struct EcsWorld {
    handle: EcsHandle,
    worker_handle: Option<JoinHandle<()>>,
}

impl EcsWorld {
    /// Create a new ECS world with a worker thread.
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<EcsCommand>();

        let worker_handle = thread::Builder::new()
            .name("ecs-worker".to_string())
            .spawn(move || {
                let mut worker = EcsWorker::new();
                worker.run(cmd_rx);
            })
            .expect("Failed to spawn ECS worker thread");

        Self {
            handle: EcsHandle { cmd_tx },
            worker_handle: Some(worker_handle),
        }
    }

    /// Get a handle for sending commands to the ECS worker.
    pub fn handle(&self) -> EcsHandle {
        self.handle.clone()
    }

    /// Shutdown the ECS world and wait for the worker to finish.
    pub fn shutdown(mut self) {
        self.handle.shutdown();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Default for EcsWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for EcsWorld {
    fn drop(&mut self) {
        self.handle.shutdown();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Worker thread that owns the Flecs World and processes commands.
struct EcsWorker {
    world: World,
    registry: ComponentRegistry,
    /// Maps custom tag names to Flecs entity IDs
    custom_tags: HashMap<String, flecs_ecs::core::Entity>,
}

impl EcsWorker {
    fn new() -> Self {
        let world = World::new();
        info!("ECS worker initialized");

        Self {
            world,
            registry: ComponentRegistry::new(),
            custom_tags: HashMap::new(),
        }
    }

    fn run(&mut self, cmd_rx: mpsc::Receiver<EcsCommand>) {
        info!("ECS worker started");

        loop {
            match cmd_rx.recv() {
                Ok(cmd) => {
                    if matches!(cmd, EcsCommand::Shutdown) {
                        info!("ECS worker shutting down");
                        break;
                    }
                    self.handle_command(cmd);
                }
                Err(_) => {
                    info!("ECS command channel closed");
                    break;
                }
            }
        }

        info!("ECS worker stopped");
    }

    fn handle_command(&mut self, cmd: EcsCommand) {
        match cmd {
            EcsCommand::CreateEntity {
                name,
                parent,
                components,
                tags,
                response,
            } => {
                let result = self.create_entity(name, parent, components, tags);
                let _ = response.send(result);
            }

            EcsCommand::DeleteEntity { entity, response } => {
                let result = self.delete_entity(entity);
                let _ = response.send(result);
            }

            EcsCommand::LookupEntity { name, response } => {
                let result = self.lookup_entity(&name);
                let _ = response.send(result);
            }

            EcsCommand::GetComponent {
                entity,
                component,
                response,
            } => {
                let result = self.get_component(entity, &component);
                let _ = response.send(result);
            }

            EcsCommand::SetComponent {
                entity,
                component,
                data,
                response,
            } => {
                let result = self.set_component(entity, &component, data);
                let _ = response.send(result);
            }

            EcsCommand::RemoveComponent {
                entity,
                component,
                response,
            } => {
                let result = self.remove_component(entity, &component);
                let _ = response.send(result);
            }

            EcsCommand::AddTag { entity, tag, response } => {
                let result = self.add_tag(entity, &tag);
                let _ = response.send(result);
            }

            EcsCommand::RemoveTag { entity, tag, response } => {
                let result = self.remove_tag(entity, &tag);
                let _ = response.send(result);
            }

            EcsCommand::HasTag { entity, tag, response } => {
                let result = self.has_tag(entity, &tag);
                let _ = response.send(result);
            }

            EcsCommand::GetParent { entity, response } => {
                let result = self.get_parent(entity);
                let _ = response.send(result);
            }

            EcsCommand::GetChildren { entity, response } => {
                let result = self.get_children(entity);
                let _ = response.send(result);
            }

            EcsCommand::SetParent {
                entity,
                parent,
                response,
            } => {
                let result = self.set_parent(entity, parent);
                let _ = response.send(result);
            }

            EcsCommand::Query {
                with_components,
                with_tags,
                relationship,
                include_data,
                response,
            } => {
                let result =
                    self.query_entities(&with_components, &with_tags, relationship, include_data);
                let _ = response.send(result);
            }

            EcsCommand::RegisterComponent { schema, response } => {
                let result = self.register_dynamic_component(schema);
                let _ = response.send(result);
            }

            EcsCommand::GetSchemas { response } => {
                let schemas = self.registry.all_schemas();
                let _ = response.send(EcsResponse::Schemas(schemas));
            }

            EcsCommand::SaveWorld { response } => {
                let result = self.save_world();
                let _ = response.send(result);
            }

            EcsCommand::LoadWorld { json, response } => {
                let result = self.load_world(&json);
                let _ = response.send(result);
            }

            EcsCommand::Shutdown => {}
        }
    }

    fn create_entity(
        &mut self,
        name: Option<String>,
        parent: Option<EntityId>,
        components: Vec<(String, Value)>,
        tags: Vec<String>,
    ) -> EcsResponse {
        // Create entity and get its ID immediately to avoid borrow conflicts
        let entity_id = {
            let entity = match name {
                Some(ref n) => self.world.entity_named(n),
                None => self.world.entity(),
            };

            // Set parent relationship
            if let Some(parent_id) = parent {
                let parent_entity = self.world.entity_from_id(parent_id.0);
                entity.child_of(parent_entity);
            }

            entity.id().0
        };

        // Add tags using ID-based helper
        for tag in &tags {
            self.add_tag_by_id(entity_id, tag);
        }

        // Set components using ID-based helper
        for (comp_name, data) in components {
            if let Err(e) = self.set_component_by_id(entity_id, &comp_name, data) {
                warn!("Failed to set component {} on entity: {}", comp_name, e);
            }
        }

        let id = EntityId(entity_id);
        debug!("Created entity {:?} with name {:?}", id, name);
        EcsResponse::EntityCreated(id)
    }

    fn delete_entity(&mut self, entity: EntityId) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);
        e.destruct();
        EcsResponse::Ok
    }

    fn lookup_entity(&self, name: &str) -> EcsResponse {
        let entity = self.world.try_lookup(name);
        match entity {
            Some(e) => EcsResponse::EntityFound(Some(EntityId(e.id().0))),
            None => EcsResponse::EntityFound(None),
        }
    }

    fn get_component(&self, entity: EntityId, component: &str) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);
        let data = self.get_component_data(&e, component);
        EcsResponse::ComponentData(data)
    }

    fn set_component(&mut self, entity: EntityId, component: &str, data: Value) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);
        match self.set_component_on_entity(&e, component, data) {
            Ok(()) => EcsResponse::Ok,
            Err(e) => EcsResponse::Error(e),
        }
    }

    fn remove_component(&mut self, entity: EntityId, component: &str) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);

        match component {
            "Temperature" => { e.remove(Temperature::id()); }
            "Setpoint" => { e.remove(Setpoint::id()); }
            "BacnetDevice" => { e.remove(BacnetDevice::id()); }
            "BacnetObjectRef" => { e.remove(BacnetObjectRef::id()); }
            "DamperPosition" => { e.remove(DamperPosition::id()); }
            _ => {
                return EcsResponse::Error(format!("Unknown component: {}", component));
            }
        }

        EcsResponse::Ok
    }

    fn add_tag(&mut self, entity: EntityId, tag: &str) -> EcsResponse {
        self.add_tag_by_id(entity.0, tag);
        EcsResponse::Ok
    }

    fn remove_tag(&mut self, entity: EntityId, tag: &str) -> EcsResponse {
        self.remove_tag_by_id(entity.0, tag);
        EcsResponse::Ok
    }

    fn has_tag(&self, entity: EntityId, tag: &str) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);
        let has = self.entity_has_tag(&e, tag);
        EcsResponse::Bool(has)
    }

    fn get_parent(&self, entity: EntityId) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);
        let parent = e.parent();
        match parent {
            Some(p) => EcsResponse::Parent(Some(EntityId(p.id().0))),
            None => EcsResponse::Parent(None),
        }
    }

    fn get_children(&self, entity: EntityId) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);
        let mut children = Vec::new();

        e.each_child(|child| {
            children.push(EntityId(child.id().0));
        });

        EcsResponse::EntityList(children)
    }

    fn set_parent(&mut self, entity: EntityId, parent: Option<EntityId>) -> EcsResponse {
        let e = self.world.entity_from_id(entity.0);

        if let Some(p) = parent {
            let parent_entity = self.world.entity_from_id(p.0);
            e.child_of(parent_entity);
        }
        // Note: Removing parent is more complex, would need remove_pair

        EcsResponse::Ok
    }

    fn query_entities(
        &self,
        _with_components: &[String],
        _with_tags: &[String],
        _relationship: Option<QueryRelationship>,
        _include_data: bool,
    ) -> EcsResponse {
        // TODO: Implement proper dynamic querying
        // This requires building queries at runtime based on component names
        EcsResponse::QueryResults(vec![])
    }

    fn register_dynamic_component(&mut self, schema: ComponentSchema) -> EcsResponse {
        let name = schema.name.clone();

        if let Err(e) = self.registry.register(schema.clone()) {
            return EcsResponse::Error(e.to_string());
        }

        // Build the component using component_untyped_named
        // We need to call .member() inside each match arm because each Id<T> is a different type
        let mut builder = self.world.component_untyped_named(&name);

        for field in &schema.fields {
            // MetaMember requires 'static lifetime, so we leak the field name.
            // This is safe since component field names live for the program lifetime.
            let field_name: &'static str = Box::leak(field.name.clone().into_boxed_str());

            builder = match field.field_type {
                FieldType::F32 => builder.member(f32::id(), field_name),
                FieldType::F64 => builder.member(f64::id(), field_name),
                FieldType::I32 => builder.member(i32::id(), field_name),
                FieldType::I64 => builder.member(i64::id(), field_name),
                FieldType::U32 => builder.member(u32::id(), field_name),
                FieldType::U64 => builder.member(u64::id(), field_name),
                FieldType::Bool => builder.member(bool::id(), field_name),
                FieldType::String => continue, // Skip string fields for now
            };
        }

        let component_id = builder.id().0;
        self.registry.set_flecs_id(&name, component_id);

        info!("Registered dynamic component: {} (id={})", name, component_id);
        EcsResponse::Ok
    }

    fn save_world(&self) -> EcsResponse {
        let json = self.world.to_json_world(None);
        EcsResponse::WorldJson(json)
    }

    fn load_world(&mut self, json: &str) -> EcsResponse {
        self.world.from_json_world(json, None);
        EcsResponse::Ok
    }

    // =========================================================================
    // Helper methods (ID-based to avoid borrow conflicts)
    // =========================================================================

    fn add_tag_by_id(&mut self, entity_id: u64, tag: &str) {
        // First check if we need a custom tag (to avoid borrowing world while custom_tags is mutated)
        let custom_tag_id = match tag {
            "VavBox" | "VAV_Box" | "Ahu" | "AHU" | "Fcu" | "FCU" | "Chiller" | "Boiler" |
            "Offline" | "InAlarm" | "NeedsService" | "Site" | "Building" | "Floor" |
            "Zone" | "Device" | "Point" => None,
            _ => {
                // Check if custom tag exists, or create it
                let tag_id = if let Some(e) = self.custom_tags.get(tag) {
                    *e
                } else {
                    let e = self.world.entity_named(tag).id();
                    self.custom_tags.insert(tag.to_string(), e);
                    e
                };
                Some(tag_id)
            }
        };

        // Now add the tag using a fresh entity reference
        let entity = self.world.entity_from_id(entity_id);
        match tag {
            "VavBox" | "VAV_Box" => { entity.add(VavBox); }
            "Ahu" | "AHU" => { entity.add(Ahu); }
            "Fcu" | "FCU" => { entity.add(Fcu); }
            "Chiller" => { entity.add(Chiller); }
            "Boiler" => { entity.add(Boiler); }
            "Offline" => { entity.add(Offline); }
            "InAlarm" => { entity.add(InAlarm); }
            "NeedsService" => { entity.add(NeedsService); }
            "Site" => { entity.add(Site); }
            "Building" => { entity.add(Building); }
            "Floor" => { entity.add(Floor); }
            "Zone" => { entity.add(Zone); }
            "Device" => { entity.add(Device); }
            "Point" => { entity.add(Point); }
            _ => {
                if let Some(tag_entity) = custom_tag_id {
                    entity.add(tag_entity);
                }
            }
        }
    }

    fn remove_tag_by_id(&mut self, entity_id: u64, tag: &str) {
        let custom_tag_id = self.custom_tags.get(tag).copied();
        let entity = self.world.entity_from_id(entity_id);

        match tag {
            "VavBox" | "VAV_Box" => { entity.remove(VavBox); }
            "Ahu" | "AHU" => { entity.remove(Ahu); }
            "Fcu" | "FCU" => { entity.remove(Fcu); }
            "Chiller" => { entity.remove(Chiller); }
            "Boiler" => { entity.remove(Boiler); }
            "Offline" => { entity.remove(Offline); }
            "InAlarm" => { entity.remove(InAlarm); }
            "NeedsService" => { entity.remove(NeedsService); }
            _ => {
                if let Some(tag_entity) = custom_tag_id {
                    entity.remove(tag_entity);
                }
            }
        }
    }

    fn entity_has_tag(&self, entity: &flecs_ecs::core::EntityView, tag: &str) -> bool {
        match tag {
            "VavBox" | "VAV_Box" => entity.has(VavBox),
            "Ahu" | "AHU" => entity.has(Ahu),
            "Fcu" | "FCU" => entity.has(Fcu),
            "Chiller" => entity.has(Chiller),
            "Boiler" => entity.has(Boiler),
            "Offline" => entity.has(Offline),
            "InAlarm" => entity.has(InAlarm),
            "NeedsService" => entity.has(NeedsService),
            _ => {
                if let Some(tag_entity) = self.custom_tags.get(tag) {
                    entity.has(*tag_entity)
                } else {
                    false
                }
            }
        }
    }

    fn set_component_by_id(&self, entity_id: u64, component: &str, data: Value) -> Result<(), String> {
        let entity = self.world.entity_from_id(entity_id);
        self.set_component_on_entity(&entity, component, data)
    }

    fn get_component_data(
        &self,
        entity: &flecs_ecs::core::EntityView,
        component: &str,
    ) -> Option<Value> {
        match component {
            "Temperature" => {
                let mut result = None;
                entity.try_get::<Option<&Temperature>>(|temp| {
                    if let Some(t) = temp {
                        result = Some(serde_json::json!({
                            "value": t.value,
                            "unit": t.unit
                        }));
                    }
                });
                result
            }
            "Setpoint" => {
                let mut result = None;
                entity.try_get::<Option<&Setpoint>>(|sp| {
                    if let Some(s) = sp {
                        result = Some(serde_json::json!({
                            "value": s.value
                        }));
                    }
                });
                result
            }
            "BacnetDevice" => {
                let mut result = None;
                entity.try_get::<Option<&BacnetDevice>>(|dev| {
                    if let Some(d) = dev {
                        result = Some(serde_json::json!({
                            "device_id": d.device_id,
                            "address": d.address,
                            "vendor_id": d.vendor_id,
                            "max_apdu": d.max_apdu,
                            "segmentation": d.segmentation
                        }));
                    }
                });
                result
            }
            _ => None,
        }
    }

    fn set_component_on_entity(
        &self,
        entity: &flecs_ecs::core::EntityView,
        component: &str,
        data: Value,
    ) -> Result<(), String> {
        match component {
            "Temperature" => {
                let value = data
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or("Missing value field")?;
                let unit = data
                    .get("unit")
                    .and_then(|v| v.as_str())
                    .unwrap_or("F")
                    .to_string();
                entity.set(Temperature { value, unit });
                Ok(())
            }
            "Setpoint" => {
                let value = data
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or("Missing value field")?;
                entity.set(Setpoint { value });
                Ok(())
            }
            "BacnetDevice" => {
                let device_id = data
                    .get("device_id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing device_id")? as u32;
                let address = data
                    .get("address")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing address")?
                    .to_string();
                let vendor_id = data
                    .get("vendor_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u16;
                let max_apdu = data
                    .get("max_apdu")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1476) as u16;
                let segmentation = data
                    .get("segmentation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("no-segmentation")
                    .to_string();
                entity.set(BacnetDevice {
                    device_id,
                    address,
                    vendor_id,
                    max_apdu,
                    segmentation,
                });
                Ok(())
            }
            "BacnetObjectRef" => {
                let object_type = data
                    .get("object_type")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing object_type")?
                    .to_string();
                let instance = data
                    .get("instance")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing instance")? as u32;
                entity.set(BacnetObjectRef {
                    object_type,
                    instance,
                });
                Ok(())
            }
            "DamperPosition" => {
                let value = data
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or("Missing value")?;
                let min = data.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let max = data.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);
                entity.set(DamperPosition { value, min, max });
                Ok(())
            }
            _ => Err(format!("Unknown component: {}", component)),
        }
    }
}

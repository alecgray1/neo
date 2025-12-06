//! ECS Service implementation
//!
//! Implements the Service trait to integrate ECS into Neo's service architecture.

use std::path::PathBuf;

use async_trait::async_trait;
use tracing::{debug, error, info, warn};

use blueprint_runtime::service::{Event, Service, ServiceContext, ServiceResult, ServiceSpec};
use neo_ecs::{ComponentSchema, EcsHandle, EcsWorld};

/// Configuration for the ECS service
#[derive(Debug, Clone)]
pub struct EcsConfig {
    /// Path to project directory (for loading schemas and state)
    pub project_path: PathBuf,
    /// Whether to load persisted state on startup
    pub load_state: bool,
    /// Whether to save state on shutdown
    pub save_state: bool,
}

impl Default for EcsConfig {
    fn default() -> Self {
        Self {
            project_path: PathBuf::from("./project"),
            load_state: true,
            save_state: true,
        }
    }
}

impl EcsConfig {
    pub fn new(project_path: impl Into<PathBuf>) -> Self {
        Self {
            project_path: project_path.into(),
            ..Default::default()
        }
    }
}

/// ECS Service
///
/// Manages the Flecs ECS world and provides entity-based data model.
pub struct EcsService {
    config: EcsConfig,
    /// The ECS world (spawns worker thread)
    world: Option<EcsWorld>,
    /// Handle for sending commands to ECS
    handle: Option<EcsHandle>,
}

impl EcsService {
    /// Create a new ECS service
    pub fn new(config: EcsConfig) -> Self {
        Self {
            config,
            world: None,
            handle: None,
        }
    }

    /// Create ECS service with an existing world
    ///
    /// Use this when you need to access the ECS handle before the service starts.
    pub fn with_world(config: EcsConfig, world: EcsWorld) -> Self {
        let handle = world.handle();
        Self {
            config,
            world: Some(world),
            handle: Some(handle),
        }
    }

    /// Get the ECS handle for external access
    pub fn handle(&self) -> Option<EcsHandle> {
        self.handle.clone()
    }

    /// Path to component schemas directory
    fn schemas_path(&self) -> PathBuf {
        self.config.project_path.join("components")
    }

    /// Path to persisted state file
    fn state_path(&self) -> PathBuf {
        self.config.project_path.join(".neo").join("ecs-state.json")
    }

    /// Load component schemas from TOML files
    async fn load_schemas(&self, handle: &EcsHandle) -> ServiceResult<()> {
        let schemas_dir = self.schemas_path();

        if !schemas_dir.exists() {
            debug!("No components directory found at {}", schemas_dir.display());
            return Ok(());
        }

        let mut entries = match tokio::fs::read_dir(&schemas_dir).await {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to read components directory: {}", e);
                return Ok(());
            }
        };

        let mut count = 0;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }

            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    match ComponentSchema::from_toml(&content) {
                        Ok(schema) => {
                            let name = schema.name.clone();
                            match handle.register_component(schema).await {
                                Ok(()) => {
                                    info!("Registered dynamic component: {}", name);
                                    count += 1;
                                }
                                Err(e) => {
                                    warn!("Failed to register component {}: {}", name, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse schema from {}: {}", path.display(), e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read {}: {}", path.display(), e);
                }
            }
        }

        if count > 0 {
            info!("Loaded {} dynamic component schemas", count);
        }

        Ok(())
    }

    /// Load persisted world state
    async fn load_state(&self, handle: &EcsHandle) -> ServiceResult<()> {
        let state_path = self.state_path();

        if !state_path.exists() {
            debug!("No persisted ECS state found at {}", state_path.display());
            return Ok(());
        }

        match tokio::fs::read_to_string(&state_path).await {
            Ok(json) => {
                match handle.load(&json).await {
                    Ok(()) => {
                        info!("Loaded ECS state from {}", state_path.display());
                    }
                    Err(e) => {
                        warn!("Failed to load ECS state: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read ECS state file: {}", e);
            }
        }

        Ok(())
    }

    /// Save world state to file
    async fn save_state(&self, handle: &EcsHandle) -> ServiceResult<()> {
        let state_path = self.state_path();

        // Ensure .neo directory exists
        if let Some(parent) = state_path.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                warn!("Failed to create .neo directory: {}", e);
                return Ok(());
            }
        }

        match handle.save().await {
            Ok(json) => {
                match tokio::fs::write(&state_path, &json).await {
                    Ok(()) => {
                        info!("Saved ECS state to {}", state_path.display());
                    }
                    Err(e) => {
                        error!("Failed to write ECS state: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to serialize ECS state: {}", e);
            }
        }

        Ok(())
    }

    /// Handle BACnet object list read event
    async fn handle_object_list(&mut self, data: &serde_json::Value) {
        let Some(handle) = &self.handle else { return };

        let device_id = data.get("device_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let objects = data.get("objects").and_then(|v| v.as_array());

        // Lookup parent device entity by name
        let device_name = format!("bacnet-device-{}", device_id);
        let parent_id = match handle.lookup(&device_name).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                warn!("No ECS entity found for device {}", device_id);
                return;
            }
            Err(e) => {
                warn!("Failed to lookup device {}: {}", device_id, e);
                return;
            }
        };

        let Some(objects) = objects else { return };

        for obj in objects {
            let object_type = obj.get("object_type").and_then(|v| v.as_str()).unwrap_or("");
            let instance = obj.get("instance").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

            let name = format!("{}-{}", object_type, instance);

            let components = vec![
                ("BacnetObjectRef".to_string(), serde_json::json!({
                    "object_type": object_type,
                    "instance": instance
                }))
            ];

            // Infer point type tag based on object type
            let mut tags = vec!["Point".to_string()];
            if object_type.contains("input") || object_type.contains("value") {
                // Could add more specific tags based on object type
            }

            match handle.create_entity(Some(name), Some(parent_id), components, tags).await {
                Ok(_entity_id) => {
                    debug!("Created ECS entity for {}.{} under device {}", object_type, instance, device_id);
                }
                Err(e) => {
                    warn!("Failed to create ECS entity for {}.{}: {}", object_type, instance, e);
                }
            }
        }
    }

    /// Handle BACnet property read event
    async fn handle_property_read(&mut self, data: &serde_json::Value) {
        let Some(handle) = &self.handle else { return };

        let _device_id = data.get("device_id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let object_type = data.get("object_type").and_then(|v| v.as_str()).unwrap_or("");
        let instance = data.get("instance").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let property = data.get("property").and_then(|v| v.as_str()).unwrap_or("");
        let value = data.get("value");

        // Look up the entity for this object by name
        let object_name = format!("{}-{}", object_type, instance);

        // Lookup entity by name
        if let Ok(Some(entity_id)) = handle.lookup(&object_name).await {
            // Map BACnet property to ECS component
            if property == "present-value" || property == "presentValue" {
                // Determine component type based on object type
                if object_type.contains("analog") {
                    if let Some(v) = value.and_then(|v| v.as_f64()) {
                        // Use Temperature for analog inputs (could be more specific)
                        let _ = handle.set_component(
                            entity_id,
                            "Temperature",
                            serde_json::json!({ "value": v, "unit": "F" })
                        ).await;
                    }
                }
            }
        }
    }
}

#[async_trait]
impl Service for EcsService {
    fn spec(&self) -> ServiceSpec {
        ServiceSpec::new("ecs/world", "ECS World Service")
            .with_subscriptions(vec![
                // Note: Devices are now added explicitly via WebSocket, not auto-added on discovery
                "bacnet/object-list".to_string(),
                "bacnet/property-read".to_string(),
                "ecs/*".to_string(),
            ])
            .with_description("Flecs ECS world for entity-based data model")
    }

    async fn on_start(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        info!("Starting ECS service");

        // Create the ECS world if not already provided (via with_world)
        if self.world.is_none() {
            let world = EcsWorld::new();
            let handle = world.handle();
            self.world = Some(world);
            self.handle = Some(handle);
        }

        let handle = self.handle.as_ref().expect("ECS handle should be set");

        // Load component schemas
        self.load_schemas(handle).await?;

        // Load persisted state if enabled
        if self.config.load_state {
            self.load_state(handle).await?;
        }

        info!("ECS service started");
        Ok(())
    }

    async fn on_event(&mut self, _ctx: &ServiceContext, event: Event) -> ServiceResult<()> {
        match event.event_type.as_str() {
            "bacnet/object-list" => {
                self.handle_object_list(&event.data).await;
            }
            "bacnet/property-read" => {
                self.handle_property_read(&event.data).await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn on_stop(&mut self, _ctx: &ServiceContext) -> ServiceResult<()> {
        info!("Stopping ECS service");

        // Save state if enabled
        if self.config.save_state {
            if let Some(ref handle) = self.handle {
                self.save_state(handle).await?;
            }
        }

        // Shutdown the ECS world
        if let Some(world) = self.world.take() {
            world.shutdown();
        }

        info!("ECS service stopped");
        Ok(())
    }
}

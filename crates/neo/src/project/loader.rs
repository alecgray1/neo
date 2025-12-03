//! Project Loader
//!
//! Loads project configuration from disk.

use std::collections::HashMap;
use std::path::Path;

use tokio::fs;
use tracing::{debug, info, warn};

use super::config::*;

/// Error type for project loading
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("Project path does not exist: {0}")]
    PathNotFound(std::path::PathBuf),

    #[error("Project manifest not found: {0}")]
    ManifestNotFound(std::path::PathBuf),

    #[error("Failed to read file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse TOML: {0}")]
    TomlParseError(#[from] toml::de::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),
}

/// Project loader
pub struct ProjectLoader;

impl ProjectLoader {
    /// Load a project from the given path
    pub async fn load(path: impl AsRef<Path>) -> Result<Project, LoadError> {
        let path = path.as_ref();

        // Check path exists
        if !path.exists() {
            return Err(LoadError::PathNotFound(path.to_path_buf()));
        }

        info!("Loading project from: {}", path.display());

        // Load manifest
        let manifest_path = path.join("project.toml");
        if !manifest_path.exists() {
            return Err(LoadError::ManifestNotFound(manifest_path));
        }

        let manifest_content = fs::read_to_string(&manifest_path).await?;
        let manifest: ProjectManifest = toml::from_str(&manifest_content)?;
        info!("Loaded project manifest: {} ({})", manifest.project.name, manifest.project.id);

        // Load devices
        let devices = Self::load_devices(path).await?;
        info!("Loaded {} devices", devices.len());

        // Load schedules
        let schedules = Self::load_schedules(path).await?;
        info!("Loaded {} schedules", schedules.len());

        // Load blueprints
        let blueprints = Self::load_blueprints(path).await?;
        info!("Loaded {} blueprints", blueprints.len());

        // Load alarm rules
        let alarm_rules = Self::load_alarm_rules(path).await?;
        if alarm_rules.is_some() {
            info!("Loaded alarm rules");
        }

        // Load plugins
        let plugins = Self::load_plugins(path).await?;
        info!("Loaded {} plugins", plugins.len());

        Ok(Project {
            path: path.to_path_buf(),
            manifest,
            devices,
            schedules,
            blueprints,
            plugins,
            alarm_rules,
        })
    }

    /// Load all devices from the devices/ directory
    async fn load_devices(project_path: &Path) -> Result<HashMap<String, DeviceConfig>, LoadError> {
        let devices_dir = project_path.join("devices");
        let mut devices = HashMap::new();

        if !devices_dir.exists() {
            debug!("No devices directory found");
            return Ok(devices);
        }

        let mut entries = fs::read_dir(&devices_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .device.toml files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".device.toml") {
                    match Self::load_device(&path).await {
                        Ok(device) => {
                            debug!("Loaded device: {}", device.device.id);
                            devices.insert(device.device.id.clone(), device);
                        }
                        Err(e) => {
                            warn!("Failed to load device from {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(devices)
    }

    /// Load a single device configuration
    async fn load_device(path: &Path) -> Result<DeviceConfig, LoadError> {
        let content = fs::read_to_string(path).await?;
        let device: DeviceConfig = toml::from_str(&content)?;
        Ok(device)
    }

    /// Load all schedules from the schedules/ directory
    async fn load_schedules(
        project_path: &Path,
    ) -> Result<HashMap<String, ScheduleConfig>, LoadError> {
        let schedules_dir = project_path.join("schedules");
        let mut schedules = HashMap::new();

        if !schedules_dir.exists() {
            debug!("No schedules directory found");
            return Ok(schedules);
        }

        let mut entries = fs::read_dir(&schedules_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .schedule.toml files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".schedule.toml") {
                    match Self::load_schedule(&path).await {
                        Ok(schedule) => {
                            debug!("Loaded schedule: {}", schedule.schedule.id);
                            schedules.insert(schedule.schedule.id.clone(), schedule);
                        }
                        Err(e) => {
                            warn!("Failed to load schedule from {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(schedules)
    }

    /// Load a single schedule configuration
    async fn load_schedule(path: &Path) -> Result<ScheduleConfig, LoadError> {
        let content = fs::read_to_string(path).await?;
        let schedule: ScheduleConfig = toml::from_str(&content)?;
        Ok(schedule)
    }

    /// Load all blueprints from the blueprints/ directory
    async fn load_blueprints(
        project_path: &Path,
    ) -> Result<HashMap<String, BlueprintConfig>, LoadError> {
        let blueprints_dir = project_path.join("blueprints");
        let mut blueprints = HashMap::new();

        if !blueprints_dir.exists() {
            debug!("No blueprints directory found");
            return Ok(blueprints);
        }

        let mut entries = fs::read_dir(&blueprints_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process .bp.json files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".bp.json") {
                    match Self::load_blueprint(&path).await {
                        Ok(blueprint) => {
                            debug!("Loaded blueprint: {}", blueprint.id);
                            blueprints.insert(blueprint.id.clone(), blueprint);
                        }
                        Err(e) => {
                            warn!("Failed to load blueprint from {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(blueprints)
    }

    /// Load a single blueprint configuration
    async fn load_blueprint(path: &Path) -> Result<BlueprintConfig, LoadError> {
        let content = fs::read_to_string(path).await?;
        let blueprint: BlueprintConfig = serde_json::from_str(&content)?;
        Ok(blueprint)
    }

    /// Load alarm rules from alarms/rules.toml
    async fn load_alarm_rules(
        project_path: &Path,
    ) -> Result<Option<AlarmRulesConfig>, LoadError> {
        let rules_path = project_path.join("alarms").join("rules.toml");

        if !rules_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&rules_path).await?;
        let rules: AlarmRulesConfig = toml::from_str(&content)?;
        Ok(Some(rules))
    }

    /// Load all plugins from the plugins/ directory
    /// Each plugin is a subdirectory containing a neo-plugin.json manifest
    async fn load_plugins(
        project_path: &Path,
    ) -> Result<HashMap<String, LoadedPlugin>, LoadError> {
        let plugins_dir = project_path.join("plugins");
        let mut plugins = HashMap::new();

        if !plugins_dir.exists() {
            debug!("No plugins directory found");
            return Ok(plugins);
        }

        let mut entries = fs::read_dir(&plugins_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Only process directories
            if !path.is_dir() {
                continue;
            }

            // Look for neo-plugin.json in the directory
            let manifest_path = path.join("neo-plugin.json");
            if !manifest_path.exists() {
                // Also check in dist/ subdirectory (common output location)
                let dist_manifest_path = path.join("dist").join("neo-plugin.json");
                if dist_manifest_path.exists() {
                    match Self::load_plugin(&path, &dist_manifest_path).await {
                        Ok(plugin) => {
                            debug!("Loaded plugin: {} from dist/", plugin.manifest.id);
                            plugins.insert(plugin.manifest.id.clone(), plugin);
                        }
                        Err(e) => {
                            warn!("Failed to load plugin from {}: {}", dist_manifest_path.display(), e);
                        }
                    }
                }
                continue;
            }

            match Self::load_plugin(&path, &manifest_path).await {
                Ok(plugin) => {
                    debug!("Loaded plugin: {}", plugin.manifest.id);
                    plugins.insert(plugin.manifest.id.clone(), plugin);
                }
                Err(e) => {
                    warn!("Failed to load plugin from {}: {}", manifest_path.display(), e);
                }
            }
        }

        Ok(plugins)
    }

    /// Load a single plugin from its manifest
    async fn load_plugin(plugin_dir: &Path, manifest_path: &Path) -> Result<LoadedPlugin, LoadError> {
        let content = fs::read_to_string(manifest_path).await?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;

        // The manifest_dir is where service/node entry paths are relative to
        let manifest_dir = manifest_path.parent().unwrap_or(plugin_dir).to_path_buf();

        // Log what we found
        debug!(
            "Loaded plugin manifest: {} ({} services, {} nodes)",
            manifest.id,
            manifest.services.len(),
            manifest.nodes.len()
        );

        Ok(LoadedPlugin {
            manifest,
            plugin_dir: plugin_dir.to_path_buf(),
            manifest_dir,
        })
    }

    /// Reload a specific device
    pub async fn reload_device(
        project_path: &Path,
        device_id: &str,
    ) -> Result<Option<DeviceConfig>, LoadError> {
        let device_path = project_path
            .join("devices")
            .join(format!("{}.device.toml", device_id));

        if !device_path.exists() {
            return Ok(None);
        }

        let device = Self::load_device(&device_path).await?;
        Ok(Some(device))
    }

    /// Reload a specific schedule
    pub async fn reload_schedule(
        project_path: &Path,
        schedule_id: &str,
    ) -> Result<Option<ScheduleConfig>, LoadError> {
        let schedule_path = project_path
            .join("schedules")
            .join(format!("{}.schedule.toml", schedule_id));

        if !schedule_path.exists() {
            return Ok(None);
        }

        let schedule = Self::load_schedule(&schedule_path).await?;
        Ok(Some(schedule))
    }

    /// Reload a specific blueprint
    pub async fn reload_blueprint(
        project_path: &Path,
        blueprint_id: &str,
    ) -> Result<Option<BlueprintConfig>, LoadError> {
        let blueprint_path = project_path
            .join("blueprints")
            .join(format!("{}.bp.json", blueprint_id));

        if !blueprint_path.exists() {
            return Ok(None);
        }

        let blueprint = Self::load_blueprint(&blueprint_path).await?;
        Ok(Some(blueprint))
    }

    /// Save a blueprint to disk
    pub async fn save_blueprint(
        project_path: &Path,
        blueprint: &BlueprintConfig,
    ) -> Result<(), LoadError> {
        let blueprint_path = project_path
            .join("blueprints")
            .join(format!("{}.bp.json", blueprint.id));

        // Ensure blueprints directory exists
        let blueprints_dir = project_path.join("blueprints");
        if !blueprints_dir.exists() {
            fs::create_dir_all(&blueprints_dir).await?;
        }

        // Serialize with pretty formatting
        let content = serde_json::to_string_pretty(blueprint)?;
        fs::write(&blueprint_path, content).await?;

        debug!("Saved blueprint: {}", blueprint.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_test_project() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path();

        // Create project.toml
        fs::write(
            path.join("project.toml"),
            r#"
[project]
id = "test-project"
name = "Test Project"
version = "1.0.0"

[runtime]
auto_start = true
"#,
        )
        .await
        .unwrap();

        // Create devices directory
        fs::create_dir(path.join("devices")).await.unwrap();
        fs::write(
            path.join("devices/vav-101.device.toml"),
            r#"
[device]
id = "vav-101"
name = "VAV 101"
type = "vav"

[connection]
protocol = "bacnet"
device_id = 101

[[points]]
id = "zone-temp"
name = "Zone Temperature"
object_type = "analog-input"
instance = 1
unit = "degF"
"#,
        )
        .await
        .unwrap();

        dir
    }

    #[tokio::test]
    async fn test_load_project() {
        let dir = create_test_project().await;
        let project = ProjectLoader::load(dir.path()).await.unwrap();

        assert_eq!(project.id(), "test-project");
        assert_eq!(project.name(), "Test Project");
        assert!(project.manifest.runtime.auto_start);
        assert_eq!(project.devices.len(), 1);
        assert!(project.devices.contains_key("vav-101"));
    }

    #[tokio::test]
    async fn test_load_device() {
        let dir = create_test_project().await;
        let project = ProjectLoader::load(dir.path()).await.unwrap();

        let device = project.get_device("vav-101").unwrap();
        assert_eq!(device.device.name, "VAV 101");
        assert_eq!(device.device.device_type, "vav");
        assert_eq!(device.connection.protocol, "bacnet");
        assert_eq!(device.points.len(), 1);
        assert_eq!(device.points[0].id, "zone-temp");
    }
}

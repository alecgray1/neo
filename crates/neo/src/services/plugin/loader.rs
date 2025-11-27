// Plugin Loader - Discovers and loads plugins from a directory

use std::path::{Path, PathBuf};

use kameo::actor::ActorRef;

use crate::services::actor::{ServiceActorRef, ServiceMetadata, ServiceType};
use crate::types::{Error, Result};

use super::actor::PluginActor;
use super::pool::JsRuntimePoolActor;
use super::service::PluginManifest;

/// Discovered plugin ready for loading
#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    /// Path to the manifest file
    pub manifest_path: PathBuf,
    /// Parsed manifest
    pub manifest: PluginManifest,
}

/// Scan a directory for Neo plugins
///
/// Looks for `neo-plugin.json` manifest files in immediate subdirectories.
///
/// # Example
/// ```text
/// plugins/
/// ├── weather-service/
/// │   ├── neo-plugin.json  <- Found
/// │   └── src/
/// │       └── index.ts
/// └── another-plugin/
///     ├── neo-plugin.json  <- Found
///     └── src/
///         └── main.ts
/// ```
pub async fn discover_plugins(plugins_dir: &Path) -> Result<Vec<DiscoveredPlugin>> {
    let mut discovered = Vec::new();

    if !plugins_dir.exists() {
        tracing::debug!("Plugins directory does not exist: {}", plugins_dir.display());
        return Ok(discovered);
    }

    let mut entries = tokio::fs::read_dir(plugins_dir)
        .await
        .map_err(|e| Error::Io(e))?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| Error::Io(e))? {
        let path = entry.path();

        // Skip non-directories
        if !path.is_dir() {
            continue;
        }

        // Look for neo-plugin.json in this directory
        let manifest_path = path.join("neo-plugin.json");
        if !manifest_path.exists() {
            continue;
        }

        // Try to parse the manifest
        match load_manifest(&manifest_path).await {
            Ok(manifest) => {
                tracing::debug!("Discovered plugin: {} at {}", manifest.id, path.display());
                discovered.push(DiscoveredPlugin {
                    manifest_path,
                    manifest,
                });
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to load plugin manifest at {}: {}",
                    manifest_path.display(),
                    e
                );
            }
        }
    }

    Ok(discovered)
}

/// Load a plugin manifest from a file
pub async fn load_manifest(path: &Path) -> Result<PluginManifest> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| Error::Io(e))?;

    let manifest: PluginManifest = serde_json::from_str(&content)
        .map_err(|e| Error::Config(format!("Invalid plugin manifest: {}", e)))?;

    Ok(manifest)
}

/// Load all plugins from a directory using the JsRuntimePool
///
/// Discovers plugins and creates ServiceActorRef instances for each PluginActor.
/// The pool manages the underlying JS runtimes.
pub async fn load_plugins(
    plugins_dir: &Path,
    pool: ActorRef<JsRuntimePoolActor>,
) -> Result<Vec<ServiceActorRef>> {
    use kameo::actor::Spawn;

    let discovered = discover_plugins(plugins_dir).await?;
    let mut services = Vec::new();

    for plugin in discovered {
        let base_path = plugin
            .manifest_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        // Create PluginActor
        let plugin_actor = PluginActor::spawn(PluginActor::new(
            plugin.manifest.clone(),
            base_path,
            pool.clone(),
        ));

        // Wrap in ServiceActorRef
        let service_ref = ServiceActorRef::new(
            plugin_actor,
            ServiceMetadata {
                id: plugin.manifest.id.clone(),
                name: plugin.manifest.name.clone(),
                description: plugin.manifest.description.clone(),
                service_type: ServiceType::Plugin,
            },
        );

        tracing::info!(
            "Loaded plugin: {} v{} ({})",
            plugin.manifest.name,
            plugin.manifest.version,
            plugin.manifest.id
        );
        services.push(service_ref);
    }

    Ok(services)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_discover_plugins_empty_dir() {
        let temp = TempDir::new().unwrap();
        let discovered = discover_plugins(temp.path()).await.unwrap();
        assert!(discovered.is_empty());
    }

    #[tokio::test]
    async fn test_discover_plugins_with_manifest() {
        let temp = TempDir::new().unwrap();

        // Create a plugin directory with manifest
        let plugin_dir = temp.path().join("test-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();

        let manifest = r#"{
            "id": "test-plugin",
            "name": "Test Plugin",
            "description": "A test plugin",
            "version": "1.0.0",
            "main": "src/index.ts"
        }"#;

        let manifest_path = plugin_dir.join("neo-plugin.json");
        let mut file = std::fs::File::create(&manifest_path).unwrap();
        file.write_all(manifest.as_bytes()).unwrap();

        // Create src/index.ts so the plugin can be loaded
        let src_dir = plugin_dir.join("src");
        std::fs::create_dir(&src_dir).unwrap();
        let index_path = src_dir.join("index.ts");
        std::fs::File::create(&index_path)
            .unwrap()
            .write_all(b"// empty")
            .unwrap();

        let discovered = discover_plugins(temp.path()).await.unwrap();
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].manifest.id, "test-plugin");
        assert_eq!(discovered[0].manifest.name, "Test Plugin");
    }

    #[tokio::test]
    async fn test_discover_plugins_skips_invalid() {
        let temp = TempDir::new().unwrap();

        // Create a plugin directory with invalid manifest
        let plugin_dir = temp.path().join("bad-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();

        let manifest_path = plugin_dir.join("neo-plugin.json");
        let mut file = std::fs::File::create(&manifest_path).unwrap();
        file.write_all(b"{ invalid json }").unwrap();

        let discovered = discover_plugins(temp.path()).await.unwrap();
        assert!(discovered.is_empty());
    }
}

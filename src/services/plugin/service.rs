// Plugin Service Types - Common types for JavaScript/TypeScript plugins
//
// This module contains the PluginManifest and module loader used by the
// actor-based plugin system.

use serde::{Deserialize, Serialize};

/// Plugin manifest loaded from neo-plugin.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique identifier for this plugin
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Plugin description
    pub description: String,
    /// Plugin version (semver)
    pub version: String,
    /// Entry point file (relative to manifest)
    pub main: String,
    /// Plugin configuration (passed to the plugin)
    #[serde(default)]
    pub config: serde_json::Value,
    /// Event patterns this plugin subscribes to
    #[serde(default)]
    pub subscriptions: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ES Module Loader
// ─────────────────────────────────────────────────────────────────────────────

pub mod module_loader {
    use deno_core::error::AnyError;
    use deno_core::{
        ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier,
        ModuleType, RequestedModuleType, ResolutionKind,
    };
    use std::path::PathBuf;

    /// A simple module loader that loads files from the filesystem
    pub struct FsModuleLoader {
        /// Base path for resolving relative imports
        pub base_path: PathBuf,
    }

    impl ModuleLoader for FsModuleLoader {
        fn resolve(
            &self,
            specifier: &str,
            referrer: &str,
            _kind: ResolutionKind,
        ) -> std::result::Result<ModuleSpecifier, AnyError> {
            // Handle file:// URLs
            if specifier.starts_with("file://") {
                return Ok(ModuleSpecifier::parse(specifier)?);
            }

            // Handle relative imports
            if specifier.starts_with("./") || specifier.starts_with("../") {
                let referrer_url = if referrer.starts_with("file://") {
                    ModuleSpecifier::parse(referrer)?
                } else {
                    // Convert path to file:// URL
                    let referrer_path = if referrer.starts_with('/') {
                        PathBuf::from(referrer)
                    } else {
                        self.base_path.join(referrer)
                    };
                    ModuleSpecifier::from_file_path(&referrer_path).map_err(|_| {
                        deno_core::error::generic_error(format!(
                            "Invalid referrer path: {}",
                            referrer
                        ))
                    })?
                };

                return Ok(referrer_url.join(specifier)?);
            }

            // For other specifiers, try to resolve as file path
            let path = self.base_path.join(specifier);
            ModuleSpecifier::from_file_path(&path).map_err(|_| {
                deno_core::error::generic_error(format!("Cannot resolve module: {}", specifier))
            })
        }

        fn load(
            &self,
            module_specifier: &ModuleSpecifier,
            _maybe_referrer: Option<&ModuleSpecifier>,
            _is_dyn_import: bool,
            _requested_module_type: RequestedModuleType,
        ) -> ModuleLoadResponse {
            let specifier = module_specifier.clone();

            ModuleLoadResponse::Sync(load_module_sync(&specifier))
        }
    }

    fn load_module_sync(
        specifier: &ModuleSpecifier,
    ) -> std::result::Result<ModuleSource, AnyError> {
        // Convert file:// URL to path
        let path = specifier.to_file_path().map_err(|_| {
            deno_core::error::generic_error(format!("Cannot convert to file path: {}", specifier))
        })?;

        // Read the file
        let code = std::fs::read_to_string(&path).map_err(|e| {
            deno_core::error::generic_error(format!("Failed to read {}: {}", path.display(), e))
        })?;

        // Determine module type from extension
        let module_type = if path.extension().map(|e| e == "json").unwrap_or(false) {
            ModuleType::Json
        } else {
            ModuleType::JavaScript
        };

        Ok(ModuleSource::new(
            module_type,
            ModuleSourceCode::String(code.into()),
            specifier,
            None,
        ))
    }
}

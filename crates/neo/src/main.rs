//! Neo Server
//!
//! Building automation server with WebSocket API and blueprint execution.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::net::TcpListener;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use blueprint_runtime::NodeRegistry;
use blueprint_runtime::service::ServiceManager;
use blueprint_types::{Blueprint, TypeRegistry};

use neo::engine::{BlueprintExecutor, register_builtin_nodes};
use neo::plugin::{JsService, JsServiceConfig};
use neo::project::{BlueprintConfig, LoadedPlugin, ProjectLoader, ProjectWatcher};
use neo::server::{AppState, create_router};
use neo_js_runtime::{scan_and_spawn_runtime, RuntimeServices};

/// Neo Building Automation Server
#[derive(Parser, Debug)]
#[command(name = "neo")]
#[command(about = "Neo Building Automation Server", long_about = None)]
struct Args {
    /// Path to the project directory
    #[arg(short, long, default_value = "./project")]
    project: PathBuf,

    /// Server host address
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// Server port
    #[arg(long, default_value = "9600")]
    port: u16,

    /// Don't start blueprint executor service
    #[arg(long)]
    no_blueprints: bool,

    /// Don't start the file watcher
    #[arg(long)]
    no_watch: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize V8 platform on main thread before any workers are spawned.
    // This must happen before any JsRuntime is created.
    neo_js_runtime::init_platform();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("neo=info,tower_http=debug")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse command line arguments
    let args = Args::parse();

    info!("Starting Neo server v{}", env!("CARGO_PKG_VERSION"));

    // Create node registry and register built-in nodes
    let mut node_registry = NodeRegistry::new();
    register_builtin_nodes(&mut node_registry);
    let node_registry = Arc::new(node_registry);

    // Create core components
    let service_manager = Arc::new(ServiceManager::new());
    let type_registry = Arc::new(TypeRegistry::new());

    // Create application state
    let state = AppState::new(service_manager.clone(), type_registry);

    // Load project from default path
    let project_path = &args.project;
    info!("Loading project from: {}", project_path.display());

    match ProjectLoader::load(project_path).await {
        Ok(project) => {
            info!("Loaded project: {} ({})", project.name(), project.id());

            // Start blueprint executor if we have blueprints and it's not disabled
            if !args.no_blueprints && !project.blueprints.is_empty() {
                start_blueprint_executor(
                    &service_manager,
                    node_registry.clone(),
                    &project.blueprints,
                )
                .await;
            }

            // Start plugins
            if !project.plugins.is_empty() {
                start_plugins(&service_manager, &project.plugins).await;
            }

            // Store project in state
            state.set_project(project, project_path.clone()).await;

            // Start file watcher if not disabled
            if !args.no_watch {
                match ProjectWatcher::new(project_path, state.clone()) {
                    Ok(watcher) => {
                        tokio::spawn(watcher.run());
                        info!("File watcher started");
                    }
                    Err(e) => {
                        error!("Failed to start file watcher: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!(
                "Failed to load project from {}: {}",
                project_path.display(),
                e
            );
            std::process::exit(1);
        }
    }

    // Log service status
    let services = service_manager.list();
    if services.is_empty() {
        info!("No services running");
    } else {
        info!("Running services:");
        for (id, state) in services {
            info!("  - {} ({:?})", id, state);
        }
    }

    // Create router
    let app = create_router(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!("Server listening on http://{}", addr);
    info!("WebSocket endpoint: ws://{}/ws", addr);

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(service_manager.clone()))
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Start the blueprint executor service with loaded blueprints
async fn start_blueprint_executor(
    service_manager: &ServiceManager,
    node_registry: Arc<NodeRegistry>,
    blueprints: &std::collections::HashMap<String, BlueprintConfig>,
) {
    // Create executor
    let mut executor =
        BlueprintExecutor::new("blueprint-executor", "Blueprint Executor", node_registry);

    // Load each blueprint
    for (id, config) in blueprints {
        // Convert BlueprintConfig to Blueprint
        let blueprint = blueprint_from_config(id, config);
        executor.load_blueprint(blueprint);
        info!("Loaded blueprint: {} ({})", config.name, id);
    }

    // Spawn the service
    match service_manager.spawn(executor).await {
        Ok(handle) => {
            info!(
                "Blueprint executor started (service_id: {})",
                handle.service_id
            );
        }
        Err(e) => {
            error!("Failed to start blueprint executor: {}", e);
        }
    }
}

/// Convert BlueprintConfig (from project file) to Blueprint (runtime type)
fn blueprint_from_config(id: &str, config: &BlueprintConfig) -> Blueprint {
    let mut blueprint = Blueprint::new(id, &config.name);

    if let Some(desc) = &config.description {
        blueprint.description = Some(desc.clone());
    }

    // Convert nodes from JSON
    for node_value in &config.nodes {
        if let Ok(node) = serde_json::from_value(node_value.clone()) {
            blueprint.nodes.push(node);
        } else {
            warn!("Failed to parse node in blueprint {}: {:?}", id, node_value);
        }
    }

    // Convert connections from JSON
    for conn_value in &config.connections {
        if let Ok(conn) = serde_json::from_value(conn_value.clone()) {
            blueprint.connections.push(conn);
        } else {
            warn!(
                "Failed to parse connection in blueprint {}: {:?}",
                id, conn_value
            );
        }
    }

    blueprint
}

/// Start all loaded plugins
///
/// Uses scan_and_spawn_runtime to discover services and create runtimes.
/// The first service reuses the scan runtime, additional services get their own.
async fn start_plugins(
    service_manager: &ServiceManager,
    plugins: &std::collections::HashMap<String, LoadedPlugin>,
) {
    for (plugin_id, plugin) in plugins {
        // Read the plugin's JavaScript code
        let code = match tokio::fs::read_to_string(&plugin.entry_path).await {
            Ok(code) => code,
            Err(e) => {
                error!("Failed to read plugin code for {}: {}", plugin_id, e);
                continue;
            }
        };

        // Scan the plugin AND get a runtime handle in one operation
        // This avoids V8 corruption from dropping and recreating runtimes
        let (scan_result, first_handle) = match scan_and_spawn_runtime(
            format!("js:{}", plugin_id),
            code.clone(),
            RuntimeServices::default(),
        ) {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to scan plugin {}: {}", plugin_id, e);
                continue;
            }
        };

        if scan_result.services.is_empty() {
            info!("Plugin {} has no services registered", plugin_id);
            // Terminate the unused runtime
            first_handle.terminate();
            continue;
        }

        info!(
            "Plugin {} registered {} service(s)",
            plugin_id,
            scan_result.services.len()
        );

        // Create a JsService for each discovered service (1:1 model)
        // First service reuses the scan runtime, others get new runtimes
        let mut first_handle_option = Some(first_handle);

        for service_def in scan_result.services {
            let config = JsServiceConfig::new(
                &service_def.id,
                &service_def.name,
                code.clone(),
            )
            .with_target_service_id(&service_def.id)
            .with_subscriptions(service_def.subscriptions)
            .with_config(plugin.manifest.config.clone());

            // Use tick interval from JS registration if present
            let config = if let Some(tick_ms) = service_def.tick_interval {
                config.with_tick_interval(std::time::Duration::from_millis(tick_ms))
            } else {
                config
            };

            // First service gets the pre-created runtime, others spawn their own
            let service = if let Some(handle) = first_handle_option.take() {
                JsService::with_runtime(config, handle)
            } else {
                JsService::new(config)
            };

            match service_manager.spawn(service).await {
                Ok(handle) => {
                    info!(
                        "Service started: {} (service_id: {})",
                        service_def.id, handle.service_id
                    );
                }
                Err(e) => {
                    error!("Failed to start service {}: {}", service_def.id, e);
                }
            }
        }
    }
}

/// Wait for shutdown signal and cleanup
async fn shutdown_signal(service_manager: Arc<ServiceManager>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down...");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down...");
        },
    }

    // Shutdown all services gracefully
    info!("Shutting down services...");
    if let Err(e) = service_manager.shutdown_all().await {
        warn!("Some services did not shut down cleanly: {}", e);
    }
}

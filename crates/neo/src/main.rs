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

use blueprint_runtime::service::ServiceManager;
use blueprint_runtime::JsNodeLibrary;
use blueprint_types::{Blueprint, TypeRegistry};

use neo::engine::BlueprintExecutor;
use neo::plugin::{JsService, JsServiceConfig};
use neo::project::{BlueprintConfig, LoadedPlugin, ProjectLoader, ProjectWatcher};
use neo::server::{AppState, create_router};

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

fn main() -> Result<()> {
    // Initialize V8 platform on the actual main thread before tokio runtime starts.
    // This must happen before any JsRuntime is created.
    // Deno's pattern: init_v8 is called before the tokio runtime.
    neo_js_runtime::init_platform();

    // Build and run tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
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

            // Load JS node definitions into library (no runtimes created yet)
            let mut js_library = JsNodeLibrary::new();
            if !project.plugins.is_empty() {
                load_plugin_definitions(&mut js_library, &project.plugins).await;
            }
            let js_library = Arc::new(js_library);

            // Start blueprint executor if we have blueprints and it's not disabled
            if !args.no_blueprints && !project.blueprints.is_empty() {
                start_blueprint_executor(
                    &service_manager,
                    js_library.clone(),
                    &project.blueprints,
                )
                .await;
            }

            // Start plugin services
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
    js_library: Arc<JsNodeLibrary>,
    blueprints: &std::collections::HashMap<String, BlueprintConfig>,
) {
    // Create executor with JS node library
    let mut executor = BlueprintExecutor::new(
        "blueprint-executor",
        "Blueprint Executor",
        js_library,
    );

    // Load each blueprint (this creates JS runtimes for blueprints with JS nodes)
    for (id, config) in blueprints {
        // Convert BlueprintConfig to Blueprint
        let blueprint = blueprint_from_config(id, config);
        executor.load_blueprint(blueprint);
        info!("Loaded blueprint: {} ({})", config.name, id);
    }

    info!(
        "Blueprint executor has {} JS runtimes for {} blueprints",
        executor.js_runtime_count(),
        executor.blueprint_count()
    );

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

/// Load plugin node definitions into the JS node library
///
/// This loads the JavaScript code for each node type but does NOT create any
/// V8 runtimes. Runtimes are created per-blueprint when blueprints are loaded.
async fn load_plugin_definitions(
    library: &mut JsNodeLibrary,
    plugins: &std::collections::HashMap<String, LoadedPlugin>,
) {
    for (_plugin_id, plugin) in plugins {
        for node_entry in &plugin.manifest.nodes {
            // Read the node's JavaScript chunk
            let entry_path = plugin.manifest_dir.join(&node_entry.entry);
            let code = match tokio::fs::read_to_string(&entry_path).await {
                Ok(code) => code,
                Err(e) => {
                    error!(
                        "Failed to read node code for {}: {} (path: {})",
                        node_entry.id, e, entry_path.display()
                    );
                    continue;
                }
            };

            // Register the code in the library (no runtime created yet)
            library.register(node_entry.id.clone(), code);
            info!("Loaded JS node definition: {}", node_entry.id);
        }
    }
}

/// Start all loaded plugins
///
/// Each service from the plugin manifest is loaded as a separate JsService
/// with its own V8 runtime. Service metadata comes from the build manifest.
async fn start_plugins(
    service_manager: &ServiceManager,
    plugins: &std::collections::HashMap<String, LoadedPlugin>,
) {
    for (plugin_id, plugin) in plugins {
        if plugin.manifest.services.is_empty() {
            info!("Plugin {} has no services", plugin_id);
            continue;
        }

        info!(
            "Loading plugin {} ({} services, {} nodes)",
            plugin_id,
            plugin.manifest.services.len(),
            plugin.manifest.nodes.len()
        );

        // Start each service defined in the manifest
        for service_entry in &plugin.manifest.services {
            // Read the service's JavaScript chunk
            let entry_path = plugin.manifest_dir.join(&service_entry.entry);
            let code = match tokio::fs::read_to_string(&entry_path).await {
                Ok(code) => code,
                Err(e) => {
                    error!(
                        "Failed to read service code for {}: {} (path: {})",
                        service_entry.id, e, entry_path.display()
                    );
                    continue;
                }
            };

            // Create service config from manifest entry
            let config = JsServiceConfig::new(
                &service_entry.id,
                &service_entry.id, // Use ID as name for now
                code,
            )
            .with_subscriptions(service_entry.subscriptions.clone());

            let service = JsService::new(config);

            match service_manager.spawn(service).await {
                Ok(handle) => {
                    info!(
                        "Service started: {} (service_id: {})",
                        service_entry.id, handle.service_id
                    );
                }
                Err(e) => {
                    error!("Failed to start service {}: {}", service_entry.id, e);
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

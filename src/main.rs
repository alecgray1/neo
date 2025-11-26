use std::path::PathBuf;

use kameo::actor::Spawn;
use kameo_actors::DeliveryStrategy;
use neo::actors::bacnet::{BACnetIOActor, BACnetNetworkActor};
use neo::actors::{EventRouter, PubSubBroker};
use neo::blueprints::{start_background_tasks, BlueprintService, ListBlueprints};
use neo::messages::NetworkMsg;
use neo::services::{
    // Actor-based services
    AlarmActor, HistoryActor, HistoryConfig,
    // Pool and plugin loading
    JsRuntimePoolActor, load_plugins,
    // Service actor infrastructure
    ServiceActorRef, ServiceMetadata, ActorServiceType,
    // Registry
    RegistryMsg, ServiceRegistry,
};
use tokio::time::{sleep, Duration};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing - respect RUST_LOG env var, default to INFO
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    info!("Neo Building Automation System");
    info!("");
    info!("Actor-based BMS with automatic device discovery and plugin support");
    info!("");

    // ─────────────────────────────────────────────────────────────────────────
    // 1. Core Infrastructure
    // ─────────────────────────────────────────────────────────────────────────

    info!("Starting core infrastructure...");

    // PubSub broker (central event bus)
    let pubsub = PubSubBroker::spawn(PubSubBroker::new(DeliveryStrategy::Guaranteed));
    info!("  PubSub broker started");

    // BACnet I/O actor
    let io_actor = BACnetIOActor::spawn(BACnetIOActor::new());
    info!("  BACnet I/O actor started");

    // JS Runtime Pool for plugins
    let js_pool = JsRuntimePoolActor::spawn(JsRuntimePoolActor::with_default_size());
    info!("  JS runtime pool started");

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Service Registry
    // ─────────────────────────────────────────────────────────────────────────

    info!("");
    info!("Starting service registry...");

    let registry = ServiceRegistry::spawn(ServiceRegistry::new(pubsub.clone()));
    info!("  Service registry started");

    // Event router (bridges PubSub -> ServiceRegistry)
    let event_router = EventRouter::spawn(EventRouter::new(registry.clone()));
    EventRouter::subscribe(event_router.clone(), &pubsub).await?;
    info!("  Event router connected to PubSub");

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Built-in Services
    // ─────────────────────────────────────────────────────────────────────────

    info!("");
    info!("Registering built-in services...");

    // History Actor
    let history_config = HistoryConfig {
        db_path: "./data/history.redb".to_string(),
        retention_days: 365,
        sample_interval_ms: 1000,
    };
    let history_actor = HistoryActor::spawn(HistoryActor::new(history_config));
    let history_ref = ServiceActorRef::new(
        history_actor,
        ServiceMetadata {
            id: "history".to_string(),
            name: "History Service".to_string(),
            description: "Time-series data storage and retrieval".to_string(),
            service_type: ActorServiceType::Native,
        },
    );
    registry
        .ask(RegistryMsg::Register {
            actor_ref: history_ref,
            subscriptions: vec!["PointValueChanged".to_string()],
        })
        .await?;
    info!("  History Service registered (subscribed to PointValueChanged)");

    // Alarm Actor
    let alarm_actor = AlarmActor::spawn(AlarmActor::new());
    let alarm_ref = ServiceActorRef::new(
        alarm_actor,
        ServiceMetadata {
            id: "alarm".to_string(),
            name: "Alarm Service".to_string(),
            description: "Alarm management and condition monitoring".to_string(),
            service_type: ActorServiceType::Native,
        },
    );
    registry
        .ask(RegistryMsg::Register {
            actor_ref: alarm_ref,
            subscriptions: vec!["PointValueChanged".to_string()],
        })
        .await?;
    info!("  Alarm Service registered (subscribed to PointValueChanged)");

    // ─────────────────────────────────────────────────────────────────────────
    // 4. Load Plugins
    // ─────────────────────────────────────────────────────────────────────────

    info!("");
    info!("Loading plugins...");

    let plugins_dir = PathBuf::from("./plugins");
    match load_plugins(&plugins_dir, js_pool.clone()).await {
        Ok(plugins) => {
            if plugins.is_empty() {
                info!("  No plugins found in {}", plugins_dir.display());
            } else {
                for plugin_ref in plugins {
                    let plugin_name = plugin_ref.name();
                    // Get subscriptions from the plugin (stored in metadata)
                    // For now, subscribe to all events
                    let subscriptions = vec!["*".to_string()];

                    match registry
                        .ask(RegistryMsg::Register {
                            actor_ref: plugin_ref,
                            subscriptions: subscriptions.clone(),
                        })
                        .await
                    {
                        Ok(_) => {
                            info!(
                                "  Plugin '{}' registered (subscriptions: {:?})",
                                plugin_name, subscriptions
                            );
                        }
                        Err(e) => {
                            tracing::error!("  Failed to register plugin '{}': {}", plugin_name, e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!("  Failed to load plugins: {}", e);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 5. Blueprint Service (Visual Scripting)
    // ─────────────────────────────────────────────────────────────────────────

    info!("");
    info!("Starting blueprint service...");

    let mut blueprint_service = BlueprintService::new("./data/blueprints");

    // Load all existing blueprints
    match blueprint_service.load_all() {
        Ok(count) => {
            info!("  Loaded {} blueprints", count);
        }
        Err(e) => {
            tracing::warn!("  Failed to load blueprints: {}", e);
        }
    }

    // Start file watching for hot reload
    if let Err(e) = blueprint_service.start_watching() {
        tracing::warn!("  Hot reload disabled: {}", e);
    } else {
        info!("  Hot reload enabled (watching data/blueprints/)");
    }

    // Spawn as actor
    let blueprint_actor = BlueprintService::spawn(blueprint_service);

    // Start background task for latent nodes and file watching
    let blueprint_handle = start_background_tasks(blueprint_actor.clone());

    // TODO: Connect BlueprintService to PubSub so blueprints can react to system events
    // (e.g., PointValueChanged, DeviceDiscovered, etc.)

    // Show loaded blueprints
    if let Ok(blueprints) = blueprint_actor.ask(ListBlueprints).await {
        if !blueprints.is_empty() {
            for bp in &blueprints {
                info!("    - {}: {} ({} nodes)", bp.id, bp.name, bp.node_count);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // 6. Start All Services
    // ─────────────────────────────────────────────────────────────────────────

    info!("");
    info!("Starting all services...");
    registry.ask(RegistryMsg::StartAll).await?;
    info!("  All services started");

    // ─────────────────────────────────────────────────────────────────────────
    // 7. BACnet Network
    // ─────────────────────────────────────────────────────────────────────────

    info!("");
    info!("Starting BACnet network...");

    let bacnet_network = BACnetNetworkActor::spawn(BACnetNetworkActor::new(
        "MainNetwork".to_string(),
        10, // Poll devices every 10 seconds
        pubsub.clone(),
        io_actor.clone(),
    ));

    info!("  Network 'MainNetwork' created");
    info!("    Auto-discovery: enabled");
    info!("    Discovery interval: 60 seconds");
    info!("    Polling interval: 10 seconds");

    // Start background tasks
    let polling_handle = BACnetNetworkActor::start_polling_task(bacnet_network.clone());
    let discovery_handle = BACnetNetworkActor::start_discovery_task(bacnet_network.clone());

    // ─────────────────────────────────────────────────────────────────────────
    // 8. System Ready
    // ─────────────────────────────────────────────────────────────────────────

    info!("");
    info!("System initialized successfully!");
    info!("");
    info!("The system will automatically:");
    info!("  - Discover BACnet devices on the network");
    info!("  - Create device actors for each discovered device");
    info!("  - Poll device values and store history");
    info!("  - Evaluate alarm conditions on value changes");
    info!("  - Route events to subscribed plugins");
    info!("  - Execute blueprints (hot reload enabled)");
    info!("");

    // Wait for initial discovery
    info!("Running initial discovery...");
    sleep(Duration::from_secs(5)).await;

    // Show current status
    match bacnet_network.ask(NetworkMsg::GetStatus).await? {
        neo::actors::bacnet::NetworkReply::Status {
            network_name,
            device_count,
        } => {
            info!("");
            info!("Current Status:");
            info!("  Network: {}", network_name);
            info!("  Devices: {}", device_count);

            if device_count > 0 {
                if let Ok(neo::actors::bacnet::NetworkReply::DeviceList(devices)) =
                    bacnet_network.ask(NetworkMsg::ListDevices).await
                {
                    info!("");
                    info!("Discovered Devices:");
                    for device_name in devices {
                        info!("  - {}", device_name);
                    }
                }
            } else {
                info!("");
                info!("No devices discovered yet.");
                info!("  The system will continue searching in the background.");
            }
        }
        _ => {}
    }

    // Show registered services
    if let neo::services::RegistryReply::ServiceList(services) =
        registry.ask(RegistryMsg::List).await?
    {
        info!("");
        info!("Registered Services:");
        for svc in services {
            info!(
                "  - {} ({:?}, {:?})",
                svc.name, svc.service_type, svc.state
            );
        }
    }

    info!("");
    info!("System running. Press Ctrl+C to exit");
    info!("");

    // ─────────────────────────────────────────────────────────────────────────
    // 9. Wait for Shutdown
    // ─────────────────────────────────────────────────────────────────────────

    tokio::signal::ctrl_c().await?;

    info!("");
    info!("Shutting down...");

    // Stop background tasks
    polling_handle.abort();
    discovery_handle.abort();
    blueprint_handle.abort();

    // Stop all services
    let _ = registry.ask(RegistryMsg::StopAll).await;

    // Drop actors
    drop(blueprint_actor);
    drop(bacnet_network);
    drop(event_router);
    drop(registry);
    drop(js_pool);
    drop(io_actor);
    drop(pubsub);

    info!("Shutdown complete");

    Ok(())
}

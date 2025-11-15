use kameo::actor::Spawn;
use kameo_actors::DeliveryStrategy;
use neo::actors::bacnet::{BACnetNetworkActor, BACnetIOActor};
use neo::messages::NetworkMsg;
use tokio::time::{Duration, sleep};
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Neo Building Automation System");
    info!("");
    info!("Actor-based BACnet BMS with automatic device discovery");
    info!("");

    // 1. Spawn the PubSub broker (central event bus)
    info!("📡 Starting PubSub broker...");
    let pubsub = neo::actors::PubSubBroker::spawn(
        neo::actors::PubSubBroker::new(DeliveryStrategy::Guaranteed)
    );

    // 2. Spawn the BACnet I/O actor (handles all BACnet protocol operations)
    info!("🔌 Starting BACnet I/O actor...");
    let io_actor = BACnetIOActor::spawn(BACnetIOActor::new());
    info!("   • Timeout: 5 seconds");
    info!("   • Retry attempts: 2");

    // 3. Create a BACnet network with auto-discovery enabled
    info!("🌐 Creating BACnet network 'MainNetwork'...");
    let bacnet_network = BACnetNetworkActor::spawn(BACnetNetworkActor::new(
        "MainNetwork".to_string(),
        10, // Poll devices every 10 seconds
        pubsub.clone(),
        io_actor.clone(),
    ));

    info!("   • Auto-discovery: enabled");
    info!("   • Discovery interval: 60 seconds");
    info!("   • Polling interval: 10 seconds");
    info!("");

    // 4. Start background tasks
    info!("⚙️  Starting background tasks...");
    let polling_handle = BACnetNetworkActor::start_polling_task(bacnet_network.clone());
    let discovery_handle = BACnetNetworkActor::start_discovery_task(bacnet_network.clone());
    info!("");

    info!("✅ System initialized successfully!");
    info!("");
    info!("The system will automatically:");
    info!("   • Discover BACnet devices on the network");
    info!("   • Create device actors for each discovered device");
    info!("   • Discover points on each device");
    info!("   • Poll device values every 10 seconds");
    info!("   • Manage device health and reconnection");
    info!("");
    info!("💡 To see devices:");
    info!("   • Start Python virtual devices: cd bacnet-test-devices && ./run_all.sh");
    info!("   • Or connect real BACnet/IP devices on your network");
    info!("");

    // Wait a bit for initial discovery
    info!("⏳ Running initial discovery (this may take a few seconds)...");
    sleep(Duration::from_secs(5)).await;

    // Show current status
    match bacnet_network.ask(NetworkMsg::GetStatus).await? {
        neo::actors::bacnet::NetworkReply::Status {
            network_name,
            device_count,
        } => {
            info!("");
            info!("📊 Current Status:");
            info!("   Network: {}", network_name);
            info!("   Devices: {}", device_count);

            if device_count > 0 {
                // List devices
                if let Ok(neo::actors::bacnet::NetworkReply::DeviceList(devices)) =
                    bacnet_network.ask(NetworkMsg::ListDevices).await
                {
                    info!("");
                    info!("📋 Discovered Devices:");
                    for device_name in devices {
                        info!("   • {}", device_name);
                    }
                }
            } else {
                info!("");
                info!("⚠️  No devices discovered yet.");
                info!("   The system will continue searching in the background.");
                info!("   Check that virtual or real BACnet devices are running.");
            }
        }
        _ => {}
    }

    info!("");
    info!("🔄 System running. Monitoring for devices...");
    info!("   Press Ctrl+C to exit");
    info!("");

    // 6. Keep running until Ctrl+C
    tokio::signal::ctrl_c().await?;

    info!("");
    info!("👋 Shutting down...");

    // Abort background tasks first
    polling_handle.abort();
    discovery_handle.abort();

    // Drop actor references
    drop(bacnet_network);
    drop(io_actor);
    drop(pubsub);

    info!("Shutdown complete");

    Ok(())
}

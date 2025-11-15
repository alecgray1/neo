use kameo::actor::Spawn;
use neo::actors::PubSubBroker;
use neo::actors::bacnet::{BACnetNetworkActor, NetworkMsg};
use neo::messages::{DeviceMsg, PointMsg};
use neo::types::{ObjectId, ObjectType, PointValue};
use rand::{Rng, SeedableRng};
use tokio::time::{Duration, sleep};
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Neo Building Automation System - Proof of Concept");
    info!("");
    info!("This demonstrates a Building Automation System built with Rust actors");
    info!("inspired by Niagara Framework but using Erlang/BEAM-style supervision");
    info!("");

    // 1. Spawn the PubSub broker (central event bus)
    info!("📡 Spawning PubSub broker...");
    let pubsub = PubSubBroker::spawn(PubSubBroker::new());

    sleep(Duration::from_millis(100)).await;

    // 2. Create a BACnet network
    info!("🌐 Creating BACnet network 'MSTP-1'...");
    let bacnet_network = BACnetNetworkActor::spawn(BACnetNetworkActor::new(
        "MSTP-1".to_string(),
        10, // Poll every 10 seconds
        pubsub.clone(),
    ));

    sleep(Duration::from_millis(200)).await;

    // 3. Discover BACnet devices on the network
    info!("");
    info!("🔍 Discovering BACnet devices on the network...");
    info!("   (Make sure Python virtual devices are running!)");
    info!("");

    let discovered = match bacnet_network.ask(NetworkMsg::DiscoverDevices).await? {
        neo::actors::bacnet::NetworkReply::DiscoveredDevices(devices) => devices,
        _ => Vec::new(),
    };

    if discovered.is_empty() {
        info!("⚠️  No BACnet devices discovered!");
        info!("");
        info!("To start Python virtual devices:");
        info!("  cd bacnet-test-devices");
        info!("  ./run_all.sh");
        info!("");
        info!("Press Ctrl+C to exit...");
        tokio::signal::ctrl_c().await?;
        return Ok(());
    }

    info!("✅ Discovered {} BACnet devices:", discovered.len());
    for (name, instance, addr) in &discovered {
        info!("   • {} (instance {}) at {}", name, instance, addr);
    }

    info!("");
    info!("🏢 Creating device actors...");

    // Create actors for each discovered device
    for (device_name, device_instance, device_address) in discovered {
        info!("  Adding device: {}", device_name);

        match bacnet_network
            .ask(NetworkMsg::AddDevice {
                device_name: device_name.clone(),
                device_instance,
                device_address: Some(device_address),
            })
            .await?
        {
            neo::actors::bacnet::NetworkReply::DeviceAdded(_dev) => {
                info!("    ✓ Created actor for {}", device_name);
            }
            _ => {
                info!("    ✗ Failed to create actor for {}", device_name);
            }
        };

        sleep(Duration::from_millis(50)).await;
    }

    sleep(Duration::from_millis(200)).await;

    // 4. Check network status
    info!("");
    info!("📊 Network status:");
    let status = bacnet_network.ask(NetworkMsg::GetStatus).await?;
    info!("  {:?}", status);

    let devices_list = bacnet_network.ask(NetworkMsg::ListDevices).await?;
    info!("  Devices: {:?}", devices_list);

    // Test reading a property from VAV-1
    info!("");
    info!("🧪 Testing BACnet property read...");
    if let Ok(neo::actors::bacnet::NetworkReply::Device(Some(vav1))) = bacnet_network
        .ask(NetworkMsg::GetDevice {
            device_name: "VAV-1".to_string(),
        })
        .await
    {
        info!("  Reading VAV-1 temperature (AI:1)...");
        match vav1
            .ask(DeviceMsg::ReadProperty {
                object_id: ObjectId {
                    object_type: ObjectType::AnalogInput,
                    instance: 1,
                },
                property_id: 85, // Present Value
            })
            .await
        {
            Ok(neo::actors::bacnet::DeviceReply::PropertyValue { value, quality }) => {
                info!("  ✓ Temperature: {} (quality: {:?})", value, quality);
            }
            Ok(neo::actors::bacnet::DeviceReply::Failure(msg)) => {
                info!("  ✗ Failed to read: {}", msg);
            }
            Err(e) => {
                info!("  ✗ Error: {}", e);
            }
            _ => {
                info!("  ✗ Unexpected response");
            }
        }
    }

    info!("");
    info!("✅ System initialized successfully:");
    info!("   • Actor-based architecture with Kameo");
    info!("   • Pub-Sub event broker for decoupled communication");
    info!("   • BACnet protocol actor hierarchy (Network → Device → Point)");
    info!("   • Real BACnet/IP communication with virtual devices");
    info!("   • Message passing between actors");
    info!("");
    info!("📡 System is now connected to real BACnet devices");
    info!("   Press Ctrl+C to stop the server");
    info!("");

    // 6. Wait for Ctrl+C signal
    tokio::signal::ctrl_c().await?;

    info!("");
    info!("👋 Received shutdown signal, shutting down gracefully...");

    Ok(())
}

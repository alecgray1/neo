use kameo::actor::Spawn;
use neo::actors::bacnet::{BACnetNetworkActor, NetworkMsg};
use neo::actors::PubSubBroker;
use neo::messages::{DeviceMsg, PointMsg};
use neo::types::{ObjectId, ObjectType, PointValue};
use rand::{Rng, SeedableRng};
use tokio::time::{sleep, Duration};
use tracing::{info, Level};
use tracing_subscriber;

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

    // 3. Create multiple BACnet devices with points
    info!("🏢 Creating building devices...");

    // Create VAV boxes (Variable Air Volume units)
    for i in 1..=5 {
        let device_name = format!("VAV-{}", i);
        info!("  Adding device: {}", device_name);

        let device = match bacnet_network.ask(NetworkMsg::AddDevice {
            device_name: device_name.clone(),
            device_instance: 100 + i,
        }).await? {
            neo::actors::bacnet::NetworkReply::DeviceAdded(dev) => dev,
            _ => continue,
        };

        sleep(Duration::from_millis(50)).await;

        // Add points to each VAV
        // Temperature sensor (AI)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::AnalogInput, instance: 1 },
            initial_value: PointValue::Real(72.0 + (i as f32 * 0.5)),
        }).await?;

        // Damper position (AO)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::AnalogOutput, instance: 1 },
            initial_value: PointValue::Real(45.0),
        }).await?;

        // Occupancy sensor (BI)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::BinaryInput, instance: 1 },
            initial_value: PointValue::Boolean(i % 2 == 0),
        }).await?;

        // Setpoint (AV)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::AnalogValue, instance: 1 },
            initial_value: PointValue::Real(72.0),
        }).await?;
    }

    // Create AHU (Air Handling Units)
    for i in 1..=2 {
        let device_name = format!("AHU-{}", i);
        info!("  Adding device: {}", device_name);

        let device = match bacnet_network.ask(NetworkMsg::AddDevice {
            device_name: device_name.clone(),
            device_instance: 200 + i,
        }).await? {
            neo::actors::bacnet::NetworkReply::DeviceAdded(dev) => dev,
            _ => continue,
        };

        sleep(Duration::from_millis(50)).await;

        // Supply air temp (AI)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::AnalogInput, instance: 1 },
            initial_value: PointValue::Real(55.0),
        }).await?;

        // Return air temp (AI)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::AnalogInput, instance: 2 },
            initial_value: PointValue::Real(72.0),
        }).await?;

        // Fan status (BV)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::BinaryValue, instance: 1 },
            initial_value: PointValue::Boolean(true),
        }).await?;

        // Fan speed (AO)
        device.tell(DeviceMsg::AddPoint {
            object_id: ObjectId { object_type: ObjectType::AnalogOutput, instance: 1 },
            initial_value: PointValue::Real(75.0),
        }).await?;
    }

    sleep(Duration::from_millis(200)).await;

    // 4. Check network status
    info!("");
    info!("📊 Network status:");
    let status = bacnet_network.ask(NetworkMsg::GetStatus).await?;
    info!("  {:?}", status);

    let devices_list = bacnet_network.ask(NetworkMsg::ListDevices).await?;
    info!("  Devices: {:?}", devices_list);

    info!("");
    info!("✅ System initialized successfully:");
    info!("   • Actor-based architecture with Kameo");
    info!("   • Pub-Sub event broker for decoupled communication");
    info!("   • BACnet protocol actor hierarchy (Network → Device → Point)");
    info!("   • 7 devices with 24 total points");
    info!("   • Message passing between actors");
    info!("");
    info!("🔄 Starting random point value updates...");
    info!("   Press Ctrl+C to stop the server");
    info!("");

    // 5. Spawn a background task to randomly update point values
    let network_clone = bacnet_network.clone();
    tokio::spawn(async move {
        let mut rng = rand::rngs::StdRng::from_entropy();

        loop {
            sleep(Duration::from_secs(3)).await;

            // Get list of devices
            if let Ok(neo::actors::bacnet::NetworkReply::DeviceList(device_names)) =
                network_clone.ask(NetworkMsg::ListDevices).await
            {
                if device_names.is_empty() {
                    continue;
                }

                // Pick a random device
                let device_name = &device_names[rng.gen_range(0..device_names.len())].clone();

                // Get the device reference
                if let Ok(neo::actors::bacnet::NetworkReply::Device(Some(device))) =
                    network_clone.ask(NetworkMsg::GetDevice { device_name: device_name.clone() }).await
                {
                    // Generate a random object_id to update
                    let object_types = [
                        ObjectType::AnalogInput,
                        ObjectType::AnalogOutput,
                        ObjectType::AnalogValue,
                        ObjectType::BinaryInput,
                        ObjectType::BinaryValue,
                    ];
                    let obj_type = object_types[rng.gen_range(0..object_types.len())];
                    let instance = rng.gen_range(1..=2); // Most devices have 1-2 instances

                    let object_id = ObjectId {
                        object_type: obj_type,
                        instance,
                    };

                    // Check if point exists
                    if let Ok(neo::actors::bacnet::DeviceReply::Point(Some(point))) =
                        device.ask(DeviceMsg::GetPoint { object_id }).await
                    {
                        // Generate new random value based on point type
                        let new_value = match object_id.object_type {
                            ObjectType::AnalogInput | ObjectType::AnalogValue => {
                                PointValue::Real(rng.gen_range(65.0..80.0))
                            }
                            ObjectType::AnalogOutput => {
                                PointValue::Real(rng.gen_range(0.0..100.0))
                            }
                            ObjectType::BinaryInput | ObjectType::BinaryValue => {
                                PointValue::Boolean(rng.gen_bool(0.5))
                            }
                            _ => continue,
                        };

                        // Update the point value
                        if let Err(e) = point.tell(PointMsg::UpdateValue(new_value.clone())).await {
                            info!("❌ Failed to update {}/{}: {}", device_name, object_id, e);
                        } else {
                            info!("📝 Updated {}/{} = {}", device_name, object_id, new_value);
                        }
                    }
                }
            }
        }
    });

    // 6. Wait for Ctrl+C signal
    tokio::signal::ctrl_c().await?;

    info!("");
    info!("👋 Received shutdown signal, shutting down gracefully...");

    Ok(())
}

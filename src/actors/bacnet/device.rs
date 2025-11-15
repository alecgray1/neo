use crate::actors::bacnet::point::{BACnetPointActor, PointReply};
use crate::actors::PubSubBroker;
use crate::messages::{DeviceMsg, Event, PointMsg, PubSubMsg};
use crate::types::{DeviceStatus, ObjectId, ObjectType, PointValue};
use bacnet::BACnetServer;
use chrono::Utc;
use dashmap::DashMap;
use kameo::actor::Spawn;
use kameo::Actor;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Represents a BACnet device with multiple points
#[derive(kameo::Actor)]
pub struct BACnetDeviceActor {
    pub device_name: String,
    pub network_name: String,
    pub device_instance: u32,
    pub device_address: Option<SocketAddr>,  // Network address for real BACnet devices
    pub status: DeviceStatus,
    pub points: Arc<DashMap<ObjectId, kameo::actor::ActorRef<BACnetPointActor>>>,
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
    bacnet_server: Option<Arc<Mutex<BACnetServer>>>,  // Real BACnet device connection wrapped in Arc<Mutex>
}

impl BACnetDeviceActor {
    pub fn new(
        device_name: String,
        network_name: String,
        device_instance: u32,
        device_address: Option<SocketAddr>,
        pubsub: kameo::actor::ActorRef<PubSubBroker>,
    ) -> Self {
        // Create BACnet server connection if we have an address
        let bacnet_server = device_address.and_then(|addr| {
            let ip = match addr.ip() {
                std::net::IpAddr::V4(ipv4) => ipv4,
                std::net::IpAddr::V6(_) => {
                    warn!("IPv6 not supported for BACnet, device {} will be unavailable", device_name);
                    return None;
                }
            };

            let mut server = BACnetServer::builder()
                .ip(ip)
                .port(addr.port())
                .device_id(device_instance)
                .build();

            // Try to connect
            match server.connect() {
                Ok(_) => {
                    info!("Connected to BACnet device {} at {}", device_name, addr);
                    Some(Arc::new(Mutex::new(server)))
                }
                Err(e) => {
                    error!("Failed to connect to BACnet device {}: {}", device_name, e);
                    None
                }
            }
        });

        Self {
            device_name,
            network_name,
            device_instance,
            device_address,
            status: DeviceStatus::Offline,
            points: Arc::new(DashMap::new()),
            pubsub: Some(pubsub),
            bacnet_server,
        }
    }

    /// Add a point to this device
    pub async fn add_point(
        &mut self,
        object_id: ObjectId,
        initial_value: PointValue,
    ) -> crate::types::Result<kameo::actor::ActorRef<BACnetPointActor>> {
        if let Some(pubsub) = &self.pubsub {
            let point = BACnetPointActor::spawn(BACnetPointActor::new(
                object_id,
                self.device_name.clone(),
                self.network_name.clone(),
                initial_value,
                pubsub.clone(),
            ));

            self.points.insert(object_id, point.clone());
            info!(
                "Added point {} to device {}/{}",
                object_id, self.network_name, self.device_name
            );

            Ok(point)
        } else {
            Err(crate::types::Error::Actor(
                "PubSub broker not available".to_string(),
            ))
        }
    }

    /// Set device status and publish event
    async fn set_status(&mut self, new_status: DeviceStatus) {
        if self.status != new_status {
            debug!(
                "Device {}/{} status changing from {:?} to {:?}",
                self.network_name, self.device_name, self.status, new_status
            );

            self.status = new_status;

            // Publish status change event
            if let Some(pubsub) = &self.pubsub {
                let topic = format!("bacnet/{}/{}/status", self.network_name, self.device_name);

                let event = Event::DeviceStatusChanged {
                    device: self.device_name.clone(),
                    network: self.network_name.clone(),
                    status: new_status,
                    timestamp: Instant::now(),
                    timestamp_utc: Utc::now(),
                };

                let _ = pubsub.tell(PubSubMsg::Publish { topic, event }).await;
            }
        }
    }

    /// Convert our ObjectType to bacnet ObjectType (u32 constant)
    fn convert_object_type(obj_type: ObjectType) -> bacnet::ObjectType {
        // BACnet standard object type numbers
        match obj_type {
            ObjectType::AnalogInput => 0,    // OBJECT_ANALOG_INPUT
            ObjectType::AnalogOutput => 1,   // OBJECT_ANALOG_OUTPUT
            ObjectType::AnalogValue => 2,    // OBJECT_ANALOG_VALUE
            ObjectType::BinaryInput => 3,    // OBJECT_BINARY_INPUT
            ObjectType::BinaryOutput => 4,   // OBJECT_BINARY_OUTPUT
            ObjectType::BinaryValue => 5,    // OBJECT_BINARY_VALUE
            ObjectType::Device => 8,         // OBJECT_DEVICE
        }
    }

    /// Read a property from the real BACnet device
    async fn read_property_real(
        &self,
        object_id: ObjectId,
        _property_id: u8,
    ) -> crate::types::Result<PointValue> {
        // Check if we have a BACnet server connection
        if let Some(server_arc) = &self.bacnet_server {
            debug!(
                "Reading {}/{} from real device {}",
                self.device_name, object_id, self.device_instance
            );

            // Convert our ObjectType to bacnet ObjectType
            let obj_type = Self::convert_object_type(object_id.object_type);

            // Read present value from the device
            // Note: This is a synchronous call wrapped in a blocking task since bacnet library doesn't support async
            // Clone the Arc (not the BACnetServer itself) to avoid connection issues
            let server_arc = Arc::clone(server_arc);
            let instance = object_id.instance;

            match tokio::task::spawn_blocking(move || {
                // Lock the mutex to access the server
                let server = server_arc.lock().unwrap();
                debug!("Calling read_prop_present_value for object type {}, instance {}", obj_type, instance);
                let result = server.read_prop_present_value(obj_type, instance);
                debug!("read_prop_present_value returned: {:?}", result);
                result
            }).await {
                Ok(Ok(value)) => {
                    // Value is a BACnetValue, convert to our PointValue
                    use bacnet::value::BACnetValue;

                    match object_id.object_type {
                        ObjectType::AnalogInput | ObjectType::AnalogOutput | ObjectType::AnalogValue => {
                            match value {
                                BACnetValue::Real(f) => Ok(PointValue::Real(f)),
                                BACnetValue::Double(d) => Ok(PointValue::Real(d as f32)),
                                BACnetValue::Uint(u) => Ok(PointValue::Real(u as f32)),
                                BACnetValue::Int(i) => Ok(PointValue::Real(i as f32)),
                                _ => Err(crate::types::Error::BACnet(format!("Unexpected value type for analog: {:?}", value)))
                            }
                        }
                        ObjectType::BinaryInput | ObjectType::BinaryOutput | ObjectType::BinaryValue => {
                            match value {
                                BACnetValue::Bool(b) => Ok(PointValue::Boolean(b)),
                                BACnetValue::Uint(u) => Ok(PointValue::Boolean(u != 0)),
                                BACnetValue::Int(i) => Ok(PointValue::Boolean(i != 0)),
                                BACnetValue::Enum(e, _) => Ok(PointValue::Boolean(e != 0)),
                                _ => Err(crate::types::Error::BACnet(format!("Unexpected value type for binary: {:?}", value)))
                            }
                        }
                        ObjectType::Device => {
                            Err(crate::types::Error::BACnet("Cannot read present value from device object".to_string()))
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("Failed to read {} from device: {}", object_id, e);
                    Err(crate::types::Error::BACnet(e.to_string()))
                }
                Err(e) => {
                    error!("Task panicked while reading {}: {}", object_id, e);
                    Err(crate::types::Error::BACnet(format!("Task panic: {}", e)))
                }
            }
        } else {
            Err(crate::types::Error::BACnet(
                "Device has no BACnet server connection".to_string(),
            ))
        }
    }

    /// Simulate polling all points on this device
    async fn poll_points(&mut self) {
        debug!(
            "Polling device {}/{} with {} points",
            self.network_name,
            self.device_name,
            self.points.len()
        );

        // In a real implementation, this would read from actual BACnet device
        // For now, we simulate by keeping points unchanged
        self.set_status(DeviceStatus::Online).await;
    }
}

impl kameo::message::Message<DeviceMsg> for BACnetDeviceActor {
    type Reply = DeviceReply;

    async fn handle(
        &mut self,
        msg: DeviceMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            DeviceMsg::GetStatus => DeviceReply::Status {
                status: self.status,
                point_count: self.points.len(),
            },

            DeviceMsg::Poll => {
                self.poll_points().await;
                DeviceReply::Polled
            }

            DeviceMsg::ReadProperty {
                object_id,
                property_id,
            } => {
                // Read directly from real BACnet device
                match self.read_property_real(object_id, property_id).await {
                    Ok(value) => DeviceReply::PropertyValue {
                        value,
                        quality: crate::types::PointQuality::Good,
                    },
                    Err(e) => {
                        error!("Failed to read {} from device: {}", object_id, e);
                        DeviceReply::Failure(format!("Failed to read property: {}", e))
                    }
                }
            }

            DeviceMsg::WriteProperty {
                object_id: _,
                property_id: _,
                value: _,
            } => {
                // Write to real BACnet device
                // TODO: Implement write_property_real() method
                error!("Write property not yet implemented for real BACnet devices");
                DeviceReply::Failure("Write property not yet implemented".to_string())
            }

            DeviceMsg::AddPoint { object_id: _, initial_value: _ } => {
                // For real BACnet devices, we don't create point actors
                // Points exist on the physical device
                info!("AddPoint called for real device - points exist on physical device");
                DeviceReply::Failure("Cannot add points to real BACnet devices".to_string())
            }

            DeviceMsg::GetPoint { object_id: _ } => {
                // For real BACnet devices, points don't have actor refs
                // We read directly from the device
                info!("GetPoint called for real device - use ReadProperty instead");
                DeviceReply::Point(None)
            }
        }
    }
}

#[derive(Debug, kameo::Reply)]
pub enum DeviceReply {
    Status {
        status: DeviceStatus,
        point_count: usize,
    },
    Polled,
    PropertyValue {
        value: PointValue,
        quality: crate::types::PointQuality,
    },
    PropertyWritten,
    Failure(String),
    PointAdded(kameo::actor::ActorRef<BACnetPointActor>),
    Point(Option<kameo::actor::ActorRef<BACnetPointActor>>),
}

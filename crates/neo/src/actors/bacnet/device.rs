use crate::actors::PubSubBroker;
use crate::actors::bacnet::io::BACnetIOActor;
use crate::messages::{BACnetIOMsg, BACnetIOReply, DeviceMsg, Event};
use crate::types::{BACnetPoint, DeviceStatus, ObjectIdentifier, ObjectType, PropertyIdentifier, PointQuality, PropertyValue};
use chrono::Utc;
use dashmap::DashMap;
use kameo_actors::pubsub::Publish;
use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Represents a BACnet device with multiple points
#[derive(kameo::Actor)]
pub struct BACnetDeviceActor {
    pub device_name: String,
    pub network_name: String,
    pub device_instance: u32,
    pub status: DeviceStatus,
    pub points: DashMap<ObjectIdentifier, BACnetPoint>, // Points as simple structs, not actors
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
    io_actor: kameo::actor::ActorRef<BACnetIOActor>,  // Direct reference to I/O actor

    // Health tracking
    pub last_seen: Instant,
    pub last_seen_utc: chrono::DateTime<Utc>,
    pub consecutive_failures: u32,
    pub max_failures_before_offline: u32,
}

impl BACnetDeviceActor {
    pub fn new(
        device_name: String,
        network_name: String,
        device_instance: u32,
        pubsub: kameo::actor::ActorRef<PubSubBroker>,
        io_actor: kameo::actor::ActorRef<BACnetIOActor>,
    ) -> Self {
        Self {
            device_name,
            network_name,
            device_instance,
            status: DeviceStatus::Offline,
            points: DashMap::new(),
            pubsub: Some(pubsub),
            io_actor,
            last_seen: Instant::now(),
            last_seen_utc: Utc::now(),
            consecutive_failures: 0,
            max_failures_before_offline: 3,
        }
    }

    /// Get a point from cache
    pub fn get_point(&self, object_id: ObjectIdentifier) -> Option<BACnetPoint> {
        self.points.get(&object_id).map(|p| p.clone())
    }

    /// Get all points
    pub fn list_points(&self) -> Vec<BACnetPoint> {
        self.points.iter().map(|p| p.value().clone()).collect()
    }

    /// Update a point value and publish event
    async fn update_point_value(
        &self,
        object_id: ObjectIdentifier,
        value: PropertyValue,
        quality: PointQuality,
    ) {
        if let Some(mut point_ref) = self.points.get_mut(&object_id) {
            let point = point_ref.value_mut();
            let old_value = point.present_value.clone();

            point.present_value = value.clone();
            point.quality = quality;
            point.last_update = Instant::now();
            point.last_update_utc = Utc::now();

            // Publish to PubSub if value changed
            if old_value != value {
                info!(
                    "📊 {}/{} {} changed: {:?} -> {:?}",
                    self.device_name, object_id, object_id, old_value, value
                );
                drop(point_ref); // Release lock before async call
                if let Some(point) = self.points.get(&object_id) {
                    self.publish_point_change(object_id, point.value()).await;
                }
            }
        } else {
            // Point doesn't exist, create it
            info!(
                "📊 {}/{} {} initialized: {:?}",
                self.device_name, object_id, object_id, value
            );
            let point = BACnetPoint {
                object_id,
                present_value: value.clone(),
                quality,
                last_update: Instant::now(),
                last_update_utc: Utc::now(),
                object_name: None,
                description: None,
                units: None,
                cov_increment: None,
            };
            self.points.insert(object_id, point.clone());
            self.publish_point_change(object_id, &point).await;
        }
    }

    /// Publish point value change to PubSub
    async fn publish_point_change(&self, object_id: ObjectIdentifier, point: &BACnetPoint) {
        if let Some(pubsub) = &self.pubsub {
            let event = Event::PointValueChanged {
                point: format!("{}/{}/{}", self.network_name, self.device_name, object_id),
                value: point.present_value.clone(),
                quality: point.quality,
                timestamp: point.last_update,
                timestamp_utc: point.last_update_utc,
            };
            let _ = pubsub.tell(Publish(event)).await;
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
                let event = Event::DeviceStatusChanged {
                    device: self.device_name.clone(),
                    network: self.network_name.clone(),
                    status: new_status,
                    timestamp: Instant::now(),
                    timestamp_utc: Utc::now(),
                };

                let _ = pubsub.tell(Publish(event)).await;
            }
        }
    }

    /// Read a property from the real BACnet device via the I/O actor
    async fn read_property_real(
        &self,
        object_id: ObjectIdentifier,
        property_id: PropertyIdentifier,
    ) -> crate::types::Result<PropertyValue> {
        debug!(
            "Reading {}/{} from device {} via I/O actor",
            self.device_name, object_id, self.device_instance
        );

        match self.io_actor
            .ask(BACnetIOMsg::ReadProperty {
                device_id: self.device_instance,
                object_id,
                property_id,
                array_index: None,  // Read whole value
                timeout_ms: Some(3000),  // 3 second timeout
            })
            .await
        {
            Ok(BACnetIOReply::PropertyValue(value)) => Ok(value),
            Ok(BACnetIOReply::IoError(e)) => {
                Err(crate::types::Error::Protocol(e))
            }
            Ok(other) => Err(crate::types::Error::Protocol(format!(
                "Unexpected reply from I/O actor: {:?}",
                other
            ))),
            Err(e) => Err(crate::types::Error::Protocol(format!(
                "Failed to send message to I/O actor: {}",
                e
            ))),
        }
    }

    /// Poll all points from this device
    async fn poll_points(&mut self) {
        info!(
            "🔄 Polling {} points from device {}",
            self.points.len(),
            self.device_name
        );

        if self.points.is_empty() {
            // No points discovered yet - just set status
            debug!(
                "No points to poll for device {} (discovery may not have run yet)",
                self.device_name
            );
            self.set_status(DeviceStatus::Online).await;
            return;
        }

        let mut success_count = 0;
        let mut failure_count = 0;

        // Read all point values from device
        // Clone the keys to avoid holding the lock
        let point_ids: Vec<ObjectIdentifier> = self.points.iter().map(|entry| *entry.key()).collect();

        for object_id in point_ids {
            match self.read_property_real(object_id, PropertyIdentifier::PresentValue).await {
                Ok(value) => {
                    self.update_point_value(object_id, value, PointQuality::Good)
                        .await;
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Failed to poll {}: {}", object_id, e);
                    if let Some(mut point) = self.points.get_mut(&object_id) {
                        point.quality = PointQuality::Bad;
                    }
                    failure_count += 1;
                }
            }
        }

        info!(
            "✅ Polling complete for {}: {} success, {} failures",
            self.device_name, success_count, failure_count
        );

        // Update health tracking based on polling results
        if success_count > 0 {
            // At least one successful read - device is healthy
            self.consecutive_failures = 0;
            self.last_seen = Instant::now();
            self.last_seen_utc = Utc::now();
            self.set_status(DeviceStatus::Online).await;
        } else if failure_count > 0 {
            // All reads failed
            self.consecutive_failures += 1;

            if self.consecutive_failures >= self.max_failures_before_offline {
                warn!(
                    "Device {} marked offline after {} consecutive failures",
                    self.device_name, self.consecutive_failures
                );
                self.set_status(DeviceStatus::Offline).await;
            } else {
                warn!(
                    "Device {} experiencing issues: {} consecutive failures",
                    self.device_name, self.consecutive_failures
                );
                self.set_status(DeviceStatus::Timeout).await;
            }
        }
    }

    /// Discover all objects/points on this BACnet device by reading the object-list property
    async fn discover_points(&mut self) -> crate::types::Result<usize> {
        info!("Discovering points on device {} using object-list property", self.device_name);

        // Create the device object identifier
        let device_object_id = match ObjectIdentifier::new(ObjectType::Device, self.device_instance) {
            Ok(id) => id,
            Err(e) => return Err(crate::types::Error::Protocol(format!("Invalid device instance: {:?}", e))),
        };

        // Step 1: Read the object-list array length (property ObjectList, index 0)
        let length_reply = self.io_actor
            .ask(BACnetIOMsg::ReadPropertyRaw {
                device_id: self.device_instance,
                object_id: device_object_id,
                property_id: PropertyIdentifier::ObjectList,
                array_index: Some(0),  // Index 0 = array length
                timeout_ms: Some(3000),
            })
            .await;

        let array_length: u32 = match length_reply {
            Ok(BACnetIOReply::PropertyValue(PropertyValue::Unsigned(len))) => {
                info!("Device {} object-list contains {} objects", self.device_name, len);
                len
            }
            Ok(BACnetIOReply::IoError(e)) => {
                return Err(crate::types::Error::Protocol(format!("Failed to read object-list length: {}", e)));
            }
            Ok(other) => {
                return Err(crate::types::Error::Protocol(format!("Unexpected reply for object-list length: {:?}", other)));
            }
            Err(e) => {
                return Err(crate::types::Error::Protocol(format!("Failed to send message: {}", e)));
            }
        };

        if array_length == 0 {
            info!("Device {} has no objects in object-list", self.device_name);
            return Ok(0);
        }

        // Step 2: Read each object identifier from the array
        let mut object_identifiers = Vec::new();

        for index in 1..=array_length {
            let obj_reply = self.io_actor
                .ask(BACnetIOMsg::ReadPropertyRaw {
                    device_id: self.device_instance,
                    object_id: device_object_id,
                    property_id: PropertyIdentifier::ObjectList,
                    array_index: Some(index),
                    timeout_ms: Some(3000),
                })
                .await;

            match obj_reply {
                Ok(BACnetIOReply::PropertyValue(PropertyValue::ObjectIdentifier(oid))) => {
                    // Only include object types that have a present-value property
                    if is_pollable_object_type(oid.object_type) {
                        object_identifiers.push(oid);
                    }
                }
                Ok(BACnetIOReply::IoError(e)) => {
                    warn!("Failed to read object-list[{}]: {}", index, e);
                }
                Ok(other) => {
                    warn!("Expected ObjectIdentifier at index {}, got {:?}", index, other);
                }
                Err(e) => {
                    warn!("Failed to send message for object-list[{}]: {}", index, e);
                }
            }
        }

        info!(
            "Found {} point objects on device {}",
            object_identifiers.len(),
            self.device_name
        );

        let mut discovered_count = 0;

        // Step 3: Create a point for each object and try to read its present value
        for object_id in &object_identifiers {
            // Try to read the present value
            match self.read_property_real(*object_id, PropertyIdentifier::PresentValue).await {
                Ok(value) => {
                    let point = BACnetPoint {
                        object_id: *object_id,
                        present_value: value,
                        quality: PointQuality::Good,
                        last_update: Instant::now(),
                        last_update_utc: Utc::now(),
                        object_name: None,
                        description: None,
                        units: None,
                        cov_increment: None,
                    };

                    self.points.insert(*object_id, point);
                    discovered_count += 1;
                    debug!("Discovered point: {}", object_id);
                }
                Err(e) => {
                    debug!("Could not read present-value for {}: {}", object_id, e);
                    // Still create the point but with null value
                    let point = BACnetPoint {
                        object_id: *object_id,
                        present_value: PropertyValue::Null,
                        quality: PointQuality::Uncertain,
                        last_update: Instant::now(),
                        last_update_utc: Utc::now(),
                        object_name: None,
                        description: None,
                        units: None,
                        cov_increment: None,
                    };

                    self.points.insert(*object_id, point);
                    discovered_count += 1;
                }
            }
        }

        info!(
            "Discovered {} points on device {}",
            discovered_count, self.device_name
        );

        Ok(discovered_count)
    }

    /// Read metadata for a specific point (object-name, description, units)
    #[allow(dead_code)]
    async fn read_point_metadata(
        &self,
        _object_id: ObjectIdentifier,
    ) -> crate::types::Result<(Option<String>, Option<String>, Option<String>)> {
        // Property IDs from BACnet standard:
        // 77 = object-name
        // 28 = description
        // 117 = units (for analog objects)

        // TODO: Implement using I/O actor ReadProperty with specific property IDs
        Ok((None, None, None))
    }

    /// Attempt to reconnect to the device
    async fn reconnect(&mut self) -> crate::types::Result<()> {
        // TODO: Implement reconnection via I/O actor
        // The I/O actor should handle disconnecting and reconnecting to devices
        Err(crate::types::Error::Protocol(
            "Reconnect not yet implemented for I/O actor architecture".to_string(),
        ))
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
                    Ok(value) => {
                        // Update cache
                        self.update_point_value(object_id, value.clone(), PointQuality::Good)
                            .await;
                        DeviceReply::PropertyValue {
                            value,
                            quality: PointQuality::Good,
                        }
                    }
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

            DeviceMsg::DiscoverPoints => match self.discover_points().await {
                Ok(count) => DeviceReply::PointsDiscovered { count },
                Err(e) => DeviceReply::Failure(format!("Discovery failed: {}", e)),
            },

            DeviceMsg::ListPoints => {
                let points = self.list_points();
                DeviceReply::Points(points)
            }

            DeviceMsg::GetPoint { object_id } => {
                let point = self.get_point(object_id);
                DeviceReply::Point(point)
            }

            DeviceMsg::Reconnect => match self.reconnect().await {
                Ok(_) => {
                    info!("Device {} reconnected successfully", self.device_name);
                    DeviceReply::Status {
                        status: self.status,
                        point_count: self.points.len(),
                    }
                }
                Err(e) => DeviceReply::Failure(format!("Reconnection failed: {}", e)),
            },
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
        value: PropertyValue,
        quality: PointQuality,
    },
    PropertyWritten,
    PointsDiscovered {
        count: usize,
    },
    Points(Vec<BACnetPoint>),
    Point(Option<BACnetPoint>),
    Failure(String),
}

/// Check if an object type has a present-value property that can be polled
fn is_pollable_object_type(object_type: ObjectType) -> bool {
    matches!(
        object_type,
        ObjectType::AnalogInput
            | ObjectType::AnalogOutput
            | ObjectType::AnalogValue
            | ObjectType::BinaryInput
            | ObjectType::BinaryOutput
            | ObjectType::BinaryValue
            | ObjectType::MultiStateInput
            | ObjectType::MultiStateOutput
            | ObjectType::MultiStateValue
            | ObjectType::IntegerValue
            | ObjectType::PositiveIntegerValue
            | ObjectType::LargeAnalogValue
            | ObjectType::LightingOutput
            | ObjectType::BinaryLightingOutput
            | ObjectType::Accumulator
            | ObjectType::PulseConverter
    )
}

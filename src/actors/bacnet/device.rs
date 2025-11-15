use crate::actors::bacnet::point::{BACnetPointActor, PointReply};
use crate::actors::PubSubBroker;
use crate::messages::{DeviceMsg, Event, PointMsg, PubSubMsg};
use crate::types::{DeviceStatus, ObjectId, PointValue};
use chrono::Utc;
use dashmap::DashMap;
use kameo::actor::Spawn;
use kameo::Actor;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info};

/// Represents a BACnet device with multiple points
#[derive(kameo::Actor)]
pub struct BACnetDeviceActor {
    pub device_name: String,
    pub network_name: String,
    pub device_instance: u32,
    pub status: DeviceStatus,
    pub points: Arc<DashMap<ObjectId, kameo::actor::ActorRef<BACnetPointActor>>>,
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
}

impl BACnetDeviceActor {
    pub fn new(
        device_name: String,
        network_name: String,
        device_instance: u32,
        pubsub: kameo::actor::ActorRef<PubSubBroker>,
    ) -> Self {
        Self {
            device_name,
            network_name,
            device_instance,
            status: DeviceStatus::Offline,
            points: Arc::new(DashMap::new()),
            pubsub: Some(pubsub),
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
                // Look up the point actor
                if let Some(point_ref) = self.points.get(&object_id) {
                    match point_ref.ask(PointMsg::GetValue).await {
                        Ok(PointReply::Data { value, quality }) => {
                            DeviceReply::PropertyValue { value, quality }
                        }
                        Err(e) => {
                            error!("Failed to read point {}: {}", object_id, e);
                            DeviceReply::Failure(format!("Failed to read point: {}", e))
                        }
                        _ => DeviceReply::Failure("Unexpected reply".to_string()),
                    }
                } else {
                    DeviceReply::Failure(format!("Point {} not found", object_id))
                }
            }

            DeviceMsg::WriteProperty {
                object_id,
                property_id,
                value,
            } => {
                // Look up the point actor and update its value
                if let Some(point_ref) = self.points.get(&object_id) {
                    match point_ref.tell(PointMsg::UpdateValue(value)).await {
                        Ok(_) => DeviceReply::PropertyWritten,
                        Err(e) => {
                            error!("Failed to write point {}: {}", object_id, e);
                            DeviceReply::Failure(format!("Failed to write point: {}", e))
                        }
                    }
                } else {
                    DeviceReply::Failure(format!("Point {} not found", object_id))
                }
            }

            DeviceMsg::AddPoint { object_id, initial_value } => {
                match self.add_point(object_id, initial_value).await {
                    Ok(point_ref) => DeviceReply::PointAdded(point_ref),
                    Err(e) => {
                        error!("Failed to add point {}: {}", object_id, e);
                        DeviceReply::Failure(format!("Failed to add point: {}", e))
                    }
                }
            }

            DeviceMsg::GetPoint { object_id } => {
                let point = self.points.get(&object_id).map(|p| p.clone());
                DeviceReply::Point(point)
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

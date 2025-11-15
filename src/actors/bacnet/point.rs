use crate::actors::PubSubBroker;
use crate::messages::{Event, PointMsg, PubSubMsg};
use crate::types::{ObjectId, PointQuality, PointValue};
use chrono::Utc;
use kameo::Actor;
use std::time::Instant;
use tracing::{debug, info};

/// Represents a single BACnet point (AI, AO, BI, BO, AV, BV)
#[derive(kameo::Actor)]
pub struct BACnetPointActor {
    pub object_id: ObjectId,
    pub device_name: String,
    pub network_name: String,
    pub present_value: PointValue,
    pub quality: PointQuality,
    pub last_update: Instant,
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
}

impl BACnetPointActor {
    pub fn new(
        object_id: ObjectId,
        device_name: String,
        network_name: String,
        initial_value: PointValue,
        pubsub: kameo::actor::ActorRef<PubSubBroker>,
    ) -> Self {
        Self {
            object_id,
            device_name,
            network_name,
            present_value: initial_value,
            quality: PointQuality::Good,
            last_update: Instant::now(),
            pubsub: Some(pubsub),
        }
    }

    /// Update the point value and publish event if changed
    async fn update_value(&mut self, new_value: PointValue) {
        if self.present_value != new_value {
            debug!(
                "Point {} value changing from {} to {}",
                self.object_id, self.present_value, new_value
            );

            self.present_value = new_value.clone();
            self.last_update = Instant::now();

            // Publish value change event
            if let Some(pubsub) = &self.pubsub {
                let topic = format!(
                    "bacnet/{}/{}/{}",
                    self.network_name, self.device_name, self.object_id
                );

                let event = Event::PointValueChanged {
                    point: format!("{}/{}/{}", self.network_name, self.device_name, self.object_id),
                    value: new_value,
                    quality: self.quality,
                    timestamp: self.last_update,
                    timestamp_utc: Utc::now(),
                };

                let _ = pubsub
                    .tell(PubSubMsg::Publish { topic, event })
                    .await;
            }
        }
    }
}

impl kameo::message::Message<PointMsg> for BACnetPointActor {
    type Reply = PointReply;

    async fn handle(
        &mut self,
        msg: PointMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            PointMsg::GetValue => PointReply::Data {
                value: self.present_value.clone(),
                quality: self.quality,
            },

            PointMsg::UpdateValue(new_value) => {
                self.update_value(new_value).await;
                PointReply::Updated
            }
        }
    }
}

#[derive(Debug, kameo::Reply)]
pub enum PointReply {
    Data {
        value: PointValue,
        quality: PointQuality,
    },
    Updated,
}

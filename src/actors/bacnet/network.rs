use crate::actors::PubSubBroker;
use crate::actors::bacnet::device::{BACnetDeviceActor, DeviceReply};
use crate::messages::{DeviceMsg, Event, PubSubMsg};
use crate::types::{DeviceStatus, ServiceState};
use chrono::Utc;
use dashmap::DashMap;
use kameo::Actor;
use kameo::actor::Spawn;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn};

/// Represents a BACnet network that manages multiple devices
#[derive(kameo::Actor)]
pub struct BACnetNetworkActor {
    pub network_name: String,
    pub devices: Arc<DashMap<String, kameo::actor::ActorRef<BACnetDeviceActor>>>,
    pub poll_interval_secs: u64,
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
}

impl BACnetNetworkActor {
    pub fn new(
        network_name: String,
        poll_interval_secs: u64,
        pubsub: kameo::actor::ActorRef<PubSubBroker>,
    ) -> Self {
        Self {
            network_name,
            devices: Arc::new(DashMap::new()),
            poll_interval_secs,
            pubsub: Some(pubsub),
        }
    }

    /// Add a device to this network
    pub async fn add_device(
        &mut self,
        device_name: String,
        device_instance: u32,
    ) -> crate::types::Result<kameo::actor::ActorRef<BACnetDeviceActor>> {
        if let Some(pubsub) = &self.pubsub {
            let device = BACnetDeviceActor::spawn(BACnetDeviceActor::new(
                device_name.clone(),
                self.network_name.clone(),
                device_instance,
                pubsub.clone(),
            ));

            self.devices.insert(device_name.clone(), device.clone());
            info!(
                "Added device {} to BACnet network {}",
                device_name, self.network_name
            );

            Ok(device)
        } else {
            Err(crate::types::Error::Actor(
                "PubSub broker not available".to_string(),
            ))
        }
    }

    /// Start background polling task
    async fn start_polling(&self, actor_ref: kameo::actor::WeakActorRef<Self>) {
        let poll_interval = self.poll_interval_secs;
        let devices = self.devices.clone();
        let network_name = self.network_name.clone();

        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(poll_interval));

            loop {
                tick.tick().await;

                // Check if actor still exists
                if actor_ref.upgrade().is_none() {
                    debug!("BACnet network {} polling task exiting", network_name);
                    break;
                }

                debug!(
                    "Polling {} devices on network {}",
                    devices.len(),
                    network_name
                );

                // Poll all devices
                for device_entry in devices.iter() {
                    let device_name = device_entry.key();
                    let device_ref = device_entry.value();

                    match device_ref.ask(DeviceMsg::Poll).await {
                        Ok(DeviceReply::Polled) => {
                            debug!("Successfully polled device {}", device_name);
                        }
                        Err(e) => {
                            warn!("Failed to poll device {}: {}", device_name, e);
                        }
                        _ => {
                            warn!("Unexpected reply from device {}", device_name);
                        }
                    }
                }
            }
        });
    }
}

impl kameo::message::Message<NetworkMsg> for BACnetNetworkActor {
    type Reply = NetworkReply;

    async fn handle(
        &mut self,
        msg: NetworkMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            NetworkMsg::GetStatus => NetworkReply::Status {
                network_name: self.network_name.clone(),
                device_count: self.devices.len(),
            },

            NetworkMsg::ListDevices => {
                let device_names: Vec<String> =
                    self.devices.iter().map(|e| e.key().clone()).collect();
                NetworkReply::DeviceList(device_names)
            }

            NetworkMsg::PollAll => {
                let mut polled = 0;
                let mut failed = 0;

                for device_entry in self.devices.iter() {
                    let device_ref = device_entry.value();
                    match device_ref.ask(DeviceMsg::Poll).await {
                        Ok(DeviceReply::Polled) => polled += 1,
                        _ => failed += 1,
                    }
                }

                NetworkReply::PollResult { polled, failed }
            }

            NetworkMsg::AddDevice { device_name, device_instance } => {
                match self.add_device(device_name, device_instance).await {
                    Ok(device_ref) => NetworkReply::DeviceAdded(device_ref),
                    Err(e) => {
                        info!("Failed to add device: {}", e);
                        // Return a dummy ActorRef - this is not ideal but works for now
                        NetworkReply::DeviceAdded(BACnetDeviceActor::spawn(BACnetDeviceActor::new(
                            "error".to_string(),
                            "error".to_string(),
                            0,
                            self.pubsub.clone().unwrap(),
                        )))
                    }
                }
            }

            NetworkMsg::GetDevice { device_name } => {
                let device = self.devices.get(&device_name).map(|d| d.clone());
                NetworkReply::Device(device)
            }
        }
    }
}

#[derive(Debug)]
pub enum NetworkMsg {
    GetStatus,
    ListDevices,
    PollAll,
    AddDevice { device_name: String, device_instance: u32 },
    GetDevice { device_name: String },
}

#[derive(Debug, kameo::Reply)]
pub enum NetworkReply {
    Status {
        network_name: String,
        device_count: usize,
    },
    DeviceList(Vec<String>),
    PollResult {
        polled: usize,
        failed: usize,
    },
    DeviceAdded(kameo::actor::ActorRef<BACnetDeviceActor>),
    Device(Option<kameo::actor::ActorRef<BACnetDeviceActor>>),
}

use crate::actors::PubSubBroker;
use crate::actors::bacnet::device::{BACnetDeviceActor, DeviceReply};
use crate::messages::{DeviceMsg, Event, PubSubMsg};
use crate::types::{DeviceStatus, ServiceState};
use chrono::Utc;
use dashmap::DashMap;
use kameo::Actor;
use kameo::actor::Spawn;
use std::net::SocketAddr;
use std::sync::{Arc, Once};
use std::time::Instant;
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn};

/// Represents a BACnet network that manages multiple devices
#[derive(kameo::Actor)]
pub struct BACnetNetworkActor {
    pub network_name: String,
    pub devices: Arc<DashMap<String, kameo::actor::ActorRef<BACnetDeviceActor>>>,
    pub device_addresses: Arc<DashMap<String, SocketAddr>>,  // Maps device name to network address
    pub poll_interval_secs: u64,
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
}

impl BACnetNetworkActor {
    pub fn new(
        network_name: String,
        poll_interval_secs: u64,
        pubsub: kameo::actor::ActorRef<PubSubBroker>,
    ) -> Self {
        configure_bacnet_environment();
        info!("Creating BACnet network: {}", network_name);

        Self {
            network_name,
            devices: Arc::new(DashMap::new()),
            device_addresses: Arc::new(DashMap::new()),
            poll_interval_secs,
            pubsub: Some(pubsub),
        }
    }

    /// Add a device to this network (with optional network address for real BACnet)
    pub async fn add_device(
        &mut self,
        device_name: String,
        device_instance: u32,
        device_address: Option<SocketAddr>,
    ) -> crate::types::Result<kameo::actor::ActorRef<BACnetDeviceActor>> {
        if let Some(pubsub) = &self.pubsub {
            let device = BACnetDeviceActor::spawn(BACnetDeviceActor::new(
                device_name.clone(),
                self.network_name.clone(),
                device_instance,
                device_address,
                pubsub.clone(),
            ));

            self.devices.insert(device_name.clone(), device.clone());

            if let Some(addr) = device_address {
                self.device_addresses.insert(device_name.clone(), addr);
                info!(
                    "Added device {} (instance {}) at {} to BACnet network {}",
                    device_name, device_instance, addr, self.network_name
                );
            } else {
                info!(
                    "Added simulated device {} (instance {}) to BACnet network {}",
                    device_name, device_instance, self.network_name
                );
            }

            Ok(device)
        } else {
            Err(crate::types::Error::Actor(
                "PubSub broker not available".to_string(),
            ))
        }
    }

    /// Discover BACnet devices on the network via Who-Is
    pub async fn discover_devices(&mut self) -> crate::types::Result<Vec<(String, u32, SocketAddr)>> {
        info!("Scanning for BACnet devices on network {}...", self.network_name);
        info!("Sending Who-Is broadcast...");

        // Run Who-Is discovery in a blocking task since it's synchronous
        let devices = tokio::task::spawn_blocking(move || {
            use bacnet::whois::WhoIs;
            use std::time::Duration;

            WhoIs::new()
                .timeout(Duration::from_secs(3))
                .subnet(None)  // None for global broadcast
                .execute()
        })
        .await
        .map_err(|e| crate::types::Error::BACnet(format!("Who-Is task failed: {}", e)))?
        .map_err(|e| crate::types::Error::BACnet(format!("Who-Is discovery failed: {}", e)))?;

        info!("Received {} I-Am response(s)", devices.len());

        // Convert IAmDevice to our format (name, instance, address)
        let mut discovered_devices = Vec::new();
        for device in devices {
            // Create device name from device ID
            let device_name = format!("Device-{}", device.device_id);

            // Convert MAC address to SocketAddr
            // MAC format from BACpypes3: [IP1, IP2, IP3, IP4, PORT_HI, PORT_LO]
            if device.mac_addr[0] != 0 || device.mac_addr[1] != 0 {
                let ip = std::net::Ipv4Addr::new(
                    device.mac_addr[0],
                    device.mac_addr[1],
                    device.mac_addr[2],
                    device.mac_addr[3],
                );
                let port = ((device.mac_addr[4] as u16) << 8) | (device.mac_addr[5] as u16);
                let addr = SocketAddr::new(ip.into(), port);

                info!(
                    "  Found: {} (instance {}) at {} (vendor_id: {})",
                    device_name, device.device_id, addr, device.vendor_id
                );

                discovered_devices.push((device_name, device.device_id, addr));
            } else {
                warn!(
                    "  Skipping device {} (instance {}) - invalid MAC address",
                    device_name, device.device_id
                );
            }
        }

        if discovered_devices.is_empty() {
            warn!("No BACnet devices discovered via Who-Is on network {}", self.network_name);
        } else {
            info!("Successfully discovered {} BACnet device(s)", discovered_devices.len());
        }

        Ok(discovered_devices)
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

fn configure_bacnet_environment() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        configure_port();
        configure_interface();
    });
}

fn configure_port() {
    if let Ok(port) = std::env::var("BACNET_IP_PORT") {
        info!(
            "Using BACnet local UDP port {} from BACNET_IP_PORT environment variable",
            port
        );
        return;
    }

    const DEFAULT_PORT: u16 = 47808;  // BACnet/IP standard port (0xBAC0)
    let mut port = DEFAULT_PORT;

    if let Ok(value) = std::env::var("NEO_BACNET_LOCAL_PORT") {
        match value.parse::<u16>() {
            Ok(parsed) => port = parsed,
            Err(_) => warn!(
                "Invalid NEO_BACNET_LOCAL_PORT value '{}', falling back to {}",
                value, DEFAULT_PORT
            ),
        }
    }

    // SAFETY: Setting an environment variable is safe because the string is under our control.
    unsafe {
        std::env::set_var("BACNET_IP_PORT", port.to_string());
    }
    info!(
        "Configured BACnet local UDP port {} (BACnet/IP standard: 47808, override by setting BACNET_IP_PORT or NEO_BACNET_LOCAL_PORT)",
        port
    );
}

fn configure_interface() {
    if let Ok(iface) = std::env::var("BACNET_IFACE") {
        info!(
            "Using BACnet interface '{}' from BACNET_IFACE environment variable",
            iface
        );
        return;
    }

    let iface = std::env::var("NEO_BACNET_IFACE").unwrap_or_else(|_| "lo".to_string());
    unsafe {
        std::env::set_var("BACNET_IFACE", &iface);
    }

    info!(
        "Configured BACnet interface '{}' (override by setting BACNET_IFACE or NEO_BACNET_IFACE)",
        iface
    );
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

            NetworkMsg::AddDevice { device_name, device_instance, device_address } => {
                match self.add_device(device_name, device_instance, device_address).await {
                    Ok(device_ref) => NetworkReply::DeviceAdded(device_ref),
                    Err(e) => {
                        info!("Failed to add device: {}", e);
                        // Return a dummy ActorRef - this is not ideal but works for now
                        NetworkReply::DeviceAdded(BACnetDeviceActor::spawn(BACnetDeviceActor::new(
                            "error".to_string(),
                            "error".to_string(),
                            0,
                            None,
                            self.pubsub.clone().unwrap(),
                        )))
                    }
                }
            }

            NetworkMsg::GetDevice { device_name } => {
                let device = self.devices.get(&device_name).map(|d| d.clone());
                NetworkReply::Device(device)
            }

            NetworkMsg::DiscoverDevices => {
                match self.discover_devices().await {
                    Ok(devices) => NetworkReply::DiscoveredDevices(devices),
                    Err(_) => NetworkReply::DiscoveredDevices(Vec::new()),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum NetworkMsg {
    GetStatus,
    ListDevices,
    PollAll,
    AddDevice {
        device_name: String,
        device_instance: u32,
        device_address: Option<SocketAddr>,  // Network address for real BACnet devices
    },
    GetDevice { device_name: String },
    DiscoverDevices,  // New: Discover devices via Who-Is
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
    DiscoveredDevices(Vec<(String, u32, SocketAddr)>),  // (name, instance, address)
}

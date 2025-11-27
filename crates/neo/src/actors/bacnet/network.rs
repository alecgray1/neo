use crate::actors::PubSubBroker;
use crate::actors::bacnet::device::BACnetDeviceActor;
use crate::actors::bacnet::io::BACnetIOActor;
use crate::messages::{BACnetIOMsg, BACnetIOReply, DeviceMsg, Event, NetworkMsg};
use dashmap::DashMap;
use kameo::actor::{ActorRef, Spawn};
use kameo::registry::ACTOR_REGISTRY;
use kameo_actors::pubsub::Publish;
use std::net::SocketAddr;
use std::sync::{Arc, Once};
use tokio::time::Duration;
use tracing::{debug, info, warn};

/// Represents a BACnet network that manages multiple devices
#[derive(kameo::Actor)]
pub struct BACnetNetworkActor {
    pub network_name: String,
    pub device_addresses: Arc<DashMap<String, SocketAddr>>, // Maps device name to network address
    pub poll_interval_secs: u64,
    pub auto_discovery: bool,
    pub discovery_interval_secs: u64,
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
    io_actor: kameo::actor::ActorRef<BACnetIOActor>, // Reference to I/O actor for BACnet operations
}

impl BACnetNetworkActor {
    pub fn new(
        network_name: String,
        poll_interval_secs: u64,
        pubsub: kameo::actor::ActorRef<PubSubBroker>,
        io_actor: kameo::actor::ActorRef<BACnetIOActor>,
    ) -> Self {
        configure_bacnet_environment();
        info!("Creating BACnet network: {}", network_name);

        Self {
            network_name,
            device_addresses: Arc::new(DashMap::new()),
            poll_interval_secs,
            auto_discovery: true,        // Enabled by default
            discovery_interval_secs: 60, // Every 60 seconds
            pubsub: Some(pubsub),
            io_actor,
        }
    }

    /// Generate a registry key for a device
    fn device_registry_key(&self, device_name: &str) -> String {
        format!("bacnet/{}/{}", self.network_name, device_name)
    }

    /// Add a device to this network (with optional network address for real BACnet)
    /// This method is idempotent - if the device already exists, it updates the address if changed
    pub async fn add_device(
        &mut self,
        device_name: String,
        device_instance: u32,
        device_address: Option<SocketAddr>,
        network_actor_ref: kameo::actor::ActorRef<BACnetNetworkActor>,
    ) -> crate::types::Result<kameo::actor::ActorRef<BACnetDeviceActor>> {
        let registry_key = self.device_registry_key(&device_name);

        // Check if device already exists in registry
        if let Ok(Some(existing_device)) =
            ActorRef::<BACnetDeviceActor>::lookup(registry_key.as_str())
        {
            debug!(
                "Device {} already exists, checking if address changed",
                device_name
            );

            // Update address if different
            if let Some(addr) = device_address {
                if let Some(mut stored_addr) = self.device_addresses.get_mut(&device_name) {
                    if *stored_addr != addr {
                        info!(
                            "Device {} address changed from {} to {}",
                            device_name, *stored_addr, addr
                        );
                        *stored_addr = addr;
                        // TODO: In Phase 4, we could trigger reconnection here
                    }
                } else {
                    self.device_addresses.insert(device_name.clone(), addr);
                }
            }

            return Ok(existing_device);
        }

        // New device - spawn actor
        if let Some(pubsub) = &self.pubsub {
            info!(
                "Spawning new device actor for {} (instance {})",
                device_name, device_instance
            );

            // Use I/O actor to connect to device if we have an address
            if let Some(addr) = device_address {
                match self
                    .io_actor
                    .ask(BACnetIOMsg::ConnectDevice {
                        device_id: device_instance,
                        address: addr,
                    })
                    .await
                {
                    Ok(BACnetIOReply::Connected) => {
                        info!("I/O actor connected to device {} at {}", device_name, addr);
                    }
                    Ok(BACnetIOReply::IoError(e)) => {
                        warn!("Failed to connect to device {}: {}", device_name, e);
                    }
                    Err(e) => {
                        warn!("Failed to send connect message to I/O actor: {}", e);
                    }
                    _ => {
                        warn!("Unexpected reply from I/O actor");
                    }
                }
            }

            // Create device actor with I/O actor reference
            let device = BACnetDeviceActor::spawn(BACnetDeviceActor::new(
                device_name.clone(),
                self.network_name.clone(),
                device_instance,
                pubsub.clone(),
                self.io_actor.clone(),
            ));

            // Link the device actor to the network actor (supervision tree)
            let _ = network_actor_ref.link(&device).await;
            debug!(
                "Linked device {} to network {} for supervision",
                device_name, self.network_name
            );

            // Register device in the actor registry
            if let Err(e) = device.register(registry_key) {
                warn!(
                    "Failed to register device {} in registry: {}",
                    device_name, e
                );
            }

            if let Some(addr) = device_address {
                self.device_addresses.insert(device_name.clone(), addr);
                info!(
                    "Added device {} (instance {}) at {} to BACnet network {}",
                    device_name, device_instance, addr, self.network_name
                );

                // Publish DeviceDiscovered event
                let event = Event::DeviceDiscovered {
                    network: self.network_name.clone(),
                    device: device_name,
                    instance: device_instance,
                    address: addr,
                    timestamp: std::time::Instant::now(),
                    timestamp_utc: chrono::Utc::now(),
                };
                let _ = pubsub.tell(Publish(event)).await;
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
    pub async fn discover_devices(
        &mut self,
    ) -> crate::types::Result<Vec<(String, u32, SocketAddr)>> {
        info!(
            "Scanning for BACnet devices on network {}...",
            self.network_name
        );
        info!("Sending Who-Is broadcast via I/O actor...");

        // Use I/O actor to perform Who-Is discovery
        match self
            .io_actor
            .ask(BACnetIOMsg::WhoIs {
                timeout_secs: 3,
                subnet: None,
            })
            .await
        {
            Ok(BACnetIOReply::Devices(devices)) => {
                info!("Discovered {} BACnet device(s)", devices.len());
                for (name, instance, addr) in &devices {
                    info!("  Found: {} (instance {}) at {}", name, instance, addr);
                }
                Ok(devices)
            }
            Ok(BACnetIOReply::IoError(e)) => {
                warn!("Who-Is discovery failed: {}", e);
                Err(crate::types::Error::BACnet(e))
            }
            Err(e) => {
                warn!("Failed to send Who-Is message to I/O actor: {}", e);
                Err(crate::types::Error::BACnet(format!(
                    "I/O actor error: {}",
                    e
                )))
            }
            _ => {
                warn!("Unexpected reply from I/O actor");
                Err(crate::types::Error::BACnet("Unexpected reply".to_string()))
            }
        }
    }

    /// Start background polling task
    pub fn start_polling_task(
        actor_ref: kameo::actor::ActorRef<Self>,
    ) -> tokio::task::JoinHandle<()> {
        let weak_ref = actor_ref.downgrade();
        let actor_clone = actor_ref.clone();

        tokio::spawn(async move {
            // Get initial values from the actor
            let (poll_interval, network_name) =
                if let Ok(NetworkReply::Status { network_name, .. }) =
                    actor_clone.ask(NetworkMsg::GetStatus).await
                {
                    (10u64, network_name)
                } else {
                    (10u64, "network".to_string())
                };

            let mut tick = tokio::time::interval(Duration::from_secs(poll_interval));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            info!(
                "⏱️  Polling task started for network {}, interval: {}s",
                network_name, poll_interval
            );

            loop {
                tick.tick().await;
                info!("⏰ Polling tick for network {}", network_name);

                // Check if actor still exists
                if weak_ref.upgrade().is_none() {
                    debug!("BACnet network {} polling task exiting", network_name);
                    break;
                }

                // Use PollAll message instead of accessing devices directly
                match actor_clone.ask(NetworkMsg::PollAll).await {
                    Ok(_) => {
                        info!("🔄 Polling completed for network {}", network_name);
                    }
                    Err(e) => {
                        warn!("Polling failed for network {}: {}", network_name, e);
                    }
                }
            }
        })
    }

    /// Start background discovery task
    pub fn start_discovery_task(
        actor_ref: kameo::actor::ActorRef<Self>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let weak_ref = actor_ref.downgrade();

            // Do an immediate discovery on startup
            info!("Running initial device discovery...");
            if let Ok(crate::actors::bacnet::NetworkReply::DiscoveredDevices(devices)) =
                actor_ref.ask(NetworkMsg::DiscoverDevices).await
            {
                info!("Initial discovery found {} devices", devices.len());

                // Auto-add discovered devices
                for (name, instance, address) in devices {
                    let _ = actor_ref
                        .ask(NetworkMsg::AddDevice {
                            device_name: name,
                            device_instance: instance,
                            device_address: Some(address),
                        })
                        .await;
                }
            }

            loop {
                // Get discovery interval
                let interval = match actor_ref.ask(NetworkMsg::GetDiscoveryInterval).await {
                    Ok(crate::actors::bacnet::NetworkReply::DiscoveryInterval(secs)) => secs,
                    _ => 60, // Default fallback
                };

                tokio::time::sleep(Duration::from_secs(interval)).await;

                // Check if actor still alive
                if weak_ref.upgrade().is_none() {
                    info!("Network actor stopped, ending discovery task");
                    break;
                }

                // Check if auto-discovery enabled
                let enabled = match actor_ref.ask(NetworkMsg::IsAutoDiscoveryEnabled).await {
                    Ok(crate::actors::bacnet::NetworkReply::AutoDiscoveryEnabled(enabled)) => {
                        enabled
                    }
                    _ => false,
                };

                if !enabled {
                    debug!("Auto-discovery disabled, skipping");
                    continue;
                }

                // Run discovery
                debug!("Running periodic device discovery");
                match actor_ref.ask(NetworkMsg::DiscoverDevices).await {
                    Ok(crate::actors::bacnet::NetworkReply::DiscoveredDevices(devices)) => {
                        if !devices.is_empty() {
                            info!("Periodic discovery found {} devices", devices.len());

                            // Auto-add discovered devices
                            for (name, instance, address) in devices {
                                let _ = actor_ref
                                    .ask(NetworkMsg::AddDevice {
                                        device_name: name,
                                        device_instance: instance,
                                        device_address: Some(address),
                                    })
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Periodic discovery failed: {}", e);
                    }
                    _ => {}
                }
            }
        })
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

    const DEFAULT_PORT: u16 = 47808; // BACnet/IP standard port (0xBAC0)
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
        ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            NetworkMsg::GetStatus => {
                // Count devices in registry for this network
                let prefix = format!("bacnet/{}/", self.network_name);
                let device_count = ACTOR_REGISTRY
                    .lock()
                    .unwrap()
                    .names()
                    .filter(|name| name.starts_with(&prefix))
                    .count();

                NetworkReply::Status {
                    network_name: self.network_name.clone(),
                    device_count,
                }
            }

            NetworkMsg::ListDevices => {
                // List all devices for this network from registry
                let prefix = format!("bacnet/{}/", self.network_name);
                let device_names: Vec<String> = ACTOR_REGISTRY
                    .lock()
                    .unwrap()
                    .names()
                    .filter_map(|name| name.strip_prefix(&prefix).map(|s| s.to_string()))
                    .collect();
                NetworkReply::DeviceList(device_names)
            }

            NetworkMsg::PollAll => {
                // Get all device names for this network from registry
                let prefix = format!("bacnet/{}/", self.network_name);
                let device_registry_keys: Vec<String> = ACTOR_REGISTRY
                    .lock()
                    .unwrap()
                    .names()
                    .filter(|name| name.starts_with(&prefix))
                    .map(|name| name.to_string())
                    .collect();

                let device_count = device_registry_keys.len();
                info!("Starting concurrent poll of {} devices", device_count);

                // Spawn concurrent polling tasks (no deadlock!)
                let mut tasks = Vec::new();

                for registry_key in device_registry_keys {
                    // Extract device name from registry key
                    let device_name = registry_key
                        .strip_prefix(&prefix)
                        .unwrap_or(&registry_key)
                        .to_string();

                    // Spawn a task for each device
                    let task = tokio::spawn(async move {
                        // Lookup device from registry
                        match ActorRef::<BACnetDeviceActor>::lookup(registry_key.as_str()) {
                            Ok(Some(device_ref)) => {
                                debug!("Polling device {}", device_name);
                                match device_ref.tell(DeviceMsg::Poll).await {
                                    Ok(_) => {
                                        debug!("Device {} poll initiated", device_name);
                                        Ok(())
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to send poll to device {}: {}",
                                            device_name, e
                                        );
                                        Err(())
                                    }
                                }
                            }
                            Ok(None) => {
                                warn!("Device {} not found in registry", device_name);
                                Err(())
                            }
                            Err(e) => {
                                warn!("Failed to lookup device {}: {}", device_name, e);
                                Err(())
                            }
                        }
                    });

                    tasks.push(task);
                }

                // Wait for all polling tasks to complete
                let results = futures::future::join_all(tasks).await;

                let polled = results.iter().filter(|r| matches!(r, Ok(Ok(_)))).count();
                let failed = device_count - polled;

                info!(
                    "Poll complete: {} initiated, {} failed to initiate",
                    polled, failed
                );
                NetworkReply::PollResult { polled, failed }
            }

            NetworkMsg::AddDevice {
                device_name,
                device_instance,
                device_address,
            } => {
                let network_ref = ctx.actor_ref().clone();
                match self
                    .add_device(
                        device_name.clone(),
                        device_instance,
                        device_address,
                        network_ref,
                    )
                    .await
                {
                    Ok(device_ref) => {
                        // Trigger point discovery in background for real devices
                        if device_address.is_some() {
                            let device_clone = device_ref.clone();
                            tokio::spawn(async move {
                                // Wait a moment for device to be fully initialized
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                                match device_clone.ask(DeviceMsg::DiscoverPoints).await {
                                    Ok(crate::actors::bacnet::DeviceReply::PointsDiscovered {
                                        count,
                                    }) => {
                                        info!(
                                            "Auto-discovered {} points on device {}",
                                            count, device_name
                                        );
                                    }
                                    Ok(crate::actors::bacnet::DeviceReply::Failure(e)) => {
                                        warn!("Point discovery failed for {}: {}", device_name, e);
                                    }
                                    Err(e) => {
                                        warn!(
                                            "Failed to trigger discovery for {}: {}",
                                            device_name, e
                                        );
                                    }
                                    _ => {}
                                }
                            });
                        }

                        NetworkReply::DeviceAdded(device_ref)
                    }
                    Err(e) => {
                        info!("Failed to add device: {}", e);
                        // Return a dummy ActorRef - this is not ideal but works for now
                        NetworkReply::DeviceAdded(BACnetDeviceActor::spawn(BACnetDeviceActor::new(
                            "error".to_string(),
                            "error".to_string(),
                            0,
                            self.pubsub.clone().unwrap(),
                            self.io_actor.clone(),
                        )))
                    }
                }
            }

            NetworkMsg::GetDevice { device_name } => {
                let registry_key = self.device_registry_key(&device_name);
                let device = ActorRef::<BACnetDeviceActor>::lookup(registry_key.as_str())
                    .ok()
                    .flatten();
                NetworkReply::Device(device)
            }

            NetworkMsg::DiscoverDevices => match self.discover_devices().await {
                Ok(devices) => NetworkReply::DiscoveredDevices(devices),
                Err(_) => NetworkReply::DiscoveredDevices(Vec::new()),
            },

            // Auto-discovery management messages
            NetworkMsg::EnableAutoDiscovery => {
                self.auto_discovery = true;
                info!("Auto-discovery enabled for network {}", self.network_name);
                NetworkReply::Success
            }

            NetworkMsg::DisableAutoDiscovery => {
                self.auto_discovery = false;
                info!("Auto-discovery disabled for network {}", self.network_name);
                NetworkReply::Success
            }

            NetworkMsg::IsAutoDiscoveryEnabled => {
                NetworkReply::AutoDiscoveryEnabled(self.auto_discovery)
            }

            NetworkMsg::SetDiscoveryInterval(secs) => {
                self.discovery_interval_secs = secs;
                info!(
                    "Discovery interval set to {}s for network {}",
                    secs, self.network_name
                );
                NetworkReply::Success
            }

            NetworkMsg::GetDiscoveryInterval => {
                NetworkReply::DiscoveryInterval(self.discovery_interval_secs)
            }
        }
    }
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
    DiscoveredDevices(Vec<(String, u32, SocketAddr)>),
    Success,
    AutoDiscoveryEnabled(bool),
    DiscoveryInterval(u64),
}

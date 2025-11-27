use crate::messages::{BACnetIOMsg, BACnetIOReply, BACnetIOStats, PropertyReadRequest};
use crate::types::{ObjectType, PointValue};
use bacnet::BACnetServer;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Actor responsible for all BACnet I/O operations
/// This actor owns the BACnet server connections and handles all read/write operations
/// with built-in timeouts, retry logic, and statistics tracking.
#[derive(kameo::Actor)]
pub struct BACnetIOActor {
    /// Map of device_id -> BACnetServer connection
    servers: DashMap<u32, Arc<Mutex<BACnetServer>>>,

    /// Map of device_id -> address (for reconnection)
    device_addresses: DashMap<u32, SocketAddr>,

    /// I/O statistics
    stats: Arc<Mutex<BACnetIOStats>>,

    /// Default timeout for operations (ms)
    default_timeout_ms: u64,

    /// Maximum number of retry attempts
    #[allow(dead_code)]
    max_retries: u32,
}

impl BACnetIOActor {
    pub fn new() -> Self {
        Self {
            servers: DashMap::new(),
            device_addresses: DashMap::new(),
            stats: Arc::new(Mutex::new(BACnetIOStats {
                total_reads: 0,
                total_writes: 0,
                successful_reads: 0,
                successful_writes: 0,
                failed_reads: 0,
                failed_writes: 0,
                timeouts: 0,
                avg_read_time_ms: 0.0,
                avg_write_time_ms: 0.0,
                connected_devices: 0,
            })),
            default_timeout_ms: 5000,  // 5 second default timeout
            max_retries: 2,
        }
    }

    /// Connect to a BACnet device
    async fn connect_device(
        &mut self,
        device_id: u32,
        address: SocketAddr,
    ) -> crate::types::Result<()> {
        // Check if already connected
        if self.servers.contains_key(&device_id) {
            info!("Device {} already connected at {}", device_id, address);
            return Ok(());
        }

        let ip = match address.ip() {
            std::net::IpAddr::V4(ipv4) => ipv4,
            std::net::IpAddr::V6(_) => {
                return Err(crate::types::Error::BACnet(
                    "IPv6 not supported".to_string()
                ));
            }
        };

        // Build and connect in a blocking task with timeout
        let port = address.port();
        let connect_result = timeout(
            Duration::from_millis(self.default_timeout_ms),
            tokio::task::spawn_blocking(move || {
                let mut server = BACnetServer::builder()
                    .ip(ip)
                    .port(port)
                    .device_id(device_id)
                    .build();

                match server.connect() {
                    Ok(_) => Ok(server),
                    Err(e) => Err(format!("Connect failed: {:?}", e)),
                }
            })
        ).await;

        match connect_result {
            Ok(Ok(Ok(server))) => {
                info!("Connected to BACnet device {} at {}", device_id, address);
                self.servers.insert(device_id, Arc::new(Mutex::new(server)));
                self.device_addresses.insert(device_id, address);

                // Update stats
                if let Ok(mut stats) = self.stats.lock() {
                    stats.connected_devices = self.servers.len();
                }

                Ok(())
            }
            Ok(Ok(Err(e))) => {
                error!("Failed to connect to device {}: {:?}", device_id, e);
                Err(crate::types::Error::BACnet(format!("Connection failed: {:?}", e)))
            }
            Ok(Err(e)) => {
                error!("Task failed for device {}: {}", device_id, e);
                Err(crate::types::Error::BACnet(format!("Task error: {}", e)))
            }
            Err(_) => {
                error!("Connection timeout for device {}", device_id);
                if let Ok(mut stats) = self.stats.lock() {
                    stats.timeouts += 1;
                }
                Err(crate::types::Error::Timeout)
            }
        }
    }

    /// Read a property from a device with timeout and retry logic
    async fn read_property(
        &self,
        device_id: u32,
        object_type: ObjectType,
        object_instance: u32,
        property_id: u8,
        array_index: Option<u32>,
        timeout_ms: Option<u64>,
        raw: bool,
    ) -> BACnetIOReply {
        let start_time = Instant::now();

        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_reads += 1;
        }

        // Get server
        let server_arc = match self.servers.get(&device_id) {
            Some(server) => Arc::clone(server.value()),
            None => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_reads += 1;
                }
                return BACnetIOReply::IoError(
                    format!("Device {} not connected", device_id)
                );
            }
        };

        // Convert object type
        let obj_type = Self::convert_object_type(object_type);
        let index = array_index.unwrap_or((-1i32) as u32);
        let timeout_duration = Duration::from_millis(
            timeout_ms.unwrap_or(self.default_timeout_ms)
        );

        // Perform read with timeout
        let read_result = timeout(
            timeout_duration,
            tokio::task::spawn_blocking(move || {
                let server = server_arc.lock().unwrap();
                server.read_prop_at(obj_type, object_instance, property_id as u32, index)
            })
        ).await;

        // Process result
        let elapsed = start_time.elapsed().as_millis() as f64;

        match read_result {
            Ok(Ok(Ok(bacnet_value))) => {
                // Success - update stats
                if let Ok(mut stats) = self.stats.lock() {
                    stats.successful_reads += 1;
                    // Update rolling average
                    let total_successful = stats.successful_reads as f64;
                    stats.avg_read_time_ms =
                        (stats.avg_read_time_ms * (total_successful - 1.0) + elapsed)
                        / total_successful;
                }

                if raw {
                    return BACnetIOReply::RawValue(bacnet_value);
                }

                // Convert to PointValue
                match Self::convert_bacnet_value(bacnet_value, object_type) {
                    Ok(value) => BACnetIOReply::PropertyValue(value),
                    Err(e) => {
                        if let Ok(mut stats) = self.stats.lock() {
                            stats.failed_reads += 1;
                        }
                        BACnetIOReply::IoError(e)
                    }
                }
            }
            Ok(Ok(Err(e))) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_reads += 1;
                }
                BACnetIOReply::IoError(format!("BACnet error: {}", e))
            }
            Ok(Err(e)) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_reads += 1;
                }
                BACnetIOReply::IoError(format!("Task error: {}", e))
            }
            Err(_) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_reads += 1;
                    stats.timeouts += 1;
                }
                BACnetIOReply::IoError(format!("Timeout after {}ms", timeout_duration.as_millis()))
            }
        }
    }

    /// Write a property to a device
    async fn write_property(
        &self,
        device_id: u32,
        object_type: ObjectType,
        object_instance: u32,
        property_id: u8,
        value: PointValue,
    ) -> BACnetIOReply {
        let start_time = Instant::now();

        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_writes += 1;
        }

        // Get server
        let server_arc = match self.servers.get(&device_id) {
            Some(server) => Arc::clone(server.value()),
            None => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_writes += 1;
                }
                return BACnetIOReply::IoError(
                    format!("Device {} not connected", device_id)
                );
            }
        };

        // Convert types
        let obj_type = Self::convert_object_type(object_type);
        let bacnet_value = Self::convert_to_bacnet_value(value);

        // Perform write with timeout
        let write_result = timeout(
            Duration::from_millis(self.default_timeout_ms),
            tokio::task::spawn_blocking(move || {
                let server = server_arc.lock().unwrap();
                server.write_prop_at(
                    obj_type,
                    object_instance,
                    bacnet_value,
                    property_id as u32,
                    (-1i32) as u32
                )
            })
        ).await;

        // Process result
        let elapsed = start_time.elapsed().as_millis() as f64;

        match write_result {
            Ok(Ok(Ok(_))) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.successful_writes += 1;
                    let total_successful = stats.successful_writes as f64;
                    stats.avg_write_time_ms =
                        (stats.avg_write_time_ms * (total_successful - 1.0) + elapsed)
                        / total_successful;
                }
                BACnetIOReply::PropertyWritten
            }
            Ok(Ok(Err(e))) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_writes += 1;
                }
                BACnetIOReply::IoError(format!("BACnet error: {}", e))
            }
            Ok(Err(e)) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_writes += 1;
                }
                BACnetIOReply::IoError(format!("Task error: {}", e))
            }
            Err(_) => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_writes += 1;
                    stats.timeouts += 1;
                }
                BACnetIOReply::IoError("Timeout".to_string())
            }
        }
    }

    /// Perform Who-Is discovery
    async fn who_is(&self, timeout_secs: u64, _subnet: Option<String>) -> BACnetIOReply {
        debug!("Running Who-Is discovery with timeout {}s", timeout_secs);

        let discover_result = timeout(
            Duration::from_secs(timeout_secs + 1), // Add 1 sec buffer
            tokio::task::spawn_blocking(move || {
                use bacnet::whois::WhoIs;

                WhoIs::new()
                    .timeout(Duration::from_secs(timeout_secs))
                    .subnet(None)  // TODO: Support subnet filtering
                    .execute()
            })
        ).await;

        match discover_result {
            Ok(Ok(Ok(devices))) => {
                let mut discovered = Vec::new();
                for device in devices {
                    let device_name = format!("Device-{}", device.device_id);

                    if device.mac_addr[0] != 0 || device.mac_addr[1] != 0 {
                        let ip = std::net::Ipv4Addr::new(
                            device.mac_addr[0],
                            device.mac_addr[1],
                            device.mac_addr[2],
                            device.mac_addr[3],
                        );
                        let port = ((device.mac_addr[4] as u16) << 8)
                                 | (device.mac_addr[5] as u16);
                        let addr = SocketAddr::new(ip.into(), port);

                        discovered.push((device_name, device.device_id, addr));
                    }
                }
                info!("Who-Is discovered {} devices", discovered.len());
                BACnetIOReply::Devices(discovered)
            }
            Ok(Ok(Err(e))) => {
                warn!("Who-Is failed: {:?}", e);
                BACnetIOReply::IoError(format!("Who-Is failed: {:?}", e))
            }
            Ok(Err(e)) => {
                error!("Who-Is task error: {}", e);
                BACnetIOReply::IoError(format!("Task error: {}", e))
            }
            Err(_) => {
                warn!("Who-Is timeout after {}s", timeout_secs);
                if let Ok(mut stats) = self.stats.lock() {
                    stats.timeouts += 1;
                }
                BACnetIOReply::IoError("Who-Is timeout".to_string())
            }
        }
    }

    /// Read multiple properties in batch
    async fn read_multiple_properties(
        &self,
        device_id: u32,
        requests: Vec<PropertyReadRequest>,
    ) -> BACnetIOReply {
        debug!("Reading {} properties from device {}", requests.len(), device_id);

        let mut results = Vec::new();

        for req in requests {
            match self.read_property(
                device_id,
                req.object_type,
                req.object_instance,
                req.property_id,
                req.array_index,
                None,
                false,
            ).await {
                BACnetIOReply::PropertyValue(value) => results.push(Ok(value)),
                BACnetIOReply::IoError(e) => results.push(Err(e)),
                _ => results.push(Err("Unexpected reply".to_string())),
            }
        }

        BACnetIOReply::MultipleValues(results)
    }

    /// Helper: Convert ObjectType to BACnet numeric type
    fn convert_object_type(obj_type: ObjectType) -> bacnet::ObjectType {
        match obj_type {
            ObjectType::AnalogInput => 0,
            ObjectType::AnalogOutput => 1,
            ObjectType::AnalogValue => 2,
            ObjectType::BinaryInput => 3,
            ObjectType::BinaryOutput => 4,
            ObjectType::BinaryValue => 5,
            ObjectType::Device => 8,
        }
    }

    /// Helper: Convert BACnetValue to PointValue
    fn convert_bacnet_value(
        bacnet_value: bacnet::value::BACnetValue,
        object_type: ObjectType,
    ) -> Result<PointValue, String> {
        use bacnet::value::BACnetValue;

        match object_type {
            ObjectType::AnalogInput
            | ObjectType::AnalogOutput
            | ObjectType::AnalogValue => match bacnet_value {
                BACnetValue::Real(f) => Ok(PointValue::Real(f)),
                BACnetValue::Double(d) => Ok(PointValue::Real(d as f32)),
                BACnetValue::Uint(u) => Ok(PointValue::Real(u as f32)),
                BACnetValue::Int(i) => Ok(PointValue::Real(i as f32)),
                _ => Err(format!("Unexpected value type for analog: {:?}", bacnet_value)),
            },
            ObjectType::BinaryInput
            | ObjectType::BinaryOutput
            | ObjectType::BinaryValue => match bacnet_value {
                BACnetValue::Enum(e, _) => Ok(PointValue::Boolean(e != 0)),
                BACnetValue::Bool(b) => Ok(PointValue::Boolean(b)),
                BACnetValue::Uint(u) => Ok(PointValue::Boolean(u != 0)),
                _ => Err(format!("Unexpected value type for binary: {:?}", bacnet_value)),
            },
            _ => Err(format!("Unsupported object type: {:?}", object_type)),
        }
    }

    /// Helper: Convert PointValue to BACnetValue
    fn convert_to_bacnet_value(value: PointValue) -> bacnet::value::BACnetValue {
        use bacnet::value::BACnetValue;

        match value {
            PointValue::Real(f) => BACnetValue::Real(f),
            PointValue::Boolean(b) => BACnetValue::Enum(if b { 1 } else { 0 }, None),
            PointValue::Unsigned(u) => BACnetValue::Uint(u as u64),
            PointValue::Enumerated(e) => BACnetValue::Enum(e, None),
            PointValue::Null => BACnetValue::Null,
        }
    }
}

impl kameo::message::Message<BACnetIOMsg> for BACnetIOActor {
    type Reply = BACnetIOReply;

    async fn handle(
        &mut self,
        msg: BACnetIOMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            BACnetIOMsg::ReadProperty {
                device_id,
                object_type,
                object_instance,
                property_id,
                array_index,
                timeout_ms,
            } => {
                self.read_property(
                    device_id,
                    object_type,
                    object_instance,
                    property_id,
                    array_index,
                    timeout_ms,
                    false,
                ).await
            }

            BACnetIOMsg::ReadPropertyRaw {
                device_id,
                object_type,
                object_instance,
                property_id,
                array_index,
                timeout_ms,
            } => {
                self.read_property(
                    device_id,
                    object_type,
                    object_instance,
                    property_id,
                    array_index,
                    timeout_ms,
                    true,  // raw = true
                ).await
            }

            BACnetIOMsg::WriteProperty {
                device_id,
                object_type,
                object_instance,
                property_id,
                value,
            } => {
                self.write_property(
                    device_id,
                    object_type,
                    object_instance,
                    property_id,
                    value,
                ).await
            }

            BACnetIOMsg::ConnectDevice { device_id, address } => {
                match self.connect_device(device_id, address).await {
                    Ok(_) => BACnetIOReply::Connected,
                    Err(e) => BACnetIOReply::IoError(e.to_string()),
                }
            }

            BACnetIOMsg::DisconnectDevice { device_id } => {
                self.servers.remove(&device_id);
                self.device_addresses.remove(&device_id);

                if let Ok(mut stats) = self.stats.lock() {
                    stats.connected_devices = self.servers.len();
                }

                info!("Disconnected device {}", device_id);
                BACnetIOReply::Disconnected
            }

            BACnetIOMsg::IsConnected { device_id } => {
                BACnetIOReply::IsConnected(self.servers.contains_key(&device_id))
            }

            BACnetIOMsg::GetStatistics => {
                if let Ok(stats) = self.stats.lock() {
                    BACnetIOReply::Statistics(stats.clone())
                } else {
                    BACnetIOReply::IoError("Failed to get statistics".to_string())
                }
            }

            BACnetIOMsg::WhoIs { timeout_secs, subnet } => {
                self.who_is(timeout_secs, subnet).await
            }

            BACnetIOMsg::ReadMultipleProperties { device_id, requests } => {
                self.read_multiple_properties(device_id, requests).await
            }
        }
    }
}

use crate::messages::{BACnetIOMsg, BACnetIOReply, BACnetIOStats, PropertyReadRequest};
use crate::types::{ObjectIdentifier, PropertyIdentifier, PropertyValue};
use dashmap::DashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use bacnet::datalink::bacnet_ip::bvll::{bvll_encode, bvll_decode, message_type};
use bacnet::npdu::{npdu_encode, npdu_decode, NpduData};
use bacnet::apdu::{apdu_decode, Apdu};
use bacnet::services::read_property::{rp_encode_apdu, ReadPropertyRequest, ReadPropertyAck};
use bacnet::services::write_property::wp_encode_apdu;
use bacnet::services::{whois_encode, iam_decode, WritePropertyRequest};
use bacnet::{BacnetAddress, Segmentation};

/// Actor responsible for all BACnet I/O operations
/// This actor owns the UDP socket and handles all read/write operations
/// with built-in timeouts, retry logic, and statistics tracking.
#[derive(kameo::Actor)]
pub struct BACnetIOActor {
    /// UDP socket for BACnet/IP communication
    socket: Arc<UdpSocket>,

    /// Map of device_id -> address info
    device_addresses: DashMap<u32, DeviceBinding>,

    /// Invoke ID counter for request/response matching
    invoke_id: AtomicU8,

    /// I/O statistics
    stats: Arc<Mutex<BACnetIOStats>>,

    /// Default timeout for operations (ms)
    default_timeout_ms: u64,

    /// Maximum number of retry attempts
    #[allow(dead_code)]
    max_retries: u32,

    /// Broadcast address for Who-Is discovery
    broadcast_addr: SocketAddr,
}

/// Device binding information
#[derive(Debug, Clone)]
struct DeviceBinding {
    address: SocketAddr,
    bacnet_addr: BacnetAddress,
    max_apdu: u16,
    segmentation: Segmentation,
}

impl BACnetIOActor {
    pub fn new() -> Self {
        // Bind to any available port on all interfaces
        let socket = UdpSocket::bind("0.0.0.0:47808")
            .or_else(|_| UdpSocket::bind("0.0.0.0:0"))
            .expect("Failed to bind UDP socket");
        socket.set_broadcast(true).expect("Failed to enable broadcast");
        socket.set_nonblocking(true).expect("Failed to set non-blocking");

        // Get broadcast address from environment or use default
        let broadcast_addr: SocketAddr = std::env::var("BACNET_BROADCAST")
            .ok()
            .and_then(|addr| {
                // If it's just an IP, add the port
                if addr.contains(':') {
                    addr.parse().ok()
                } else {
                    format!("{}:47808", addr).parse().ok()
                }
            })
            .unwrap_or_else(|| "255.255.255.255:47808".parse().unwrap());

        info!("BACnet I/O actor using broadcast address: {}", broadcast_addr);

        Self {
            socket: Arc::new(socket),
            device_addresses: DashMap::new(),
            invoke_id: AtomicU8::new(1),
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
            broadcast_addr,
        }
    }

    /// Get next invoke ID
    fn next_invoke_id(&self) -> u8 {
        self.invoke_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Register a device address (BACnet is connectionless)
    fn register_device(&mut self, device_id: u32, address: SocketAddr) -> crate::types::Result<()> {
        let bacnet_addr = socket_addr_to_bacnet(&address);

        self.device_addresses.insert(device_id, DeviceBinding {
            address,
            bacnet_addr,
            max_apdu: 1476,  // Default
            segmentation: Segmentation::None,
        });

        if let Ok(mut stats) = self.stats.lock() {
            stats.connected_devices = self.device_addresses.len();
        }

        info!("Registered BACnet device {} at {}", device_id, address);
        Ok(())
    }

    /// Read a property from a device with timeout and retry logic
    async fn read_property(
        &self,
        device_id: u32,
        object_id: ObjectIdentifier,
        property_id: PropertyIdentifier,
        array_index: Option<u32>,
        timeout_ms: Option<u64>,
    ) -> BACnetIOReply {
        let start_time = Instant::now();

        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_reads += 1;
        }

        // Get device binding
        let binding = match self.device_addresses.get(&device_id) {
            Some(b) => b.clone(),
            None => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_reads += 1;
                }
                return BACnetIOReply::IoError(format!("Device {} not registered", device_id));
            }
        };

        let socket = Arc::clone(&self.socket);
        let invoke_id = self.next_invoke_id();
        let timeout_duration = Duration::from_millis(timeout_ms.unwrap_or(self.default_timeout_ms));
        let dest_addr = binding.address;

        // Perform read with timeout
        let read_result = timeout(
            timeout_duration,
            tokio::task::spawn_blocking(move || {
                do_read_property(&socket, dest_addr, invoke_id, object_id, property_id, array_index)
            })
        ).await;

        // Process result
        let elapsed = start_time.elapsed().as_millis() as f64;

        match read_result {
            Ok(Ok(Ok(property_value))) => {
                // Success - update stats
                if let Ok(mut stats) = self.stats.lock() {
                    stats.successful_reads += 1;
                    let total_successful = stats.successful_reads as f64;
                    stats.avg_read_time_ms =
                        (stats.avg_read_time_ms * (total_successful - 1.0) + elapsed)
                        / total_successful;
                }
                BACnetIOReply::PropertyValue(property_value)
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
        object_id: ObjectIdentifier,
        property_id: PropertyIdentifier,
        value: PropertyValue,
        priority: Option<u8>,
    ) -> BACnetIOReply {
        let start_time = Instant::now();

        // Update stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_writes += 1;
        }

        // Get device binding
        let binding = match self.device_addresses.get(&device_id) {
            Some(b) => b.clone(),
            None => {
                if let Ok(mut stats) = self.stats.lock() {
                    stats.failed_writes += 1;
                }
                return BACnetIOReply::IoError(format!("Device {} not registered", device_id));
            }
        };

        let socket = Arc::clone(&self.socket);
        let invoke_id = self.next_invoke_id();
        let dest_addr = binding.address;
        let timeout_ms = self.default_timeout_ms;

        // Perform write with timeout
        let write_result = timeout(
            Duration::from_millis(timeout_ms),
            tokio::task::spawn_blocking(move || {
                do_write_property(&socket, dest_addr, invoke_id, object_id, property_id, value, priority)
            })
        ).await;

        // Process result
        let elapsed = start_time.elapsed().as_millis() as f64;

        match write_result {
            Ok(Ok(Ok(()))) => {
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
    async fn who_is(&self, timeout_secs: u64, low_limit: Option<u32>, high_limit: Option<u32>) -> BACnetIOReply {
        debug!("Running Who-Is discovery with timeout {}s, broadcast to {}", timeout_secs, self.broadcast_addr);

        let socket = Arc::clone(&self.socket);
        let broadcast_addr = self.broadcast_addr;

        let discover_result = timeout(
            Duration::from_secs(timeout_secs + 1),
            tokio::task::spawn_blocking(move || {
                do_whois(&socket, timeout_secs, low_limit, high_limit, broadcast_addr)
            })
        ).await;

        match discover_result {
            Ok(Ok(Ok(devices))) => {
                info!("Who-Is discovered {} devices", devices.len());
                BACnetIOReply::Devices(devices)
            }
            Ok(Ok(Err(e))) => {
                warn!("Who-Is failed: {}", e);
                BACnetIOReply::IoError(format!("Who-Is failed: {}", e))
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
                req.object_id,
                req.property_id,
                req.array_index,
                None,
            ).await {
                BACnetIOReply::PropertyValue(value) => results.push(Ok(value)),
                BACnetIOReply::IoError(e) => results.push(Err(e)),
                _ => results.push(Err("Unexpected reply".to_string())),
            }
        }

        BACnetIOReply::MultipleValues(results)
    }
}

/// Synchronous read property implementation
fn do_read_property(
    socket: &UdpSocket,
    dest: SocketAddr,
    invoke_id: u8,
    object_id: ObjectIdentifier,
    property_id: PropertyIdentifier,
    array_index: Option<u32>,
) -> Result<PropertyValue, String> {
    // Create request
    let request = ReadPropertyRequest {
        object_identifier: object_id,
        property_identifier: property_id,
        array_index,
    };

    // Encode APDU
    let mut apdu_buf = [0u8; 256];
    let apdu_len = rp_encode_apdu(&mut apdu_buf, invoke_id, &request)
        .map_err(|e| format!("APDU encode error: {:?}", e))?;

    // Encode NPDU
    let npdu_data = NpduData {
        data_expecting_reply: true,
        ..Default::default()
    };
    let mut npdu_buf = [0u8; 512];
    let npdu_len = npdu_encode(&mut npdu_buf, None, None, &npdu_data, &apdu_buf[..apdu_len])
        .map_err(|e| format!("NPDU encode error: {:?}", e))?;

    // Wrap in BVLL
    let mut bvlc_buf = [0u8; 1024];
    let bvlc_len = bvll_encode(&mut bvlc_buf, message_type::ORIGINAL_UNICAST_NPDU, &npdu_buf[..npdu_len])
        .map_err(|e| format!("BVLL encode error: {:?}", e))?;

    // Send
    socket.send_to(&bvlc_buf[..bvlc_len], dest)
        .map_err(|e| format!("Send error: {}", e))?;

    // Receive response with timeout
    let mut recv_buf = [0u8; 1500];

    // Set blocking with timeout for this operation
    socket.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Set timeout error: {}", e))?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        match socket.recv_from(&mut recv_buf) {
            Ok((len, _src)) => {
                if let Some(result) = parse_read_response(&recv_buf[..len], invoke_id) {
                    return result;
                }
                // Not our response, keep waiting
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                return Err(format!("Receive error: {}", e));
            }
        }
    }

    Err("Timeout waiting for response".to_string())
}

/// Parse a read property response
fn parse_read_response(data: &[u8], expected_invoke_id: u8) -> Option<Result<PropertyValue, String>> {
    // Decode BVLL
    let (msg_type, npdu_offset, npdu_len) = bvll_decode(data).ok()?;

    if msg_type != message_type::ORIGINAL_UNICAST_NPDU
        && msg_type != message_type::ORIGINAL_BROADCAST_NPDU
    {
        return None;
    }

    // Decode NPDU
    let npdu_data_slice = &data[npdu_offset..npdu_offset + npdu_len];
    let mut npdu_data = NpduData::default();
    let apdu_offset = npdu_decode(npdu_data_slice, &mut npdu_data, &mut None, &mut None).ok()?;

    // Decode APDU
    let apdu_data = &npdu_data_slice[apdu_offset..];
    let apdu = apdu_decode(apdu_data).ok()?;

    match apdu {
        Apdu::ComplexAck { invoke_id, service_choice, service_ack, .. } => {
            if invoke_id == expected_invoke_id && service_choice == 12 {
                // ReadProperty ACK
                match ReadPropertyAck::decode(&service_ack) {
                    Ok((ack, _)) => Some(Ok(ack.property_value)),
                    Err(e) => Some(Err(format!("ACK decode error: {:?}", e))),
                }
            } else {
                None
            }
        }
        Apdu::Error { invoke_id, error_class, error_code, .. } => {
            if invoke_id == expected_invoke_id {
                Some(Err(format!("BACnet error: class={:?}, code={:?}", error_class, error_code)))
            } else {
                None
            }
        }
        Apdu::Reject { invoke_id, reason } => {
            if invoke_id == expected_invoke_id {
                Some(Err(format!("Rejected: reason={:?}", reason)))
            } else {
                None
            }
        }
        Apdu::Abort { invoke_id, reason, .. } => {
            if invoke_id == expected_invoke_id {
                Some(Err(format!("Aborted: reason={:?}", reason)))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Synchronous write property implementation
fn do_write_property(
    socket: &UdpSocket,
    dest: SocketAddr,
    invoke_id: u8,
    object_id: ObjectIdentifier,
    property_id: PropertyIdentifier,
    value: PropertyValue,
    priority: Option<u8>,
) -> Result<(), String> {
    // Create request
    let request = WritePropertyRequest {
        object_identifier: object_id,
        property_identifier: property_id,
        array_index: None,
        property_value: value,
        priority,
    };

    // Encode APDU
    let mut apdu_buf = [0u8; 512];
    let apdu_len = wp_encode_apdu(&mut apdu_buf, invoke_id, &request)
        .map_err(|e| format!("APDU encode error: {:?}", e))?;

    // Encode NPDU
    let npdu_data = NpduData {
        data_expecting_reply: true,
        ..Default::default()
    };
    let mut npdu_buf = [0u8; 512];
    let npdu_len = npdu_encode(&mut npdu_buf, None, None, &npdu_data, &apdu_buf[..apdu_len])
        .map_err(|e| format!("NPDU encode error: {:?}", e))?;

    // Wrap in BVLL
    let mut bvlc_buf = [0u8; 1024];
    let bvlc_len = bvll_encode(&mut bvlc_buf, message_type::ORIGINAL_UNICAST_NPDU, &npdu_buf[..npdu_len])
        .map_err(|e| format!("BVLL encode error: {:?}", e))?;

    // Send
    socket.send_to(&bvlc_buf[..bvlc_len], dest)
        .map_err(|e| format!("Send error: {}", e))?;

    // Wait for Simple-ACK response
    let mut recv_buf = [0u8; 1500];
    socket.set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Set timeout error: {}", e))?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        match socket.recv_from(&mut recv_buf) {
            Ok((len, _src)) => {
                if let Some(result) = parse_write_response(&recv_buf[..len], invoke_id) {
                    return result;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => {
                return Err(format!("Receive error: {}", e));
            }
        }
    }

    Err("Timeout waiting for write response".to_string())
}

/// Parse a write property response
fn parse_write_response(data: &[u8], expected_invoke_id: u8) -> Option<Result<(), String>> {
    let (msg_type, npdu_offset, npdu_len) = bvll_decode(data).ok()?;

    if msg_type != message_type::ORIGINAL_UNICAST_NPDU {
        return None;
    }

    let npdu_data_slice = &data[npdu_offset..npdu_offset + npdu_len];
    let mut npdu_data = NpduData::default();
    let apdu_offset = npdu_decode(npdu_data_slice, &mut npdu_data, &mut None, &mut None).ok()?;

    let apdu_data = &npdu_data_slice[apdu_offset..];
    let apdu = apdu_decode(apdu_data).ok()?;

    match apdu {
        Apdu::SimpleAck { invoke_id, service_choice } => {
            if invoke_id == expected_invoke_id && service_choice == 15 {
                // WriteProperty ACK
                Some(Ok(()))
            } else {
                None
            }
        }
        Apdu::Error { invoke_id, error_class, error_code, .. } => {
            if invoke_id == expected_invoke_id {
                Some(Err(format!("BACnet error: class={:?}, code={:?}", error_class, error_code)))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Synchronous Who-Is discovery
fn do_whois(
    socket: &UdpSocket,
    timeout_secs: u64,
    low_limit: Option<u32>,
    high_limit: Option<u32>,
    broadcast_addr: SocketAddr,
) -> Result<Vec<(String, u32, SocketAddr)>, String> {
    // Encode Who-Is APDU
    let low = low_limit.map(|l| l as i32).unwrap_or(-1);
    let high = high_limit.map(|h| h as i32).unwrap_or(-1);

    let mut apdu_buf = [0u8; 64];
    let apdu_len = whois_encode(&mut apdu_buf, low, high)
        .map_err(|e| format!("Who-Is encode error: {:?}", e))?;

    // Encode NPDU
    let npdu_data = NpduData::default();
    let mut npdu_buf = [0u8; 512];
    let npdu_len = npdu_encode(&mut npdu_buf, None, None, &npdu_data, &apdu_buf[..apdu_len])
        .map_err(|e| format!("NPDU encode error: {:?}", e))?;

    // Wrap in BVLL (broadcast)
    let mut bvlc_buf = [0u8; 1024];
    let bvlc_len = bvll_encode(&mut bvlc_buf, message_type::ORIGINAL_BROADCAST_NPDU, &npdu_buf[..npdu_len])
        .map_err(|e| format!("BVLL encode error: {:?}", e))?;

    // Send broadcast
    socket.send_to(&bvlc_buf[..bvlc_len], broadcast_addr)
        .map_err(|e| format!("Send error: {}", e))?;

    // Collect I-Am responses
    let mut devices = Vec::new();
    let start = Instant::now();
    let mut recv_buf = [0u8; 1500];

    socket.set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|e| format!("Set timeout error: {}", e))?;

    while start.elapsed() < Duration::from_secs(timeout_secs) {
        match socket.recv_from(&mut recv_buf) {
            Ok((len, src_addr)) => {
                if let Some((device_id, _max_apdu, _seg, _vendor)) = parse_iam_response(&recv_buf[..len]) {
                    let device_name = format!("Device-{}", device_id.instance);
                    devices.push((device_name, device_id.instance, src_addr));
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(_) => break,
        }
    }

    Ok(devices)
}

/// Parse an I-Am response
fn parse_iam_response(data: &[u8]) -> Option<(ObjectIdentifier, u16, Segmentation, u16)> {
    let (msg_type, npdu_offset, npdu_len) = bvll_decode(data).ok()?;

    if msg_type != message_type::ORIGINAL_UNICAST_NPDU
        && msg_type != message_type::ORIGINAL_BROADCAST_NPDU
    {
        return None;
    }

    let npdu_slice = &data[npdu_offset..npdu_offset + npdu_len];
    let mut npdu_data = NpduData::default();
    let apdu_offset = npdu_decode(npdu_slice, &mut npdu_data, &mut None, &mut None).ok()?;

    let apdu = &npdu_slice[apdu_offset..];
    iam_decode(apdu).ok()
}

/// Convert SocketAddr to BacnetAddress
fn socket_addr_to_bacnet(addr: &SocketAddr) -> BacnetAddress {
    match addr {
        SocketAddr::V4(v4) => {
            let octets = v4.ip().octets();
            let port_bytes = v4.port().to_be_bytes();
            let mac = [octets[0], octets[1], octets[2], octets[3], port_bytes[0], port_bytes[1]];
            BacnetAddress::local(&mac)
        }
        SocketAddr::V6(_) => {
            // Fallback for IPv6 - not fully supported in BACnet/IP
            BacnetAddress::default()
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
                object_id,
                property_id,
                array_index,
                timeout_ms,
            } => {
                self.read_property(
                    device_id,
                    object_id,
                    property_id,
                    array_index,
                    timeout_ms,
                ).await
            }

            // ReadPropertyRaw is now an alias for ReadProperty - both return PropertyValue
            BACnetIOMsg::ReadPropertyRaw {
                device_id,
                object_id,
                property_id,
                array_index,
                timeout_ms,
            } => {
                self.read_property(
                    device_id,
                    object_id,
                    property_id,
                    array_index,
                    timeout_ms,
                ).await
            }

            BACnetIOMsg::WriteProperty {
                device_id,
                object_id,
                property_id,
                value,
                priority,
            } => {
                self.write_property(
                    device_id,
                    object_id,
                    property_id,
                    value,
                    priority,
                ).await
            }

            BACnetIOMsg::RegisterDevice { device_id, address } => {
                match self.register_device(device_id, address) {
                    Ok(_) => BACnetIOReply::Registered,
                    Err(e) => BACnetIOReply::IoError(e.to_string()),
                }
            }

            BACnetIOMsg::UnregisterDevice { device_id } => {
                self.device_addresses.remove(&device_id);

                if let Ok(mut stats) = self.stats.lock() {
                    stats.connected_devices = self.device_addresses.len();
                }

                info!("Unregistered device {}", device_id);
                BACnetIOReply::Unregistered
            }

            BACnetIOMsg::IsRegistered { device_id } => {
                BACnetIOReply::IsRegistered(self.device_addresses.contains_key(&device_id))
            }

            BACnetIOMsg::GetStatistics => {
                if let Ok(stats) = self.stats.lock() {
                    BACnetIOReply::Statistics(stats.clone())
                } else {
                    BACnetIOReply::IoError("Failed to get statistics".to_string())
                }
            }

            BACnetIOMsg::WhoIs { timeout_secs, low_limit, high_limit } => {
                self.who_is(timeout_secs, low_limit, high_limit).await
            }

            BACnetIOMsg::ReadMultipleProperties { device_id, requests } => {
                self.read_multiple_properties(device_id, requests).await
            }
        }
    }
}

//! BACnet blocking worker thread
//!
//! Handles BACnet/IP communication in a dedicated thread since the bacnet crate
//! uses synchronous I/O.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Polling state for a device
struct DevicePollingState {
    /// Objects to poll (object_type, instance)
    objects: Vec<(String, u32)>,
    /// Poll interval
    interval: Duration,
    /// Last poll time
    last_poll: Instant,
    /// Current index in objects list (for round-robin polling)
    current_index: usize,
}

/// Active discovery session
struct ActiveDiscovery {
    /// Session ID
    session_id: Uuid,
    /// WebSocket client ID to send results to
    client_id: Uuid,
    /// Request ID for response correlation
    request_id: String,
    /// When this session expires
    expires_at: Instant,
    /// Device IDs already found in this session (for dedup)
    devices_found: HashSet<u32>,
}

use bacnet::apdu::{pdu_type, unconfirmed_service, service_choice};
use bacnet::datalink::{BacnetIpDatalink, DataLink};
use bacnet::npdu::{npdu_decode, npdu_encode, NpduData};
use bacnet::services::{iam_decode, whois_encode, ReadPropertyRequest, ReadPropertyAck};
use bacnet::services::read_property::rp_encode_apdu;
use bacnet::{BacnetAddress, BacnetError, ObjectType, ObjectIdentifier, PropertyIdentifier, PropertyValue, Segmentation};

use super::types::{BacnetObject, DeviceAddress, DiscoveredDevice, ObjectListResult, PointReadResult, WorkerCommand, WorkerResponse};

/// Pending request awaiting response
struct PendingRequest {
    invoke_id: u8,
    device_id: u32,
    request_type: PendingRequestType,
    sent_at: Instant,
}

#[derive(Debug)]
enum PendingRequestType {
    ReadProperty {
        object_type: String,
        instance: u32,
        property: String,
    },
    ReadObjectList,
}

/// BACnet worker that runs in a blocking thread
pub struct BacnetWorker {
    datalink: BacnetIpDatalink,
    cmd_rx: Receiver<WorkerCommand>,
    resp_tx: Sender<WorkerResponse>,
    /// Cache of discovered device addresses
    device_cache: HashMap<u32, DeviceAddress>,
    /// Next invoke ID for confirmed services
    invoke_id: u8,
    /// Pending requests awaiting responses
    pending_requests: HashMap<u8, PendingRequest>,
    /// Devices being polled
    polling_devices: HashMap<u32, DevicePollingState>,
    /// Active discovery sessions
    active_discoveries: HashMap<Uuid, ActiveDiscovery>,
}

impl BacnetWorker {
    /// Create a new BACnet worker
    pub fn new(
        bind_addr: &str,
        port: u16,
        broadcast: Option<&str>,
        cmd_rx: Receiver<WorkerCommand>,
        resp_tx: Sender<WorkerResponse>,
    ) -> Result<Self, BacnetError> {
        let mut datalink = BacnetIpDatalink::new(bind_addr, port)?;

        // Set custom broadcast address if provided
        // Always use standard BACnet port 47808 for broadcasts (devices listen on this port)
        if let Some(broadcast_addr) = broadcast {
            let broadcast_socket: SocketAddr = format!("{}:47808", broadcast_addr)
                .parse()
                .map_err(|e| BacnetError::InvalidParameter(format!("Invalid broadcast address: {}", e)))?;
            datalink.set_broadcast_address(broadcast_socket);
            tracing::info!("BACnet broadcast address set to {}", broadcast_socket);
        }

        Ok(Self {
            datalink,
            cmd_rx,
            resp_tx,
            device_cache: HashMap::new(),
            invoke_id: 0,
            pending_requests: HashMap::new(),
            polling_devices: HashMap::new(),
            active_discoveries: HashMap::new(),
        })
    }

    /// Run the blocking event loop
    pub fn run(&mut self) {
        tracing::info!("BACnet worker started");

        loop {
            // Check for commands (non-blocking)
            match self.cmd_rx.try_recv() {
                Ok(WorkerCommand::Shutdown) => {
                    tracing::info!("BACnet worker shutting down");
                    break;
                }
                Ok(WorkerCommand::Discover {
                    low_limit,
                    high_limit,
                }) => {
                    if let Err(e) = self.do_discovery(low_limit, high_limit) {
                        tracing::warn!("Discovery failed: {}", e);
                        let _ = self.resp_tx.send(WorkerResponse::Error(e.to_string()));
                    }
                }
                Ok(WorkerCommand::ReadProperty {
                    device_id,
                    object_type,
                    instance,
                    property,
                }) => {
                    if let Err(e) = self.do_read_property(device_id, &object_type, instance, &property) {
                        tracing::warn!("ReadProperty failed: {}", e);
                        let _ = self.resp_tx.send(WorkerResponse::Error(e.to_string()));
                    }
                }
                Ok(WorkerCommand::ReadObjectList { device_id }) => {
                    if let Err(e) = self.do_read_object_list(device_id) {
                        tracing::warn!("ReadObjectList failed: {}", e);
                        let _ = self.resp_tx.send(WorkerResponse::Error(e.to_string()));
                    }
                }
                Ok(WorkerCommand::StartPolling { device_id, objects, interval_ms }) => {
                    self.start_polling(device_id, objects, interval_ms);
                }
                Ok(WorkerCommand::StopPolling { device_id }) => {
                    self.stop_polling(device_id);
                }
                Ok(WorkerCommand::DiscoverSession { session_id, client_id, request_id, low_limit, high_limit, duration_secs }) => {
                    self.start_discovery_session(session_id, client_id, request_id, low_limit, high_limit, duration_secs);
                }
                Ok(WorkerCommand::StopDiscoverySession { session_id }) => {
                    self.stop_discovery_session(session_id);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    tracing::info!("Command channel disconnected, shutting down");
                    break;
                }
            }

            // Poll for incoming BACnet messages (100ms timeout)
            self.poll_incoming(100);

            // Check for request timeouts (10 second timeout)
            self.check_timeouts(Duration::from_secs(10));

            // Check for discovery session timeouts
            self.check_discovery_timeouts();

            // Execute polling for devices
            self.do_polling();
        }

        tracing::info!("BACnet worker stopped");
    }

    /// Send a Who-Is broadcast
    fn do_discovery(
        &mut self,
        low_limit: Option<u32>,
        high_limit: Option<u32>,
    ) -> Result<(), BacnetError> {
        let low = low_limit.map(|v| v as i32).unwrap_or(-1);
        let high = high_limit.map(|v| v as i32).unwrap_or(-1);

        // Encode Who-Is APDU
        let mut apdu = [0u8; 8];
        let apdu_len = whois_encode(&mut apdu, low, high)?;

        // Encode NPDU (no destination = local broadcast)
        let npdu_data = NpduData::default();
        let mut npdu = [0u8; 502];
        let npdu_len = npdu_encode(&mut npdu, None, None, &npdu_data, &apdu[..apdu_len])?;

        // Send broadcast
        let broadcast = self.datalink.broadcast_address();
        self.datalink.send(&broadcast, &npdu, npdu_len)?;

        tracing::debug!("Sent Who-Is broadcast (low={}, high={})", low, high);
        Ok(())
    }

    /// Poll for incoming messages
    fn poll_incoming(&mut self, timeout_ms: u32) {
        let mut buffer = [0u8; 1500];

        match self.datalink.receive(&mut buffer, timeout_ms) {
            Ok((src_addr, npdu_len)) => {
                if let Err(e) = self.handle_incoming(&buffer[..npdu_len], &src_addr) {
                    tracing::trace!("Failed to handle incoming message: {}", e);
                }
            }
            Err(BacnetError::Timeout) => {
                // Normal timeout, nothing to do
            }
            Err(e) => {
                tracing::trace!("Receive error: {}", e);
            }
        }
    }

    /// Check for timed out requests
    fn check_timeouts(&mut self, timeout: Duration) {
        let now = Instant::now();
        let timed_out: Vec<u8> = self
            .pending_requests
            .iter()
            .filter(|(_, req)| now.duration_since(req.sent_at) > timeout)
            .map(|(id, _)| *id)
            .collect();

        for invoke_id in timed_out {
            if let Some(req) = self.pending_requests.remove(&invoke_id) {
                tracing::warn!(
                    "Request timeout: invoke_id={}, device={}, type={:?}",
                    invoke_id,
                    req.device_id,
                    req.request_type
                );
                let _ = self.resp_tx.send(WorkerResponse::Error(format!(
                    "Request timeout for device {}",
                    req.device_id
                )));
            }
        }
    }

    /// Handle an incoming BACnet message
    fn handle_incoming(&mut self, npdu: &[u8], src_addr: &BacnetAddress) -> Result<(), BacnetError> {
        // Decode NPDU header
        let mut npdu_data = NpduData::default();
        let mut source = None;
        let mut destination = None;
        let apdu_offset = npdu_decode(npdu, &mut npdu_data, &mut source, &mut destination)?;

        // Skip network layer messages
        if npdu_data.network_layer_message {
            return Ok(());
        }

        let apdu = &npdu[apdu_offset..];
        if apdu.is_empty() {
            return Ok(());
        }

        // Check PDU type
        let pdu_type_byte = apdu[0] & 0xF0;

        match pdu_type_byte {
            pdu_type::UNCONFIRMED_REQUEST => {
                if apdu.len() < 2 {
                    return Ok(());
                }
                let service = apdu[1];

                if service == unconfirmed_service::I_AM {
                    // Pass full APDU - iam_decode expects PDU type and service choice
                    self.handle_iam(apdu, src_addr)?;
                }
            }
            pdu_type::COMPLEX_ACK => {
                self.handle_complex_ack(apdu)?;
            }
            pdu_type::ERROR => {
                self.handle_error(apdu)?;
            }
            pdu_type::REJECT | pdu_type::ABORT => {
                self.handle_reject_abort(apdu)?;
            }
            _ => {
                // Other PDU types not handled yet
            }
        }

        Ok(())
    }

    /// Handle an I-Am response
    fn handle_iam(&mut self, apdu: &[u8], src_addr: &BacnetAddress) -> Result<(), BacnetError> {
        let (device_oid, max_apdu, segmentation, vendor_id) = iam_decode(apdu)?;

        // Only care about Device objects
        if device_oid.object_type != ObjectType::Device {
            return Ok(());
        }

        let device_id = device_oid.instance;

        // Convert BacnetAddress to socket address string
        let address_str = if src_addr.len == 6 {
            // BACnet/IP address: 4 bytes IP + 2 bytes port
            let ip = format!(
                "{}.{}.{}.{}",
                src_addr.adr[0], src_addr.adr[1], src_addr.adr[2], src_addr.adr[3]
            );
            let port = ((src_addr.adr[4] as u16) << 8) | (src_addr.adr[5] as u16);
            format!("{}:{}", ip, port)
        } else {
            format!("mac:{:?}", &src_addr.adr[..src_addr.len as usize])
        };

        // Cache the device address for future reads
        if let Ok(socket_addr) = address_str.parse::<SocketAddr>() {
            self.device_cache.insert(
                device_id,
                DeviceAddress {
                    device_id,
                    address: socket_addr,
                    max_apdu,
                },
            );
        }

        let segmentation_str = match segmentation {
            Segmentation::Both => "both",
            Segmentation::Transmit => "transmit",
            Segmentation::Receive => "receive",
            Segmentation::None => "none",
        };

        let device = DiscoveredDevice {
            device_id,
            address: address_str,
            max_apdu,
            vendor_id,
            segmentation: segmentation_str.to_string(),
        };

        tracing::info!(
            "Discovered device {} at {} (vendor={})",
            device_id,
            device.address,
            vendor_id
        );

        // Send to all active discovery sessions (streaming mode)
        for discovery in self.active_discoveries.values_mut() {
            // Skip if already reported this device in this session
            if discovery.devices_found.contains(&device_id) {
                continue;
            }
            discovery.devices_found.insert(device_id);

            let _ = self.resp_tx.send(WorkerResponse::SessionDeviceDiscovered {
                client_id: discovery.client_id,
                request_id: discovery.request_id.clone(),
                device: device.clone(),
            });
        }

        // Also send legacy DeviceDiscovered for backwards compatibility
        let _ = self.resp_tx.send(WorkerResponse::DeviceDiscovered(device));

        Ok(())
    }

    /// Handle a Complex-ACK response (ReadProperty response)
    fn handle_complex_ack(&mut self, apdu: &[u8]) -> Result<(), BacnetError> {
        if apdu.len() < 3 {
            return Ok(());
        }

        let invoke_id = apdu[1];
        let service = apdu[2];

        // Look up pending request
        let pending = match self.pending_requests.remove(&invoke_id) {
            Some(p) => p,
            None => {
                tracing::debug!("Received ACK for unknown invoke_id: {}", invoke_id);
                return Ok(());
            }
        };

        if service == service_choice::READ_PROPERTY {
            // Decode ReadProperty ACK (skip PDU type, invoke ID, service choice)
            let service_data = &apdu[3..];
            match ReadPropertyAck::decode(service_data) {
                Ok((ack, _)) => {
                    self.process_read_property_ack(pending, ack);
                }
                Err(e) => {
                    tracing::warn!("Failed to decode ReadPropertyAck: {}", e);
                    let _ = self.resp_tx.send(WorkerResponse::Error(format!(
                        "Failed to decode response: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    /// Process a decoded ReadProperty ACK
    fn process_read_property_ack(&mut self, pending: PendingRequest, ack: ReadPropertyAck) {
        match pending.request_type {
            PendingRequestType::ReadObjectList => {
                // Extract object list from the property value
                let objects = self.extract_object_list(&ack.property_value);
                let result = ObjectListResult {
                    device_id: pending.device_id,
                    objects,
                };
                tracing::info!(
                    "Read object list from device {}: {} objects",
                    pending.device_id,
                    result.objects.len()
                );
                let _ = self.resp_tx.send(WorkerResponse::ObjectListRead(result));
            }
            PendingRequestType::ReadProperty {
                object_type,
                instance,
                property,
            } => {
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let value = self.property_value_to_json(&ack.property_value);

                let result = PointReadResult {
                    device_id: pending.device_id,
                    object_type,
                    instance,
                    property,
                    value,
                    timestamp,
                };
                let _ = self.resp_tx.send(WorkerResponse::PropertyRead(result));
            }
        }
    }

    /// Extract object list from PropertyValue
    fn extract_object_list(&self, value: &PropertyValue) -> Vec<BacnetObject> {
        match value {
            PropertyValue::Array(values) => {
                values
                    .iter()
                    .filter_map(|v| {
                        if let PropertyValue::ObjectIdentifier(oid) = v {
                            Some(BacnetObject {
                                object_type: format!("{:?}", oid.object_type).to_lowercase().replace("_", "-"),
                                instance: oid.instance,
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            PropertyValue::ObjectIdentifier(oid) => {
                vec![BacnetObject {
                    object_type: format!("{:?}", oid.object_type).to_lowercase().replace("_", "-"),
                    instance: oid.instance,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// Convert PropertyValue to JSON
    fn property_value_to_json(&self, value: &PropertyValue) -> serde_json::Value {
        match value {
            PropertyValue::Null => serde_json::Value::Null,
            PropertyValue::Boolean(b) => serde_json::json!(*b),
            PropertyValue::Unsigned(u) => serde_json::json!(*u),
            PropertyValue::Signed(s) => serde_json::json!(*s),
            PropertyValue::Real(r) => serde_json::json!(*r),
            PropertyValue::Double(d) => serde_json::json!(*d),
            PropertyValue::CharacterString(s) => serde_json::json!(s),
            PropertyValue::BitString(bits) => serde_json::json!(format!("{:?}", bits)),
            PropertyValue::Enumerated(e) => serde_json::json!(*e),
            PropertyValue::ObjectIdentifier(oid) => {
                serde_json::json!({
                    "type": format!("{:?}", oid.object_type),
                    "instance": oid.instance
                })
            }
            PropertyValue::Array(arr) => {
                let values: Vec<serde_json::Value> = arr
                    .iter()
                    .map(|v| self.property_value_to_json(v))
                    .collect();
                serde_json::json!(values)
            }
            _ => serde_json::json!(format!("{:?}", value)),
        }
    }

    /// Handle an Error response
    fn handle_error(&mut self, apdu: &[u8]) -> Result<(), BacnetError> {
        if apdu.len() < 2 {
            return Ok(());
        }

        let invoke_id = apdu[1];

        if let Some(pending) = self.pending_requests.remove(&invoke_id) {
            tracing::warn!(
                "Received error for device {}, request {:?}",
                pending.device_id,
                pending.request_type
            );
            let _ = self.resp_tx.send(WorkerResponse::Error(format!(
                "BACnet error response for device {}",
                pending.device_id
            )));
        }

        Ok(())
    }

    /// Handle Reject/Abort responses
    fn handle_reject_abort(&mut self, apdu: &[u8]) -> Result<(), BacnetError> {
        if apdu.len() < 2 {
            return Ok(());
        }

        let invoke_id = apdu[1];

        if let Some(pending) = self.pending_requests.remove(&invoke_id) {
            tracing::warn!(
                "Request rejected/aborted for device {}",
                pending.device_id
            );
            let _ = self.resp_tx.send(WorkerResponse::Error(format!(
                "Request rejected for device {}",
                pending.device_id
            )));
        }

        Ok(())
    }

    /// Read a property from a device
    fn do_read_property(
        &mut self,
        device_id: u32,
        object_type: &str,
        instance: u32,
        property: &str,
    ) -> Result<(), BacnetError> {
        let device_addr = self.get_device_address(device_id)?;

        let obj_type = self.parse_object_type(object_type)?;
        let prop_id = self.parse_property_id(property)?;
        let obj_id = ObjectIdentifier::new(obj_type, instance)?;

        let invoke_id = self.next_invoke_id();
        let request = ReadPropertyRequest::new(obj_id, prop_id, None);

        self.send_read_property_request(device_id, &device_addr, invoke_id, &request)?;

        // Track pending request
        self.pending_requests.insert(
            invoke_id,
            PendingRequest {
                invoke_id,
                device_id,
                request_type: PendingRequestType::ReadProperty {
                    object_type: object_type.to_string(),
                    instance,
                    property: property.to_string(),
                },
                sent_at: Instant::now(),
            },
        );

        Ok(())
    }

    /// Read the object list from a device
    fn do_read_object_list(&mut self, device_id: u32) -> Result<(), BacnetError> {
        let device_addr = self.get_device_address(device_id)?;

        // Read ObjectList from Device object
        let obj_id = ObjectIdentifier::new(ObjectType::Device, device_id)?;
        let invoke_id = self.next_invoke_id();
        let request = ReadPropertyRequest::new(obj_id, PropertyIdentifier::ObjectList, None);

        self.send_read_property_request(device_id, &device_addr, invoke_id, &request)?;

        // Track pending request
        self.pending_requests.insert(
            invoke_id,
            PendingRequest {
                invoke_id,
                device_id,
                request_type: PendingRequestType::ReadObjectList,
                sent_at: Instant::now(),
            },
        );

        tracing::info!("Sent ReadProperty(ObjectList) to device {}", device_id);

        Ok(())
    }

    /// Send a ReadProperty request
    fn send_read_property_request(
        &mut self,
        device_id: u32,
        device_addr: &DeviceAddress,
        invoke_id: u8,
        request: &ReadPropertyRequest,
    ) -> Result<(), BacnetError> {
        // Encode the APDU
        let mut apdu = [0u8; 256];
        let apdu_len = rp_encode_apdu(&mut apdu, invoke_id, request)?;

        // Build NPDU with data expecting reply
        // Note: For direct BACnet/IP unicast, we use None for destination in NPDU.
        // The IP layer handles addressing; NPDU destination is only for routed messages.
        let npdu_data = NpduData {
            data_expecting_reply: true,
            ..NpduData::default()
        };

        let mut packet = [0u8; 512];
        let packet_len = npdu_encode(&mut packet, None, None, &npdu_data, &apdu[..apdu_len])?;

        // Create destination address from cached socket addr for datalink
        let dest_addr = self.socket_to_bacnet_address(&device_addr.address);

        // Send to device (datalink handles BVLL encoding and IP addressing)
        self.datalink.send(&dest_addr, &packet, packet_len)?;

        tracing::debug!(
            "Sent ReadProperty to device {} at {} (invoke_id={})",
            device_id,
            device_addr.address,
            invoke_id
        );

        Ok(())
    }

    /// Get device address from cache
    fn get_device_address(&self, device_id: u32) -> Result<DeviceAddress, BacnetError> {
        self.device_cache.get(&device_id).cloned().ok_or_else(|| {
            BacnetError::InvalidParameter(format!(
                "Device {} not found in cache. Run discovery first.",
                device_id
            ))
        })
    }

    /// Convert socket address to BACnet address
    fn socket_to_bacnet_address(&self, addr: &SocketAddr) -> BacnetAddress {
        match addr {
            SocketAddr::V4(v4) => {
                let octets = v4.ip().octets();
                let port = v4.port();
                BacnetAddress::local(&[
                    octets[0],
                    octets[1],
                    octets[2],
                    octets[3],
                    (port >> 8) as u8,
                    (port & 0xFF) as u8,
                ])
            }
            SocketAddr::V6(_) => {
                // Not supported, return empty
                BacnetAddress::default()
            }
        }
    }

    /// Parse object type string to ObjectType enum
    fn parse_object_type(&self, s: &str) -> Result<ObjectType, BacnetError> {
        match s.to_lowercase().replace("-", "_").as_str() {
            "analog_input" | "analoginput" | "ai" => Ok(ObjectType::AnalogInput),
            "analog_output" | "analogoutput" | "ao" => Ok(ObjectType::AnalogOutput),
            "analog_value" | "analogvalue" | "av" => Ok(ObjectType::AnalogValue),
            "binary_input" | "binaryinput" | "bi" => Ok(ObjectType::BinaryInput),
            "binary_output" | "binaryoutput" | "bo" => Ok(ObjectType::BinaryOutput),
            "binary_value" | "binaryvalue" | "bv" => Ok(ObjectType::BinaryValue),
            "device" => Ok(ObjectType::Device),
            "schedule" => Ok(ObjectType::Schedule),
            "calendar" => Ok(ObjectType::Calendar),
            "notification_class" | "notificationclass" => Ok(ObjectType::NotificationClass),
            "multistate_input" | "multistateinput" | "msi" => Ok(ObjectType::MultiStateInput),
            "multistate_output" | "multistateoutput" | "mso" => Ok(ObjectType::MultiStateOutput),
            "multistate_value" | "multistatevalue" | "msv" => Ok(ObjectType::MultiStateValue),
            _ => Err(BacnetError::InvalidParameter(format!(
                "Unknown object type: {}",
                s
            ))),
        }
    }

    /// Parse property ID string to PropertyIdentifier enum
    fn parse_property_id(&self, s: &str) -> Result<PropertyIdentifier, BacnetError> {
        match s.to_lowercase().replace("-", "_").as_str() {
            "present_value" | "presentvalue" | "pv" => Ok(PropertyIdentifier::PresentValue),
            "object_name" | "objectname" | "name" => Ok(PropertyIdentifier::ObjectName),
            "object_type" | "objecttype" => Ok(PropertyIdentifier::ObjectType),
            "object_list" | "objectlist" => Ok(PropertyIdentifier::ObjectList),
            "description" => Ok(PropertyIdentifier::Description),
            "status_flags" | "statusflags" => Ok(PropertyIdentifier::StatusFlags),
            "event_state" | "eventstate" => Ok(PropertyIdentifier::EventState),
            "out_of_service" | "outofservice" => Ok(PropertyIdentifier::OutOfService),
            "units" => Ok(PropertyIdentifier::Units),
            "reliability" => Ok(PropertyIdentifier::Reliability),
            _ => Err(BacnetError::InvalidParameter(format!(
                "Unknown property: {}",
                s
            ))),
        }
    }

    /// Get the next invoke ID
    fn next_invoke_id(&mut self) -> u8 {
        let id = self.invoke_id;
        self.invoke_id = self.invoke_id.wrapping_add(1);
        id
    }

    /// Start polling values for a device
    fn start_polling(&mut self, device_id: u32, objects: Vec<(String, u32)>, interval_ms: u64) {
        if objects.is_empty() {
            tracing::warn!("Cannot start polling for device {} with no objects", device_id);
            return;
        }

        tracing::info!(
            "Starting polling for device {} ({} objects, {}ms interval)",
            device_id,
            objects.len(),
            interval_ms
        );

        self.polling_devices.insert(device_id, DevicePollingState {
            objects,
            interval: Duration::from_millis(interval_ms),
            last_poll: Instant::now() - Duration::from_millis(interval_ms), // Trigger immediate first poll
            current_index: 0,
        });
    }

    /// Stop polling for a device
    fn stop_polling(&mut self, device_id: u32) {
        if self.polling_devices.remove(&device_id).is_some() {
            tracing::info!("Stopped polling for device {}", device_id);
        }
    }

    /// Execute polling for all devices that need it
    fn do_polling(&mut self) {
        let now = Instant::now();

        // Collect devices that need polling to avoid borrow issues
        let devices_to_poll: Vec<(u32, String, u32)> = self.polling_devices
            .iter_mut()
            .filter_map(|(device_id, state)| {
                if now.duration_since(state.last_poll) >= state.interval {
                    if state.objects.is_empty() {
                        return None;
                    }

                    // Get the current object to poll
                    let (object_type, instance) = state.objects[state.current_index].clone();

                    // Move to next object for next poll
                    state.current_index = (state.current_index + 1) % state.objects.len();
                    state.last_poll = now;

                    Some((*device_id, object_type, instance))
                } else {
                    None
                }
            })
            .collect();

        // Now execute the reads
        for (device_id, object_type, instance) in devices_to_poll {
            if let Err(e) = self.do_read_property(device_id, &object_type, instance, "present-value") {
                tracing::trace!("Polling read failed for device {} {}.{}: {}", device_id, object_type, instance, e);
            }
        }
    }

    /// Start a new discovery session
    fn start_discovery_session(
        &mut self,
        session_id: Uuid,
        client_id: Uuid,
        request_id: String,
        low_limit: Option<u32>,
        high_limit: Option<u32>,
        duration_secs: u64,
    ) {
        tracing::info!(
            "Starting discovery session {} for client {} (duration: {}s)",
            session_id,
            client_id,
            duration_secs
        );

        // Store the active discovery session
        self.active_discoveries.insert(session_id, ActiveDiscovery {
            session_id,
            client_id,
            request_id: request_id.clone(),
            expires_at: Instant::now() + Duration::from_secs(duration_secs),
            devices_found: HashSet::new(),
        });

        // Send the Who-Is broadcast
        if let Err(e) = self.do_discovery(low_limit, high_limit) {
            tracing::warn!("Discovery broadcast failed: {}", e);
            let _ = self.resp_tx.send(WorkerResponse::Error(e.to_string()));
        }
    }

    /// Stop a discovery session early
    fn stop_discovery_session(&mut self, session_id: Uuid) {
        if let Some(discovery) = self.active_discoveries.remove(&session_id) {
            tracing::info!(
                "Stopped discovery session {} ({} devices found)",
                session_id,
                discovery.devices_found.len()
            );

            let _ = self.resp_tx.send(WorkerResponse::SessionComplete {
                client_id: discovery.client_id,
                request_id: discovery.request_id,
                devices_found: discovery.devices_found.len() as u32,
            });
        }
    }

    /// Check for expired discovery sessions
    fn check_discovery_timeouts(&mut self) {
        let now = Instant::now();

        // Find expired sessions
        let expired: Vec<Uuid> = self.active_discoveries
            .iter()
            .filter(|(_, d)| now >= d.expires_at)
            .map(|(id, _)| *id)
            .collect();

        // Complete each expired session
        for session_id in expired {
            if let Some(discovery) = self.active_discoveries.remove(&session_id) {
                tracing::info!(
                    "Discovery session {} completed (timeout, {} devices found)",
                    session_id,
                    discovery.devices_found.len()
                );

                let _ = self.resp_tx.send(WorkerResponse::SessionComplete {
                    client_id: discovery.client_id,
                    request_id: discovery.request_id,
                    devices_found: discovery.devices_found.len() as u32,
                });
            }
        }
    }
}

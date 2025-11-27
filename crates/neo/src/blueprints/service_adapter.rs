// Blueprint Service Adapter - Wraps a blueprint as a service
//
// This adapter allows blueprints to be registered in the ServiceRegistry
// and respond to ServiceMsg just like native services.

use std::collections::HashMap;
use std::sync::Arc;

use kameo::actor::ActorRef;
use kameo::message::{Context, Message};
use serde_json::Value;
use tokio::sync::oneshot;
use tracing::{debug, error, info};

use crate::messages::Event;
use crate::services::actor::{ServiceMetadata, ServiceMsg, ServiceReply, ServiceStateTracker, ServiceType};
use crate::services::messages::{ServiceRequest, ServiceResponse};
use crate::types::ServiceState;

use blueprint_types::{Blueprint, ExecutionTrigger};

use super::service::{BlueprintService, TriggerEvent};

/// Actor that wraps a blueprint as a service
///
/// The adapter translates ServiceMsg to blueprint execution triggers:
/// - Start -> triggers OnServiceStart nodes
/// - Stop -> triggers OnServiceStop nodes
/// - OnEvent -> triggers matching event nodes
/// - HandleRequest -> triggers OnServiceRequest nodes
#[derive(kameo::Actor)]
pub struct BlueprintServiceAdapter {
    /// The blueprint being wrapped
    blueprint: Arc<Blueprint>,
    /// Reference to the BlueprintService actor
    blueprint_service: ActorRef<BlueprintService>,
    /// Service state tracker
    state_tracker: ServiceStateTracker,
    /// Service metadata
    metadata: ServiceMetadata,
    /// Pending request responses (request_id -> reply channel)
    pending_requests: HashMap<String, oneshot::Sender<ServiceResponse>>,
    /// Request ID counter
    request_counter: u64,
}

impl BlueprintServiceAdapter {
    /// Create a new adapter for a blueprint
    pub fn new(
        blueprint: Arc<Blueprint>,
        blueprint_service: ActorRef<BlueprintService>,
    ) -> Self {
        let service_config = blueprint.service.as_ref();
        let description = service_config
            .and_then(|s| s.description.clone())
            .unwrap_or_else(|| blueprint.description.clone().unwrap_or_default());

        let metadata = ServiceMetadata {
            id: format!("blueprint:{}", blueprint.id),
            name: blueprint.name.clone(),
            description,
            service_type: ServiceType::Native, // Blueprints appear as native services
        };

        Self {
            blueprint,
            blueprint_service,
            state_tracker: ServiceStateTracker::new(),
            metadata,
            pending_requests: HashMap::new(),
            request_counter: 0,
        }
    }

    /// Get service metadata
    pub fn metadata(&self) -> &ServiceMetadata {
        &self.metadata
    }

    /// Find event node IDs that handle the given trigger type
    fn find_lifecycle_nodes(&self, node_type: &str) -> Vec<String> {
        self.blueprint
            .nodes
            .iter()
            .filter(|n| n.node_type == node_type)
            .map(|n| n.id.clone())
            .collect()
    }

    /// Execute lifecycle nodes
    async fn execute_lifecycle(&self, trigger: ExecutionTrigger) {
        let node_type = match &trigger {
            ExecutionTrigger::ServiceStart => "neo/OnServiceStart",
            ExecutionTrigger::ServiceStop => "neo/OnServiceStop",
            ExecutionTrigger::ServiceRequest { .. } => "neo/OnServiceRequest",
            ExecutionTrigger::ServiceEvent { .. } => return, // Handled by TriggerEvent
            _ => return,
        };

        let nodes = self.find_lifecycle_nodes(node_type);

        for node_id in nodes {
            debug!(
                blueprint_id = %self.blueprint.id,
                node_id = %node_id,
                trigger = ?node_type,
                "Executing lifecycle node"
            );

            // Use TriggerEvent for now - we'll handle the lifecycle events there
            let event_type = match &trigger {
                ExecutionTrigger::ServiceStart => "service_start",
                ExecutionTrigger::ServiceStop => "service_stop",
                _ => continue,
            };

            let _ = self.blueprint_service.tell(TriggerEvent {
                event_type: event_type.to_string(),
                data: Value::Object(serde_json::Map::new()),
            }).await;
        }
    }
}

impl Message<ServiceMsg> for BlueprintServiceAdapter {
    type Reply = ServiceReply;

    async fn handle(
        &mut self,
        msg: ServiceMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            ServiceMsg::Start => {
                info!(
                    blueprint_id = %self.blueprint.id,
                    "Starting blueprint service"
                );

                self.state_tracker.set_starting();

                // Execute OnServiceStart nodes
                self.execute_lifecycle(ExecutionTrigger::ServiceStart).await;

                self.state_tracker.set_running();
                ServiceReply::Started
            }

            ServiceMsg::Stop => {
                info!(
                    blueprint_id = %self.blueprint.id,
                    "Stopping blueprint service"
                );

                self.state_tracker.set_stopping();

                // Execute OnServiceStop nodes
                self.execute_lifecycle(ExecutionTrigger::ServiceStop).await;

                self.state_tracker.set_stopped();
                ServiceReply::Stopped
            }

            ServiceMsg::GetStatus => {
                ServiceReply::Status {
                    id: self.metadata.id.clone(),
                    name: self.metadata.name.clone(),
                    state: self.state_tracker.state(),
                    uptime_secs: self.state_tracker.uptime_secs(),
                    extra: Some(serde_json::json!({
                        "blueprint_id": self.blueprint.id,
                        "node_count": self.blueprint.nodes.len(),
                    })),
                }
            }

            ServiceMsg::GetConfig => {
                // Return the blueprint's service config
                let config = self.blueprint.service.as_ref()
                    .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
                    .unwrap_or(Value::Null);
                ServiceReply::Config { config }
            }

            ServiceMsg::SetConfig { config: _ } => {
                // Blueprint configs are read-only from files
                debug!(
                    blueprint_id = %self.blueprint.id,
                    "SetConfig called on blueprint service (ignored - use file)"
                );
                ServiceReply::ConfigSet
            }

            ServiceMsg::OnEvent { event } => {
                if self.state_tracker.state() != ServiceState::Running {
                    return ServiceReply::EventHandled;
                }

                // Convert Event to TriggerEvent
                let (event_type, data) = match &event {
                    Event::ServiceStateChanged { service, state, .. } => {
                        ("ServiceStateChanged".to_string(), serde_json::json!({
                            "service": service,
                            "state": format!("{:?}", state),
                        }))
                    }
                    Event::PointValueChanged { point, value, .. } => {
                        ("PointValueChanged".to_string(), serde_json::json!({
                            "point": point,
                            "value": value,
                        }))
                    }
                    Event::AlarmRaised { source, message, severity, .. } => {
                        ("AlarmRaised".to_string(), serde_json::json!({
                            "source": source,
                            "message": message,
                            "severity": format!("{:?}", severity),
                        }))
                    }
                    Event::AlarmCleared { source, .. } => {
                        ("AlarmCleared".to_string(), serde_json::json!({
                            "source": source,
                        }))
                    }
                    Event::DeviceStatusChanged { device, network, status, .. } => {
                        ("DeviceStatusChanged".to_string(), serde_json::json!({
                            "device": device,
                            "network": network,
                            "status": format!("{:?}", status),
                        }))
                    }
                    Event::DeviceDiscovered { network, device, instance, address, .. } => {
                        ("DeviceDiscovered".to_string(), serde_json::json!({
                            "network": network,
                            "device": device,
                            "instance": instance,
                            "address": address.to_string(),
                        }))
                    }
                    Event::Custom { event_type, source, data, .. } => {
                        (event_type.clone(), serde_json::json!({
                            "source": source,
                            "data": data,
                        }))
                    }
                };

                // Forward to BlueprintService
                let _ = self.blueprint_service.tell(TriggerEvent {
                    event_type,
                    data,
                }).await;

                ServiceReply::EventHandled
            }

            ServiceMsg::HandleRequest { request, reply } => {
                if self.state_tracker.state() != ServiceState::Running {
                    let _ = reply.send(ServiceResponse::Error {
                        code: "NOT_RUNNING".to_string(),
                        message: "Blueprint service is not running".to_string(),
                    });
                    return ServiceReply::RequestHandled;
                }

                // Generate request ID
                self.request_counter += 1;
                let request_id = format!("{}-{}", self.blueprint.id, self.request_counter);

                // Store the reply channel
                self.pending_requests.insert(request_id.clone(), reply);

                // Extract action and payload from request
                let (action, payload) = match &request {
                    ServiceRequest::Custom { action, payload } => {
                        (action.clone(), payload.clone())
                    }
                    _ => {
                        // For built-in requests, serialize them
                        let action = format!("{:?}", request).split('{').next().unwrap_or("Unknown").to_string();
                        let payload = serde_json::to_value(&request).unwrap_or(Value::Null);
                        (action, payload)
                    }
                };

                // Trigger OnServiceRequest nodes
                let _ = self.blueprint_service.tell(TriggerEvent {
                    event_type: "service_request".to_string(),
                    data: serde_json::json!({
                        "request_id": request_id,
                        "action": action,
                        "payload": payload,
                    }),
                }).await;

                // Note: The response will be sent when the blueprint calls RespondToRequest
                // For now, we set a timeout for pending requests
                // TODO: Implement timeout cleanup for pending requests

                ServiceReply::RequestHandled
            }
        }
    }
}

/// Message to send a response for a pending request
#[derive(Debug)]
pub struct SendResponse {
    pub request_id: String,
    pub response: Value,
    pub success: bool,
}

impl Message<SendResponse> for BlueprintServiceAdapter {
    type Reply = bool;

    async fn handle(
        &mut self,
        msg: SendResponse,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(reply) = self.pending_requests.remove(&msg.request_id) {
            let response = if msg.success {
                ServiceResponse::Custom { payload: msg.response }
            } else {
                ServiceResponse::Error {
                    code: "BLUEPRINT_ERROR".to_string(),
                    message: msg.response.as_str().unwrap_or("Unknown error").to_string(),
                }
            };

            reply.send(response).is_ok()
        } else {
            error!(
                request_id = %msg.request_id,
                "No pending request found for response"
            );
            false
        }
    }
}

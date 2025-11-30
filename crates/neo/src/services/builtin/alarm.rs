// Alarm Service - Monitors points and raises alarms based on conditions
//
// Actor-based alarm service that evaluates point values against configurable
// conditions and manages alarm lifecycle (active, acknowledged, cleared).

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use kameo::message::{Context, Message};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;
use wildmatch::WildMatch;

use crate::messages::Event;
use crate::services::actor::{ServiceMsg, ServiceReply, ServiceStateTracker};
use crate::services::messages::{Alarm, AlarmState, ServiceRequest, ServiceResponse};
use crate::types::{AlarmSeverity, PropertyValue, ServiceState};

/// Configuration for an alarm rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmConfig {
    /// Unique identifier for this alarm configuration
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// Glob pattern to match point paths (e.g., "*/VAV-*/AI:1")
    pub source_pattern: String,
    /// Condition that triggers the alarm
    pub condition: AlarmCondition,
    /// Severity of the alarm when triggered
    pub severity: AlarmSeverity,
    /// Delay in seconds before raising alarm (debounce)
    #[serde(default)]
    pub delay_seconds: u32,
    /// Message template (can include {point}, {value}, {threshold})
    pub message_template: String,
    /// Whether this alarm config is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Alarm trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlarmCondition {
    /// Value exceeds threshold
    HighLimit { value: f32 },
    /// Value falls below threshold
    LowLimit { value: f32 },
    /// Value is outside range
    OutOfRange { low: f32, high: f32 },
    /// Value equals a specific value
    Equals { value: PropertyValue },
    /// Value does not equal a specific value
    NotEquals { value: PropertyValue },
    /// Boolean is true
    IsTrue,
    /// Boolean is false
    IsFalse,
}

impl AlarmCondition {
    /// Evaluate the condition against a point value
    fn evaluate(&self, value: &PropertyValue) -> bool {
        match (self, value) {
            // Numeric comparisons
            (AlarmCondition::HighLimit { value: threshold }, PropertyValue::Real(v)) => {
                v > threshold
            }
            (AlarmCondition::HighLimit { value: threshold }, PropertyValue::Unsigned(v)) => {
                (*v as f32) > *threshold
            }
            (AlarmCondition::LowLimit { value: threshold }, PropertyValue::Real(v)) => {
                v < threshold
            }
            (AlarmCondition::LowLimit { value: threshold }, PropertyValue::Unsigned(v)) => {
                (*v as f32) < *threshold
            }
            (AlarmCondition::OutOfRange { low, high }, PropertyValue::Real(v)) => {
                v < low || v > high
            }
            (AlarmCondition::OutOfRange { low, high }, PropertyValue::Unsigned(v)) => {
                let v = *v as f32;
                v < *low || v > *high
            }

            // Equality comparisons
            (AlarmCondition::Equals { value: expected }, actual) => expected == actual,
            (AlarmCondition::NotEquals { value: expected }, actual) => expected != actual,

            // Boolean comparisons
            (AlarmCondition::IsTrue, PropertyValue::Boolean(v)) => *v,
            (AlarmCondition::IsFalse, PropertyValue::Boolean(v)) => !*v,

            // Type mismatches don't trigger
            _ => false,
        }
    }

    /// Get a description of the condition for messages
    fn description(&self) -> String {
        match self {
            AlarmCondition::HighLimit { value } => format!("exceeds {}", value),
            AlarmCondition::LowLimit { value } => format!("below {}", value),
            AlarmCondition::OutOfRange { low, high } => format!("outside {} to {}", low, high),
            AlarmCondition::Equals { value } => format!("equals {:?}", value),
            AlarmCondition::NotEquals { value } => format!("not equal to {:?}", value),
            AlarmCondition::IsTrue => "is true".to_string(),
            AlarmCondition::IsFalse => "is false".to_string(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Alarm Actor Messages
// ─────────────────────────────────────────────────────────────────────────────

/// Alarm-specific messages (beyond common ServiceMsg)
#[derive(Debug)]
pub enum AlarmMsg {
    /// Get all active (non-cleared) alarms
    GetActiveAlarms {
        reply: oneshot::Sender<Vec<Alarm>>,
    },
    /// Acknowledge an alarm
    AcknowledgeAlarm {
        alarm_id: Uuid,
        acknowledged_by: Option<String>,
        reply: oneshot::Sender<Option<Alarm>>,
    },
    /// Get alarm history within a time range
    GetAlarmHistory {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        source_filter: Option<String>,
        reply: oneshot::Sender<Vec<Alarm>>,
    },
    /// Add an alarm configuration
    AddConfig { config: AlarmConfig },
    /// Remove an alarm configuration
    RemoveConfig {
        id: Uuid,
        reply: oneshot::Sender<bool>,
    },
    /// Get all alarm configurations
    GetConfigs {
        reply: oneshot::Sender<Vec<AlarmConfig>>,
    },
}

/// Reply type for AlarmMsg (for messages that don't use oneshot)
#[derive(Debug, kameo::Reply)]
pub enum AlarmReply {
    /// Configuration added
    ConfigAdded,
}

// ─────────────────────────────────────────────────────────────────────────────
// Alarm Actor
// ─────────────────────────────────────────────────────────────────────────────

/// Actor-based Alarm Service
#[derive(kameo::Actor)]
pub struct AlarmActor {
    id: String,
    name: String,
    #[allow(dead_code)]
    description: String,
    state: ServiceStateTracker,
    /// Alarm configurations
    configs: Vec<AlarmConfig>,
    /// Currently active alarms (keyed by alarm ID)
    active_alarms: HashMap<Uuid, Alarm>,
    /// Alarm history (most recent first, using VecDeque for efficient front insertion)
    alarm_history: VecDeque<Alarm>,
    /// Tracks which config triggered which alarm for each point
    /// Key: (config_id, point_name) -> alarm_id
    alarm_tracker: HashMap<(Uuid, String), Uuid>,
    /// Maximum history entries to keep
    max_history: usize,
}

impl AlarmActor {
    /// Create a new AlarmActor with default settings
    pub fn new() -> Self {
        Self {
            id: "alarm".to_string(),
            name: "Alarm Service".to_string(),
            description: "Monitors points and raises alarms based on configurable conditions"
                .to_string(),
            state: ServiceStateTracker::new(),
            configs: Vec::new(),
            active_alarms: HashMap::new(),
            alarm_history: VecDeque::new(),
            alarm_tracker: HashMap::new(),
            max_history: 10000,
        }
    }

    /// Create a new AlarmActor with custom ID and max history
    pub fn with_config(id: impl Into<String>, max_history: usize) -> Self {
        let id = id.into();
        Self {
            name: format!("Alarm Service ({})", id),
            id,
            description: "Monitors points and raises alarms based on configurable conditions"
                .to_string(),
            state: ServiceStateTracker::new(),
            configs: Vec::new(),
            active_alarms: HashMap::new(),
            alarm_history: VecDeque::new(),
            alarm_tracker: HashMap::new(),
            max_history,
        }
    }

    /// Add an alarm configuration
    pub fn add_config(&mut self, config: AlarmConfig) {
        self.configs.push(config);
    }

    /// Evaluate a point value against all alarm configs
    fn evaluate_point(&mut self, point: &str, value: &PropertyValue, timestamp: DateTime<Utc>) {
        for config in &self.configs.clone() {
            if !config.enabled {
                continue;
            }

            // Check if point matches the pattern
            if !WildMatch::new(&config.source_pattern).matches(point) {
                continue;
            }

            let tracker_key = (config.id, point.to_string());
            let condition_met = config.condition.evaluate(value);

            if condition_met {
                // Check if alarm already exists for this config/point combo
                let existing_alarm = self.alarm_tracker.get(&tracker_key).copied();

                if existing_alarm.is_none() {
                    // Raise new alarm
                    let alarm = self.create_alarm(config, point, value, timestamp);
                    let alarm_id = alarm.id;

                    self.active_alarms.insert(alarm_id, alarm.clone());
                    self.alarm_tracker.insert(tracker_key, alarm_id);

                    // Add to history (front for most recent first)
                    self.alarm_history.push_front(alarm);
                    if self.alarm_history.len() > self.max_history {
                        self.alarm_history.pop_back();
                    }

                    tracing::warn!(
                        "Alarm raised: {} - {} ({})",
                        config.name,
                        point,
                        config.condition.description()
                    );
                }
            } else {
                // Check if we need to clear an existing alarm
                let existing_alarm = self.alarm_tracker.get(&tracker_key).copied();

                if let Some(alarm_id) = existing_alarm {
                    // Clear the alarm
                    if let Some(alarm) = self.active_alarms.get_mut(&alarm_id) {
                        alarm.state = AlarmState::Cleared;
                        alarm.cleared_at = Some(timestamp);

                        tracing::info!("Alarm cleared: {} - {}", config.name, point);
                    }

                    // Remove from tracker
                    self.alarm_tracker.remove(&tracker_key);
                }
            }
        }
    }

    /// Create a new alarm from a config
    fn create_alarm(
        &self,
        config: &AlarmConfig,
        point: &str,
        value: &PropertyValue,
        timestamp: DateTime<Utc>,
    ) -> Alarm {
        let message = config
            .message_template
            .replace("{point}", point)
            .replace("{value}", &format!("{:?}", value))
            .replace("{threshold}", &config.condition.description());

        Alarm {
            id: Uuid::new_v4(),
            source: point.to_string(),
            message,
            severity: config.severity,
            state: AlarmState::Active,
            triggered_at: timestamp,
            acknowledged_at: None,
            acknowledged_by: None,
            cleared_at: None,
            value_at_trigger: Some(value.clone()),
        }
    }

    /// Get all active (non-cleared) alarms
    fn get_active_alarms(&self) -> Vec<Alarm> {
        self.active_alarms
            .values()
            .filter(|a| a.state != AlarmState::Cleared)
            .cloned()
            .collect()
    }

    /// Acknowledge an alarm
    fn acknowledge_alarm(
        &mut self,
        alarm_id: Uuid,
        acknowledged_by: Option<String>,
    ) -> Option<Alarm> {
        if let Some(alarm) = self.active_alarms.get_mut(&alarm_id) {
            if alarm.state == AlarmState::Active {
                alarm.state = AlarmState::Acknowledged;
                alarm.acknowledged_at = Some(Utc::now());
                alarm.acknowledged_by = acknowledged_by;
                return Some(alarm.clone());
            }
        }
        None
    }
}

impl Default for AlarmActor {
    fn default() -> Self {
        Self::new()
    }
}

// Handle common ServiceMsg
impl Message<ServiceMsg> for AlarmActor {
    type Reply = ServiceReply;

    async fn handle(
        &mut self,
        msg: ServiceMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            ServiceMsg::Start => {
                self.state.set_starting();
                self.state.set_running();
                let config_count = self.configs.len();
                tracing::info!(
                    "Alarm actor started ({} alarm configurations)",
                    config_count
                );
                ServiceReply::Started
            }

            ServiceMsg::Stop => {
                self.state.set_stopping();
                // Clear tracker but keep history
                self.alarm_tracker.clear();
                self.state.set_stopped();
                tracing::info!("Alarm actor stopped");
                ServiceReply::Stopped
            }

            ServiceMsg::GetStatus => ServiceReply::Status {
                id: self.id.clone(),
                name: self.name.clone(),
                state: self.state.state(),
                uptime_secs: self.state.uptime_secs(),
                extra: Some(serde_json::json!({
                    "active_alarm_count": self.active_alarms.len(),
                    "config_count": self.configs.len(),
                })),
            },

            ServiceMsg::GetConfig => ServiceReply::Config {
                config: serde_json::json!({ "alarms": self.configs }),
            },

            ServiceMsg::SetConfig { config } => {
                if let Ok(alarms) = serde_json::from_value::<Vec<AlarmConfig>>(
                    config.get("alarms").cloned().unwrap_or_default(),
                ) {
                    self.configs = alarms;
                    ServiceReply::ConfigSet
                } else {
                    ServiceReply::Failed("Invalid alarm configuration format".to_string())
                }
            }

            ServiceMsg::OnEvent { event } => {
                if self.state.state() != ServiceState::Running {
                    return ServiceReply::EventHandled;
                }

                if let Event::PointValueChanged {
                    point,
                    value,
                    timestamp_utc,
                    ..
                } = event
                {
                    self.evaluate_point(&point, &value, timestamp_utc);
                }
                ServiceReply::EventHandled
            }

            ServiceMsg::HandleRequest { request, reply } => {
                let response = match request {
                    ServiceRequest::GetStatus => ServiceResponse::Status {
                        id: self.id.clone(),
                        name: self.name.clone(),
                        state: self.state.state(),
                        uptime_seconds: self.state.uptime_secs(),
                        extra: Some(serde_json::json!({
                            "active_alarm_count": self.active_alarms.len(),
                            "config_count": self.configs.len(),
                        })),
                    },

                    ServiceRequest::GetConfig => ServiceResponse::Config {
                        config: serde_json::json!({ "alarms": self.configs }),
                    },

                    ServiceRequest::GetActiveAlarms => {
                        let alarms = self.get_active_alarms();
                        ServiceResponse::ActiveAlarms { alarms }
                    }

                    ServiceRequest::AcknowledgeAlarm { alarm_id, comment } => {
                        if self.acknowledge_alarm(alarm_id, comment).is_some() {
                            ServiceResponse::AlarmAcknowledged { alarm_id }
                        } else {
                            ServiceResponse::Error {
                                code: "NOT_FOUND".to_string(),
                                message: format!(
                                    "Alarm {} not found or already acknowledged",
                                    alarm_id
                                ),
                            }
                        }
                    }

                    ServiceRequest::GetAlarmHistory {
                        start,
                        end,
                        source_filter,
                    } => {
                        let alarms: Vec<Alarm> = self
                            .alarm_history
                            .iter()
                            .filter(|a| {
                                a.triggered_at >= start
                                    && a.triggered_at <= end
                                    && source_filter
                                        .as_ref()
                                        .map(|f| WildMatch::new(f).matches(&a.source))
                                        .unwrap_or(true)
                            })
                            .cloned()
                            .collect();
                        ServiceResponse::AlarmHistory { alarms }
                    }

                    _ => ServiceResponse::Error {
                        code: "UNSUPPORTED".to_string(),
                        message: "Request not supported by Alarm service".to_string(),
                    },
                };

                let _ = reply.send(response);
                ServiceReply::RequestHandled
            }
        }
    }
}

// Handle alarm-specific messages
impl Message<AlarmMsg> for AlarmActor {
    type Reply = AlarmReply;

    async fn handle(
        &mut self,
        msg: AlarmMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            AlarmMsg::GetActiveAlarms { reply } => {
                let alarms = self.get_active_alarms();
                let _ = reply.send(alarms);
                AlarmReply::ConfigAdded // Placeholder reply
            }

            AlarmMsg::AcknowledgeAlarm {
                alarm_id,
                acknowledged_by,
                reply,
            } => {
                let result = self.acknowledge_alarm(alarm_id, acknowledged_by);
                let _ = reply.send(result);
                AlarmReply::ConfigAdded // Placeholder reply
            }

            AlarmMsg::GetAlarmHistory {
                start,
                end,
                source_filter,
                reply,
            } => {
                let alarms: Vec<Alarm> = self
                    .alarm_history
                    .iter()
                    .filter(|a| {
                        a.triggered_at >= start
                            && a.triggered_at <= end
                            && source_filter
                                .as_ref()
                                .map(|f| WildMatch::new(f).matches(&a.source))
                                .unwrap_or(true)
                    })
                    .cloned()
                    .collect();
                let _ = reply.send(alarms);
                AlarmReply::ConfigAdded // Placeholder reply
            }

            AlarmMsg::AddConfig { config } => {
                self.configs.push(config);
                AlarmReply::ConfigAdded
            }

            AlarmMsg::RemoveConfig { id, reply } => {
                let removed = if let Some(pos) = self.configs.iter().position(|c| c.id == id) {
                    self.configs.remove(pos);
                    true
                } else {
                    false
                };
                let _ = reply.send(removed);
                AlarmReply::ConfigAdded // Placeholder reply
            }

            AlarmMsg::GetConfigs { reply } => {
                let _ = reply.send(self.configs.clone());
                AlarmReply::ConfigAdded // Placeholder reply
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PointQuality;
    use kameo::actor::Spawn;

    #[tokio::test]
    async fn test_alarm_actor_lifecycle() {
        let mut actor_state = AlarmActor::new();

        // Add a high limit alarm config
        actor_state.add_config(AlarmConfig {
            id: Uuid::new_v4(),
            name: "High Temperature".to_string(),
            source_pattern: "*/temperature".to_string(),
            condition: AlarmCondition::HighLimit { value: 80.0 },
            severity: AlarmSeverity::High,
            delay_seconds: 0,
            message_template: "Temperature at {point} {threshold}: {value}".to_string(),
            enabled: true,
        });

        // Spawn the actor
        let actor = AlarmActor::spawn(actor_state);

        // Start service
        let reply = actor.ask(ServiceMsg::Start).await.unwrap();
        assert!(matches!(reply, ServiceReply::Started));

        // Check status
        let reply = actor.ask(ServiceMsg::GetStatus).await.unwrap();
        if let ServiceReply::Status { state, .. } = reply {
            assert_eq!(state, ServiceState::Running);
        } else {
            panic!("Expected Status reply");
        }

        // Send a normal value - should not trigger alarm
        let event = Event::PointValueChanged {
            point: "zone1/temperature".to_string(),
            value: PropertyValue::Real(72.0),
            quality: PointQuality::Good,
            timestamp: std::time::Instant::now(),
            timestamp_utc: Utc::now(),
        };
        let _ = actor.ask(ServiceMsg::OnEvent { event }).await;

        // Query active alarms
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = actor.ask(AlarmMsg::GetActiveAlarms { reply: reply_tx }).await;
        let active = reply_rx.await.unwrap();
        assert!(active.is_empty());

        // Send a high value - should trigger alarm
        let event = Event::PointValueChanged {
            point: "zone1/temperature".to_string(),
            value: PropertyValue::Real(85.0),
            quality: PointQuality::Good,
            timestamp: std::time::Instant::now(),
            timestamp_utc: Utc::now(),
        };
        let _ = actor.ask(ServiceMsg::OnEvent { event }).await;

        // Query active alarms again
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = actor.ask(AlarmMsg::GetActiveAlarms { reply: reply_tx }).await;
        let active = reply_rx.await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].severity, AlarmSeverity::High);
        assert_eq!(active[0].state, AlarmState::Active);

        // Acknowledge the alarm
        let alarm_id = active[0].id;
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = actor
            .ask(AlarmMsg::AcknowledgeAlarm {
                alarm_id,
                acknowledged_by: Some("operator1".to_string()),
                reply: reply_tx,
            })
            .await;
        let acked = reply_rx.await.unwrap();
        assert!(acked.is_some());
        assert_eq!(acked.unwrap().state, AlarmState::Acknowledged);

        // Stop service
        let reply = actor.ask(ServiceMsg::Stop).await.unwrap();
        assert!(matches!(reply, ServiceReply::Stopped));
    }

    #[test]
    fn test_alarm_conditions() {
        // High limit
        let cond = AlarmCondition::HighLimit { value: 80.0 };
        assert!(cond.evaluate(&PropertyValue::Real(85.0)));
        assert!(!cond.evaluate(&PropertyValue::Real(75.0)));

        // Low limit
        let cond = AlarmCondition::LowLimit { value: 50.0 };
        assert!(cond.evaluate(&PropertyValue::Real(45.0)));
        assert!(!cond.evaluate(&PropertyValue::Real(55.0)));

        // Out of range
        let cond = AlarmCondition::OutOfRange {
            low: 60.0,
            high: 80.0,
        };
        assert!(cond.evaluate(&PropertyValue::Real(55.0)));
        assert!(cond.evaluate(&PropertyValue::Real(85.0)));
        assert!(!cond.evaluate(&PropertyValue::Real(70.0)));

        // Boolean conditions
        let cond = AlarmCondition::IsTrue;
        assert!(cond.evaluate(&PropertyValue::Boolean(true)));
        assert!(!cond.evaluate(&PropertyValue::Boolean(false)));

        let cond = AlarmCondition::IsFalse;
        assert!(cond.evaluate(&PropertyValue::Boolean(false)));
        assert!(!cond.evaluate(&PropertyValue::Boolean(true)));
    }
}

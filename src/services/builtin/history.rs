// History Service - Time-series storage for point data
//
// Actor-based history service that stores point values in a redb database
// and provides querying capabilities.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use chrono::{DateTime, Utc};
use kameo::message::{Context, Message};
use redb::{Database, TableDefinition};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use crate::messages::Event;
use crate::services::actor::{ServiceMsg, ServiceReply, ServiceStateTracker};
use crate::services::messages::{HistorySample, ServiceRequest, ServiceResponse};
use crate::types::{Error, PointQuality, PointValue, Result, ServiceState};

// Table definition: key is "point_name:timestamp_micros", value is serialized sample
const HISTORY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("history");

/// Configuration for the History Service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Path to the database file
    pub db_path: String,
    /// Number of days to retain history
    pub retention_days: u32,
    /// Minimum interval between samples in milliseconds
    pub sample_interval_ms: u64,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            db_path: "./data/history.redb".to_string(),
            retention_days: 365,
            sample_interval_ms: 1000,
        }
    }
}

/// Stored sample format (compact for storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredSample {
    /// Timestamp in microseconds since epoch
    ts: i64,
    /// Point value
    v: PointValue,
    /// Quality indicator
    q: PointQuality,
}

impl From<StoredSample> for HistorySample {
    fn from(s: StoredSample) -> Self {
        HistorySample {
            timestamp: DateTime::from_timestamp_micros(s.ts).unwrap_or_else(|| Utc::now()),
            value: s.v,
            quality: s.q,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// History Actor Messages
// ─────────────────────────────────────────────────────────────────────────────

/// History-specific messages (beyond common ServiceMsg)
#[derive(Debug)]
pub enum HistoryMsg {
    /// Query history samples for a point
    QueryHistory {
        point: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: Option<u32>,
        reply: oneshot::Sender<Result<Vec<HistorySample>>>,
    },
    /// Get retention configuration
    GetRetention,
    /// Set retention days
    SetRetention { days: u32 },
}

/// Reply type for HistoryMsg
#[derive(Debug, kameo::Reply)]
pub enum HistoryReply {
    /// Query sent response via oneshot
    QuerySent,
    /// Retention days
    Retention { days: u32 },
    /// Retention updated
    RetentionSet,
}

// ─────────────────────────────────────────────────────────────────────────────
// History Actor
// ─────────────────────────────────────────────────────────────────────────────

/// Actor-based History Service
#[derive(kameo::Actor)]
pub struct HistoryActor {
    id: String,
    name: String,
    #[allow(dead_code)]
    description: String,
    state: ServiceStateTracker,
    config: HistoryConfig,
    db: Option<Database>,
    /// Rate limiting: track last sample time per point
    last_samples: HashMap<String, Instant>,
}

impl HistoryActor {
    /// Create a new HistoryActor with the given configuration
    pub fn new(config: HistoryConfig) -> Self {
        Self {
            id: "history".to_string(),
            name: "History Service".to_string(),
            description: "Stores time-series point data for trending and analysis".to_string(),
            state: ServiceStateTracker::new(),
            config,
            db: None,
            last_samples: HashMap::new(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(HistoryConfig::default())
    }

    /// Start the service (internal implementation)
    fn do_start(&mut self) -> Result<()> {
        self.state.set_starting();

        // Ensure parent directory exists
        let db_path = PathBuf::from(&self.config.db_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        // Open database
        let db = Database::create(&self.config.db_path).map_err(|e| {
            self.state.set_failed();
            Error::Database(format!("Failed to open database: {}", e))
        })?;

        // Ensure table exists
        let write_txn = db.begin_write().map_err(|e| Error::Database(e.to_string()))?;
        write_txn
            .open_table(HISTORY_TABLE)
            .map_err(|e| Error::Database(e.to_string()))?;
        write_txn
            .commit()
            .map_err(|e| Error::Database(e.to_string()))?;

        self.db = Some(db);
        self.state.set_running();

        tracing::info!(
            "History service started (db: {}, retention: {} days)",
            self.config.db_path,
            self.config.retention_days
        );

        Ok(())
    }

    /// Stop the service (internal implementation)
    fn do_stop(&mut self) {
        self.state.set_stopping();
        self.db = None;
        self.last_samples.clear();
        self.state.set_stopped();
        tracing::info!("History service stopped");
    }

    /// Store a point value
    fn store_sample(
        &mut self,
        point: &str,
        value: &PointValue,
        quality: PointQuality,
        timestamp: DateTime<Utc>,
    ) -> Result<()> {
        // Rate limiting check
        let now = Instant::now();
        if let Some(last) = self.last_samples.get(point) {
            if now.duration_since(*last).as_millis() < self.config.sample_interval_ms as u128 {
                return Ok(()); // Skip - too soon
            }
        }
        self.last_samples.insert(point.to_string(), now);

        let db = self
            .db
            .as_ref()
            .ok_or_else(|| Error::Service("Database not open".to_string()))?;

        let sample = StoredSample {
            ts: timestamp.timestamp_micros(),
            v: value.clone(),
            q: quality,
        };

        let key = format!("{}:{:020}", point, timestamp.timestamp_micros());
        let value_bytes =
            serde_json::to_vec(&sample).map_err(|e| Error::Service(e.to_string()))?;

        let write_txn = db.begin_write().map_err(|e| Error::Database(e.to_string()))?;
        {
            let mut table = write_txn
                .open_table(HISTORY_TABLE)
                .map_err(|e| Error::Database(e.to_string()))?;
            table
                .insert(key.as_str(), value_bytes.as_slice())
                .map_err(|e| Error::Database(e.to_string()))?;
        }
        write_txn
            .commit()
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Query samples for a point within a time range
    fn query_samples(
        &self,
        point: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: Option<u32>,
    ) -> Result<Vec<HistorySample>> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| Error::Service("Database not open".to_string()))?;

        let start_key = format!("{}:{:020}", point, start.timestamp_micros());
        let end_key = format!("{}:{:020}", point, end.timestamp_micros());

        let read_txn = db.begin_read().map_err(|e| Error::Database(e.to_string()))?;
        let table = read_txn
            .open_table(HISTORY_TABLE)
            .map_err(|e| Error::Database(e.to_string()))?;

        let mut samples = Vec::new();
        let limit = limit.unwrap_or(10000) as usize;

        let range = table
            .range(start_key.as_str()..=end_key.as_str())
            .map_err(|e| Error::Database(e.to_string()))?;

        for result in range {
            if samples.len() >= limit {
                break;
            }

            let (_, value) = result.map_err(|e| Error::Database(e.to_string()))?;
            let stored: StoredSample =
                serde_json::from_slice(value.value()).map_err(|e| Error::Service(e.to_string()))?;
            samples.push(stored.into());
        }

        Ok(samples)
    }
}

// Handle common ServiceMsg
impl Message<ServiceMsg> for HistoryActor {
    type Reply = ServiceReply;

    async fn handle(
        &mut self,
        msg: ServiceMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            ServiceMsg::Start => match self.do_start() {
                Ok(_) => ServiceReply::Started,
                Err(e) => ServiceReply::Failed(e.to_string()),
            },

            ServiceMsg::Stop => {
                self.do_stop();
                ServiceReply::Stopped
            }

            ServiceMsg::GetStatus => ServiceReply::Status {
                id: self.id.clone(),
                name: self.name.clone(),
                state: self.state.state(),
                uptime_secs: self.state.uptime_secs(),
                extra: Some(serde_json::json!({
                    "db_path": self.config.db_path,
                    "retention_days": self.config.retention_days,
                })),
            },

            ServiceMsg::GetConfig => ServiceReply::Config {
                config: serde_json::to_value(&self.config).unwrap_or_default(),
            },

            ServiceMsg::SetConfig { config } => {
                if let Ok(new_config) = serde_json::from_value::<HistoryConfig>(config) {
                    self.config = new_config;
                    ServiceReply::ConfigSet
                } else {
                    ServiceReply::Failed("Invalid configuration format".to_string())
                }
            }

            ServiceMsg::OnEvent { event } => {
                if self.state.state() != ServiceState::Running {
                    return ServiceReply::EventHandled;
                }

                if let Event::PointValueChanged {
                    point,
                    value,
                    quality,
                    timestamp_utc,
                    ..
                } = event
                {
                    if let Err(e) = self.store_sample(&point, &value, quality, timestamp_utc) {
                        tracing::warn!("Failed to store history sample: {}", e);
                    }
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
                            "db_path": self.config.db_path,
                            "retention_days": self.config.retention_days,
                        })),
                    },

                    ServiceRequest::GetConfig => ServiceResponse::Config {
                        config: serde_json::to_value(&self.config).unwrap_or_default(),
                    },

                    ServiceRequest::QueryHistory {
                        point,
                        start,
                        end,
                        limit,
                    } => match self.query_samples(&point, start, end, limit) {
                        Ok(samples) => ServiceResponse::HistoryData { samples },
                        Err(e) => ServiceResponse::Error {
                            code: "QUERY_FAILED".to_string(),
                            message: e.to_string(),
                        },
                    },

                    _ => ServiceResponse::Error {
                        code: "UNSUPPORTED".to_string(),
                        message: "Request not supported by History service".to_string(),
                    },
                };

                let _ = reply.send(response);
                ServiceReply::RequestHandled
            }
        }
    }
}

// Handle history-specific messages
impl Message<HistoryMsg> for HistoryActor {
    type Reply = HistoryReply;

    async fn handle(
        &mut self,
        msg: HistoryMsg,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            HistoryMsg::QueryHistory {
                point,
                start,
                end,
                limit,
                reply,
            } => {
                let result = self.query_samples(&point, start, end, limit);
                let _ = reply.send(result);
                HistoryReply::QuerySent
            }

            HistoryMsg::GetRetention => HistoryReply::Retention {
                days: self.config.retention_days,
            },

            HistoryMsg::SetRetention { days } => {
                self.config.retention_days = days;
                HistoryReply::RetentionSet
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
    use kameo::actor::Spawn;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_history_service_lifecycle() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_history.redb");

        let config = HistoryConfig {
            db_path: db_path.to_string_lossy().to_string(),
            retention_days: 30,
            sample_interval_ms: 100,
        };

        let actor = HistoryActor::spawn(HistoryActor::new(config));

        // Start
        let reply = actor.ask(ServiceMsg::Start).await.unwrap();
        assert!(matches!(reply, ServiceReply::Started));

        // Check status
        let reply = actor.ask(ServiceMsg::GetStatus).await.unwrap();
        if let ServiceReply::Status { state, .. } = reply {
            assert_eq!(state, ServiceState::Running);
        } else {
            panic!("Expected Status reply");
        }

        // Store a sample via event
        let event = Event::PointValueChanged {
            point: "test/point/1".to_string(),
            value: PointValue::Real(72.5),
            quality: PointQuality::Good,
            timestamp: std::time::Instant::now(),
            timestamp_utc: Utc::now(),
        };
        let _ = actor.ask(ServiceMsg::OnEvent { event }).await;

        // Query it back
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = actor
            .ask(HistoryMsg::QueryHistory {
                point: "test/point/1".to_string(),
                start: Utc::now() - chrono::Duration::hours(1),
                end: Utc::now() + chrono::Duration::hours(1),
                limit: None,
                reply: reply_tx,
            })
            .await;

        let samples = reply_rx.await.unwrap().unwrap();
        assert_eq!(samples.len(), 1);
        assert!(matches!(samples[0].value, PointValue::Real(v) if (v - 72.5).abs() < 0.01));

        // Stop
        let reply = actor.ask(ServiceMsg::Stop).await.unwrap();
        assert!(matches!(reply, ServiceReply::Stopped));
    }
}

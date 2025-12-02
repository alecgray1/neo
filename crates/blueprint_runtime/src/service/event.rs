//! Service Events
//!
//! Events are the primary communication mechanism between services.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use super::ServiceError;

// ─────────────────────────────────────────────────────────────────────────────
// Event
// ─────────────────────────────────────────────────────────────────────────────

/// An event that can be published and received by services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event type identifier (e.g., "PointValueChanged", "DeviceConnected")
    pub event_type: String,

    /// Source service or system that generated the event
    pub source: String,

    /// Event payload data
    pub data: serde_json::Value,

    /// Timestamp when event was created (Unix milliseconds)
    pub timestamp: u64,
}

impl Event {
    /// Create a new event
    pub fn new(
        event_type: impl Into<String>,
        source: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            source: source.into(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Create a new event with the current timestamp
    pub fn now(event_type: impl Into<String>, source: impl Into<String>) -> Self {
        Self::new(event_type, source, serde_json::Value::Null)
    }

    /// Create an event with object data
    pub fn with_data<T: Serialize>(
        event_type: impl Into<String>,
        source: impl Into<String>,
        data: &T,
    ) -> Result<Self, serde_json::Error> {
        Ok(Self::new(
            event_type,
            source,
            serde_json::to_value(data)?,
        ))
    }

    /// Check if this event matches a subscription pattern
    ///
    /// Patterns support:
    /// - Exact match: "PointValueChanged" matches "PointValueChanged"
    /// - Wildcard suffix: "Device/*" matches "Device/Connected", "Device/Disconnected"
    /// - Global wildcard: "*" matches everything
    pub fn matches(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 1]; // Keep the trailing slash: "Device/"
            return self.event_type.starts_with(prefix);
        }

        self.event_type == pattern
    }

    /// Get a field from the event data
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Get a string field from the event data
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    /// Get a number field from the event data
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.data.get(key).and_then(|v| v.as_f64())
    }

    /// Get a boolean field from the event data
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(|v| v.as_bool())
    }

    /// Deserialize the event data to a specific type
    pub fn parse_data<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Event Publisher
// ─────────────────────────────────────────────────────────────────────────────

/// A handle for publishing events
#[derive(Clone)]
pub struct EventPublisher {
    tx: broadcast::Sender<Event>,
}

impl EventPublisher {
    /// Create a new event publisher
    pub fn new(tx: broadcast::Sender<Event>) -> Self {
        Self { tx }
    }

    /// Publish an event
    pub fn publish(&self, event: Event) -> Result<(), super::ServiceError> {
        self.tx
            .send(event)
            .map(|_| ())
            .map_err(|_| ServiceError::ChannelClosed)
    }

    /// Create and publish an event
    pub fn emit(
        &self,
        event_type: impl Into<String>,
        source: impl Into<String>,
        data: serde_json::Value,
    ) -> Result<(), super::ServiceError> {
        self.publish(Event::new(event_type, source, data))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            "TestEvent",
            "test-service",
            serde_json::json!({"value": 42}),
        );

        assert_eq!(event.event_type, "TestEvent");
        assert_eq!(event.source, "test-service");
        assert_eq!(event.get_f64("value"), Some(42.0));
    }

    #[test]
    fn test_event_matching() {
        let event = Event::now("Device/Connected", "system");

        // Exact match
        assert!(event.matches("Device/Connected"));
        assert!(!event.matches("Device/Disconnected"));

        // Wildcard
        assert!(event.matches("*"));
        assert!(event.matches("Device/*"));
        assert!(!event.matches("Other/*"));

        // Prefix match edge cases
        let event2 = Event::now("Device", "system");
        assert!(!event2.matches("Device/*")); // "Device" should not match "Device/*"
        assert!(event2.matches("Device"));
    }

    #[test]
    fn test_event_data_parsing() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let event = Event::new(
            "TestEvent",
            "source",
            serde_json::json!({"name": "test", "value": 123}),
        );

        let data: TestData = event.parse_data().unwrap();
        assert_eq!(data.name, "test");
        assert_eq!(data.value, 123);
    }
}

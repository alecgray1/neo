use crate::messages::{Event, PubSubMsg};
use crate::types::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, trace};
use wildmatch::WildMatch;

/// Central pub-sub broker for event distribution
#[derive(kameo::Actor)]
pub struct PubSubBroker {
    /// Topic pattern -> broadcast channel
    channels: DashMap<String, broadcast::Sender<Event>>,

    /// Statistics
    total_messages_published: Arc<std::sync::atomic::AtomicU64>,
    total_subscribers: Arc<std::sync::atomic::AtomicUsize>,
}

impl PubSubBroker {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
            total_messages_published: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_subscribers: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Publish an event to a topic
    pub fn publish(&self, topic: &str, event: Event) -> Result<usize> {
        debug!("Publishing to topic '{}': {:?}", topic, event);

        let mut notified = 0;

        // Send to all matching topic patterns
        for entry in self.channels.iter() {
            let pattern = entry.key();

            if Self::topic_matches(topic, pattern) {
                let sender = entry.value();
                // broadcast::send returns number of receivers
                match sender.send(event.clone()) {
                    Ok(n) => notified += n,
                    Err(_) => {
                        // No receivers, that's ok
                    }
                }
            }
        }

        self.total_messages_published
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        trace!("Published to topic '{}', notified {} subscribers", topic, notified);

        Ok(notified)
    }

    /// Subscribe to a topic pattern
    pub fn subscribe(&self, topic_pattern: &str) -> broadcast::Receiver<Event> {
        let sender = self
            .channels
            .entry(topic_pattern.to_string())
            .or_insert_with(|| {
                // Create channel with buffer of 100 events
                let (tx, _) = broadcast::channel(100);
                tx
            })
            .clone();

        self.total_subscribers
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        debug!("New subscription to topic pattern '{}'", topic_pattern);

        sender.subscribe()
    }

    /// Check if a topic matches a pattern (MQTT-style wildcards)
    /// Examples:
    ///   topic: "points/VAV-101/temp"
    ///   pattern: "points/+/temp"  -> matches (+ is single level)
    ///   pattern: "points/#"       -> matches (# is multi-level)
    ///   pattern: "points/VAV-101/temp" -> matches (exact)
    fn topic_matches(topic: &str, pattern: &str) -> bool {
        if pattern.contains('#') || pattern.contains('+') {
            // Convert MQTT wildcards to glob wildcards
            let glob_pattern = pattern
                .replace('+', "*") // + matches one level
                .replace('#', "**"); // # matches multiple levels

            WildMatch::new(&glob_pattern).matches(topic)
        } else {
            // Exact match
            topic == pattern
        }
    }

    pub fn get_stats(&self) -> PubSubStats {
        PubSubStats {
            total_messages: self
                .total_messages_published
                .load(std::sync::atomic::Ordering::Relaxed),
            total_subscribers: self
                .total_subscribers
                .load(std::sync::atomic::Ordering::Relaxed),
            topics_count: self.channels.len(),
        }
    }
}

impl kameo::message::Message<PubSubMsg> for PubSubBroker {
    type Reply = PubSubReply;

    async fn handle(
        &mut self,
        msg: PubSubMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            PubSubMsg::Publish { topic, event } => {
                let count = self.publish(&topic, event).unwrap_or(0);
                PubSubReply::Published { count }
            }

            PubSubMsg::Subscribe {
                topic_pattern,
                subscriber_id: _,
            } => {
                let _receiver = self.subscribe(&topic_pattern);
                // In a real implementation, we'd track the subscriber_id
                // and return a receiver handle
                PubSubReply::Subscribed
            }

            PubSubMsg::Unsubscribe {
                topic_pattern: _,
                subscriber_id: _,
            } => {
                // TODO: Implement unsubscribe
                PubSubReply::Unsubscribed
            }

            PubSubMsg::GetStats => {
                let stats = self.get_stats();
                PubSubReply::Stats(stats)
            }
        }
    }
}

#[derive(Debug, kameo::Reply)]
pub enum PubSubReply {
    Published { count: usize },
    Subscribed,
    Unsubscribed,
    Stats(PubSubStats),
}

#[derive(Debug, Clone)]
pub struct PubSubStats {
    pub total_messages: u64,
    pub total_subscribers: usize,
    pub topics_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_matching() {
        assert!(PubSubBroker::topic_matches(
            "points/VAV-101/temp",
            "points/VAV-101/temp"
        ));
        assert!(PubSubBroker::topic_matches("points/VAV-101/temp", "points/+/temp"));
        assert!(PubSubBroker::topic_matches("points/VAV-101/temp", "points/#"));
        assert!(PubSubBroker::topic_matches("points/VAV-101/temp/value", "points/#"));
        assert!(!PubSubBroker::topic_matches("points/VAV-101/temp", "devices/#"));
        assert!(!PubSubBroker::topic_matches("points/VAV-101/temp", "points/+/humidity"));
    }
}

//! Pub/sub event bus (task 2.8, 2.9).
//!
//! In-memory broadcast with optional persistence hooks.

use crate::models::Event;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};
use ulid::Ulid;
use chrono::Utc;

/// Channel capacity per topic.
const TOPIC_CAPACITY: usize = 256;

/// The event bus — topic-keyed broadcast channels.
pub struct EventBus {
    topics: Mutex<HashMap<String, broadcast::Sender<Event>>>,
    /// Monotonic sequence counter.
    seq:    Mutex<u64>,
}

impl EventBus {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            topics: Mutex::new(HashMap::new()),
            seq:    Mutex::new(0),
        })
    }

    /// Publish an event to a topic.
    pub async fn publish(&self, topic: &str, event_type: &str, source: &str, payload: serde_json::Value) -> Event {
        let mut seq = self.seq.lock().await;
        *seq += 1;
        let event = Event {
            id:         Ulid::new().to_string(),
            topic:      topic.into(),
            event_type: event_type.into(),
            payload,
            source:     source.into(),
            sequence:   *seq,
            created_at: Utc::now(),
        };
        drop(seq);

        let mut topics = self.topics.lock().await;
        if let Some(tx) = topics.get(topic) {
            match tx.send(event.clone()) {
                Ok(n) => debug!(topic, seq = event.sequence, receivers = n, "event published"),
                Err(_) => warn!(topic, "no active subscribers"),
            }
        }
        event
    }

    /// Subscribe to a topic. Returns a receiver.
    pub async fn subscribe(&self, topic: &str) -> broadcast::Receiver<Event> {
        let mut topics = self.topics.lock().await;
        let tx = topics.entry(topic.into())
            .or_insert_with(|| broadcast::channel(TOPIC_CAPACITY).0);
        tx.subscribe()
    }

    /// Subscribe to all events via a wildcard channel.
    pub async fn subscribe_all(&self) -> broadcast::Receiver<Event> {
        self.subscribe("*").await
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self {
            topics: Mutex::new(HashMap::new()),
            seq:    Mutex::new(0),
        }
    }
}

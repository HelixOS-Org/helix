//! # Cognitive Message Bus
//!
//! High-performance message bus for inter-domain communication.
//! Supports pub/sub, request/response, and broadcast patterns.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// MESSAGE TYPES
// ============================================================================

/// A message on the bus
#[derive(Debug, Clone)]
pub struct BusMessage {
    /// Unique message ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Destination (None = broadcast)
    pub destination: Option<DomainId>,
    /// Topic
    pub topic: MessageTopic,
    /// Message type
    pub msg_type: MessageType,
    /// Priority
    pub priority: MessagePriority,
    /// Payload
    pub payload: MessagePayload,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Correlation ID (for request/response)
    pub correlation_id: Option<u64>,
    /// TTL (cycles)
    pub ttl: u64,
}

/// Message topic
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageTopic {
    /// Signals from SENSE
    Signals,
    /// Patterns from UNDERSTAND
    Patterns,
    /// Causal chains from REASON
    Causality,
    /// Options from DECIDE
    Decisions,
    /// Effects from ACT
    Actions,
    /// Memories from LTM
    Memory,
    /// Insights from REFLECT
    Insights,
    /// Learnings from LEARN
    Learning,
    /// System events
    System,
    /// Health updates
    Health,
    /// Error notifications
    Errors,
    /// Custom topic
    Custom(String),
}

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Data push
    Data,
    /// Request for data
    Request,
    /// Response to request
    Response,
    /// Command
    Command,
    /// Event notification
    Event,
    /// Heartbeat
    Heartbeat,
    /// Error
    Error,
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MessagePriority {
    Low      = 0,
    Normal   = 1,
    High     = 2,
    Critical = 3,
}

/// Message payload
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// Empty payload
    Empty,
    /// Binary data
    Bytes(Vec<u8>),
    /// Text data
    Text(String),
    /// Numeric value
    Number(f64),
    /// Key-value pairs
    KeyValue(BTreeMap<String, String>),
    /// Array of values
    Array(Vec<MessagePayload>),
}

// ============================================================================
// SUBSCRIPTION
// ============================================================================

/// Subscription to a topic
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Subscription ID
    pub id: u64,
    /// Subscriber domain
    pub subscriber: DomainId,
    /// Subscribed topic
    pub topic: MessageTopic,
    /// Filter (optional)
    pub filter: Option<MessageFilter>,
    /// Created timestamp
    pub created: Timestamp,
    /// Message count
    pub message_count: u64,
}

/// Message filter
#[derive(Debug, Clone)]
pub struct MessageFilter {
    /// Minimum priority
    pub min_priority: Option<MessagePriority>,
    /// Message types to include
    pub msg_types: Option<Vec<MessageType>>,
    /// Source domains to include
    pub sources: Option<Vec<DomainId>>,
}

impl MessageFilter {
    /// Check if message matches filter
    pub fn matches(&self, msg: &BusMessage) -> bool {
        if let Some(min_pri) = self.min_priority {
            if msg.priority < min_pri {
                return false;
            }
        }

        if let Some(ref types) = self.msg_types {
            if !types.contains(&msg.msg_type) {
                return false;
            }
        }

        if let Some(ref sources) = self.sources {
            if !sources.contains(&msg.source) {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// MESSAGE BUS
// ============================================================================

/// High-performance message bus
pub struct MessageBus {
    /// Messages in transit
    messages: Vec<BusMessage>,
    /// Subscriptions by topic
    subscriptions: BTreeMap<MessageTopic, Vec<Subscription>>,
    /// Pending responses
    pending_responses: BTreeMap<u64, PendingRequest>,
    /// Next message ID
    next_msg_id: AtomicU64,
    /// Next subscription ID
    next_sub_id: AtomicU64,
    /// Current cycle
    current_cycle: u64,
    /// Configuration
    config: BusConfig,
    /// Statistics
    stats: BusStats,
}

/// Pending request awaiting response
#[derive(Debug)]
struct PendingRequest {
    /// Request message ID
    request_id: u64,
    /// Requester domain
    requester: DomainId,
    /// Timestamp sent
    sent: Timestamp,
    /// Timeout (cycles)
    timeout: u64,
}

/// Bus configuration
#[derive(Debug, Clone)]
pub struct BusConfig {
    /// Maximum messages in queue
    pub max_messages: usize,
    /// Maximum subscriptions per topic
    pub max_subscriptions_per_topic: usize,
    /// Default TTL (cycles)
    pub default_ttl: u64,
    /// Request timeout (cycles)
    pub request_timeout: u64,
    /// Enable message ordering
    pub ordered_delivery: bool,
}

impl Default for BusConfig {
    fn default() -> Self {
        Self {
            max_messages: 10000,
            max_subscriptions_per_topic: 100,
            default_ttl: 100,
            request_timeout: 50,
            ordered_delivery: true,
        }
    }
}

/// Bus statistics
#[derive(Debug, Clone, Default)]
pub struct BusStats {
    /// Total messages sent
    pub total_sent: u64,
    /// Total messages delivered
    pub total_delivered: u64,
    /// Total messages expired
    pub total_expired: u64,
    /// Total requests
    pub total_requests: u64,
    /// Total responses
    pub total_responses: u64,
    /// Timeout count
    pub timeouts: u64,
    /// Current queue depth
    pub queue_depth: u64,
    /// Peak queue depth
    pub peak_queue_depth: u64,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new(config: BusConfig) -> Self {
        Self {
            messages: Vec::new(),
            subscriptions: BTreeMap::new(),
            pending_responses: BTreeMap::new(),
            next_msg_id: AtomicU64::new(1),
            next_sub_id: AtomicU64::new(1),
            current_cycle: 0,
            config,
            stats: BusStats::default(),
        }
    }

    /// Subscribe to a topic
    pub fn subscribe(
        &mut self,
        subscriber: DomainId,
        topic: MessageTopic,
        filter: Option<MessageFilter>,
    ) -> u64 {
        let id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);

        let subscription = Subscription {
            id,
            subscriber,
            topic: topic.clone(),
            filter,
            created: Timestamp::now(),
            message_count: 0,
        };

        self.subscriptions
            .entry(topic)
            .or_default()
            .push(subscription);

        id
    }

    /// Unsubscribe
    pub fn unsubscribe(&mut self, sub_id: u64) -> bool {
        for subs in self.subscriptions.values_mut() {
            if let Some(pos) = subs.iter().position(|s| s.id == sub_id) {
                subs.remove(pos);
                return true;
            }
        }
        false
    }

    /// Publish a message
    pub fn publish(&mut self, mut msg: BusMessage) -> u64 {
        msg.id = self.next_msg_id.fetch_add(1, Ordering::Relaxed);

        if msg.ttl == 0 {
            msg.ttl = self.config.default_ttl;
        }

        let id = msg.id;

        // Check queue capacity
        if self.messages.len() >= self.config.max_messages {
            self.evict_oldest();
        }

        self.messages.push(msg);
        self.stats.total_sent += 1;
        self.stats.queue_depth = self.messages.len() as u64;

        if self.stats.queue_depth > self.stats.peak_queue_depth {
            self.stats.peak_queue_depth = self.stats.queue_depth;
        }

        id
    }

    /// Send a request and expect a response
    pub fn request(&mut self, mut msg: BusMessage) -> u64 {
        msg.msg_type = MessageType::Request;
        let id = self.publish(msg);

        self.pending_responses.insert(id, PendingRequest {
            request_id: id,
            requester: DomainId::new(0), // Will be filled
            sent: Timestamp::now(),
            timeout: self.config.request_timeout,
        });

        self.stats.total_requests += 1;
        id
    }

    /// Respond to a request
    pub fn respond(&mut self, request_id: u64, mut response: BusMessage) {
        response.msg_type = MessageType::Response;
        response.correlation_id = Some(request_id);

        self.publish(response);
        self.pending_responses.remove(&request_id);
        self.stats.total_responses += 1;
    }

    /// Get messages for a domain
    pub fn receive(&mut self, domain: DomainId) -> Vec<BusMessage> {
        let mut result = Vec::new();

        // Get direct messages
        let direct: Vec<_> = self
            .messages
            .iter()
            .filter(|m| m.destination == Some(domain))
            .cloned()
            .collect();
        result.extend(direct);

        // Get messages matching subscriptions
        for (topic, subs) in &mut self.subscriptions {
            for sub in subs {
                if sub.subscriber != domain {
                    continue;
                }

                let matching: Vec<_> = self
                    .messages
                    .iter()
                    .filter(|m| {
                        m.topic == *topic
                            && m.destination.is_none()
                            && sub.filter.as_ref().map(|f| f.matches(m)).unwrap_or(true)
                    })
                    .cloned()
                    .collect();

                sub.message_count += matching.len() as u64;
                result.extend(matching);
            }
        }

        // Remove delivered messages
        self.messages.retain(|m| {
            !(m.destination == Some(domain)
                || (m.destination.is_none() && self.is_subscribed(domain, &m.topic)))
        });

        self.stats.total_delivered += result.len() as u64;
        self.stats.queue_depth = self.messages.len() as u64;

        result
    }

    /// Check if domain is subscribed to topic
    fn is_subscribed(&self, domain: DomainId, topic: &MessageTopic) -> bool {
        self.subscriptions
            .get(topic)
            .map(|subs| subs.iter().any(|s| s.subscriber == domain))
            .unwrap_or(false)
    }

    /// Process tick - handle TTL and timeouts
    pub fn tick(&mut self) {
        self.current_cycle += 1;

        // Expire messages
        let before = self.messages.len();
        self.messages.retain(|m| {
            let age = self.current_cycle - m.timestamp.as_cycles();
            age < m.ttl
        });
        self.stats.total_expired += (before - self.messages.len()) as u64;

        // Handle request timeouts
        let mut timed_out = Vec::new();
        for (id, pending) in &self.pending_responses {
            let age = self.current_cycle - pending.sent.as_cycles();
            if age >= pending.timeout {
                timed_out.push(*id);
            }
        }

        for id in timed_out {
            self.pending_responses.remove(&id);
            self.stats.timeouts += 1;
        }

        self.stats.queue_depth = self.messages.len() as u64;
    }

    /// Evict oldest message
    fn evict_oldest(&mut self) {
        if !self.messages.is_empty() {
            // Find lowest priority first
            let idx = self
                .messages
                .iter()
                .enumerate()
                .min_by_key(|(_, m)| (m.priority as u8, m.timestamp.raw()))
                .map(|(i, _)| i);

            if let Some(i) = idx {
                self.messages.remove(i);
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &BusStats {
        &self.stats
    }

    /// Get queue depth
    pub fn queue_depth(&self) -> usize {
        self.messages.len()
    }

    /// Get subscription count
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.values().map(|s| s.len()).sum()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_receive() {
        let config = BusConfig::default();
        let mut bus = MessageBus::new(config);

        let domain = DomainId::new(1);

        let msg = BusMessage {
            id: 0,
            source: DomainId::new(0),
            destination: Some(domain),
            topic: MessageTopic::System,
            msg_type: MessageType::Data,
            priority: MessagePriority::Normal,
            payload: MessagePayload::Text("test".into()),
            timestamp: Timestamp::now(),
            correlation_id: None,
            ttl: 100,
        };

        bus.publish(msg);

        let received = bus.receive(domain);
        assert_eq!(received.len(), 1);
    }

    #[test]
    fn test_subscription() {
        let config = BusConfig::default();
        let mut bus = MessageBus::new(config);

        let sub_domain = DomainId::new(1);
        let pub_domain = DomainId::new(2);

        bus.subscribe(sub_domain, MessageTopic::Signals, None);

        let msg = BusMessage {
            id: 0,
            source: pub_domain,
            destination: None, // Broadcast
            topic: MessageTopic::Signals,
            msg_type: MessageType::Data,
            priority: MessagePriority::Normal,
            payload: MessagePayload::Empty,
            timestamp: Timestamp::now(),
            correlation_id: None,
            ttl: 100,
        };

        bus.publish(msg);

        let received = bus.receive(sub_domain);
        assert_eq!(received.len(), 1);
    }

    #[test]
    fn test_priority_eviction() {
        let mut config = BusConfig::default();
        config.max_messages = 2;
        let mut bus = MessageBus::new(config);

        // Publish low priority
        bus.publish(BusMessage {
            id: 0,
            source: DomainId::new(0),
            destination: Some(DomainId::new(1)),
            topic: MessageTopic::System,
            msg_type: MessageType::Data,
            priority: MessagePriority::Low,
            payload: MessagePayload::Empty,
            timestamp: Timestamp::now(),
            correlation_id: None,
            ttl: 100,
        });

        // Publish high priority
        bus.publish(BusMessage {
            id: 0,
            source: DomainId::new(0),
            destination: Some(DomainId::new(1)),
            topic: MessageTopic::System,
            msg_type: MessageType::Data,
            priority: MessagePriority::High,
            payload: MessagePayload::Empty,
            timestamp: Timestamp::now(),
            correlation_id: None,
            ttl: 100,
        });

        // Publish another - should evict low priority
        bus.publish(BusMessage {
            id: 0,
            source: DomainId::new(0),
            destination: Some(DomainId::new(1)),
            topic: MessageTopic::System,
            msg_type: MessageType::Data,
            priority: MessagePriority::Normal,
            payload: MessagePayload::Empty,
            timestamp: Timestamp::now(),
            correlation_id: None,
            ttl: 100,
        });

        assert!(bus.queue_depth() <= 2);
    }
}

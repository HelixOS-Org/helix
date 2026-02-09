//! # Cognitive Message Bus
//!
//! Message passing infrastructure for cognitive domains.
//! Supports publish-subscribe, request-reply, and streaming patterns.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// MESSAGE TYPES
// ============================================================================

/// A cognitive message
#[derive(Debug, Clone)]
pub struct Message {
    /// Message ID
    pub id: u64,
    /// Message type
    pub msg_type: MessageType,
    /// Topic/channel
    pub topic: String,
    /// Sender
    pub sender: DomainId,
    /// Recipient (None for broadcast)
    pub recipient: Option<DomainId>,
    /// Payload
    pub payload: MessagePayload,
    /// Headers
    pub headers: BTreeMap<String, String>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Reply-to message ID
    pub reply_to: Option<u64>,
    /// Correlation ID
    pub correlation_id: Option<u64>,
    /// TTL (time to live in ns)
    pub ttl_ns: Option<u64>,
    /// Priority
    pub priority: MessagePriority,
}

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Event notification
    Event,
    /// Command/action
    Command,
    /// Query/request
    Query,
    /// Reply/response
    Reply,
    /// Error
    Error,
    /// Heartbeat
    Heartbeat,
    /// Control
    Control,
}

/// Message payload
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// Empty
    Empty,
    /// Binary data
    Binary(Vec<u8>),
    /// Text
    Text(String),
    /// Structured data
    Structured(BTreeMap<String, PayloadValue>),
}

/// Payload value
#[derive(Debug, Clone)]
pub enum PayloadValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<PayloadValue>),
    Map(BTreeMap<String, PayloadValue>),
    Binary(Vec<u8>),
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

// ============================================================================
// SUBSCRIPTION
// ============================================================================

/// Subscription
#[derive(Debug, Clone)]
pub struct Subscription {
    /// Subscription ID
    pub id: u64,
    /// Subscriber domain
    pub subscriber: DomainId,
    /// Topic pattern
    pub topic_pattern: TopicPattern,
    /// Filter
    pub filter: Option<MessageFilter>,
    /// Created time
    pub created: Timestamp,
    /// Active
    pub active: bool,
}

/// Topic pattern
#[derive(Debug, Clone)]
pub struct TopicPattern {
    /// Pattern string
    pub pattern: String,
    /// Regex-like matching
    pub is_wildcard: bool,
}

impl TopicPattern {
    /// Create exact match pattern
    #[inline]
    pub fn exact(topic: &str) -> Self {
        Self {
            pattern: topic.into(),
            is_wildcard: false,
        }
    }

    /// Create prefix pattern (topic.*)
    #[inline]
    pub fn prefix(prefix: &str) -> Self {
        Self {
            pattern: format!("{}.*", prefix),
            is_wildcard: true,
        }
    }

    /// Create all pattern (*)
    #[inline]
    pub fn all() -> Self {
        Self {
            pattern: "*".into(),
            is_wildcard: true,
        }
    }

    /// Check if pattern matches topic
    pub fn matches(&self, topic: &str) -> bool {
        if !self.is_wildcard {
            self.pattern == topic
        } else if self.pattern == "*" {
            true
        } else if self.pattern.ends_with(".*") {
            let prefix = &self.pattern[..self.pattern.len() - 2];
            topic.starts_with(prefix)
        } else {
            self.pattern == topic
        }
    }
}

/// Message filter
#[derive(Debug, Clone)]
pub struct MessageFilter {
    /// Required message types
    pub msg_types: Option<Vec<MessageType>>,
    /// Minimum priority
    pub min_priority: Option<MessagePriority>,
    /// Required headers
    pub required_headers: Vec<String>,
    /// Header filters
    pub header_filters: BTreeMap<String, String>,
}

impl MessageFilter {
    /// Check if message passes filter
    pub fn matches(&self, msg: &Message) -> bool {
        // Check message type
        if let Some(types) = &self.msg_types {
            if !types.contains(&msg.msg_type) {
                return false;
            }
        }

        // Check priority
        if let Some(min) = self.min_priority {
            if msg.priority < min {
                return false;
            }
        }

        // Check required headers
        for header in &self.required_headers {
            if !msg.headers.contains_key(header) {
                return false;
            }
        }

        // Check header values
        for (key, expected) in &self.header_filters {
            if msg.headers.get(key) != Some(expected) {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// MESSAGE BUS
// ============================================================================

/// Message bus
pub struct MessageBus {
    /// Subscriptions
    subscriptions: BTreeMap<u64, Subscription>,
    /// Subscriptions by topic
    by_topic: BTreeMap<String, Vec<u64>>,
    /// Pending messages
    pending: VecDeque<Message>,
    /// Message history
    history: VecDeque<Message>,
    /// Next message ID
    next_message_id: AtomicU64,
    /// Next subscription ID
    next_sub_id: AtomicU64,
    /// Pending requests (waiting for reply)
    pending_requests: BTreeMap<u64, PendingRequest>,
    /// Configuration
    config: MessageBusConfig,
    /// Statistics
    stats: MessageBusStats,
}

/// Pending request
#[derive(Debug)]
struct PendingRequest {
    /// Request message ID
    message_id: u64,
    /// Requester domain
    requester: DomainId,
    /// Timestamp
    timestamp: Timestamp,
    /// Timeout (ns)
    timeout_ns: u64,
}

/// Message bus configuration
#[derive(Debug, Clone)]
pub struct MessageBusConfig {
    /// Maximum pending messages
    pub max_pending: usize,
    /// Maximum history
    pub max_history: usize,
    /// Default TTL (ns)
    pub default_ttl_ns: u64,
    /// Request timeout (ns)
    pub request_timeout_ns: u64,
    /// Maximum message size
    pub max_message_size: usize,
}

impl Default for MessageBusConfig {
    fn default() -> Self {
        Self {
            max_pending: 10000,
            max_history: 1000,
            default_ttl_ns: 60_000_000_000,    // 1 minute
            request_timeout_ns: 30_000_000_000, // 30 seconds
            max_message_size: 1048576,          // 1MB
        }
    }
}

/// Message bus statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MessageBusStats {
    /// Total messages published
    pub total_published: u64,
    /// Total messages delivered
    pub total_delivered: u64,
    /// Total messages expired
    pub total_expired: u64,
    /// Active subscriptions
    pub active_subscriptions: u64,
    /// Pending requests
    pub pending_requests: u64,
    /// Average delivery latency (ns)
    pub avg_latency_ns: f64,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new(config: MessageBusConfig) -> Self {
        Self {
            subscriptions: BTreeMap::new(),
            by_topic: BTreeMap::new(),
            pending: VecDeque::new(),
            history: VecDeque::new(),
            next_message_id: AtomicU64::new(1),
            next_sub_id: AtomicU64::new(1),
            pending_requests: BTreeMap::new(),
            config,
            stats: MessageBusStats::default(),
        }
    }

    /// Subscribe to topic
    pub fn subscribe(
        &mut self,
        subscriber: DomainId,
        pattern: TopicPattern,
        filter: Option<MessageFilter>,
    ) -> u64 {
        let id = self.next_sub_id.fetch_add(1, Ordering::Relaxed);

        let subscription = Subscription {
            id,
            subscriber,
            topic_pattern: pattern.clone(),
            filter,
            created: Timestamp::now(),
            active: true,
        };

        self.subscriptions.insert(id, subscription);

        // Index by topic (for exact match)
        if !pattern.is_wildcard {
            self.by_topic
                .entry(pattern.pattern.clone())
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.stats.active_subscriptions = self.subscriptions.len() as u64;
        id
    }

    /// Unsubscribe
    pub fn unsubscribe(&mut self, subscription_id: u64) -> bool {
        if let Some(sub) = self.subscriptions.remove(&subscription_id) {
            if !sub.topic_pattern.is_wildcard {
                if let Some(subs) = self.by_topic.get_mut(&sub.topic_pattern.pattern) {
                    subs.retain(|&id| id != subscription_id);
                }
            }
            self.stats.active_subscriptions = self.subscriptions.len() as u64;
            true
        } else {
            false
        }
    }

    /// Publish message
    pub fn publish(&mut self, message: Message) -> u64 {
        let id = message.id;
        self.stats.total_published += 1;
        self.pending.push_back(message);

        // Limit pending
        while self.pending.len() > self.config.max_pending {
            self.pending.pop_front();
            self.stats.total_expired += 1;
        }

        id
    }

    /// Create and publish a message
    pub fn send(
        &mut self,
        sender: DomainId,
        topic: &str,
        msg_type: MessageType,
        payload: MessagePayload,
    ) -> u64 {
        let id = self.next_message_id.fetch_add(1, Ordering::Relaxed);

        let message = Message {
            id,
            msg_type,
            topic: topic.into(),
            sender,
            recipient: None,
            payload,
            headers: BTreeMap::new(),
            timestamp: Timestamp::now(),
            reply_to: None,
            correlation_id: None,
            ttl_ns: Some(self.config.default_ttl_ns),
            priority: MessagePriority::Normal,
        };

        self.publish(message)
    }

    /// Send direct message to recipient
    pub fn send_direct(
        &mut self,
        sender: DomainId,
        recipient: DomainId,
        msg_type: MessageType,
        payload: MessagePayload,
    ) -> u64 {
        let id = self.next_message_id.fetch_add(1, Ordering::Relaxed);

        let message = Message {
            id,
            msg_type,
            topic: String::new(),
            sender,
            recipient: Some(recipient),
            payload,
            headers: BTreeMap::new(),
            timestamp: Timestamp::now(),
            reply_to: None,
            correlation_id: None,
            ttl_ns: Some(self.config.default_ttl_ns),
            priority: MessagePriority::Normal,
        };

        self.publish(message)
    }

    /// Request-reply pattern
    pub fn request(
        &mut self,
        sender: DomainId,
        recipient: DomainId,
        payload: MessagePayload,
    ) -> u64 {
        let id = self.next_message_id.fetch_add(1, Ordering::Relaxed);

        let message = Message {
            id,
            msg_type: MessageType::Query,
            topic: String::new(),
            sender,
            recipient: Some(recipient),
            payload,
            headers: BTreeMap::new(),
            timestamp: Timestamp::now(),
            reply_to: None,
            correlation_id: Some(id),
            ttl_ns: Some(self.config.request_timeout_ns),
            priority: MessagePriority::Normal,
        };

        self.pending_requests.insert(id, PendingRequest {
            message_id: id,
            requester: sender,
            timestamp: Timestamp::now(),
            timeout_ns: self.config.request_timeout_ns,
        });

        self.stats.pending_requests = self.pending_requests.len() as u64;
        self.publish(message)
    }

    /// Reply to a message
    pub fn reply(
        &mut self,
        original: &Message,
        sender: DomainId,
        payload: MessagePayload,
    ) -> u64 {
        let id = self.next_message_id.fetch_add(1, Ordering::Relaxed);

        let message = Message {
            id,
            msg_type: MessageType::Reply,
            topic: String::new(),
            sender,
            recipient: Some(original.sender),
            payload,
            headers: BTreeMap::new(),
            timestamp: Timestamp::now(),
            reply_to: Some(original.id),
            correlation_id: original.correlation_id,
            ttl_ns: Some(self.config.default_ttl_ns),
            priority: original.priority,
        };

        // Clear pending request
        if let Some(corr_id) = original.correlation_id {
            self.pending_requests.remove(&corr_id);
            self.stats.pending_requests = self.pending_requests.len() as u64;
        }

        self.publish(message)
    }

    /// Get messages for subscriber
    pub fn poll(&mut self, subscriber: DomainId) -> Vec<Message> {
        let now = Timestamp::now();
        let mut delivered = Vec::new();
        let mut expired_indices = Vec::new();

        for (i, msg) in self.pending.iter().enumerate() {
            // Check expiration
            if let Some(ttl) = msg.ttl_ns {
                if now.elapsed_since(msg.timestamp) > ttl {
                    expired_indices.push(i);
                    continue;
                }
            }

            // Check direct recipient
            if let Some(recipient) = msg.recipient {
                if recipient == subscriber {
                    delivered.push(msg.clone());
                    continue;
                }
            }

            // Check subscriptions
            for sub in self.subscriptions.values() {
                if sub.subscriber != subscriber || !sub.active {
                    continue;
                }

                if sub.topic_pattern.matches(&msg.topic) {
                    if let Some(filter) = &sub.filter {
                        if !filter.matches(msg) {
                            continue;
                        }
                    }
                    delivered.push(msg.clone());
                    break;
                }
            }
        }

        // Remove expired
        for &i in expired_indices.iter().rev() {
            self.pending.remove(i);
            self.stats.total_expired += 1;
        }

        // Remove delivered from pending
        for msg in &delivered {
            self.pending.retain(|m| m.id != msg.id);
            self.stats.total_delivered += 1;

            // Add to history
            if self.history.len() >= self.config.max_history {
                self.history.pop_front();
            }
            self.history.push_back(msg.clone());
        }

        delivered
    }

    /// Get reply for request
    pub fn get_reply(&mut self, request_id: u64) -> Option<Message> {
        // Find reply in pending
        let pos = self.pending.iter().position(|msg| {
            msg.correlation_id == Some(request_id) && msg.msg_type == MessageType::Reply
        });

        if let Some(i) = pos {
            let msg = self.pending.remove(i);
            self.pending_requests.remove(&request_id);
            self.stats.pending_requests = self.pending_requests.len() as u64;
            return Some(msg);
        }

        // Check timeout
        let now = Timestamp::now();
        if let Some(req) = self.pending_requests.get(&request_id) {
            if now.elapsed_since(req.timestamp) > req.timeout_ns {
                self.pending_requests.remove(&request_id);
                self.stats.pending_requests = self.pending_requests.len() as u64;
            }
        }

        None
    }

    /// Cleanup expired messages and requests
    pub fn cleanup(&mut self) {
        let now = Timestamp::now();

        // Remove expired messages
        self.pending.retain(|msg| {
            msg.ttl_ns.map(|ttl| now.elapsed_since(msg.timestamp) < ttl).unwrap_or(true)
        });

        // Remove timed out requests
        self.pending_requests.retain(|_, req| {
            now.elapsed_since(req.timestamp) < req.timeout_ns
        });

        self.stats.pending_requests = self.pending_requests.len() as u64;
    }

    /// Get subscription
    #[inline(always)]
    pub fn get_subscription(&self, id: u64) -> Option<&Subscription> {
        self.subscriptions.get(&id)
    }

    /// Get subscriptions for domain
    #[inline]
    pub fn subscriptions_for(&self, domain: DomainId) -> Vec<&Subscription> {
        self.subscriptions.values()
            .filter(|s| s.subscriber == domain)
            .collect()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &MessageBusStats {
        &self.stats
    }

    /// Get pending count
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new(MessageBusConfig::default())
    }
}

// ============================================================================
// MESSAGE BUILDER
// ============================================================================

/// Message builder
pub struct MessageBuilder {
    msg_type: MessageType,
    topic: String,
    sender: DomainId,
    recipient: Option<DomainId>,
    payload: MessagePayload,
    headers: BTreeMap<String, String>,
    reply_to: Option<u64>,
    correlation_id: Option<u64>,
    ttl_ns: Option<u64>,
    priority: MessagePriority,
}

impl MessageBuilder {
    /// Create new builder
    pub fn new(sender: DomainId, msg_type: MessageType) -> Self {
        Self {
            msg_type,
            topic: String::new(),
            sender,
            recipient: None,
            payload: MessagePayload::Empty,
            headers: BTreeMap::new(),
            reply_to: None,
            correlation_id: None,
            ttl_ns: None,
            priority: MessagePriority::Normal,
        }
    }

    #[inline(always)]
    pub fn topic(mut self, topic: &str) -> Self {
        self.topic = topic.into();
        self
    }

    #[inline(always)]
    pub fn recipient(mut self, recipient: DomainId) -> Self {
        self.recipient = Some(recipient);
        self
    }

    #[inline(always)]
    pub fn payload(mut self, payload: MessagePayload) -> Self {
        self.payload = payload;
        self
    }

    #[inline(always)]
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    #[inline(always)]
    pub fn reply_to(mut self, msg_id: u64) -> Self {
        self.reply_to = Some(msg_id);
        self
    }

    #[inline(always)]
    pub fn correlation_id(mut self, id: u64) -> Self {
        self.correlation_id = Some(id);
        self
    }

    #[inline(always)]
    pub fn ttl(mut self, ttl_ns: u64) -> Self {
        self.ttl_ns = Some(ttl_ns);
        self
    }

    #[inline(always)]
    pub fn priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn build(self, id: u64) -> Message {
        Message {
            id,
            msg_type: self.msg_type,
            topic: self.topic,
            sender: self.sender,
            recipient: self.recipient,
            payload: self.payload,
            headers: self.headers,
            timestamp: Timestamp::now(),
            reply_to: self.reply_to,
            correlation_id: self.correlation_id,
            ttl_ns: self.ttl_ns,
            priority: self.priority,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_subscribe() {
        let mut bus = MessageBus::default();
        let sender = DomainId::new(1);
        let subscriber = DomainId::new(2);

        bus.subscribe(subscriber, TopicPattern::exact("events.test"), None);

        bus.send(sender, "events.test", MessageType::Event, MessagePayload::Text("hello".into()));

        let messages = bus.poll(subscriber);
        assert_eq!(messages.len(), 1);
        if let MessagePayload::Text(text) = &messages[0].payload {
            assert_eq!(text, "hello");
        }
    }

    #[test]
    fn test_pattern_matching() {
        let mut bus = MessageBus::default();
        let sender = DomainId::new(1);
        let subscriber = DomainId::new(2);

        bus.subscribe(subscriber, TopicPattern::prefix("events"), None);

        bus.send(sender, "events.a", MessageType::Event, MessagePayload::Empty);
        bus.send(sender, "events.b", MessageType::Event, MessagePayload::Empty);
        bus.send(sender, "other.c", MessageType::Event, MessagePayload::Empty);

        let messages = bus.poll(subscriber);
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_direct_message() {
        let mut bus = MessageBus::default();
        let sender = DomainId::new(1);
        let recipient = DomainId::new(2);
        let other = DomainId::new(3);

        bus.send_direct(sender, recipient, MessageType::Command, MessagePayload::Empty);

        assert!(bus.poll(other).is_empty());
        assert_eq!(bus.poll(recipient).len(), 1);
    }

    #[test]
    fn test_request_reply() {
        let mut bus = MessageBus::default();
        let client = DomainId::new(1);
        let server = DomainId::new(2);

        let req_id = bus.request(client, server, MessagePayload::Text("ping".into()));

        // Server receives request
        let requests = bus.poll(server);
        assert_eq!(requests.len(), 1);

        // Server sends reply
        bus.reply(&requests[0], server, MessagePayload::Text("pong".into()));

        // Client gets reply
        let reply = bus.get_reply(req_id).unwrap();
        if let MessagePayload::Text(text) = &reply.payload {
            assert_eq!(text, "pong");
        }
    }

    #[test]
    fn test_message_filter() {
        let mut bus = MessageBus::default();
        let sender = DomainId::new(1);
        let subscriber = DomainId::new(2);

        let filter = MessageFilter {
            msg_types: Some(vec![MessageType::Command]),
            min_priority: Some(MessagePriority::High),
            required_headers: vec![],
            header_filters: BTreeMap::new(),
        };

        bus.subscribe(subscriber, TopicPattern::all(), Some(filter));

        // Low priority event (should be filtered)
        bus.send(sender, "test", MessageType::Event, MessagePayload::Empty);

        // High priority command (should pass)
        let id = bus.next_message_id.fetch_add(1, Ordering::Relaxed);
        let msg = MessageBuilder::new(sender, MessageType::Command)
            .topic("test")
            .priority(MessagePriority::High)
            .build(id);
        bus.publish(msg);

        let messages = bus.poll(subscriber);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].msg_type, MessageType::Command);
    }
}

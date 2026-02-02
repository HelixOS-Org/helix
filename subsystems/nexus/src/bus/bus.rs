//! Message Bus
//!
//! Central message bus for NEXUS inter-domain communication.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use super::domain::Domain;
use super::message::{Message, MessagePayload, MessagePriority};
use super::router::{Router, RouterStats};
use crate::types::*;

// ============================================================================
// MESSAGE BUS
// ============================================================================

/// The central message bus for NEXUS
pub struct MessageBus {
    /// Router
    router: Router,
    /// Subscribers per topic
    subscribers: BTreeMap<String, Vec<Box<dyn Fn(&Message) + Send + Sync>>>,
    /// Is running
    running: AtomicBool,
    /// Bus ID
    id: NexusId,
}

impl MessageBus {
    /// Create new message bus
    pub fn new() -> Self {
        let mut bus = Self {
            router: Router::new(),
            subscribers: BTreeMap::new(),
            running: AtomicBool::new(false),
            id: NexusId::generate(),
        };

        // Initialize all valid channels
        bus.router.initialize_all_channels();

        bus
    }

    /// Get bus ID
    pub fn id(&self) -> NexusId {
        self.id
    }

    /// Start the bus
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop the bus
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Send message
    pub fn send(&mut self, message: Message) -> NexusResult<()> {
        if !self.is_running() {
            return Err(NexusError::new(ErrorCode::InvalidState, "Bus not running"));
        }

        self.router.route(message)
    }

    /// Send to domain
    pub fn send_to(
        &mut self,
        source: Domain,
        target: Domain,
        payload: MessagePayload,
    ) -> NexusResult<MessageId> {
        let message = Message::new(source, target, payload);
        let id = message.id;
        self.send(message)?;
        Ok(id)
    }

    /// Send with priority
    pub fn send_priority(
        &mut self,
        source: Domain,
        target: Domain,
        payload: MessagePayload,
        priority: MessagePriority,
    ) -> NexusResult<MessageId> {
        let message = Message::new(source, target, payload).with_priority(priority);
        let id = message.id;
        self.send(message)?;
        Ok(id)
    }

    /// Broadcast
    pub fn broadcast(&mut self, source: Domain, payload: MessagePayload) -> usize {
        if !self.is_running() {
            return 0;
        }
        self.router.broadcast(source, payload)
    }

    /// Receive from channel
    pub fn receive(&mut self, source: Domain, target: Domain) -> Option<Message> {
        self.router
            .get_channel(source, target)
            .and_then(|c| c.receive())
    }

    /// Receive all pending for target
    pub fn receive_all(&mut self, target: Domain) -> Vec<Message> {
        let mut messages = Vec::new();
        let domains = Domain::cognitive_domains();

        for source in domains {
            if let Some(channel) = self.router.get_channel(source, target) {
                while let Some(msg) = channel.receive() {
                    messages.push(msg);
                }
            }
        }

        // Sort by priority (highest first), then by timestamp
        messages.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.timestamp.cmp(&b.timestamp))
        });

        messages
    }

    /// Receive all from specific source
    pub fn receive_from(&mut self, source: Domain, target: Domain) -> Vec<Message> {
        self.router
            .get_channel(source, target)
            .map(|c| c.receive_all())
            .unwrap_or_default()
    }

    /// Get pending count for target
    pub fn pending_for(&self, target: Domain) -> usize {
        let mut total = 0;
        let domains = Domain::cognitive_domains();

        for source in domains {
            if let Some(channel) = self.router.get_channel_ref(source, target) {
                total += channel.pending();
            }
        }

        total
    }

    /// Get total pending count
    pub fn total_pending(&self) -> usize {
        self.router.stats().total_pending() as usize
    }

    /// Expire old messages
    pub fn expire(&mut self, now: Timestamp) -> usize {
        self.router.expire_all(now)
    }

    /// Get bus stats
    pub fn stats(&self) -> BusStats {
        BusStats {
            id: self.id,
            running: self.is_running(),
            router_stats: self.router.stats(),
        }
    }

    /// Reset stats
    pub fn reset_stats(&self) {
        self.router.reset_stats();
    }

    /// Pause bus (close all channels)
    pub fn pause(&self) {
        self.router.close_all();
    }

    /// Resume bus (reopen all channels)
    pub fn resume(&self) {
        self.router.reopen_all();
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// BUS STATS
// ============================================================================

/// Bus statistics
#[derive(Debug, Clone)]
pub struct BusStats {
    /// Bus ID
    pub id: NexusId,
    /// Is running
    pub running: bool,
    /// Router stats
    pub router_stats: RouterStats,
}

impl BusStats {
    /// Get total messages processed
    pub fn total_processed(&self) -> u64 {
        self.router_stats.total_routed
    }

    /// Get total pending
    pub fn total_pending(&self) -> u64 {
        self.router_stats.total_pending()
    }
}

// ============================================================================
// MESSAGE FILTER
// ============================================================================

/// Message filter for subscriptions
#[derive(Debug, Clone, Default)]
pub struct MessageFilter {
    /// Source domains
    pub sources: Vec<Domain>,
    /// Payload types
    pub payload_types: Vec<String>,
    /// Minimum priority
    pub min_priority: Option<MessagePriority>,
}

impl MessageFilter {
    /// Create new filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by source
    pub fn from_source(mut self, source: Domain) -> Self {
        self.sources.push(source);
        self
    }

    /// Filter by payload type
    pub fn of_type(mut self, payload_type: &str) -> Self {
        self.payload_types.push(String::from(payload_type));
        self
    }

    /// Filter by minimum priority
    pub fn min_priority(mut self, priority: MessagePriority) -> Self {
        self.min_priority = Some(priority);
        self
    }

    /// Check if message matches filter
    pub fn matches(&self, message: &Message) -> bool {
        // Check source
        if !self.sources.is_empty() && !self.sources.contains(&message.source) {
            return false;
        }

        // Check payload type
        if !self.payload_types.is_empty() {
            let msg_type = message.payload.type_name();
            if !self.payload_types.iter().any(|t| t == msg_type) {
                return false;
            }
        }

        // Check priority
        if let Some(min) = self.min_priority {
            if message.priority < min {
                return false;
            }
        }

        true
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_bus() {
        let mut bus = MessageBus::new();
        bus.start();

        let id = bus
            .send_to(
                Domain::Sense,
                Domain::Understand,
                MessagePayload::HealthCheckRequest,
            )
            .unwrap();

        assert!(!id.is_null());

        let messages = bus.receive_all(Domain::Understand);
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_bus_not_running() {
        let mut bus = MessageBus::new();
        // Don't start

        let result = bus.send_to(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_message_filter() {
        let filter = MessageFilter::new()
            .from_source(Domain::Sense)
            .min_priority(MessagePriority::High);

        let msg1 = Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        )
        .with_priority(MessagePriority::High);

        let msg2 = Message::new(
            Domain::Reason,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        )
        .with_priority(MessagePriority::High);

        assert!(filter.matches(&msg1));
        assert!(!filter.matches(&msg2)); // Wrong source
    }

    #[test]
    fn test_broadcast() {
        let mut bus = MessageBus::new();
        bus.start();

        let sent = bus.broadcast(Domain::Core, MessagePayload::HealthCheckRequest);
        assert!(sent > 0);
    }
}

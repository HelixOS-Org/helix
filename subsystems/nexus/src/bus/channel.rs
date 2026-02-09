//! Message Channel
//!
//! Point-to-point channel between domains.

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::domain::Domain;
use super::message::Message;
use super::queue::MessageQueue;
use crate::types::*;

// ============================================================================
// CHANNEL
// ============================================================================

/// A typed channel between domains
pub struct Channel {
    /// Channel ID
    pub id: StreamId,
    /// Source domain
    pub source: Domain,
    /// Target domain
    pub target: Domain,
    /// Message queue
    queue: MessageQueue,
    /// Is open
    open: AtomicBool,
    /// Messages sent
    sent: AtomicU64,
    /// Messages received
    received: AtomicU64,
    /// Messages dropped
    dropped: AtomicU64,
}

impl Channel {
    /// Create new channel
    pub fn new(source: Domain, target: Domain) -> Self {
        Self::with_capacity(source, target, 1000)
    }

    /// Create with specific capacity
    pub fn with_capacity(source: Domain, target: Domain, capacity: usize) -> Self {
        Self {
            id: StreamId::generate(),
            source,
            target,
            queue: MessageQueue::new(capacity),
            open: AtomicBool::new(true),
            sent: AtomicU64::new(0),
            received: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    /// Send message
    pub fn send(&mut self, message: Message) -> NexusResult<()> {
        if !self.open.load(Ordering::Acquire) {
            return Err(NexusError::new(
                ErrorCode::InvalidState,
                "Channel is closed",
            ));
        }

        if !message.source.can_send_to(&message.target) {
            return Err(NexusError::new(
                ErrorCode::PolicyViolation,
                alloc::format!(
                    "Flow not allowed: {} -> {}",
                    message.source.name(),
                    message.target.name()
                ),
            ));
        }

        if self.queue.push(message) {
            self.sent.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            Err(NexusError::new(ErrorCode::MemoryFull, "Channel queue full"))
        }
    }

    /// Receive message
    #[inline]
    pub fn receive(&mut self) -> Option<Message> {
        let msg = self.queue.pop();
        if msg.is_some() {
            self.received.fetch_add(1, Ordering::Relaxed);
        }
        msg
    }

    /// Receive all pending messages
    #[inline]
    pub fn receive_all(&mut self) -> alloc::vec::Vec<Message> {
        let messages = self.queue.drain();
        self.received
            .fetch_add(messages.len() as u64, Ordering::Relaxed);
        messages
    }

    /// Peek at next message
    #[inline(always)]
    pub fn peek(&self) -> Option<&Message> {
        self.queue.peek()
    }

    /// Pending count
    #[inline(always)]
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    /// Close channel
    #[inline(always)]
    pub fn close(&self) {
        self.open.store(false, Ordering::Release);
    }

    /// Reopen channel
    #[inline(always)]
    pub fn reopen(&self) {
        self.open.store(true, Ordering::Release);
    }

    /// Is open
    #[inline(always)]
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    /// Get stats
    pub fn stats(&self) -> ChannelStats {
        ChannelStats {
            id: self.id,
            source: self.source,
            target: self.target,
            sent: self.sent.load(Ordering::Relaxed),
            received: self.received.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            pending: self.queue.len() as u64,
            open: self.is_open(),
        }
    }

    /// Reset stats
    #[inline]
    pub fn reset_stats(&self) {
        self.sent.store(0, Ordering::Relaxed);
        self.received.store(0, Ordering::Relaxed);
        self.dropped.store(0, Ordering::Relaxed);
    }

    /// Expire old messages
    #[inline(always)]
    pub fn expire(&mut self, now: Timestamp) -> usize {
        self.queue.expire(now)
    }
}

// ============================================================================
// CHANNEL STATS
// ============================================================================

/// Channel statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ChannelStats {
    /// Channel ID
    pub id: StreamId,
    /// Source domain
    pub source: Domain,
    /// Target domain
    pub target: Domain,
    /// Messages sent
    pub sent: u64,
    /// Messages received
    pub received: u64,
    /// Messages dropped
    pub dropped: u64,
    /// Pending messages
    pub pending: u64,
    /// Is open
    pub open: bool,
}

impl ChannelStats {
    /// Get throughput (received / sent ratio)
    #[inline]
    pub fn throughput(&self) -> f64 {
        if self.sent == 0 {
            1.0
        } else {
            self.received as f64 / self.sent as f64
        }
    }

    /// Get drop rate
    #[inline]
    pub fn drop_rate(&self) -> f64 {
        let total = self.sent + self.dropped;
        if total == 0 {
            0.0
        } else {
            self.dropped as f64 / total as f64
        }
    }
}

impl Default for ChannelStats {
    fn default() -> Self {
        Self {
            id: StreamId::generate(),
            source: Domain::Core,
            target: Domain::Core,
            sent: 0,
            received: 0,
            dropped: 0,
            pending: 0,
            open: true,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::message::MessagePayload;
    use super::*;

    #[test]
    fn test_channel_send_receive() {
        let mut channel = Channel::new(Domain::Sense, Domain::Understand);

        let msg = Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        );

        channel.send(msg).unwrap();
        assert_eq!(channel.pending(), 1);

        let received = channel.receive();
        assert!(received.is_some());
        assert_eq!(channel.pending(), 0);
    }

    #[test]
    fn test_channel_flow_enforcement() {
        let mut channel = Channel::new(Domain::Act, Domain::Sense);

        let msg = Message::new(
            Domain::Act,
            Domain::Sense,
            MessagePayload::HealthCheckRequest,
        );

        // Should fail: Act cannot send to Sense
        let result = channel.send(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_close() {
        let mut channel = Channel::new(Domain::Sense, Domain::Understand);
        channel.close();

        let msg = Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        );

        let result = channel.send(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_channel_stats() {
        let mut channel = Channel::new(Domain::Sense, Domain::Understand);

        for _ in 0..10 {
            let msg = Message::new(
                Domain::Sense,
                Domain::Understand,
                MessagePayload::HealthCheckRequest,
            );
            channel.send(msg).unwrap();
        }

        for _ in 0..5 {
            channel.receive();
        }

        let stats = channel.stats();
        assert_eq!(stats.sent, 10);
        assert_eq!(stats.received, 5);
        assert_eq!(stats.pending, 5);
    }
}

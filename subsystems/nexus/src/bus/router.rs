//! Message Router
//!
//! Routes messages between domains through channels.

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::*;
use super::domain::Domain;
use super::message::{Message, MessagePayload};
use super::channel::{Channel, ChannelStats};

// ============================================================================
// ROUTE KEY
// ============================================================================

/// Route key for channel lookup
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RouteKey {
    /// Source domain
    pub source: Domain,
    /// Target domain
    pub target: Domain,
}

impl RouteKey {
    /// Create new route key
    pub fn new(source: Domain, target: Domain) -> Self {
        Self { source, target }
    }

    /// Convert to u64 for BTreeMap
    pub fn to_u64(&self) -> u64 {
        ((self.source as u64) << 32) | (self.target as u64)
    }
}

// ============================================================================
// ROUTER
// ============================================================================

/// Message router
pub struct Router {
    /// Channels by route
    channels: BTreeMap<u64, Channel>,
    /// Total messages routed
    total_routed: AtomicU64,
    /// Failed routes
    failed_routes: AtomicU64,
    /// Dropped messages
    dropped: AtomicU64,
}

impl Router {
    /// Create new router
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            total_routed: AtomicU64::new(0),
            failed_routes: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    /// Create router with all valid channels pre-initialized
    pub fn with_all_channels() -> Self {
        let mut router = Self::new();
        router.initialize_all_channels();
        router
    }

    /// Initialize all valid channels
    pub fn initialize_all_channels(&mut self) {
        let domains = Domain::cognitive_domains();

        for source in &domains {
            for target in &domains {
                if source.can_send_to(target) {
                    self.create_channel(*source, *target);
                }
            }
        }
    }

    /// Create channel for route
    pub fn create_channel(&mut self, source: Domain, target: Domain) -> StreamId {
        let key = RouteKey::new(source, target).to_u64();
        if let Some(existing) = self.channels.get(&key) {
            return existing.id;
        }

        let channel = Channel::new(source, target);
        let id = channel.id;
        self.channels.insert(key, channel);
        id
    }

    /// Create channel with specific capacity
    pub fn create_channel_with_capacity(
        &mut self,
        source: Domain,
        target: Domain,
        capacity: usize,
    ) -> StreamId {
        let key = RouteKey::new(source, target).to_u64();
        if let Some(existing) = self.channels.get(&key) {
            return existing.id;
        }

        let channel = Channel::with_capacity(source, target, capacity);
        let id = channel.id;
        self.channels.insert(key, channel);
        id
    }

    /// Get channel
    pub fn get_channel(&mut self, source: Domain, target: Domain) -> Option<&mut Channel> {
        let key = RouteKey::new(source, target).to_u64();
        self.channels.get_mut(&key)
    }

    /// Get channel (immutable)
    pub fn get_channel_ref(&self, source: Domain, target: Domain) -> Option<&Channel> {
        let key = RouteKey::new(source, target).to_u64();
        self.channels.get(&key)
    }

    /// Route message
    pub fn route(&mut self, message: Message) -> NexusResult<()> {
        let key = RouteKey::new(message.source, message.target).to_u64();

        if let Some(channel) = self.channels.get_mut(&key) {
            match channel.send(message) {
                Ok(()) => {
                    self.total_routed.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
                Err(e) => {
                    self.dropped.fetch_add(1, Ordering::Relaxed);
                    Err(e)
                }
            }
        } else {
            self.failed_routes.fetch_add(1, Ordering::Relaxed);
            Err(NexusError::new(
                ErrorCode::NotFound,
                alloc::format!(
                    "No channel for route: {} -> {}",
                    message.source.name(),
                    message.target.name()
                ),
            ))
        }
    }

    /// Broadcast message to all domains
    pub fn broadcast(&mut self, source: Domain, payload: MessagePayload) -> usize {
        let mut sent = 0;
        let targets: Vec<Domain> = Domain::cognitive_domains()
            .into_iter()
            .filter(|d| *d != source && source.can_send_to(d))
            .collect();

        for target in targets {
            let message = Message::new(source, target, payload.clone());
            if self.route(message).is_ok() {
                sent += 1;
            }
        }
        sent
    }

    /// Get all channels to a target domain
    pub fn channels_to(&mut self, target: Domain) -> Vec<&mut Channel> {
        // Collect all valid route keys first to avoid borrowing self in the loop
        let valid_keys: Vec<u64> = Domain::cognitive_domains()
            .into_iter()
            .map(|domain| RouteKey::new(domain, target).to_u64())
            .collect();
        
        self.channels
            .iter_mut()
            .filter(|(key, _)| valid_keys.contains(key))
            .map(|(_, channel)| channel)
            .collect()
    }

    /// Close all channels
    pub fn close_all(&self) {
        for channel in self.channels.values() {
            channel.close();
        }
    }

    /// Reopen all channels
    pub fn reopen_all(&self) {
        for channel in self.channels.values() {
            channel.reopen();
        }
    }

    /// Expire messages in all channels
    pub fn expire_all(&mut self, now: Timestamp) -> usize {
        let mut total = 0;
        for channel in self.channels.values_mut() {
            total += channel.expire(now);
        }
        total
    }

    /// Get stats
    pub fn stats(&self) -> RouterStats {
        let mut channel_stats = Vec::new();
        for channel in self.channels.values() {
            channel_stats.push(channel.stats());
        }

        RouterStats {
            total_channels: self.channels.len() as u64,
            total_routed: self.total_routed.load(Ordering::Relaxed),
            failed_routes: self.failed_routes.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            channel_stats,
        }
    }

    /// Reset all stats
    pub fn reset_stats(&self) {
        self.total_routed.store(0, Ordering::Relaxed);
        self.failed_routes.store(0, Ordering::Relaxed);
        self.dropped.store(0, Ordering::Relaxed);
        for channel in self.channels.values() {
            channel.reset_stats();
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ROUTER STATS
// ============================================================================

/// Router statistics
#[derive(Debug, Clone)]
pub struct RouterStats {
    /// Total channels
    pub total_channels: u64,
    /// Total messages routed
    pub total_routed: u64,
    /// Failed routes
    pub failed_routes: u64,
    /// Dropped messages
    pub dropped: u64,
    /// Per-channel stats
    pub channel_stats: Vec<ChannelStats>,
}

impl RouterStats {
    /// Get total pending messages
    pub fn total_pending(&self) -> u64 {
        self.channel_stats.iter().map(|s| s.pending).sum()
    }

    /// Get overall drop rate
    pub fn drop_rate(&self) -> f64 {
        let total = self.total_routed + self.dropped;
        if total == 0 {
            0.0
        } else {
            self.dropped as f64 / total as f64
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::message::MessagePayload;

    #[test]
    fn test_router_create_channel() {
        let mut router = Router::new();
        let id1 = router.create_channel(Domain::Sense, Domain::Understand);
        let id2 = router.create_channel(Domain::Sense, Domain::Understand);

        // Should return same ID for existing channel
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_router_route() {
        let mut router = Router::with_all_channels();

        let msg = Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        );

        router.route(msg).unwrap();

        let stats = router.stats();
        assert_eq!(stats.total_routed, 1);
    }

    #[test]
    fn test_router_broadcast() {
        let mut router = Router::with_all_channels();

        let sent = router.broadcast(Domain::Core, MessagePayload::HealthCheckRequest);
        assert!(sent > 0);
    }

    #[test]
    fn test_router_missing_channel() {
        let mut router = Router::new(); // No channels

        let msg = Message::new(
            Domain::Sense,
            Domain::Understand,
            MessagePayload::HealthCheckRequest,
        );

        let result = router.route(msg);
        assert!(result.is_err());
    }
}

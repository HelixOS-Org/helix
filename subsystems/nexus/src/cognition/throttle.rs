//! # Cognitive Throttling
//!
//! Rate limiting and throttling for cognitive operations.
//! Prevents overload and ensures fair resource allocation.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// THROTTLE TYPES
// ============================================================================

/// Rate limiter using token bucket algorithm
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Maximum tokens
    pub capacity: u64,
    /// Current tokens
    pub tokens: f64,
    /// Refill rate (tokens per nanosecond)
    pub refill_rate: f64,
    /// Last refill time
    pub last_refill: Timestamp,
}

impl TokenBucket {
    /// Create a new token bucket
    pub fn new(capacity: u64, refill_rate_per_second: f64) -> Self {
        Self {
            capacity,
            tokens: capacity as f64,
            refill_rate: refill_rate_per_second / 1_000_000_000.0,
            last_refill: Timestamp::now(),
        }
    }

    /// Try to consume tokens
    pub fn try_consume(&mut self, tokens: u64) -> bool {
        self.refill();

        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            true
        } else {
            false
        }
    }

    /// Consume tokens (blocking simulation - returns wait time if not available)
    pub fn consume(&mut self, tokens: u64) -> Option<u64> {
        self.refill();

        if self.tokens >= tokens as f64 {
            self.tokens -= tokens as f64;
            None
        } else {
            // Calculate wait time
            let needed = tokens as f64 - self.tokens;
            let wait_ns = (needed / self.refill_rate) as u64;
            Some(wait_ns)
        }
    }

    /// Refill tokens
    pub fn refill(&mut self) {
        let now = Timestamp::now();
        let elapsed = now.elapsed_since(self.last_refill);

        let new_tokens = elapsed as f64 * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity as f64);
        self.last_refill = now;
    }

    /// Get current tokens
    pub fn available(&self) -> u64 {
        self.tokens as u64
    }

    /// Get fill percentage
    pub fn fill_percentage(&self) -> f64 {
        self.tokens / self.capacity as f64
    }
}

/// Sliding window rate limiter
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    /// Window size (ns)
    pub window_ns: u64,
    /// Maximum requests per window
    pub max_requests: u64,
    /// Request timestamps
    requests: Vec<Timestamp>,
}

impl SlidingWindow {
    /// Create a new sliding window
    pub fn new(window_ns: u64, max_requests: u64) -> Self {
        Self {
            window_ns,
            max_requests,
            requests: Vec::new(),
        }
    }

    /// Try to record a request
    pub fn try_acquire(&mut self) -> bool {
        let now = Timestamp::now();
        self.cleanup(now);

        if self.requests.len() < self.max_requests as usize {
            self.requests.push(now);
            true
        } else {
            false
        }
    }

    /// Get remaining requests in window
    pub fn remaining(&mut self) -> u64 {
        self.cleanup(Timestamp::now());
        self.max_requests - self.requests.len() as u64
    }

    /// Get reset time (when oldest request expires)
    pub fn reset_time(&self) -> Option<u64> {
        self.requests.first().map(|t| t.raw() + self.window_ns)
    }

    /// Cleanup old requests
    fn cleanup(&mut self, now: Timestamp) {
        let cutoff = now.raw().saturating_sub(self.window_ns);
        self.requests.retain(|t| t.raw() > cutoff);
    }

    /// Get current request count
    pub fn current_count(&self) -> usize {
        self.requests.len()
    }
}

/// Leaky bucket rate limiter
#[derive(Debug, Clone)]
pub struct LeakyBucket {
    /// Bucket capacity
    pub capacity: u64,
    /// Current level
    pub level: f64,
    /// Leak rate (per nanosecond)
    pub leak_rate: f64,
    /// Last update time
    pub last_update: Timestamp,
}

impl LeakyBucket {
    /// Create a new leaky bucket
    pub fn new(capacity: u64, leak_rate_per_second: f64) -> Self {
        Self {
            capacity,
            level: 0.0,
            leak_rate: leak_rate_per_second / 1_000_000_000.0,
            last_update: Timestamp::now(),
        }
    }

    /// Try to add water (request)
    pub fn try_add(&mut self, amount: u64) -> bool {
        self.leak();

        if self.level + amount as f64 <= self.capacity as f64 {
            self.level += amount as f64;
            true
        } else {
            false
        }
    }

    /// Leak water
    pub fn leak(&mut self) {
        let now = Timestamp::now();
        let elapsed = now.elapsed_since(self.last_update);

        let leaked = elapsed as f64 * self.leak_rate;
        self.level = (self.level - leaked).max(0.0);
        self.last_update = now;
    }

    /// Get current fill level
    pub fn fill_level(&self) -> f64 {
        self.level / self.capacity as f64
    }
}

// ============================================================================
// THROTTLER
// ============================================================================

/// Throttle configuration
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    /// Throttle name
    pub name: String,
    /// Algorithm
    pub algorithm: ThrottleAlgorithm,
    /// Burst capacity
    pub burst: u64,
    /// Sustained rate (per second)
    pub rate_per_second: f64,
    /// Window size (for sliding window)
    pub window_ns: u64,
}

/// Throttle algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleAlgorithm {
    /// Token bucket
    TokenBucket,
    /// Sliding window
    SlidingWindow,
    /// Leaky bucket
    LeakyBucket,
    /// Fixed window
    FixedWindow,
}

/// A throttle entry
#[derive(Debug)]
pub struct ThrottleEntry {
    /// Entry ID
    pub id: u64,
    /// Configuration
    pub config: ThrottleConfig,
    /// Owner domain
    pub owner: DomainId,
    /// Token bucket (if used)
    token_bucket: Option<TokenBucket>,
    /// Sliding window (if used)
    sliding_window: Option<SlidingWindow>,
    /// Leaky bucket (if used)
    leaky_bucket: Option<LeakyBucket>,
    /// Statistics
    pub stats: ThrottleStats,
}

/// Throttle statistics
#[derive(Debug, Clone, Default)]
pub struct ThrottleStats {
    /// Total requests
    pub total_requests: u64,
    /// Allowed requests
    pub allowed: u64,
    /// Throttled requests
    pub throttled: u64,
    /// Current rate
    pub current_rate: f64,
}

impl ThrottleEntry {
    /// Create a new throttle entry
    pub fn new(id: u64, config: ThrottleConfig, owner: DomainId) -> Self {
        let (token_bucket, sliding_window, leaky_bucket) = match config.algorithm {
            ThrottleAlgorithm::TokenBucket => (
                Some(TokenBucket::new(config.burst, config.rate_per_second)),
                None,
                None,
            ),
            ThrottleAlgorithm::SlidingWindow => (
                None,
                Some(SlidingWindow::new(config.window_ns, config.burst)),
                None,
            ),
            ThrottleAlgorithm::LeakyBucket => (
                None,
                None,
                Some(LeakyBucket::new(config.burst, config.rate_per_second)),
            ),
            ThrottleAlgorithm::FixedWindow => {
                // Use sliding window with fixed behavior
                (
                    None,
                    Some(SlidingWindow::new(config.window_ns, config.burst)),
                    None,
                )
            },
        };

        Self {
            id,
            config,
            owner,
            token_bucket,
            sliding_window,
            leaky_bucket,
            stats: ThrottleStats::default(),
        }
    }

    /// Try to acquire (returns true if allowed)
    pub fn try_acquire(&mut self) -> bool {
        self.stats.total_requests += 1;

        let allowed = match self.config.algorithm {
            ThrottleAlgorithm::TokenBucket => self
                .token_bucket
                .as_mut()
                .map(|b| b.try_consume(1))
                .unwrap_or(true),
            ThrottleAlgorithm::SlidingWindow | ThrottleAlgorithm::FixedWindow => self
                .sliding_window
                .as_mut()
                .map(|w| w.try_acquire())
                .unwrap_or(true),
            ThrottleAlgorithm::LeakyBucket => self
                .leaky_bucket
                .as_mut()
                .map(|b| b.try_add(1))
                .unwrap_or(true),
        };

        if allowed {
            self.stats.allowed += 1;
        } else {
            self.stats.throttled += 1;
        }

        allowed
    }

    /// Try to acquire multiple
    pub fn try_acquire_n(&mut self, n: u64) -> bool {
        self.stats.total_requests += n;

        let allowed = match self.config.algorithm {
            ThrottleAlgorithm::TokenBucket => self
                .token_bucket
                .as_mut()
                .map(|b| b.try_consume(n))
                .unwrap_or(true),
            ThrottleAlgorithm::SlidingWindow | ThrottleAlgorithm::FixedWindow => {
                // Sliding window: try to acquire n times
                if let Some(w) = self.sliding_window.as_mut() {
                    if w.remaining() >= n {
                        for _ in 0..n {
                            w.try_acquire();
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            },
            ThrottleAlgorithm::LeakyBucket => self
                .leaky_bucket
                .as_mut()
                .map(|b| b.try_add(n))
                .unwrap_or(true),
        };

        if allowed {
            self.stats.allowed += n;
        } else {
            self.stats.throttled += n;
        }

        allowed
    }

    /// Get remaining capacity
    pub fn remaining(&mut self) -> u64 {
        match self.config.algorithm {
            ThrottleAlgorithm::TokenBucket => self
                .token_bucket
                .as_ref()
                .map(|b| b.available())
                .unwrap_or(0),
            ThrottleAlgorithm::SlidingWindow | ThrottleAlgorithm::FixedWindow => self
                .sliding_window
                .as_mut()
                .map(|w| w.remaining())
                .unwrap_or(0),
            ThrottleAlgorithm::LeakyBucket => self
                .leaky_bucket
                .as_ref()
                .map(|b| (b.capacity as f64 - b.level) as u64)
                .unwrap_or(0),
        }
    }
}

// ============================================================================
// THROTTLE MANAGER
// ============================================================================

/// Manages throttles across the system
pub struct ThrottleManager {
    /// Throttles
    throttles: BTreeMap<u64, ThrottleEntry>,
    /// Throttles by name
    by_name: BTreeMap<String, u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Global throttle (applies to all)
    global: Option<ThrottleEntry>,
    /// Statistics
    stats: ThrottleManagerStats,
}

/// Manager statistics
#[derive(Debug, Clone, Default)]
pub struct ThrottleManagerStats {
    /// Total requests
    pub total_requests: u64,
    /// Total allowed
    pub total_allowed: u64,
    /// Total throttled
    pub total_throttled: u64,
    /// Active throttles
    pub active_throttles: u64,
}

impl ThrottleManager {
    /// Create a new throttle manager
    pub fn new() -> Self {
        Self {
            throttles: BTreeMap::new(),
            by_name: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            global: None,
            stats: ThrottleManagerStats::default(),
        }
    }

    /// Create a throttle
    pub fn create(&mut self, config: ThrottleConfig, owner: DomainId) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let name = config.name.clone();

        let entry = ThrottleEntry::new(id, config, owner);
        self.throttles.insert(id, entry);
        self.by_name.insert(name, id);
        self.stats.active_throttles = self.throttles.len() as u64;

        id
    }

    /// Delete a throttle
    pub fn delete(&mut self, id: u64) -> bool {
        if let Some(entry) = self.throttles.remove(&id) {
            self.by_name.remove(&entry.config.name);
            self.stats.active_throttles = self.throttles.len() as u64;
            true
        } else {
            false
        }
    }

    /// Set global throttle
    pub fn set_global(&mut self, config: ThrottleConfig) {
        let entry = ThrottleEntry::new(0, config, DomainId::new(0));
        self.global = Some(entry);
    }

    /// Clear global throttle
    pub fn clear_global(&mut self) {
        self.global = None;
    }

    /// Try to acquire from throttle
    pub fn try_acquire(&mut self, throttle_id: u64) -> bool {
        self.stats.total_requests += 1;

        // Check global throttle first
        if let Some(global) = &mut self.global {
            if !global.try_acquire() {
                self.stats.total_throttled += 1;
                return false;
            }
        }

        // Check specific throttle
        if let Some(entry) = self.throttles.get_mut(&throttle_id) {
            let allowed = entry.try_acquire();
            if allowed {
                self.stats.total_allowed += 1;
            } else {
                self.stats.total_throttled += 1;
            }
            allowed
        } else {
            self.stats.total_allowed += 1;
            true // No throttle = allow
        }
    }

    /// Try to acquire by name
    pub fn try_acquire_by_name(&mut self, name: &str) -> bool {
        if let Some(&id) = self.by_name.get(name) {
            self.try_acquire(id)
        } else {
            true // No throttle = allow
        }
    }

    /// Get throttle
    pub fn get(&self, id: u64) -> Option<&ThrottleEntry> {
        self.throttles.get(&id)
    }

    /// Get throttle by name
    pub fn get_by_name(&self, name: &str) -> Option<&ThrottleEntry> {
        self.by_name.get(name).and_then(|id| self.throttles.get(id))
    }

    /// Get remaining for throttle
    pub fn remaining(&mut self, id: u64) -> u64 {
        self.throttles
            .get_mut(&id)
            .map(|e| e.remaining())
            .unwrap_or(u64::MAX)
    }

    /// Get throttles by owner
    pub fn by_owner(&self, owner: DomainId) -> Vec<&ThrottleEntry> {
        self.throttles
            .values()
            .filter(|e| e.owner == owner)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &ThrottleManagerStats {
        &self.stats
    }

    /// Get all throttles
    pub fn all(&self) -> Vec<&ThrottleEntry> {
        self.throttles.values().collect()
    }
}

impl Default for ThrottleManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, 100.0); // 10 tokens, 100/s refill

        // Consume all tokens
        for _ in 0..10 {
            assert!(bucket.try_consume(1));
        }

        // No more tokens
        assert!(!bucket.try_consume(1));
    }

    #[test]
    fn test_sliding_window() {
        let mut window = SlidingWindow::new(1_000_000_000, 5); // 5 requests per second

        // Use all allowance
        for _ in 0..5 {
            assert!(window.try_acquire());
        }

        // Should be throttled
        assert!(!window.try_acquire());
    }

    #[test]
    fn test_leaky_bucket() {
        let mut bucket = LeakyBucket::new(10, 100.0); // 10 capacity, 100/s leak

        // Fill bucket
        for _ in 0..10 {
            assert!(bucket.try_add(1));
        }

        // Bucket full
        assert!(!bucket.try_add(1));
    }

    #[test]
    fn test_throttle_manager() {
        let mut manager = ThrottleManager::default();
        let domain = DomainId::new(1);

        let config = ThrottleConfig {
            name: "api_limit".into(),
            algorithm: ThrottleAlgorithm::TokenBucket,
            burst: 5,
            rate_per_second: 100.0,
            window_ns: 1_000_000_000,
        };

        let id = manager.create(config, domain);

        // Use allowance
        for _ in 0..5 {
            assert!(manager.try_acquire(id));
        }

        // Should be throttled
        assert!(!manager.try_acquire(id));
    }

    #[test]
    fn test_global_throttle() {
        let mut manager = ThrottleManager::default();
        let domain = DomainId::new(1);

        // Set very restrictive global throttle
        manager.set_global(ThrottleConfig {
            name: "global".into(),
            algorithm: ThrottleAlgorithm::TokenBucket,
            burst: 2,
            rate_per_second: 1.0,
            window_ns: 0,
        });

        let config = ThrottleConfig {
            name: "local".into(),
            algorithm: ThrottleAlgorithm::TokenBucket,
            burst: 100,
            rate_per_second: 1000.0,
            window_ns: 0,
        };

        let id = manager.create(config, domain);

        // Only 2 requests allowed due to global
        assert!(manager.try_acquire(id));
        assert!(manager.try_acquire(id));
        assert!(!manager.try_acquire(id));
    }
}

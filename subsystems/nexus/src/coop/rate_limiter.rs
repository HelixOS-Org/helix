//! # Cooperative Rate Limiter
//!
//! Cooperative rate limiting across subsystems:
//! - Token bucket algorithm with smooth refill
//! - Leaky bucket for burst control
//! - Sliding window rate tracking
//! - Per-subsystem and global quotas
//! - Hierarchical rate limiting (parent/child)
//! - Fair-share allocation among cooperating consumers

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Rate limiter algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitAlgorithm {
    TokenBucket,
    LeakyBucket,
    SlidingWindow,
    FixedWindow,
}

/// Rate decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateDecision {
    Allowed,
    Throttled,
    Queued,
    Dropped,
}

/// Token bucket state
#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub capacity: u64,
    pub tokens: u64,
    pub refill_rate: u64, // tokens per second
    pub last_refill_ns: u64,
    pub burst_allowance: u64,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill_ns: 0,
            burst_allowance: capacity / 4,
        }
    }

    #[inline]
    pub fn refill(&mut self, now_ns: u64) {
        if now_ns <= self.last_refill_ns { return; }
        let elapsed_ns = now_ns - self.last_refill_ns;
        let new_tokens = (elapsed_ns / 1_000_000_000) * self.refill_rate
            + (elapsed_ns % 1_000_000_000) * self.refill_rate / 1_000_000_000;
        self.tokens = (self.tokens + new_tokens).min(self.capacity + self.burst_allowance);
        self.last_refill_ns = now_ns;
    }

    #[inline]
    pub fn try_consume(&mut self, count: u64, now_ns: u64) -> bool {
        self.refill(now_ns);
        if self.tokens >= count {
            self.tokens -= count;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn available(&self) -> u64 { self.tokens }

    #[inline(always)]
    pub fn fill_ratio(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.tokens as f64 / self.capacity as f64
    }
}

/// Sliding window counter
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SlidingWindowCounter {
    pub window_ns: u64,
    pub max_requests: u64,
    slots: Vec<(u64, u64)>, // (timestamp, count)
    pub total_counted: u64,
}

impl SlidingWindowCounter {
    pub fn new(window_ns: u64, max_requests: u64) -> Self {
        Self {
            window_ns,
            max_requests,
            slots: Vec::new(),
            total_counted: 0,
        }
    }

    #[inline]
    pub fn record(&mut self, now_ns: u64, count: u64) {
        // Prune old entries
        let cutoff = now_ns.saturating_sub(self.window_ns);
        self.slots.retain(|&(ts, _)| ts >= cutoff);
        self.slots.push((now_ns, count));
        self.total_counted += count;
    }

    #[inline]
    pub fn current_count(&self, now_ns: u64) -> u64 {
        let cutoff = now_ns.saturating_sub(self.window_ns);
        self.slots.iter()
            .filter(|&&(ts, _)| ts >= cutoff)
            .map(|&(_, c)| c)
            .sum()
    }

    #[inline(always)]
    pub fn is_exceeded(&self, now_ns: u64) -> bool {
        self.current_count(now_ns) >= self.max_requests
    }

    #[inline(always)]
    pub fn remaining(&self, now_ns: u64) -> u64 {
        self.max_requests.saturating_sub(self.current_count(now_ns))
    }
}

/// Per-consumer rate state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConsumerRateState {
    pub consumer_id: u64,
    pub bucket: TokenBucket,
    pub window: SlidingWindowCounter,
    pub total_allowed: u64,
    pub total_throttled: u64,
    pub total_dropped: u64,
    pub parent_id: Option<u64>,
    pub weight: u32,
}

impl ConsumerRateState {
    pub fn new(consumer_id: u64, capacity: u64, refill_rate: u64, window_ns: u64) -> Self {
        Self {
            consumer_id,
            bucket: TokenBucket::new(capacity, refill_rate),
            window: SlidingWindowCounter::new(window_ns, capacity * 2),
            total_allowed: 0,
            total_throttled: 0,
            total_dropped: 0,
            parent_id: None,
            weight: 1,
        }
    }

    #[inline]
    pub fn throttle_ratio(&self) -> f64 {
        let total = self.total_allowed + self.total_throttled + self.total_dropped;
        if total == 0 { return 0.0; }
        self.total_throttled as f64 / total as f64
    }

    #[inline]
    pub fn drop_ratio(&self) -> f64 {
        let total = self.total_allowed + self.total_throttled + self.total_dropped;
        if total == 0 { return 0.0; }
        self.total_dropped as f64 / total as f64
    }
}

/// Cooperative Rate Limiter
pub struct CoopRateLimiter {
    consumers: BTreeMap<u64, ConsumerRateState>,
    algorithm: RateLimitAlgorithm,
    global_bucket: TokenBucket,
    total_requests: u64,
    total_allowed: u64,
    total_throttled: u64,
}

impl CoopRateLimiter {
    pub fn new(algorithm: RateLimitAlgorithm, global_capacity: u64, global_refill: u64) -> Self {
        Self {
            consumers: BTreeMap::new(),
            algorithm,
            global_bucket: TokenBucket::new(global_capacity, global_refill),
            total_requests: 0,
            total_allowed: 0,
            total_throttled: 0,
        }
    }

    #[inline]
    pub fn register_consumer(
        &mut self,
        consumer_id: u64,
        capacity: u64,
        refill_rate: u64,
        window_ns: u64,
        weight: u32,
        parent_id: Option<u64>,
    ) {
        let mut state = ConsumerRateState::new(consumer_id, capacity, refill_rate, window_ns);
        state.weight = weight;
        state.parent_id = parent_id;
        self.consumers.insert(consumer_id, state);
    }

    /// Check rate limit and decide
    pub fn check(&mut self, consumer_id: u64, cost: u64, now_ns: u64) -> RateDecision {
        self.total_requests += 1;

        // Check global limit first
        if !self.global_bucket.try_consume(cost, now_ns) {
            self.total_throttled += 1;
            if let Some(c) = self.consumers.get_mut(&consumer_id) {
                c.total_throttled += 1;
            }
            return RateDecision::Throttled;
        }

        // Check hierarchical: parent first
        if let Some(parent_id) = self.consumers.get(&consumer_id).and_then(|c| c.parent_id) {
            if let Some(parent) = self.consumers.get_mut(&parent_id) {
                if !parent.bucket.try_consume(cost, now_ns) {
                    self.total_throttled += 1;
                    if let Some(c) = self.consumers.get_mut(&consumer_id) {
                        c.total_throttled += 1;
                    }
                    return RateDecision::Throttled;
                }
            }
        }

        // Check per-consumer limit
        if let Some(consumer) = self.consumers.get_mut(&consumer_id) {
            match self.algorithm {
                RateLimitAlgorithm::TokenBucket => {
                    if consumer.bucket.try_consume(cost, now_ns) {
                        consumer.total_allowed += 1;
                        consumer.window.record(now_ns, cost);
                        self.total_allowed += 1;
                        RateDecision::Allowed
                    } else {
                        consumer.total_throttled += 1;
                        self.total_throttled += 1;
                        RateDecision::Throttled
                    }
                }
                RateLimitAlgorithm::SlidingWindow => {
                    if consumer.window.is_exceeded(now_ns) {
                        consumer.total_throttled += 1;
                        self.total_throttled += 1;
                        RateDecision::Throttled
                    } else {
                        consumer.window.record(now_ns, cost);
                        consumer.total_allowed += 1;
                        self.total_allowed += 1;
                        RateDecision::Allowed
                    }
                }
                _ => {
                    if consumer.bucket.try_consume(cost, now_ns) {
                        consumer.total_allowed += 1;
                        self.total_allowed += 1;
                        RateDecision::Allowed
                    } else {
                        consumer.total_throttled += 1;
                        self.total_throttled += 1;
                        RateDecision::Throttled
                    }
                }
            }
        } else {
            // Unknown consumer: allow but don't track
            self.total_allowed += 1;
            RateDecision::Allowed
        }
    }

    /// Redistribute unused capacity among consumers based on weight
    pub fn redistribute(&mut self, now_ns: u64) {
        let total_weight: u32 = self.consumers.values().map(|c| c.weight).sum();
        if total_weight == 0 { return; }

        let global_available = self.global_bucket.available();
        for consumer in self.consumers.values_mut() {
            let share = (global_available * consumer.weight as u64) / total_weight as u64;
            consumer.bucket.refill(now_ns);
            // Grant bonus tokens proportional to weight
            let bonus = share / 10;
            consumer.bucket.tokens = (consumer.bucket.tokens + bonus).min(consumer.bucket.capacity);
        }
    }

    #[inline(always)]
    pub fn consumer(&self, id: u64) -> Option<&ConsumerRateState> {
        self.consumers.get(&id)
    }

    #[inline(always)]
    pub fn global_throttle_ratio(&self) -> f64 {
        if self.total_requests == 0 { return 0.0; }
        self.total_throttled as f64 / self.total_requests as f64
    }

    #[inline(always)]
    pub fn global_fill_ratio(&self) -> f64 {
        self.global_bucket.fill_ratio()
    }
}

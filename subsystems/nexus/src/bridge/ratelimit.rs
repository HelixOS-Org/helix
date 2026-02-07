//! # Bridge Rate Limiter
//!
//! Syscall rate limiting and flow control:
//! - Token bucket rate limiters
//! - Sliding window counters
//! - Per-process rate limits
//! - Per-syscall rate limits
//! - Burst handling
//! - Adaptive rate adjustment

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TOKEN BUCKET
// ============================================================================

/// Token bucket rate limiter
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Maximum tokens (capacity)
    pub capacity: u64,
    /// Current tokens
    pub tokens: u64,
    /// Refill rate (tokens per second)
    pub refill_rate: u64,
    /// Last refill time (ns)
    pub last_refill: u64,
    /// Burst size (extra tokens above capacity)
    pub burst: u64,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: 0,
            burst: 0,
        }
    }

    pub fn with_burst(mut self, burst: u64) -> Self {
        self.burst = burst;
        self
    }

    /// Refill tokens based on elapsed time
    pub fn refill(&mut self, now_ns: u64) {
        if self.last_refill == 0 {
            self.last_refill = now_ns;
            return;
        }

        let elapsed_ns = now_ns.saturating_sub(self.last_refill);
        let new_tokens = (elapsed_ns as u128 * self.refill_rate as u128 / 1_000_000_000u128) as u64;

        if new_tokens > 0 {
            self.tokens = (self.tokens + new_tokens).min(self.capacity + self.burst);
            self.last_refill = now_ns;
        }
    }

    /// Try to consume tokens
    pub fn try_consume(&mut self, count: u64, now_ns: u64) -> bool {
        self.refill(now_ns);
        if self.tokens >= count {
            self.tokens -= count;
            true
        } else {
            false
        }
    }

    /// Tokens available
    pub fn available(&self) -> u64 {
        self.tokens
    }

    /// Utilization (fraction of capacity used)
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        1.0 - (self.tokens as f64 / self.capacity as f64)
    }

    /// Time until next token available (ns)
    pub fn time_to_token_ns(&self) -> u64 {
        if self.tokens > 0 || self.refill_rate == 0 {
            return 0;
        }
        1_000_000_000 / self.refill_rate
    }
}

// ============================================================================
// SLIDING WINDOW COUNTER
// ============================================================================

/// Sliding window slot
#[derive(Debug, Clone)]
struct WindowSlot {
    /// Count
    count: u64,
    /// Slot start time
    start_ns: u64,
}

/// Sliding window rate counter
#[derive(Debug, Clone)]
pub struct SlidingWindowCounter {
    /// Slots
    slots: Vec<WindowSlot>,
    /// Window duration (ns)
    pub window_ns: u64,
    /// Slot duration (ns)
    pub slot_ns: u64,
    /// Max rate per window
    pub max_rate: u64,
    /// Total rejected
    pub rejected: u64,
}

impl SlidingWindowCounter {
    pub fn new(window_ns: u64, num_slots: usize, max_rate: u64) -> Self {
        let slot_ns = window_ns / num_slots as u64;
        Self {
            slots: Vec::new(),
            window_ns,
            slot_ns,
            max_rate,
            rejected: 0,
        }
    }

    /// Get current rate
    pub fn current_rate(&self, now_ns: u64) -> u64 {
        let window_start = now_ns.saturating_sub(self.window_ns);
        self.slots
            .iter()
            .filter(|s| s.start_ns >= window_start)
            .map(|s| s.count)
            .sum()
    }

    /// Try increment
    pub fn try_increment(&mut self, now_ns: u64) -> bool {
        self.cleanup(now_ns);

        let current = self.current_rate(now_ns);
        if current >= self.max_rate {
            self.rejected += 1;
            return false;
        }

        // Find or create current slot
        let slot_start = (now_ns / self.slot_ns) * self.slot_ns;
        if let Some(slot) = self.slots.iter_mut().find(|s| s.start_ns == slot_start) {
            slot.count += 1;
        } else {
            self.slots.push(WindowSlot {
                count: 1,
                start_ns: slot_start,
            });
        }
        true
    }

    fn cleanup(&mut self, now_ns: u64) {
        let window_start = now_ns.saturating_sub(self.window_ns);
        self.slots.retain(|s| s.start_ns >= window_start);
    }

    /// Rejection rate
    pub fn rejection_rate(&self) -> f64 {
        let total: u64 = self.slots.iter().map(|s| s.count).sum::<u64>() + self.rejected;
        if total == 0 {
            return 0.0;
        }
        self.rejected as f64 / total as f64
    }
}

// ============================================================================
// RATE LIMIT POLICY
// ============================================================================

/// Rate limit scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitScope {
    /// Global (all processes)
    Global,
    /// Per-process
    PerProcess,
    /// Per-syscall
    PerSyscall,
    /// Per-process per-syscall
    PerProcessPerSyscall,
}

/// Rate limit policy
#[derive(Debug, Clone)]
pub struct RateLimitPolicy {
    /// Policy ID
    pub id: u64,
    /// Scope
    pub scope: RateLimitScope,
    /// Syscall number (0 = all)
    pub syscall_nr: u32,
    /// Rate (operations per second)
    pub rate: u64,
    /// Burst
    pub burst: u64,
    /// Enabled
    pub enabled: bool,
}

impl RateLimitPolicy {
    pub fn new(id: u64, scope: RateLimitScope, rate: u64) -> Self {
        Self {
            id,
            scope,
            syscall_nr: 0,
            rate,
            burst: rate / 10,
            enabled: true,
        }
    }

    pub fn for_syscall(mut self, nr: u32) -> Self {
        self.syscall_nr = nr;
        self
    }

    pub fn with_burst(mut self, burst: u64) -> Self {
        self.burst = burst;
        self
    }
}

// ============================================================================
// RATE LIMIT RESULT
// ============================================================================

/// Rate limit decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Allowed
    Allowed,
    /// Rate limited (delayed)
    Delayed(u64), // delay in ns
    /// Rejected
    Rejected,
}

// ============================================================================
// RATE LIMITER MANAGER
// ============================================================================

/// Rate limiter stats
#[derive(Debug, Clone, Default)]
pub struct RateLimiterStats {
    /// Total checks
    pub total_checks: u64,
    /// Total allowed
    pub allowed: u64,
    /// Total rejected
    pub rejected: u64,
    /// Active limiters
    pub active_limiters: usize,
    /// Overall rejection rate
    pub rejection_rate: f64,
}

/// Bridge rate limiter
pub struct BridgeRateLimiter {
    /// Policies
    policies: Vec<RateLimitPolicy>,
    /// Global buckets (policy_id → bucket)
    global_buckets: BTreeMap<u64, TokenBucket>,
    /// Per-process buckets (policy_id, pid) → bucket
    process_buckets: BTreeMap<(u64, u64), TokenBucket>,
    /// Sliding window counters (policy_id, scope_key) → counter
    windows: BTreeMap<(u64, u64), SlidingWindowCounter>,
    /// Stats
    stats: RateLimiterStats,
}

impl BridgeRateLimiter {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            global_buckets: BTreeMap::new(),
            process_buckets: BTreeMap::new(),
            windows: BTreeMap::new(),
            stats: RateLimiterStats::default(),
        }
    }

    /// Add policy
    pub fn add_policy(&mut self, policy: RateLimitPolicy) {
        match policy.scope {
            RateLimitScope::Global | RateLimitScope::PerSyscall => {
                let bucket = TokenBucket::new(policy.rate, policy.rate).with_burst(policy.burst);
                self.global_buckets.insert(policy.id, bucket);
            }
            _ => {
                // Per-process buckets created on demand
            }
        }
        self.policies.push(policy);
        self.stats.active_limiters = self.policies.iter().filter(|p| p.enabled).count();
    }

    /// Check rate limit
    pub fn check(&mut self, pid: u64, syscall_nr: u32, now_ns: u64) -> RateLimitDecision {
        self.stats.total_checks += 1;

        for policy in &self.policies {
            if !policy.enabled {
                continue;
            }
            if policy.syscall_nr != 0 && policy.syscall_nr != syscall_nr {
                continue;
            }

            let allowed = match policy.scope {
                RateLimitScope::Global | RateLimitScope::PerSyscall => {
                    if let Some(bucket) = self.global_buckets.get_mut(&policy.id) {
                        bucket.try_consume(1, now_ns)
                    } else {
                        true
                    }
                }
                RateLimitScope::PerProcess | RateLimitScope::PerProcessPerSyscall => {
                    let key = (policy.id, pid);
                    let bucket = self.process_buckets.entry(key).or_insert_with(|| {
                        TokenBucket::new(policy.rate, policy.rate).with_burst(policy.burst)
                    });
                    bucket.try_consume(1, now_ns)
                }
            };

            if !allowed {
                self.stats.rejected += 1;
                self.update_rate();
                return RateLimitDecision::Rejected;
            }
        }

        self.stats.allowed += 1;
        self.update_rate();
        RateLimitDecision::Allowed
    }

    fn update_rate(&mut self) {
        if self.stats.total_checks > 0 {
            self.stats.rejection_rate =
                self.stats.rejected as f64 / self.stats.total_checks as f64;
        }
    }

    /// Cleanup per-process buckets for exited processes
    pub fn cleanup_process(&mut self, pid: u64) {
        self.process_buckets.retain(|&(_, p), _| p != pid);
    }

    /// Stats
    pub fn stats(&self) -> &RateLimiterStats {
        &self.stats
    }
}

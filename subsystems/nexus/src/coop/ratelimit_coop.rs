//! # Cooperative Rate Limiting
//!
//! Cooperative rate limiting between processes:
//! - Token bucket per-group rate limits
//! - Sliding window counters
//! - Adaptive rate based on system load
//! - Fair sharing within groups
//! - Burst allowance management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// RATE LIMIT TYPES
// ============================================================================

/// Rate limit algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitAlgorithm {
    /// Token bucket
    TokenBucket,
    /// Sliding window
    SlidingWindow,
    /// Leaky bucket
    LeakyBucket,
    /// Fixed window
    FixedWindow,
}

/// Rate limit scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitScope {
    /// Per-process
    PerProcess,
    /// Per-group
    PerGroup,
    /// Global
    Global,
    /// Per-resource
    PerResource,
}

/// Rate limit decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    /// Allowed
    Allow,
    /// Denied (over limit)
    Deny,
    /// Throttled (delayed)
    Throttle,
    /// Warning (approaching limit)
    Warn,
}

// ============================================================================
// TOKEN BUCKET
// ============================================================================

/// Token bucket
#[derive(Debug, Clone)]
pub struct CoopTokenBucket {
    /// Current tokens
    pub tokens: f64,
    /// Max tokens (burst capacity)
    pub max_tokens: f64,
    /// Refill rate (tokens per second)
    pub refill_rate: f64,
    /// Last refill timestamp (ns)
    last_refill: u64,
}

impl CoopTokenBucket {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: 0,
        }
    }

    /// Refill tokens based on elapsed time
    pub fn refill(&mut self, now: u64) {
        if self.last_refill == 0 {
            self.last_refill = now;
            return;
        }
        let elapsed_s = (now - self.last_refill) as f64 / 1_000_000_000.0;
        self.tokens += elapsed_s * self.refill_rate;
        if self.tokens > self.max_tokens {
            self.tokens = self.max_tokens;
        }
        self.last_refill = now;
    }

    /// Try consume tokens
    #[inline]
    pub fn try_consume(&mut self, count: f64, now: u64) -> bool {
        self.refill(now);
        if self.tokens >= count {
            self.tokens -= count;
            true
        } else {
            false
        }
    }

    /// Time until tokens available (ns)
    #[inline]
    pub fn time_until_available(&self, count: f64) -> u64 {
        if self.tokens >= count {
            return 0;
        }
        let deficit = count - self.tokens;
        let seconds = deficit / self.refill_rate;
        (seconds * 1_000_000_000.0) as u64
    }

    /// Fill ratio
    #[inline]
    pub fn fill_ratio(&self) -> f64 {
        if self.max_tokens <= 0.0 {
            return 0.0;
        }
        self.tokens / self.max_tokens
    }
}

// ============================================================================
// SLIDING WINDOW
// ============================================================================

/// Sliding window counter
#[derive(Debug)]
pub struct CoopSlidingWindow {
    /// Window size (ns)
    pub window_ns: u64,
    /// Max events per window
    pub max_events: u64,
    /// Event timestamps
    events: Vec<u64>,
}

impl CoopSlidingWindow {
    pub fn new(window_ns: u64, max_events: u64) -> Self {
        Self {
            window_ns,
            max_events,
            events: Vec::new(),
        }
    }

    /// Record event, returns whether allowed
    pub fn record(&mut self, now: u64) -> bool {
        // Evict old events
        let cutoff = now.saturating_sub(self.window_ns);
        self.events.retain(|&t| t >= cutoff);

        if (self.events.len() as u64) < self.max_events {
            self.events.push(now);
            true
        } else {
            false
        }
    }

    /// Current count
    #[inline(always)]
    pub fn count(&self) -> u64 {
        self.events.len() as u64
    }

    /// Usage ratio
    #[inline]
    pub fn usage_ratio(&self) -> f64 {
        if self.max_events == 0 {
            return 0.0;
        }
        self.events.len() as f64 / self.max_events as f64
    }

    /// Events per second
    #[inline]
    pub fn rate(&self, now: u64) -> f64 {
        let cutoff = now.saturating_sub(self.window_ns);
        let recent = self.events.iter().filter(|&&t| t >= cutoff).count();
        let window_s = self.window_ns as f64 / 1_000_000_000.0;
        if window_s <= 0.0 {
            return 0.0;
        }
        recent as f64 / window_s
    }
}

// ============================================================================
// PER-GROUP RATE LIMITER
// ============================================================================

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Algorithm
    pub algorithm: RateLimitAlgorithm,
    /// Max rate (events per second)
    pub max_rate: f64,
    /// Burst size
    pub burst: u64,
    /// Window (ns, for sliding window)
    pub window_ns: u64,
    /// Warning threshold (fraction)
    pub warn_threshold: f64,
}

/// Per-process rate state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessRateState {
    /// Process id
    pub pid: u64,
    /// Token bucket
    bucket: CoopTokenBucket,
    /// Total allowed
    pub total_allowed: u64,
    /// Total denied
    pub total_denied: u64,
    /// Total throttled
    pub total_throttled: u64,
}

impl ProcessRateState {
    pub fn new(pid: u64, config: &RateLimitConfig) -> Self {
        Self {
            pid,
            bucket: CoopTokenBucket::new(config.burst as f64, config.max_rate),
            total_allowed: 0,
            total_denied: 0,
            total_throttled: 0,
        }
    }

    /// Check rate
    pub fn check(&mut self, now: u64) -> RateLimitDecision {
        if self.bucket.try_consume(1.0, now) {
            self.total_allowed += 1;
            if self.bucket.fill_ratio() < 0.2 {
                RateLimitDecision::Warn
            } else {
                RateLimitDecision::Allow
            }
        } else {
            self.total_denied += 1;
            RateLimitDecision::Deny
        }
    }

    /// Denial rate
    #[inline]
    pub fn denial_rate(&self) -> f64 {
        let total = self.total_allowed + self.total_denied;
        if total == 0 {
            return 0.0;
        }
        self.total_denied as f64 / total as f64
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Rate limit group
#[derive(Debug)]
pub struct RateLimitGroup {
    /// Group id
    pub id: u64,
    /// Configuration
    pub config: RateLimitConfig,
    /// Per-process states
    members: BTreeMap<u64, ProcessRateState>,
    /// Group-wide window
    group_window: CoopSlidingWindow,
}

impl RateLimitGroup {
    pub fn new(id: u64, config: RateLimitConfig) -> Self {
        let window_ns = config.window_ns;
        let burst = config.burst;
        Self {
            id,
            config,
            members: BTreeMap::new(),
            group_window: CoopSlidingWindow::new(window_ns, burst * 10),
        }
    }

    /// Add member
    #[inline(always)]
    pub fn add_member(&mut self, pid: u64) {
        let state = ProcessRateState::new(pid, &self.config);
        self.members.insert(pid, state);
    }

    /// Remove member
    #[inline(always)]
    pub fn remove_member(&mut self, pid: u64) {
        self.members.remove(&pid);
    }

    /// Check rate for process
    pub fn check(&mut self, pid: u64, now: u64) -> RateLimitDecision {
        // Check group-wide first
        if !self.group_window.record(now) {
            return RateLimitDecision::Deny;
        }
        // Check per-process
        if let Some(state) = self.members.get_mut(&pid) {
            state.check(now)
        } else {
            RateLimitDecision::Deny
        }
    }
}

/// Rate limit stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopRateLimitStats {
    /// Total groups
    pub total_groups: usize,
    /// Total members
    pub total_members: usize,
    /// Total allowed
    pub total_allowed: u64,
    /// Total denied
    pub total_denied: u64,
}

/// Cooperative rate limit manager
pub struct CoopRateLimitManager {
    /// Groups
    groups: BTreeMap<u64, RateLimitGroup>,
    /// Next group id
    next_id: u64,
    /// Stats
    stats: CoopRateLimitStats,
}

impl CoopRateLimitManager {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            next_id: 1,
            stats: CoopRateLimitStats::default(),
        }
    }

    /// Create group
    #[inline]
    pub fn create_group(&mut self, config: RateLimitConfig) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let group = RateLimitGroup::new(id, config);
        self.groups.insert(id, group);
        self.update_stats();
        id
    }

    /// Add process to group
    #[inline]
    pub fn add_to_group(&mut self, group_id: u64, pid: u64) -> bool {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_member(pid);
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Check rate limit
    pub fn check(&mut self, group_id: u64, pid: u64, now: u64) -> RateLimitDecision {
        if let Some(group) = self.groups.get_mut(&group_id) {
            let decision = group.check(pid, now);
            match decision {
                RateLimitDecision::Allow | RateLimitDecision::Warn => {
                    self.stats.total_allowed += 1;
                },
                RateLimitDecision::Deny | RateLimitDecision::Throttle => {
                    self.stats.total_denied += 1;
                },
            }
            decision
        } else {
            RateLimitDecision::Deny
        }
    }

    /// Remove group
    #[inline(always)]
    pub fn remove_group(&mut self, group_id: u64) {
        self.groups.remove(&group_id);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_groups = self.groups.len();
        self.stats.total_members = self.groups.values().map(|g| g.members.len()).sum();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopRateLimitStats {
        &self.stats
    }
}

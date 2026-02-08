//! # Cooperative Throttle Protocol
//!
//! Cooperative throttling for resource management:
//! - Voluntary throttling
//! - Backpressure signaling
//! - Rate limiting coordination
//! - Congestion avoidance
//! - Fair bandwidth sharing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// THROTTLE TYPES
// ============================================================================

/// Throttle resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThrottleResource {
    /// CPU utilization
    Cpu,
    /// Memory allocation rate
    MemoryAlloc,
    /// I/O operations
    IoOps,
    /// Network send rate
    NetSend,
    /// Network receive rate
    NetRecv,
    /// Syscall rate
    SyscallRate,
}

/// Throttle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleState {
    /// Not throttled
    Normal,
    /// Lightly throttled (voluntary slowdown)
    Light,
    /// Moderately throttled
    Moderate,
    /// Heavily throttled
    Heavy,
    /// Suspended (complete stop)
    Suspended,
}

impl ThrottleState {
    /// Rate multiplier (1.0 = full speed)
    pub fn rate_multiplier(&self) -> f64 {
        match self {
            Self::Normal => 1.0,
            Self::Light => 0.75,
            Self::Moderate => 0.5,
            Self::Heavy => 0.25,
            Self::Suspended => 0.0,
        }
    }
}

/// Backpressure signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureSignal {
    /// No pressure
    None,
    /// Mild pressure (start slowing)
    Mild,
    /// Strong pressure (slow significantly)
    Strong,
    /// Critical (must stop immediately)
    Critical,
}

// ============================================================================
// THROTTLE CONFIG
// ============================================================================

/// Throttle configuration
#[derive(Debug, Clone)]
pub struct ThrottleConfig {
    /// Resource being throttled
    pub resource: ThrottleResource,
    /// Target rate (units/sec)
    pub target_rate: u64,
    /// Burst allowance
    pub burst: u64,
    /// Window size (ns)
    pub window_ns: u64,
}

impl ThrottleConfig {
    pub fn new(resource: ThrottleResource, target_rate: u64) -> Self {
        Self {
            resource,
            target_rate,
            burst: target_rate / 10,
            window_ns: 1_000_000_000, // 1 second
        }
    }
}

// ============================================================================
// TOKEN BUCKET
// ============================================================================

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
pub struct CoopTokenBucket {
    /// Tokens available
    pub tokens: u64,
    /// Max tokens (burst)
    pub max_tokens: u64,
    /// Refill rate (tokens/sec)
    pub refill_rate: u64,
    /// Last refill time
    pub last_refill: u64,
}

impl CoopTokenBucket {
    pub fn new(rate: u64, burst: u64) -> Self {
        Self {
            tokens: burst,
            max_tokens: burst,
            refill_rate: rate,
            last_refill: 0,
        }
    }

    /// Try to consume tokens
    pub fn try_consume(&mut self, count: u64, now: u64) -> bool {
        self.refill(now);
        if self.tokens >= count {
            self.tokens -= count;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    pub fn refill(&mut self, now: u64) {
        if self.last_refill == 0 {
            self.last_refill = now;
            return;
        }
        let elapsed = now.saturating_sub(self.last_refill);
        let new_tokens = (elapsed as u128 * self.refill_rate as u128 / 1_000_000_000) as u64;
        if new_tokens > 0 {
            self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
            self.last_refill = now;
        }
    }

    /// Available tokens
    pub fn available(&self) -> u64 {
        self.tokens
    }

    /// Utilization (0-1, 1 = no tokens left)
    pub fn utilization(&self) -> f64 {
        if self.max_tokens == 0 {
            return 1.0;
        }
        1.0 - (self.tokens as f64 / self.max_tokens as f64)
    }
}

// ============================================================================
// PROCESS THROTTLE STATE
// ============================================================================

/// Per-process throttle state
#[derive(Debug)]
pub struct ProcessThrottleState {
    /// Process id
    pub pid: u64,
    /// Token buckets per resource
    pub buckets: BTreeMap<u8, CoopTokenBucket>,
    /// Current throttle states
    pub states: BTreeMap<u8, ThrottleState>,
    /// Voluntary throttle
    pub voluntary: bool,
    /// Backpressure received
    pub backpressure: BackpressureSignal,
    /// Total throttled time (ns)
    pub throttled_time_ns: u64,
    /// Last state change
    pub last_change: u64,
}

impl ProcessThrottleState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            buckets: BTreeMap::new(),
            states: BTreeMap::new(),
            voluntary: false,
            backpressure: BackpressureSignal::None,
            throttled_time_ns: 0,
            last_change: 0,
        }
    }

    /// Set throttle for resource
    pub fn set_throttle(&mut self, config: &ThrottleConfig) {
        let key = config.resource as u8;
        self.buckets.insert(
            key,
            CoopTokenBucket::new(config.target_rate, config.target_rate + config.burst),
        );
        self.states.insert(key, ThrottleState::Normal);
    }

    /// Try to use resource
    pub fn try_use(&mut self, resource: ThrottleResource, amount: u64, now: u64) -> bool {
        let key = resource as u8;
        if let Some(bucket) = self.buckets.get_mut(&key) {
            bucket.try_consume(amount, now)
        } else {
            true // No throttle configured = allow
        }
    }

    /// Update throttle state
    pub fn update_state(&mut self, resource: ThrottleResource, state: ThrottleState, now: u64) {
        let key = resource as u8;
        let prev = self.states.get(&key).copied().unwrap_or(ThrottleState::Normal);
        if prev != ThrottleState::Normal && state == ThrottleState::Normal {
            // Was throttled, now normal: accumulate time
            self.throttled_time_ns += now.saturating_sub(self.last_change);
        }
        self.states.insert(key, state);
        self.last_change = now;
    }

    /// Overall throttle state (worst across resources)
    pub fn worst_state(&self) -> ThrottleState {
        self.states
            .values()
            .max_by(|a, b| {
                (*a as u8).cmp(&(*b as u8))
            })
            .copied()
            .unwrap_or(ThrottleState::Normal)
    }

    /// Volunteer to throttle
    pub fn volunteer_throttle(&mut self) {
        self.voluntary = true;
    }

    /// End voluntary throttle
    pub fn end_voluntary(&mut self) {
        self.voluntary = false;
    }
}

// ============================================================================
// THROTTLE MANAGER
// ============================================================================

/// Throttle stats
#[derive(Debug, Clone, Default)]
pub struct CoopThrottleStats {
    /// Processes throttled
    pub throttled_count: usize,
    /// Voluntary throttles
    pub voluntary_count: usize,
    /// Total throttle time (ns)
    pub total_throttle_ns: u64,
}

/// Cooperative throttle manager
pub struct CoopThrottleManager {
    /// Process states
    states: BTreeMap<u64, ProcessThrottleState>,
    /// Global backpressure
    global_pressure: BackpressureSignal,
    /// Stats
    stats: CoopThrottleStats,
}

impl CoopThrottleManager {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            global_pressure: BackpressureSignal::None,
            stats: CoopThrottleStats::default(),
        }
    }

    /// Configure throttle for process
    pub fn configure(&mut self, pid: u64, config: ThrottleConfig) {
        let state = self
            .states
            .entry(pid)
            .or_insert_with(|| ProcessThrottleState::new(pid));
        state.set_throttle(&config);
    }

    /// Try resource use
    pub fn try_use(&mut self, pid: u64, resource: ThrottleResource, amount: u64, now: u64) -> bool {
        if let Some(state) = self.states.get_mut(&pid) {
            state.try_use(resource, amount, now)
        } else {
            true
        }
    }

    /// Set global backpressure
    pub fn set_global_pressure(&mut self, signal: BackpressureSignal) {
        self.global_pressure = signal;
        // Propagate to all processes
        for state in self.states.values_mut() {
            state.backpressure = signal;
        }
    }

    /// Process volunteers to throttle
    pub fn volunteer(&mut self, pid: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.volunteer_throttle();
        }
        self.update_stats();
    }

    /// Get throttle state
    pub fn throttle_state(&self, pid: u64) -> ThrottleState {
        self.states
            .get(&pid)
            .map(|s| s.worst_state())
            .unwrap_or(ThrottleState::Normal)
    }

    fn update_stats(&mut self) {
        self.stats.throttled_count = self
            .states
            .values()
            .filter(|s| s.worst_state() != ThrottleState::Normal)
            .count();
        self.stats.voluntary_count = self.states.values().filter(|s| s.voluntary).count();
        self.stats.total_throttle_ns = self.states.values().map(|s| s.throttled_time_ns).sum();
    }

    /// Stats
    pub fn stats(&self) -> &CoopThrottleStats {
        &self.stats
    }
}

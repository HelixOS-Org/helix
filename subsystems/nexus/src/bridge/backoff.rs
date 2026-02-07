//! # Bridge Backoff Manager
//!
//! Adaptive backoff strategies for failed syscall retries:
//! - Exponential backoff with jitter
//! - Per-error-class backoff policies
//! - Circuit-breaker integration
//! - Adaptive ceiling based on system load
//! - Cooldown tracking per process

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// BACKOFF TYPES
// ============================================================================

/// Backoff strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// Fixed delay
    Fixed,
    /// Linear increase
    Linear,
    /// Exponential increase
    Exponential,
    /// Exponential with decorrelated jitter
    ExponentialJitter,
    /// Full jitter (random 0..cap)
    FullJitter,
}

/// Backoff state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffState {
    /// Not backing off
    Idle,
    /// Active backoff
    Active,
    /// At maximum delay
    Ceiling,
    /// Permanently failed (exceeded max attempts)
    Failed,
}

/// Error class for categorized backoff
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient, likely to succeed on retry
    Transient,
    /// Throttled — resource exhaustion
    Throttled,
    /// Server unavailable
    Unavailable,
    /// Timeout
    Timeout,
    /// Permanent — no retry
    Permanent,
}

// ============================================================================
// BACKOFF CONFIG
// ============================================================================

/// Backoff configuration
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Strategy
    pub strategy: BackoffStrategy,
    /// Initial delay (ns)
    pub initial_delay_ns: u64,
    /// Maximum delay (ns)
    pub max_delay_ns: u64,
    /// Multiplier (for exponential)
    pub multiplier: f64,
    /// Maximum attempts
    pub max_attempts: u32,
    /// Jitter factor (0.0-1.0)
    pub jitter_factor: f64,
}

impl BackoffConfig {
    /// Default config for transient errors
    pub fn transient() -> Self {
        Self {
            strategy: BackoffStrategy::ExponentialJitter,
            initial_delay_ns: 1_000_000,  // 1ms
            max_delay_ns: 30_000_000_000, // 30s
            multiplier: 2.0,
            max_attempts: 10,
            jitter_factor: 0.25,
        }
    }

    /// Default config for throttled errors
    pub fn throttled() -> Self {
        Self {
            strategy: BackoffStrategy::Exponential,
            initial_delay_ns: 10_000_000, // 10ms
            max_delay_ns: 60_000_000_000, // 60s
            multiplier: 2.5,
            max_attempts: 8,
            jitter_factor: 0.1,
        }
    }

    /// Default config for timeout
    pub fn timeout() -> Self {
        Self {
            strategy: BackoffStrategy::Linear,
            initial_delay_ns: 5_000_000,  // 5ms
            max_delay_ns: 10_000_000_000, // 10s
            multiplier: 1.0,
            max_attempts: 5,
            jitter_factor: 0.0,
        }
    }
}

// ============================================================================
// BACKOFF STATE MACHINE
// ============================================================================

/// Active backoff tracker for one operation
#[derive(Debug, Clone)]
pub struct BackoffTracker {
    /// Config
    pub config: BackoffConfig,
    /// Current attempt
    pub attempt: u32,
    /// Current delay (ns)
    pub current_delay_ns: u64,
    /// State
    pub state: BackoffState,
    /// Total wait time so far (ns)
    pub total_wait_ns: u64,
    /// Last failure timestamp
    pub last_failure_ns: u64,
    /// Pseudo-random state for jitter
    rng_state: u64,
}

impl BackoffTracker {
    pub fn new(config: BackoffConfig) -> Self {
        let initial = config.initial_delay_ns;
        Self {
            config,
            attempt: 0,
            current_delay_ns: initial,
            state: BackoffState::Idle,
            total_wait_ns: 0,
            last_failure_ns: 0,
            rng_state: 0x12345678_9abcdef0,
        }
    }

    /// Record a failure — returns next delay or None if max attempts exceeded
    pub fn record_failure(&mut self, now: u64) -> Option<u64> {
        self.attempt += 1;
        self.last_failure_ns = now;

        if self.attempt >= self.config.max_attempts {
            self.state = BackoffState::Failed;
            return None;
        }

        self.state = BackoffState::Active;

        let base_delay = match self.config.strategy {
            BackoffStrategy::Fixed => self.config.initial_delay_ns,
            BackoffStrategy::Linear => {
                self.config.initial_delay_ns + (self.attempt as u64 * self.config.initial_delay_ns)
            },
            BackoffStrategy::Exponential | BackoffStrategy::ExponentialJitter => {
                let factor = libm::pow(self.config.multiplier, self.attempt as f64);
                (self.config.initial_delay_ns as f64 * factor) as u64
            },
            BackoffStrategy::FullJitter => {
                let factor = libm::pow(self.config.multiplier, self.attempt as f64);
                let cap = (self.config.initial_delay_ns as f64 * factor) as u64;
                self.pseudo_random() % (cap.max(1))
            },
        };

        let mut delay = base_delay.min(self.config.max_delay_ns);

        // Apply jitter
        if self.config.jitter_factor > 0.0 && self.config.strategy != BackoffStrategy::FullJitter {
            let jitter_range = (delay as f64 * self.config.jitter_factor) as u64;
            if jitter_range > 0 {
                let jitter = self.pseudo_random() % (jitter_range * 2);
                delay = delay.saturating_sub(jitter_range).saturating_add(jitter);
            }
        }

        delay = delay.min(self.config.max_delay_ns);
        if delay >= self.config.max_delay_ns {
            self.state = BackoffState::Ceiling;
        }

        self.current_delay_ns = delay;
        self.total_wait_ns += delay;
        Some(delay)
    }

    /// Record success — reset
    pub fn record_success(&mut self) {
        self.attempt = 0;
        self.current_delay_ns = self.config.initial_delay_ns;
        self.state = BackoffState::Idle;
        self.total_wait_ns = 0;
    }

    /// Check if should retry now
    pub fn should_retry_now(&self, now: u64) -> bool {
        match self.state {
            BackoffState::Active | BackoffState::Ceiling => {
                now >= self.last_failure_ns + self.current_delay_ns
            },
            BackoffState::Idle => true,
            BackoffState::Failed => false,
        }
    }

    /// Pseudo-random number (xorshift64)
    fn pseudo_random(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }
}

// ============================================================================
// PER-PROCESS BACKOFF
// ============================================================================

/// Per-process backoff state
#[derive(Debug)]
pub struct ProcessBackoff {
    /// Process ID
    pub pid: u64,
    /// Active trackers, keyed by operation hash
    trackers: BTreeMap<u64, BackoffTracker>,
    /// Total failures
    pub total_failures: u64,
    /// Total successes after retry
    pub retry_successes: u64,
}

impl ProcessBackoff {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            trackers: BTreeMap::new(),
            total_failures: 0,
            retry_successes: 0,
        }
    }

    /// Get or create tracker for operation
    pub fn tracker_mut(
        &mut self,
        operation_key: u64,
        config: BackoffConfig,
    ) -> &mut BackoffTracker {
        self.trackers
            .entry(operation_key)
            .or_insert_with(|| BackoffTracker::new(config))
    }

    /// Record failure
    pub fn record_failure(
        &mut self,
        operation_key: u64,
        config: BackoffConfig,
        now: u64,
    ) -> Option<u64> {
        self.total_failures += 1;
        let tracker = self.tracker_mut(operation_key, config);
        tracker.record_failure(now)
    }

    /// Record success
    pub fn record_success(&mut self, operation_key: u64) {
        if let Some(tracker) = self.trackers.get_mut(&operation_key) {
            if tracker.attempt > 0 {
                self.retry_successes += 1;
            }
            tracker.record_success();
        }
    }

    /// Cleanup completed trackers
    pub fn cleanup(&mut self) {
        self.trackers.retain(|_, t| t.state != BackoffState::Idle);
    }

    /// Active backoff count
    pub fn active_count(&self) -> usize {
        self.trackers
            .values()
            .filter(|t| t.state == BackoffState::Active || t.state == BackoffState::Ceiling)
            .count()
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Backoff manager stats
#[derive(Debug, Clone, Default)]
pub struct BridgeBackoffStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Active backoffs
    pub active_backoffs: usize,
    /// Total failures handled
    pub total_failures: u64,
    /// Retry successes
    pub retry_successes: u64,
    /// Permanently failed
    pub permanently_failed: u64,
}

/// Bridge backoff manager
pub struct BridgeBackoffManager {
    /// Per-process state
    processes: BTreeMap<u64, ProcessBackoff>,
    /// Per-error-class configs
    configs: BTreeMap<u8, BackoffConfig>,
    /// Stats
    stats: BridgeBackoffStats,
}

impl BridgeBackoffManager {
    pub fn new() -> Self {
        let mut configs = BTreeMap::new();
        configs.insert(ErrorClass::Transient as u8, BackoffConfig::transient());
        configs.insert(ErrorClass::Throttled as u8, BackoffConfig::throttled());
        configs.insert(ErrorClass::Timeout as u8, BackoffConfig::timeout());
        Self {
            processes: BTreeMap::new(),
            configs,
            stats: BridgeBackoffStats::default(),
        }
    }

    /// Operation key (FNV-1a)
    fn operation_key(syscall_nr: u32, arg_hash: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= syscall_nr as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= arg_hash;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Record failure
    pub fn record_failure(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        arg_hash: u64,
        error_class: ErrorClass,
        now: u64,
    ) -> Option<u64> {
        if error_class == ErrorClass::Permanent {
            return None;
        }
        let config = self
            .configs
            .get(&(error_class as u8))
            .cloned()
            .unwrap_or_else(BackoffConfig::transient);
        let key = Self::operation_key(syscall_nr, arg_hash);
        let proc = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessBackoff::new(pid));
        let result = proc.record_failure(key, config, now);
        self.stats.total_failures += 1;
        if result.is_none() {
            self.stats.permanently_failed += 1;
        }
        self.update_stats();
        result
    }

    /// Record success
    pub fn record_success(&mut self, pid: u64, syscall_nr: u32, arg_hash: u64) {
        let key = Self::operation_key(syscall_nr, arg_hash);
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.record_success(key);
            self.stats.retry_successes += 1;
        }
        self.update_stats();
    }

    /// Should retry now
    pub fn should_retry(&self, pid: u64, syscall_nr: u32, arg_hash: u64, now: u64) -> bool {
        let key = Self::operation_key(syscall_nr, arg_hash);
        self.processes
            .get(&pid)
            .and_then(|p| p.trackers.get(&key))
            .map(|t| t.should_retry_now(now))
            .unwrap_or(true)
    }

    /// Cleanup idle trackers
    pub fn cleanup(&mut self) {
        for proc in self.processes.values_mut() {
            proc.cleanup();
        }
        self.processes
            .retain(|_, p| p.active_count() > 0 || p.total_failures > 0);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.active_backoffs = self.processes.values().map(|p| p.active_count()).sum();
    }

    /// Stats
    pub fn stats(&self) -> &BridgeBackoffStats {
        &self.stats
    }
}

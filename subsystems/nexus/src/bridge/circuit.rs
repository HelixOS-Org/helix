//! # Bridge Circuit Breaker
//!
//! Circuit breaker pattern for syscall bridge:
//! - Automatic failure detection
//! - Half-open probing
//! - Recovery tracking
//! - Per-syscall circuit breakers
//! - Cascading failure prevention

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CIRCUIT BREAKER TYPES
// ============================================================================

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Closed (normal operation)
    Closed,
    /// Open (rejecting calls)
    Open,
    /// Half-open (probing recovery)
    HalfOpen,
}

/// Failure type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeFailureType {
    /// Timeout
    Timeout,
    /// Error return
    Error,
    /// Panic
    Panic,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Invalid argument
    InvalidArgument,
}

/// Circuit breaker config
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to trip
    pub failure_threshold: u32,
    /// Window size for failure counting (ns)
    pub window_ns: u64,
    /// Cooldown before half-open (ns)
    pub cooldown_ns: u64,
    /// Probe count in half-open
    pub probe_count: u32,
    /// Success threshold to close
    pub success_threshold: u32,
}

impl CircuitBreakerConfig {
    #[inline]
    pub fn default_config() -> Self {
        Self {
            failure_threshold: 5,
            window_ns: 10_000_000_000, // 10s
            cooldown_ns: 5_000_000_000,  // 5s
            probe_count: 3,
            success_threshold: 2,
        }
    }

    #[inline]
    pub fn aggressive() -> Self {
        Self {
            failure_threshold: 3,
            window_ns: 5_000_000_000,
            cooldown_ns: 2_000_000_000,
            probe_count: 2,
            success_threshold: 2,
        }
    }

    #[inline]
    pub fn conservative() -> Self {
        Self {
            failure_threshold: 10,
            window_ns: 30_000_000_000,
            cooldown_ns: 15_000_000_000,
            probe_count: 5,
            success_threshold: 4,
        }
    }
}

// ============================================================================
// FAILURE RECORD
// ============================================================================

/// Failure event
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FailureEvent {
    /// Failure type
    pub failure_type: BridgeFailureType,
    /// Timestamp
    pub timestamp: u64,
    /// Latency before failure (ns)
    pub latency_ns: u64,
}

/// Sliding window failure counter
#[derive(Debug, Clone)]
pub struct FailureWindow {
    /// Events in window
    events: Vec<FailureEvent>,
    /// Window size (ns)
    window_ns: u64,
}

impl FailureWindow {
    pub fn new(window_ns: u64) -> Self {
        Self {
            events: Vec::new(),
            window_ns,
        }
    }

    /// Record failure
    #[inline]
    pub fn record(&mut self, event: FailureEvent) {
        let cutoff = event.timestamp.saturating_sub(self.window_ns);
        self.events.retain(|e| e.timestamp >= cutoff);
        self.events.push(event);
    }

    /// Failure count in window
    #[inline(always)]
    pub fn count(&self, now: u64) -> u32 {
        let cutoff = now.saturating_sub(self.window_ns);
        self.events.iter().filter(|e| e.timestamp >= cutoff).count() as u32
    }

    /// Failure rate (per second)
    #[inline]
    pub fn rate(&self, now: u64) -> f64 {
        let c = self.count(now);
        if self.window_ns == 0 {
            return 0.0;
        }
        c as f64 / (self.window_ns as f64 / 1_000_000_000.0)
    }

    /// Most common failure type
    pub fn dominant_type(&self) -> Option<BridgeFailureType> {
        if self.events.is_empty() {
            return None;
        }
        let mut counts = BTreeMap::new();
        for e in &self.events {
            *counts.entry(e.failure_type as u8).or_insert(0u32) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(_, c)| *c)
            .and_then(|(k, _)| match k {
                0 => Some(BridgeFailureType::Timeout),
                1 => Some(BridgeFailureType::Error),
                2 => Some(BridgeFailureType::Panic),
                3 => Some(BridgeFailureType::ResourceExhaustion),
                4 => Some(BridgeFailureType::InvalidArgument),
                _ => None,
            })
    }
}

// ============================================================================
// CIRCUIT BREAKER
// ============================================================================

/// Individual circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Syscall number or category
    pub target: u32,
    /// Current state
    pub state: CircuitState,
    /// Config
    pub config: CircuitBreakerConfig,
    /// Failure window
    failures: FailureWindow,
    /// Probe successes in half-open
    probe_successes: u32,
    /// Probe attempts in half-open
    probe_attempts: u32,
    /// Time circuit opened
    opened_at: u64,
    /// Total trips
    pub total_trips: u64,
    /// Total successes
    total_success: u64,
    /// Total failures
    total_failure: u64,
}

impl CircuitBreaker {
    pub fn new(target: u32, config: CircuitBreakerConfig) -> Self {
        let window_ns = config.window_ns;
        Self {
            target,
            state: CircuitState::Closed,
            config,
            failures: FailureWindow::new(window_ns),
            probe_successes: 0,
            probe_attempts: 0,
            opened_at: 0,
            total_trips: 0,
            total_success: 0,
            total_failure: 0,
        }
    }

    /// Can a call proceed?
    #[inline]
    pub fn allow(&self, now: u64) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check cooldown
                now.saturating_sub(self.opened_at) >= self.config.cooldown_ns
            }
            CircuitState::HalfOpen => self.probe_attempts < self.config.probe_count,
        }
    }

    /// Record success
    pub fn record_success(&mut self, now: u64) {
        self.total_success += 1;

        match self.state {
            CircuitState::Closed => {}
            CircuitState::HalfOpen => {
                self.probe_successes += 1;
                self.probe_attempts += 1;
                if self.probe_successes >= self.config.success_threshold {
                    self.state = CircuitState::Closed;
                    self.probe_successes = 0;
                    self.probe_attempts = 0;
                }
            }
            CircuitState::Open => {
                if now.saturating_sub(self.opened_at) >= self.config.cooldown_ns {
                    self.state = CircuitState::HalfOpen;
                    self.probe_successes = 1;
                    self.probe_attempts = 1;
                }
            }
        }
    }

    /// Record failure
    pub fn record_failure(&mut self, failure_type: BridgeFailureType, latency_ns: u64, now: u64) {
        self.total_failure += 1;
        self.failures.record(FailureEvent {
            failure_type,
            timestamp: now,
            latency_ns,
        });

        match self.state {
            CircuitState::Closed => {
                if self.failures.count(now) >= self.config.failure_threshold {
                    self.state = CircuitState::Open;
                    self.opened_at = now;
                    self.total_trips += 1;
                }
            }
            CircuitState::HalfOpen => {
                self.probe_attempts += 1;
                // Any failure in half-open re-opens
                self.state = CircuitState::Open;
                self.opened_at = now;
                self.total_trips += 1;
                self.probe_successes = 0;
                self.probe_attempts = 0;
            }
            CircuitState::Open => {}
        }
    }

    /// Success rate
    #[inline]
    pub fn success_rate(&self) -> f64 {
        let total = self.total_success + self.total_failure;
        if total == 0 {
            return 1.0;
        }
        self.total_success as f64 / total as f64
    }

    /// Failure rate in window
    #[inline(always)]
    pub fn failure_rate(&self, now: u64) -> f64 {
        self.failures.rate(now)
    }

    /// Time in current state (ns)
    #[inline]
    pub fn state_duration(&self, now: u64) -> u64 {
        match self.state {
            CircuitState::Open | CircuitState::HalfOpen => now.saturating_sub(self.opened_at),
            CircuitState::Closed => 0,
        }
    }
}

// ============================================================================
// CIRCUIT BREAKER MANAGER
// ============================================================================

/// Circuit breaker stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CircuitBreakerStats {
    /// Total breakers
    pub total_breakers: usize,
    /// Open circuits
    pub open_circuits: usize,
    /// Half-open circuits
    pub half_open_circuits: usize,
    /// Total trips
    pub total_trips: u64,
    /// Calls rejected
    pub calls_rejected: u64,
}

/// Circuit breaker manager
#[repr(align(64))]
pub struct BridgeCircuitBreakerManager {
    /// Breakers per target
    breakers: BTreeMap<u32, CircuitBreaker>,
    /// Default config
    default_config: CircuitBreakerConfig,
    /// Stats
    stats: CircuitBreakerStats,
}

impl BridgeCircuitBreakerManager {
    pub fn new() -> Self {
        Self {
            breakers: BTreeMap::new(),
            default_config: CircuitBreakerConfig::default_config(),
            stats: CircuitBreakerStats::default(),
        }
    }

    /// Register breaker
    #[inline]
    pub fn register(&mut self, target: u32, config: CircuitBreakerConfig) {
        self.breakers
            .insert(target, CircuitBreaker::new(target, config));
        self.stats.total_breakers = self.breakers.len();
    }

    /// Check if call can proceed
    pub fn allow(&mut self, target: u32, now: u64) -> bool {
        let breaker = self
            .breakers
            .entry(target)
            .or_insert_with(|| CircuitBreaker::new(target, self.default_config.clone()));

        if breaker.allow(now) {
            true
        } else {
            self.stats.calls_rejected += 1;
            false
        }
    }

    /// Record success
    #[inline]
    pub fn record_success(&mut self, target: u32, now: u64) {
        if let Some(b) = self.breakers.get_mut(&target) {
            b.record_success(now);
            self.update_stats();
        }
    }

    /// Record failure
    #[inline]
    pub fn record_failure(
        &mut self,
        target: u32,
        failure_type: BridgeFailureType,
        latency_ns: u64,
        now: u64,
    ) {
        if let Some(b) = self.breakers.get_mut(&target) {
            b.record_failure(failure_type, latency_ns, now);
            self.update_stats();
        }
    }

    /// Get breaker state
    #[inline(always)]
    pub fn breaker_state(&self, target: u32) -> Option<CircuitState> {
        self.breakers.get(&target).map(|b| b.state)
    }

    /// All open circuits
    #[inline]
    pub fn open_circuits(&self) -> Vec<u32> {
        self.breakers
            .iter()
            .filter(|(_, b)| matches!(b.state, CircuitState::Open))
            .map(|(&t, _)| t)
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.open_circuits = self
            .breakers
            .values()
            .filter(|b| matches!(b.state, CircuitState::Open))
            .count();
        self.stats.half_open_circuits = self
            .breakers
            .values()
            .filter(|b| matches!(b.state, CircuitState::HalfOpen))
            .count();
        self.stats.total_trips = self.breakers.values().map(|b| b.total_trips).sum();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CircuitBreakerStats {
        &self.stats
    }
}

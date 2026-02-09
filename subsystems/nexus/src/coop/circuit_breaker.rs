//! # Coop Circuit Breaker
//!
//! Circuit breaker pattern for cooperative subsystem resilience:
//! - Half-open probe management
//! - Sliding window failure detection
//! - Exponential backoff on failures
//! - Bulkhead isolation
//! - Cascading failure prevention
//! - Health score-based tripping

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Circuit state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Failure type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureType {
    Timeout,
    Error,
    Overload,
    Rejected,
    Crash,
    Degraded,
}

/// Sliding window entry
#[derive(Debug, Clone, Copy)]
pub struct WindowEntry {
    pub timestamp: u64,
    pub success: bool,
    pub latency_ns: u64,
    pub failure_type: Option<FailureType>,
}

/// Circuit breaker config
#[derive(Debug, Clone, Copy)]
pub struct CircuitConfig {
    pub failure_threshold: f64,
    pub min_requests: u32,
    pub window_size_ns: u64,
    pub open_timeout_ns: u64,
    pub half_open_max_probes: u32,
    pub success_threshold_to_close: u32,
    pub max_backoff_ns: u64,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 0.5,
            min_requests: 10,
            window_size_ns: 60_000_000_000,
            open_timeout_ns: 30_000_000_000,
            half_open_max_probes: 3,
            success_threshold_to_close: 3,
            max_backoff_ns: 300_000_000_000,
        }
    }
}

/// Individual circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub name: String,
    pub state: CircuitState,
    pub config: CircuitConfig,
    pub window: Vec<WindowEntry>,
    pub total_requests: u64,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_rejections: u64,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_failure_ts: u64,
    pub last_state_change_ts: u64,
    pub half_open_probes: u32,
    pub half_open_successes: u32,
    pub trip_count: u64,
    pub current_backoff_ns: u64,
}

impl CircuitBreaker {
    pub fn new(name: String, config: CircuitConfig) -> Self {
        Self {
            name, state: CircuitState::Closed, config, window: Vec::new(),
            total_requests: 0, total_successes: 0, total_failures: 0,
            total_rejections: 0, consecutive_failures: 0, consecutive_successes: 0,
            last_failure_ts: 0, last_state_change_ts: 0, half_open_probes: 0,
            half_open_successes: 0, trip_count: 0,
            current_backoff_ns: config.open_timeout_ns,
        }
    }

    #[inline]
    pub fn can_execute(&self, now: u64) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => now.saturating_sub(self.last_state_change_ts) >= self.current_backoff_ns,
            CircuitState::HalfOpen => self.half_open_probes < self.config.half_open_max_probes,
        }
    }

    pub fn record_success(&mut self, now: u64, latency_ns: u64) {
        self.total_requests += 1;
        self.total_successes += 1;
        self.consecutive_successes += 1;
        self.consecutive_failures = 0;
        self.window.push(WindowEntry { timestamp: now, success: true, latency_ns, failure_type: None });
        self.trim_window(now);

        match self.state {
            CircuitState::HalfOpen => {
                self.half_open_successes += 1;
                if self.half_open_successes >= self.config.success_threshold_to_close {
                    self.transition(CircuitState::Closed, now);
                }
            }
            _ => {}
        }
    }

    pub fn record_failure(&mut self, now: u64, latency_ns: u64, failure: FailureType) {
        self.total_requests += 1;
        self.total_failures += 1;
        self.consecutive_failures += 1;
        self.consecutive_successes = 0;
        self.last_failure_ts = now;
        self.window.push(WindowEntry { timestamp: now, success: false, latency_ns, failure_type: Some(failure) });
        self.trim_window(now);

        match self.state {
            CircuitState::Closed => {
                if self.should_trip() { self.trip(now); }
            }
            CircuitState::HalfOpen => {
                self.trip(now);
            }
            _ => {}
        }
    }

    #[inline(always)]
    pub fn record_rejection(&mut self) { self.total_rejections += 1; }

    fn trim_window(&mut self, now: u64) {
        let cutoff = now.saturating_sub(self.config.window_size_ns);
        self.window.retain(|e| e.timestamp >= cutoff);
    }

    fn should_trip(&self) -> bool {
        let total = self.window.len() as u32;
        if total < self.config.min_requests { return false; }
        let failures = self.window.iter().filter(|e| !e.success).count() as f64;
        let rate = failures / total as f64;
        rate >= self.config.failure_threshold
    }

    fn trip(&mut self, now: u64) {
        self.transition(CircuitState::Open, now);
        self.trip_count += 1;
        // Exponential backoff
        self.current_backoff_ns = (self.current_backoff_ns * 2).min(self.config.max_backoff_ns);
    }

    fn transition(&mut self, new_state: CircuitState, now: u64) {
        self.state = new_state;
        self.last_state_change_ts = now;
        if new_state == CircuitState::HalfOpen {
            self.half_open_probes = 0;
            self.half_open_successes = 0;
        }
        if new_state == CircuitState::Closed {
            self.current_backoff_ns = self.config.open_timeout_ns;
            self.consecutive_failures = 0;
        }
    }

    #[inline]
    pub fn check_transition(&mut self, now: u64) {
        if self.state == CircuitState::Open && self.can_execute(now) {
            self.transition(CircuitState::HalfOpen, now);
        }
    }

    #[inline]
    pub fn failure_rate(&self) -> f64 {
        if self.window.is_empty() { return 0.0; }
        let failures = self.window.iter().filter(|e| !e.success).count() as f64;
        failures / self.window.len() as f64
    }

    #[inline]
    pub fn avg_latency_ns(&self) -> f64 {
        if self.window.is_empty() { return 0.0; }
        let sum: u64 = self.window.iter().map(|e| e.latency_ns).sum();
        sum as f64 / self.window.len() as f64
    }

    #[inline]
    pub fn p99_latency_ns(&self) -> u64 {
        if self.window.is_empty() { return 0; }
        let mut latencies: Vec<u64> = self.window.iter().map(|e| e.latency_ns).collect();
        latencies.sort_unstable();
        let idx = ((latencies.len() as f64) * 0.99) as usize;
        latencies[idx.min(latencies.len() - 1)]
    }
}

/// Bulkhead
#[derive(Debug, Clone)]
pub struct Bulkhead {
    pub name: String,
    pub max_concurrent: u32,
    pub current_concurrent: u32,
    pub queue_size: u32,
    pub max_queue: u32,
    pub total_accepted: u64,
    pub total_rejected: u64,
}

impl Bulkhead {
    pub fn new(name: String, max_concurrent: u32, max_queue: u32) -> Self {
        Self { name, max_concurrent, current_concurrent: 0, queue_size: 0, max_queue, total_accepted: 0, total_rejected: 0 }
    }

    #[inline]
    pub fn try_acquire(&mut self) -> bool {
        if self.current_concurrent < self.max_concurrent { self.current_concurrent += 1; self.total_accepted += 1; true }
        else if self.queue_size < self.max_queue { self.queue_size += 1; self.total_accepted += 1; true }
        else { self.total_rejected += 1; false }
    }

    #[inline]
    pub fn release(&mut self) {
        if self.current_concurrent > 0 { self.current_concurrent -= 1; }
        if self.queue_size > 0 && self.current_concurrent < self.max_concurrent {
            self.queue_size -= 1;
            self.current_concurrent += 1;
        }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.max_concurrent == 0 { return 0.0; }
        self.current_concurrent as f64 / self.max_concurrent as f64
    }
}

/// Circuit breaker manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CircuitBreakerStats {
    pub total_circuits: usize,
    pub closed_circuits: usize,
    pub open_circuits: usize,
    pub half_open_circuits: usize,
    pub total_trips: u64,
    pub total_bulkheads: usize,
    pub avg_failure_rate: f64,
}

/// Coop circuit breaker manager
pub struct CoopCircuitBreaker {
    circuits: BTreeMap<u64, CircuitBreaker>,
    bulkheads: BTreeMap<u64, Bulkhead>,
    stats: CircuitBreakerStats,
    next_id: u64,
}

impl CoopCircuitBreaker {
    pub fn new() -> Self {
        Self { circuits: BTreeMap::new(), bulkheads: BTreeMap::new(), stats: CircuitBreakerStats::default(), next_id: 1 }
    }

    fn name_hash(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash
    }

    #[inline]
    pub fn create_circuit(&mut self, name: String, config: CircuitConfig) -> u64 {
        let id = Self::name_hash(&name);
        self.circuits.entry(id).or_insert_with(|| CircuitBreaker::new(name, config));
        id
    }

    #[inline]
    pub fn create_bulkhead(&mut self, name: String, max_concurrent: u32, max_queue: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.bulkheads.insert(id, Bulkhead::new(name, max_concurrent, max_queue));
        id
    }

    #[inline(always)]
    pub fn can_execute(&self, circuit_id: u64, now: u64) -> bool {
        self.circuits.get(&circuit_id).map_or(false, |c| c.can_execute(now))
    }

    #[inline(always)]
    pub fn record_success(&mut self, circuit_id: u64, now: u64, latency_ns: u64) {
        if let Some(c) = self.circuits.get_mut(&circuit_id) { c.record_success(now, latency_ns); }
    }

    #[inline(always)]
    pub fn record_failure(&mut self, circuit_id: u64, now: u64, latency_ns: u64, failure: FailureType) {
        if let Some(c) = self.circuits.get_mut(&circuit_id) { c.record_failure(now, latency_ns, failure); }
    }

    #[inline(always)]
    pub fn tick(&mut self, now: u64) {
        for c in self.circuits.values_mut() { c.check_transition(now); }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_circuits = self.circuits.len();
        self.stats.closed_circuits = self.circuits.values().filter(|c| c.state == CircuitState::Closed).count();
        self.stats.open_circuits = self.circuits.values().filter(|c| c.state == CircuitState::Open).count();
        self.stats.half_open_circuits = self.circuits.values().filter(|c| c.state == CircuitState::HalfOpen).count();
        self.stats.total_trips = self.circuits.values().map(|c| c.trip_count).sum();
        self.stats.total_bulkheads = self.bulkheads.len();
        let rates: Vec<f64> = self.circuits.values().map(|c| c.failure_rate()).collect();
        self.stats.avg_failure_rate = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
    }

    #[inline(always)]
    pub fn circuit(&self, id: u64) -> Option<&CircuitBreaker> { self.circuits.get(&id) }
    #[inline(always)]
    pub fn bulkhead(&self, id: u64) -> Option<&Bulkhead> { self.bulkheads.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &CircuitBreakerStats { &self.stats }
}

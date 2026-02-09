//! # Coop Retry Policy
//!
//! Cooperative retry policy management:
//! - Multiple backoff strategies (exponential, linear, fibonacci, decorrelated jitter)
//! - Circuit breaker integration
//! - Per-operation retry budgets
//! - Retry storm detection and cooperative throttling
//! - Deadline-aware retry scheduling
//! - Outcome tracking and adaptive policy tuning

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Backoff strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Constant,
    /// Linearly increasing delay
    Linear,
    /// Exponentially increasing delay
    Exponential,
    /// Fibonacci sequence delay
    Fibonacci,
    /// Decorrelated jitter (AWS-style)
    DecorrelatedJitter,
}

/// Retry outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryOutcome {
    Success,
    Failure,
    Timeout,
    CircuitOpen,
    BudgetExhausted,
    DeadlineExceeded,
}

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// A retry policy definition
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub name: String,
    pub strategy: BackoffStrategy,
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter_factor: f64,
    pub deadline_ms: Option<u64>,
}

impl RetryPolicy {
    pub fn compute_delay(&self, attempt: u32, seed: u64) -> u64 {
        let raw = match self.strategy {
            BackoffStrategy::Constant => self.base_delay_ms,
            BackoffStrategy::Linear => self.base_delay_ms.saturating_mul(attempt as u64 + 1),
            BackoffStrategy::Exponential => {
                let exp = if attempt >= 20 { 20 } else { attempt };
                self.base_delay_ms.saturating_mul(1u64 << exp)
            }
            BackoffStrategy::Fibonacci => {
                let mut a: u64 = 1;
                let mut b: u64 = 1;
                for _ in 0..attempt.min(60) { let c = a.saturating_add(b); a = b; b = c; }
                self.base_delay_ms.saturating_mul(a)
            }
            BackoffStrategy::DecorrelatedJitter => {
                // delay = random_between(base, prev_delay * 3)
                let prev = if attempt == 0 { self.base_delay_ms } else {
                    let exp = if attempt - 1 >= 20 { 20 } else { attempt - 1 };
                    self.base_delay_ms.saturating_mul(1u64 << exp)
                };
                let upper = prev.saturating_mul(3).min(self.max_delay_ms);
                let range = upper.saturating_sub(self.base_delay_ms);
                if range == 0 { self.base_delay_ms } else {
                    // xorshift-based pseudo-random
                    let mut s = seed ^ (attempt as u64).wrapping_mul(0x9e3779b97f4a7c15);
                    s ^= s << 13; s ^= s >> 7; s ^= s << 17;
                    self.base_delay_ms + (s % range)
                }
            }
        };
        let capped = raw.min(self.max_delay_ms);
        // Apply jitter
        if self.jitter_factor > 0.0 && self.jitter_factor <= 1.0 {
            let jitter_range = (capped as f64 * self.jitter_factor) as u64;
            if jitter_range > 0 {
                let mut s = seed ^ capped;
                s ^= s << 13; s ^= s >> 7; s ^= s << 17;
                let jitter = s % (jitter_range * 2);
                let result = if jitter > jitter_range {
                    capped.saturating_add(jitter - jitter_range)
                } else {
                    capped.saturating_sub(jitter)
                };
                return result.min(self.max_delay_ms);
            }
        }
        capped
    }

    #[inline]
    pub fn within_deadline(&self, elapsed_ms: u64, next_delay_ms: u64) -> bool {
        match self.deadline_ms {
            Some(dl) => elapsed_ms + next_delay_ms < dl,
            None => true,
        }
    }
}

/// Per-operation retry state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RetryState {
    pub operation_id: u64,
    pub policy_name: String,
    pub attempt: u32,
    pub started_ms: u64,
    pub last_attempt_ms: u64,
    pub outcome: Option<RetryOutcome>,
    pub total_delay_ms: u64,
}

/// Circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub open_since_ms: u64,
    pub cool_down_ms: u64,
    pub half_open_attempts: u32,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, cool_down_ms: u64) -> Self {
        Self {
            state: CircuitState::Closed, failure_count: 0, success_count: 0,
            failure_threshold, success_threshold, open_since_ms: 0,
            cool_down_ms, half_open_attempts: 0,
        }
    }

    #[inline]
    pub fn can_attempt(&self, now_ms: u64) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => now_ms.saturating_sub(self.open_since_ms) >= self.cool_down_ms,
            CircuitState::HalfOpen => self.half_open_attempts < self.success_threshold,
        }
    }

    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => { self.failure_count = 0; self.success_count += 1; }
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count = 0;
                }
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&mut self, now_ms: u64) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.open_since_ms = now_ms;
                }
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.open_since_ms = now_ms;
                self.failure_count += 1;
            }
            CircuitState::Open => {}
        }
    }

    #[inline]
    pub fn check_transition(&mut self, now_ms: u64) {
        if self.state == CircuitState::Open && now_ms.saturating_sub(self.open_since_ms) >= self.cool_down_ms {
            self.state = CircuitState::HalfOpen;
            self.success_count = 0;
            self.half_open_attempts = 0;
        }
    }
}

/// Retry storm detector
#[derive(Debug, Clone)]
pub struct StormDetector {
    pub window_ms: u64,
    pub threshold: u32,
    pub recent_retries: Vec<u64>,
    pub storm_detected: bool,
}

impl StormDetector {
    pub fn new(window_ms: u64, threshold: u32) -> Self {
        Self { window_ms, threshold, recent_retries: Vec::new(), storm_detected: false }
    }

    #[inline]
    pub fn record_retry(&mut self, ts: u64) {
        self.recent_retries.push(ts);
        let cutoff = ts.saturating_sub(self.window_ms);
        self.recent_retries.retain(|&t| t >= cutoff);
        self.storm_detected = self.recent_retries.len() as u32 >= self.threshold;
    }

    #[inline(always)]
    pub fn retry_rate(&self) -> f64 {
        if self.window_ms == 0 { return 0.0; }
        (self.recent_retries.len() as f64 * 1000.0) / self.window_ms as f64
    }
}

/// Retry policy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RetryPolicyStats {
    pub total_policies: usize,
    pub active_retries: usize,
    pub total_successes: u64,
    pub total_failures: u64,
    pub total_timeouts: u64,
    pub circuit_breakers: usize,
    pub open_circuits: usize,
    pub storm_detected: bool,
}

/// Cooperative retry policy manager
pub struct CoopRetryPolicy {
    policies: BTreeMap<String, RetryPolicy>,
    active: BTreeMap<u64, RetryState>,
    breakers: BTreeMap<String, CircuitBreaker>,
    storm: StormDetector,
    next_op_id: u64,
    total_successes: u64,
    total_failures: u64,
    total_timeouts: u64,
    stats: RetryPolicyStats,
}

impl CoopRetryPolicy {
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(), active: BTreeMap::new(),
            breakers: BTreeMap::new(), storm: StormDetector::new(10_000, 100),
            next_op_id: 1, total_successes: 0, total_failures: 0,
            total_timeouts: 0, stats: RetryPolicyStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_policy(&mut self, policy: RetryPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    #[inline(always)]
    pub fn register_breaker(&mut self, name: String, breaker: CircuitBreaker) {
        self.breakers.insert(name, breaker);
    }

    #[inline]
    pub fn begin_retry(&mut self, policy_name: &str, now_ms: u64) -> Option<u64> {
        if !self.policies.contains_key(policy_name) { return None; }
        let id = self.next_op_id; self.next_op_id += 1;
        self.active.insert(id, RetryState {
            operation_id: id, policy_name: String::from(policy_name),
            attempt: 0, started_ms: now_ms, last_attempt_ms: now_ms,
            outcome: None, total_delay_ms: 0,
        });
        Some(id)
    }

    #[inline]
    pub fn next_delay(&self, op_id: u64, seed: u64) -> Option<u64> {
        let state = self.active.get(&op_id)?;
        let policy = self.policies.get(&state.policy_name)?;
        if state.attempt >= policy.max_retries { return None; }
        let delay = policy.compute_delay(state.attempt, seed);
        let elapsed = state.last_attempt_ms.saturating_sub(state.started_ms);
        if !policy.within_deadline(elapsed, delay) { return None; }
        Some(delay)
    }

    pub fn record_attempt(&mut self, op_id: u64, outcome: RetryOutcome, now_ms: u64) {
        if let Some(state) = self.active.get_mut(&op_id) {
            state.attempt += 1;
            state.last_attempt_ms = now_ms;
            match outcome {
                RetryOutcome::Success => { state.outcome = Some(outcome); self.total_successes += 1; }
                RetryOutcome::Failure => { self.total_failures += 1; self.storm.record_retry(now_ms); }
                RetryOutcome::Timeout => { self.total_timeouts += 1; self.storm.record_retry(now_ms); }
                _ => {}
            }
            // Update circuit breaker
            let policy_name = state.policy_name.clone();
            if let Some(cb) = self.breakers.get_mut(&policy_name) {
                match outcome {
                    RetryOutcome::Success => cb.record_success(),
                    RetryOutcome::Failure | RetryOutcome::Timeout => cb.record_failure(now_ms),
                    _ => {}
                }
            }
        }
    }

    #[inline(always)]
    pub fn complete(&mut self, op_id: u64) -> Option<RetryState> { self.active.remove(&op_id) }

    #[inline]
    pub fn recompute(&mut self) {
        for cb in self.breakers.values_mut() { cb.check_transition(0); }
        self.stats.total_policies = self.policies.len();
        self.stats.active_retries = self.active.len();
        self.stats.total_successes = self.total_successes;
        self.stats.total_failures = self.total_failures;
        self.stats.total_timeouts = self.total_timeouts;
        self.stats.circuit_breakers = self.breakers.len();
        self.stats.open_circuits = self.breakers.values().filter(|cb| cb.state == CircuitState::Open).count();
        self.stats.storm_detected = self.storm.storm_detected;
    }

    #[inline(always)]
    pub fn stats(&self) -> &RetryPolicyStats { &self.stats }
}

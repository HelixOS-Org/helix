//! # Action Retry
//!
//! Implements retry strategies for failed actions.
//! Supports exponential backoff and circuit breakers.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// RETRY TYPES
// ============================================================================

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Policy ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Strategy
    pub strategy: RetryStrategy,
    /// Max attempts
    pub max_attempts: u32,
    /// Initial delay (ms)
    pub initial_delay_ms: u64,
    /// Max delay (ms)
    pub max_delay_ms: u64,
    /// Jitter
    pub jitter: f64,
}

/// Retry strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    Fixed,
    Linear,
    Exponential,
    Fibonacci,
    Random,
}

/// Retry attempt
#[derive(Debug, Clone)]
pub struct RetryAttempt {
    /// Attempt ID
    pub id: u64,
    /// Action ID
    pub action_id: u64,
    /// Attempt number
    pub attempt_number: u32,
    /// Status
    pub status: AttemptStatus,
    /// Error
    pub error: Option<String>,
    /// Started
    pub started: Timestamp,
    /// Ended
    pub ended: Option<Timestamp>,
    /// Next retry after
    pub next_retry_after: Option<Timestamp>,
}

/// Attempt status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    Exhausted,
}

/// Retryable action
#[derive(Debug, Clone)]
pub struct RetryableAction {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Policy ID
    pub policy_id: u64,
    /// Attempts
    pub attempts: Vec<RetryAttempt>,
    /// Current status
    pub status: ActionStatus,
    /// Created
    pub created: Timestamp,
}

/// Action status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Abandoned,
}

/// Circuit breaker state
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Breaker ID
    pub id: u64,
    /// Name
    pub name: String,
    /// State
    pub state: CircuitState,
    /// Failure count
    pub failure_count: u32,
    /// Success count
    pub success_count: u32,
    /// Failure threshold
    pub failure_threshold: u32,
    /// Success threshold (for half-open)
    pub success_threshold: u32,
    /// Timeout (ms)
    pub timeout_ms: u64,
    /// Last state change
    pub last_state_change: Timestamp,
}

/// Circuit state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

// ============================================================================
// RETRY ENGINE
// ============================================================================

/// Retry engine
pub struct RetryEngine {
    /// Policies
    policies: BTreeMap<u64, RetryPolicy>,
    /// Actions
    actions: BTreeMap<u64, RetryableAction>,
    /// Circuit breakers
    breakers: BTreeMap<u64, CircuitBreaker>,
    /// Fibonacci cache
    fib_cache: Vec<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: RetryConfig,
    /// Statistics
    stats: RetryStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Default max attempts
    pub default_max_attempts: u32,
    /// Default initial delay
    pub default_initial_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            default_max_attempts: 3,
            default_initial_delay_ms: 1000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    /// Total attempts
    pub total_attempts: u64,
    /// Successful retries
    pub successful_retries: u64,
    /// Failed retries
    pub failed_retries: u64,
    /// Circuit breaks
    pub circuit_breaks: u64,
}

impl RetryEngine {
    /// Create new engine
    pub fn new(config: RetryConfig) -> Self {
        // Pre-compute Fibonacci numbers
        let mut fib_cache = vec![1, 1];
        for i in 2..20 {
            let next = fib_cache[i - 1] + fib_cache[i - 2];
            fib_cache.push(next);
        }

        Self {
            policies: BTreeMap::new(),
            actions: BTreeMap::new(),
            breakers: BTreeMap::new(),
            fib_cache,
            next_id: AtomicU64::new(1),
            config,
            stats: RetryStats::default(),
        }
    }

    /// Create policy
    pub fn create_policy(
        &mut self,
        name: &str,
        strategy: RetryStrategy,
        max_attempts: u32,
        initial_delay_ms: u64,
        max_delay_ms: u64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let policy = RetryPolicy {
            id,
            name: name.into(),
            strategy,
            max_attempts,
            initial_delay_ms,
            max_delay_ms,
            jitter: 0.1,
        };

        self.policies.insert(id, policy);

        id
    }

    /// Start action with retry
    pub fn start_action(&mut self, name: &str, policy_id: u64) -> Option<u64> {
        if !self.policies.contains_key(&policy_id) {
            return None;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let attempt_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let attempt = RetryAttempt {
            id: attempt_id,
            action_id: id,
            attempt_number: 1,
            status: AttemptStatus::InProgress,
            error: None,
            started: now,
            ended: None,
            next_retry_after: None,
        };

        let action = RetryableAction {
            id,
            name: name.into(),
            policy_id,
            attempts: vec![attempt],
            status: ActionStatus::InProgress,
            created: now,
        };

        self.actions.insert(id, action);
        self.stats.total_attempts += 1;

        Some(id)
    }

    /// Report success
    pub fn report_success(&mut self, action_id: u64) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            if let Some(attempt) = action.attempts.last_mut() {
                attempt.status = AttemptStatus::Success;
                attempt.ended = Some(Timestamp::now());
            }

            action.status = ActionStatus::Succeeded;
            self.stats.successful_retries += 1;
        }
    }

    /// Report failure and get next retry
    pub fn report_failure(&mut self, action_id: u64, error: &str) -> Option<Timestamp> {
        let action = self.actions.get_mut(&action_id)?;
        let policy = self.policies.get(&action.policy_id)?.clone();

        // Update current attempt
        let attempt_num = action.attempts.len() as u32;
        if let Some(attempt) = action.attempts.last_mut() {
            attempt.status = AttemptStatus::Failed;
            attempt.error = Some(error.into());
            attempt.ended = Some(Timestamp::now());
        }

        // Check if exhausted
        if attempt_num >= policy.max_attempts {
            action.status = ActionStatus::Failed;
            self.stats.failed_retries += 1;
            return None;
        }

        // Calculate delay
        let delay = self.calculate_delay(&policy, attempt_num);
        let next_retry = Timestamp(Timestamp::now().0 + delay);

        // Create next attempt
        let new_attempt_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let new_attempt = RetryAttempt {
            id: new_attempt_id,
            action_id,
            attempt_number: attempt_num + 1,
            status: AttemptStatus::Pending,
            error: None,
            started: next_retry,
            ended: None,
            next_retry_after: Some(next_retry),
        };

        action.attempts.push(new_attempt);
        self.stats.total_attempts += 1;

        Some(next_retry)
    }

    fn calculate_delay(&self, policy: &RetryPolicy, attempt: u32) -> u64 {
        let base_delay = match policy.strategy {
            RetryStrategy::Fixed => policy.initial_delay_ms,
            RetryStrategy::Linear => policy.initial_delay_ms * (attempt as u64),
            RetryStrategy::Exponential => {
                policy.initial_delay_ms * 2u64.pow(attempt.saturating_sub(1))
            },
            RetryStrategy::Fibonacci => {
                let idx = (attempt as usize).min(self.fib_cache.len() - 1);
                policy.initial_delay_ms * self.fib_cache[idx]
            },
            RetryStrategy::Random => {
                // Pseudo-random for no_std
                let factor = ((attempt as u64 * 7919) % 100) as f64 / 100.0;
                (policy.initial_delay_ms as f64 * (1.0 + factor)) as u64
            },
        };

        // Apply jitter
        let jitter_amount =
            (base_delay as f64 * policy.jitter * ((attempt as f64 * 0.7).sin() + 1.0) / 2.0) as u64;

        // Cap at max delay
        (base_delay + jitter_amount).min(policy.max_delay_ms)
    }

    /// Create circuit breaker
    pub fn create_breaker(
        &mut self,
        name: &str,
        failure_threshold: u32,
        success_threshold: u32,
        timeout_ms: u64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let breaker = CircuitBreaker {
            id,
            name: name.into(),
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            failure_threshold,
            success_threshold,
            timeout_ms,
            last_state_change: Timestamp::now(),
        };

        self.breakers.insert(id, breaker);

        id
    }

    /// Check if circuit allows request
    pub fn can_execute(&mut self, breaker_id: u64) -> bool {
        let breaker = match self.breakers.get_mut(&breaker_id) {
            Some(b) => b,
            None => return true,
        };

        match breaker.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check timeout
                let elapsed = Timestamp::now().0 - breaker.last_state_change.0;
                if elapsed >= breaker.timeout_ms {
                    breaker.state = CircuitState::HalfOpen;
                    breaker.last_state_change = Timestamp::now();
                    true
                } else {
                    false
                }
            },
            CircuitState::HalfOpen => true,
        }
    }

    /// Report success to circuit breaker
    pub fn breaker_success(&mut self, breaker_id: u64) {
        if let Some(breaker) = self.breakers.get_mut(&breaker_id) {
            match breaker.state {
                CircuitState::Closed => {
                    breaker.failure_count = 0;
                },
                CircuitState::HalfOpen => {
                    breaker.success_count += 1;
                    if breaker.success_count >= breaker.success_threshold {
                        breaker.state = CircuitState::Closed;
                        breaker.failure_count = 0;
                        breaker.success_count = 0;
                        breaker.last_state_change = Timestamp::now();
                    }
                },
                CircuitState::Open => {},
            }
        }
    }

    /// Report failure to circuit breaker
    pub fn breaker_failure(&mut self, breaker_id: u64) {
        if let Some(breaker) = self.breakers.get_mut(&breaker_id) {
            match breaker.state {
                CircuitState::Closed => {
                    breaker.failure_count += 1;
                    if breaker.failure_count >= breaker.failure_threshold {
                        breaker.state = CircuitState::Open;
                        breaker.last_state_change = Timestamp::now();
                        self.stats.circuit_breaks += 1;
                    }
                },
                CircuitState::HalfOpen => {
                    breaker.state = CircuitState::Open;
                    breaker.success_count = 0;
                    breaker.last_state_change = Timestamp::now();
                    self.stats.circuit_breaks += 1;
                },
                CircuitState::Open => {},
            }
        }
    }

    /// Get action
    pub fn get_action(&self, id: u64) -> Option<&RetryableAction> {
        self.actions.get(&id)
    }

    /// Get breaker
    pub fn get_breaker(&self, id: u64) -> Option<&CircuitBreaker> {
        self.breakers.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &RetryStats {
        &self.stats
    }
}

impl Default for RetryEngine {
    fn default() -> Self {
        Self::new(RetryConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_policy() {
        let mut engine = RetryEngine::default();

        let id = engine.create_policy("default", RetryStrategy::Exponential, 3, 1000, 30000);

        assert!(engine.policies.get(&id).is_some());
    }

    #[test]
    fn test_start_action() {
        let mut engine = RetryEngine::default();

        let policy = engine.create_policy("test", RetryStrategy::Fixed, 3, 1000, 5000);

        let action = engine.start_action("my_action", policy);
        assert!(action.is_some());

        let action = engine.get_action(action.unwrap()).unwrap();
        assert_eq!(action.attempts.len(), 1);
    }

    #[test]
    fn test_report_success() {
        let mut engine = RetryEngine::default();

        let policy = engine.create_policy("test", RetryStrategy::Fixed, 3, 1000, 5000);
        let action_id = engine.start_action("test", policy).unwrap();

        engine.report_success(action_id);

        let action = engine.get_action(action_id).unwrap();
        assert_eq!(action.status, ActionStatus::Succeeded);
    }

    #[test]
    fn test_retry_on_failure() {
        let mut engine = RetryEngine::default();

        let policy = engine.create_policy("test", RetryStrategy::Fixed, 3, 100, 5000);
        let action_id = engine.start_action("test", policy).unwrap();

        let next = engine.report_failure(action_id, "error 1");
        assert!(next.is_some());

        let action = engine.get_action(action_id).unwrap();
        assert_eq!(action.attempts.len(), 2);
    }

    #[test]
    fn test_exhausted() {
        let mut engine = RetryEngine::default();

        let policy = engine.create_policy("test", RetryStrategy::Fixed, 2, 100, 5000);
        let action_id = engine.start_action("test", policy).unwrap();

        engine.report_failure(action_id, "error 1");
        let next = engine.report_failure(action_id, "error 2");

        assert!(next.is_none()); // Exhausted

        let action = engine.get_action(action_id).unwrap();
        assert_eq!(action.status, ActionStatus::Failed);
    }

    #[test]
    fn test_circuit_breaker_closed() {
        let mut engine = RetryEngine::default();

        let breaker = engine.create_breaker("test", 3, 2, 5000);

        assert!(engine.can_execute(breaker));
    }

    #[test]
    fn test_circuit_breaker_opens() {
        let mut engine = RetryEngine::default();

        let breaker = engine.create_breaker("test", 3, 2, 5000);

        // Trip the breaker
        engine.breaker_failure(breaker);
        engine.breaker_failure(breaker);
        engine.breaker_failure(breaker);

        let b = engine.get_breaker(breaker).unwrap();
        assert_eq!(b.state, CircuitState::Open);
    }

    #[test]
    fn test_exponential_backoff() {
        let mut engine = RetryEngine::default();

        let policy = engine.create_policy("exp", RetryStrategy::Exponential, 5, 100, 10000);
        let p = engine.policies.get(&policy).unwrap().clone();

        let d1 = engine.calculate_delay(&p, 1);
        let d2 = engine.calculate_delay(&p, 2);
        let d3 = engine.calculate_delay(&p, 3);

        assert!(d2 > d1);
        assert!(d3 > d2);
    }
}

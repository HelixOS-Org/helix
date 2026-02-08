//! # Bridge Retry Engine
//!
//! Intelligent syscall retry with backoff:
//! - Exponential backoff
//! - Jitter injection
//! - Retry budgets
//! - Per-syscall retry policies
//! - Retry analytics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// RETRY TYPES
// ============================================================================

/// Retry strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// Fixed delay
    Fixed,
    /// Linear backoff
    Linear,
    /// Exponential backoff
    Exponential,
    /// Exponential with jitter
    ExponentialJitter,
    /// No retry
    None,
}

/// Retry outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryOutcome {
    /// Succeeded on retry
    Success,
    /// Still failing after retries
    Exhausted,
    /// Retry budget depleted
    BudgetDepleted,
    /// Non-retryable error
    NonRetryable,
}

/// Error category for retry decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryableCategory {
    /// Transient (retry-worthy)
    Transient,
    /// Resource temporarily unavailable
    ResourceBusy,
    /// Timeout (retry-worthy)
    Timeout,
    /// Permanent (do not retry)
    Permanent,
    /// Unknown (retry cautiously)
    Unknown,
}

impl RetryableCategory {
    /// Should retry?
    pub fn should_retry(&self) -> bool {
        matches!(
            self,
            Self::Transient | Self::ResourceBusy | Self::Timeout | Self::Unknown
        )
    }
}

// ============================================================================
// RETRY POLICY
// ============================================================================

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Strategy
    pub strategy: RetryStrategy,
    /// Max retries
    pub max_retries: u32,
    /// Base delay (ns)
    pub base_delay_ns: u64,
    /// Max delay (ns)
    pub max_delay_ns: u64,
    /// Backoff multiplier
    pub multiplier: f64,
    /// Budget per window
    pub budget_per_window: u32,
    /// Window size (ns)
    pub window_ns: u64,
}

impl RetryPolicy {
    pub fn default_policy() -> Self {
        Self {
            strategy: RetryStrategy::ExponentialJitter,
            max_retries: 3,
            base_delay_ns: 1_000_000,     // 1ms
            max_delay_ns: 100_000_000,    // 100ms
            multiplier: 2.0,
            budget_per_window: 100,
            window_ns: 10_000_000_000,    // 10s
        }
    }

    pub fn aggressive() -> Self {
        Self {
            strategy: RetryStrategy::Exponential,
            max_retries: 5,
            base_delay_ns: 500_000,
            max_delay_ns: 50_000_000,
            multiplier: 1.5,
            budget_per_window: 200,
            window_ns: 10_000_000_000,
        }
    }

    pub fn conservative() -> Self {
        Self {
            strategy: RetryStrategy::Linear,
            max_retries: 2,
            base_delay_ns: 10_000_000,
            max_delay_ns: 500_000_000,
            multiplier: 1.0,
            budget_per_window: 50,
            window_ns: 30_000_000_000,
        }
    }

    /// Compute delay for attempt N
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let raw = match self.strategy {
            RetryStrategy::Fixed => self.base_delay_ns,
            RetryStrategy::Linear => {
                self.base_delay_ns + (self.base_delay_ns * attempt as u64)
            }
            RetryStrategy::Exponential | RetryStrategy::ExponentialJitter => {
                let factor = libm::pow(self.multiplier, attempt as f64);
                let delay = self.base_delay_ns as f64 * factor;
                delay as u64
            }
            RetryStrategy::None => 0,
        };

        if raw > self.max_delay_ns {
            self.max_delay_ns
        } else {
            raw
        }
    }
}

// ============================================================================
// RETRY STATE
// ============================================================================

/// Retry state for an active operation
#[derive(Debug, Clone)]
pub struct RetryState {
    /// Operation id
    pub id: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Current attempt
    pub attempt: u32,
    /// Policy
    pub policy: RetryPolicy,
    /// Last error category
    pub last_error: RetryableCategory,
    /// Total delay accumulated (ns)
    pub total_delay_ns: u64,
    /// Started at
    pub started_at: u64,
    /// Completed?
    pub completed: bool,
    /// Outcome
    pub outcome: Option<RetryOutcome>,
}

impl RetryState {
    pub fn new(id: u64, syscall_nr: u32, policy: RetryPolicy, now: u64) -> Self {
        Self {
            id,
            syscall_nr,
            attempt: 0,
            policy,
            last_error: RetryableCategory::Unknown,
            total_delay_ns: 0,
            started_at: now,
            completed: false,
            outcome: None,
        }
    }

    /// Should retry?
    pub fn should_retry(&self) -> bool {
        !self.completed
            && self.attempt < self.policy.max_retries
            && self.last_error.should_retry()
    }

    /// Next delay
    pub fn next_delay(&self) -> u64 {
        self.policy.delay_for_attempt(self.attempt)
    }

    /// Record attempt
    pub fn record_attempt(&mut self, error: RetryableCategory) {
        self.attempt += 1;
        self.last_error = error;
        self.total_delay_ns += self.policy.delay_for_attempt(self.attempt.saturating_sub(1));
    }

    /// Mark success
    pub fn mark_success(&mut self) {
        self.completed = true;
        self.outcome = Some(RetryOutcome::Success);
    }

    /// Mark exhausted
    pub fn mark_exhausted(&mut self) {
        self.completed = true;
        self.outcome = Some(RetryOutcome::Exhausted);
    }
}

// ============================================================================
// RETRY BUDGET
// ============================================================================

/// Budget tracker
#[derive(Debug, Clone)]
pub struct RetryBudget {
    /// Budget per window
    pub limit: u32,
    /// Current usage
    pub used: u32,
    /// Window start
    pub window_start: u64,
    /// Window size
    pub window_ns: u64,
}

impl RetryBudget {
    pub fn new(limit: u32, window_ns: u64) -> Self {
        Self {
            limit,
            used: 0,
            window_start: 0,
            window_ns,
        }
    }

    /// Check and consume
    pub fn try_consume(&mut self, now: u64) -> bool {
        if now.saturating_sub(self.window_start) >= self.window_ns {
            self.used = 0;
            self.window_start = now;
        }
        if self.used < self.limit {
            self.used += 1;
            true
        } else {
            false
        }
    }

    /// Remaining
    pub fn remaining(&self) -> u32 {
        self.limit.saturating_sub(self.used)
    }

    /// Utilization
    pub fn utilization(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }
        self.used as f64 / self.limit as f64
    }
}

// ============================================================================
// RETRY MANAGER
// ============================================================================

/// Retry stats
#[derive(Debug, Clone, Default)]
pub struct BridgeRetryStats {
    /// Active retries
    pub active: usize,
    /// Total retries initiated
    pub total_initiated: u64,
    /// Successes on retry
    pub successes: u64,
    /// Exhausted
    pub exhausted: u64,
    /// Budget depleted
    pub budget_depleted: u64,
    /// Total delay time (ns)
    pub total_delay_ns: u64,
}

/// Bridge retry engine
pub struct BridgeRetryEngine {
    /// Active retries
    active: BTreeMap<u64, RetryState>,
    /// Per-syscall policies
    policies: BTreeMap<u32, RetryPolicy>,
    /// Per-syscall budgets
    budgets: BTreeMap<u32, RetryBudget>,
    /// Default policy
    default_policy: RetryPolicy,
    /// Next id
    next_id: u64,
    /// Stats
    stats: BridgeRetryStats,
}

impl BridgeRetryEngine {
    pub fn new() -> Self {
        Self {
            active: BTreeMap::new(),
            policies: BTreeMap::new(),
            budgets: BTreeMap::new(),
            default_policy: RetryPolicy::default_policy(),
            next_id: 1,
            stats: BridgeRetryStats::default(),
        }
    }

    /// Set policy for syscall
    pub fn set_policy(&mut self, syscall_nr: u32, policy: RetryPolicy) {
        let budget = RetryBudget::new(policy.budget_per_window, policy.window_ns);
        self.budgets.insert(syscall_nr, budget);
        self.policies.insert(syscall_nr, policy);
    }

    /// Initiate retry
    pub fn initiate(
        &mut self,
        syscall_nr: u32,
        error: RetryableCategory,
        now: u64,
    ) -> Option<u64> {
        if !error.should_retry() {
            return None;
        }

        // Check budget
        let budget = self.budgets.entry(syscall_nr).or_insert_with(|| {
            RetryBudget::new(
                self.default_policy.budget_per_window,
                self.default_policy.window_ns,
            )
        });

        if !budget.try_consume(now) {
            self.stats.budget_depleted += 1;
            return None;
        }

        let policy = self
            .policies
            .get(&syscall_nr)
            .cloned()
            .unwrap_or_else(|| self.default_policy.clone());

        let id = self.next_id;
        self.next_id += 1;

        let mut state = RetryState::new(id, syscall_nr, policy, now);
        state.record_attempt(error);

        self.active.insert(id, state);
        self.stats.total_initiated += 1;
        self.stats.active = self.active.len();

        Some(id)
    }

    /// Record retry result
    pub fn record_result(
        &mut self,
        id: u64,
        success: bool,
        error: Option<RetryableCategory>,
    ) {
        if let Some(state) = self.active.get_mut(&id) {
            if success {
                state.mark_success();
                self.stats.successes += 1;
                self.stats.total_delay_ns += state.total_delay_ns;
            } else if let Some(err) = error {
                state.record_attempt(err);
                if !state.should_retry() {
                    state.mark_exhausted();
                    self.stats.exhausted += 1;
                    self.stats.total_delay_ns += state.total_delay_ns;
                }
            }
        }

        // Clean completed
        self.active.retain(|_, s| !s.completed);
        self.stats.active = self.active.len();
    }

    /// Get delay for pending retry
    pub fn pending_delay(&self, id: u64) -> Option<u64> {
        self.active.get(&id).map(|s| s.next_delay())
    }

    /// Stats
    pub fn stats(&self) -> &BridgeRetryStats {
        &self.stats
    }
}

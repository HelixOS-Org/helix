//! # Fault Tolerance
//!
//! Year 3 EVOLUTION - Fault tolerance for distributed systems

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// FAULT TYPES
// ============================================================================

/// Node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// Fault ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaultId(pub u64);

static FAULT_COUNTER: AtomicU64 = AtomicU64::new(1);

impl FaultId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(FAULT_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    NodeCrash,
    NetworkPartition,
    MessageLoss,
    MessageDelay,
    MessageCorruption,
    ByzantineFault,
    TimeoutFault,
    ResourceExhaustion,
    ConfigurationError,
}

/// Fault event
#[derive(Debug, Clone)]
pub struct FaultEvent {
    /// ID
    pub id: FaultId,
    /// Type
    pub fault_type: FaultType,
    /// Affected nodes
    pub affected_nodes: Vec<NodeId>,
    /// Timestamp
    pub timestamp: u64,
    /// Duration (ticks)
    pub duration: Option<u64>,
    /// Resolved
    pub resolved: bool,
    /// Description
    pub description: String,
}

// ============================================================================
// CIRCUIT BREAKER
// ============================================================================

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker
pub struct CircuitBreaker {
    /// State
    state: CircuitState,
    /// Failure count
    failure_count: AtomicU64,
    /// Success count (in half-open)
    success_count: AtomicU64,
    /// Failure threshold
    failure_threshold: u64,
    /// Success threshold (to close from half-open)
    success_threshold: u64,
    /// Reset timeout (ticks)
    reset_timeout: u64,
    /// Last failure time
    last_failure_time: AtomicU64,
    /// Current tick
    current_tick: AtomicU64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u64, success_threshold: u64, reset_timeout: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            failure_threshold,
            success_threshold,
            reset_timeout,
            last_failure_time: AtomicU64::new(0),
            current_tick: AtomicU64::new(0),
        }
    }

    /// Check if call is allowed
    pub fn allow(&mut self) -> bool {
        let tick = self.current_tick.load(Ordering::Relaxed);

        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                if tick - last_failure >= self.reset_timeout {
                    self.state = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                    true
                } else {
                    false
                }
            },
            CircuitState::HalfOpen => true,
        }
    }

    /// Record success
    pub fn record_success(&mut self) {
        match self.state {
            CircuitState::Closed => {
                self.failure_count.store(0, Ordering::Relaxed);
            },
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                }
            },
            CircuitState::Open => {},
        }
    }

    /// Record failure
    pub fn record_failure(&mut self) {
        let tick = self.current_tick.load(Ordering::Relaxed);

        match self.state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    self.last_failure_time.store(tick, Ordering::Relaxed);
                }
            },
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.last_failure_time.store(tick, Ordering::Relaxed);
            },
            CircuitState::Open => {},
        }
    }

    /// Tick
    #[inline(always)]
    pub fn tick(&mut self) {
        self.current_tick.fetch_add(1, Ordering::Relaxed);
    }

    /// Get state
    #[inline(always)]
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Reset
    #[inline]
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// RETRY POLICY
// ============================================================================

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Max retries
    pub max_retries: u32,
    /// Base delay (ticks)
    pub base_delay: u64,
    /// Max delay (ticks)
    pub max_delay: u64,
    /// Backoff strategy
    pub backoff: BackoffStrategy,
    /// Jitter
    pub jitter: f64,
}

/// Backoff strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackoffStrategy {
    Constant,
    Linear,
    Exponential,
    Fibonacci,
}

impl RetryPolicy {
    #[inline]
    pub fn constant(max_retries: u32, delay: u64) -> Self {
        Self {
            max_retries,
            base_delay: delay,
            max_delay: delay,
            backoff: BackoffStrategy::Constant,
            jitter: 0.0,
        }
    }

    #[inline]
    pub fn exponential(max_retries: u32, base_delay: u64, max_delay: u64) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            backoff: BackoffStrategy::Exponential,
            jitter: 0.1,
        }
    }

    /// Calculate delay for attempt
    pub fn delay_for_attempt(&self, attempt: u32, random: f64) -> u64 {
        let base = match self.backoff {
            BackoffStrategy::Constant => self.base_delay,
            BackoffStrategy::Linear => self.base_delay * (attempt as u64 + 1),
            BackoffStrategy::Exponential => self.base_delay * (1u64 << attempt.min(10)),
            BackoffStrategy::Fibonacci => {
                let mut a = self.base_delay;
                let mut b = self.base_delay;
                for _ in 0..attempt {
                    let temp = a + b;
                    a = b;
                    b = temp;
                }
                a
            },
        };

        let clamped = base.min(self.max_delay);

        // Apply jitter
        let jitter_range = (clamped as f64 * self.jitter) as u64;
        let jitter_offset = ((random - 0.5) * 2.0 * jitter_range as f64) as i64;
        (clamped as i64 + jitter_offset).max(0) as u64
    }

    /// Should retry?
    #[inline(always)]
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

/// Retry executor
pub struct RetryExecutor {
    /// Policy
    policy: RetryPolicy,
    /// Random state
    random_state: AtomicU64,
}

impl RetryExecutor {
    pub fn new(policy: RetryPolicy) -> Self {
        Self {
            policy,
            random_state: AtomicU64::new(0xDEADBEEF),
        }
    }

    fn random(&self) -> f64 {
        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);
        (x as f64) / (u64::MAX as f64)
    }

    /// Execute with retry
    pub fn execute<F, T, E>(&self, mut f: F) -> Result<T, RetryError<E>>
    where
        F: FnMut() -> Result<T, E>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        loop {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);

                    if !self.policy.should_retry(attempt) {
                        break;
                    }

                    let delay = self.policy.delay_for_attempt(attempt, self.random());
                    // In real implementation, would sleep/wait for delay
                    let _ = delay;

                    attempt += 1;
                },
            }
        }

        Err(RetryError {
            attempts: attempt + 1,
            last_error: last_error.unwrap(),
        })
    }
}

/// Retry error
#[derive(Debug)]
pub struct RetryError<E> {
    /// Attempts made
    pub attempts: u32,
    /// Last error
    pub last_error: E,
}

// ============================================================================
// FAILOVER
// ============================================================================

/// Failover manager
pub struct FailoverManager {
    /// Primary node
    primary: Option<NodeId>,
    /// Backup nodes
    backups: Vec<NodeId>,
    /// Node status
    node_status: BTreeMap<NodeId, NodeStatus>,
    /// Failover in progress
    failover_in_progress: AtomicBool,
    /// Current failover target
    failover_target: Option<NodeId>,
    /// Health check interval
    health_check_interval: u64,
    /// Current tick
    tick: AtomicU64,
}

/// Node status
#[derive(Debug, Clone)]
pub struct NodeStatus {
    /// Node ID
    pub node_id: NodeId,
    /// Is healthy
    pub healthy: bool,
    /// Last health check
    pub last_health_check: u64,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Failover count
    pub failover_count: u32,
}

impl FailoverManager {
    pub fn new(primary: NodeId, backups: Vec<NodeId>) -> Self {
        let mut node_status = BTreeMap::new();

        node_status.insert(primary, NodeStatus {
            node_id: primary,
            healthy: true,
            last_health_check: 0,
            consecutive_failures: 0,
            failover_count: 0,
        });

        for &backup in &backups {
            node_status.insert(backup, NodeStatus {
                node_id: backup,
                healthy: true,
                last_health_check: 0,
                consecutive_failures: 0,
                failover_count: 0,
            });
        }

        Self {
            primary: Some(primary),
            backups,
            node_status,
            failover_in_progress: AtomicBool::new(false),
            failover_target: None,
            health_check_interval: 100,
            tick: AtomicU64::new(0),
        }
    }

    /// Report health check result
    pub fn health_check_result(&mut self, node: NodeId, healthy: bool) -> Option<FailoverAction> {
        let tick = self.tick.load(Ordering::Relaxed);

        if let Some(status) = self.node_status.get_mut(&node) {
            status.last_health_check = tick;

            if healthy {
                status.healthy = true;
                status.consecutive_failures = 0;
            } else {
                status.consecutive_failures += 1;

                // Check if failover needed
                if status.consecutive_failures >= 3 {
                    status.healthy = false;

                    if self.primary == Some(node) {
                        return self.initiate_failover();
                    }
                }
            }
        }

        None
    }

    /// Initiate failover
    fn initiate_failover(&mut self) -> Option<FailoverAction> {
        if self.failover_in_progress.load(Ordering::Relaxed) {
            return None;
        }

        // Find healthy backup
        let new_primary = self
            .backups
            .iter()
            .find(|&&id| {
                self.node_status
                    .get(&id)
                    .map(|s| s.healthy)
                    .unwrap_or(false)
            })
            .copied();

        if let Some(new) = new_primary {
            self.failover_in_progress.store(true, Ordering::Relaxed);
            self.failover_target = Some(new);

            let old = self.primary;

            return Some(FailoverAction::Initiate {
                old_primary: old,
                new_primary: new,
            });
        }

        None
    }

    /// Complete failover
    pub fn complete_failover(&mut self, success: bool) -> Option<FailoverAction> {
        if !self.failover_in_progress.load(Ordering::Relaxed) {
            return None;
        }

        if success {
            if let Some(new) = self.failover_target {
                let old = self.primary;
                self.primary = Some(new);

                // Remove from backups
                self.backups.retain(|&id| id != new);

                // Add old primary to backups if it's recovering
                if let Some(old_id) = old {
                    if !self.backups.contains(&old_id) {
                        self.backups.push(old_id);
                    }
                }

                // Update failover count
                if let Some(status) = self.node_status.get_mut(&new) {
                    status.failover_count += 1;
                }
            }
        }

        self.failover_in_progress.store(false, Ordering::Relaxed);
        self.failover_target = None;

        Some(FailoverAction::Complete { success })
    }

    /// Tick
    #[inline(always)]
    pub fn tick(&mut self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }

    /// Get primary
    #[inline(always)]
    pub fn primary(&self) -> Option<NodeId> {
        self.primary
    }

    /// Get backups
    #[inline(always)]
    pub fn backups(&self) -> &[NodeId] {
        &self.backups
    }
}

/// Failover action
#[derive(Debug, Clone)]
pub enum FailoverAction {
    Initiate {
        old_primary: Option<NodeId>,
        new_primary: NodeId,
    },
    Complete {
        success: bool,
    },
}

// ============================================================================
// CHECKPOINTING
// ============================================================================

/// Checkpoint ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckpointId(pub u64);

static CHECKPOINT_COUNTER: AtomicU64 = AtomicU64::new(1);

impl CheckpointId {
    #[inline(always)]
    pub fn generate() -> Self {
        Self(CHECKPOINT_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Checkpoint
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// ID
    pub id: CheckpointId,
    /// Timestamp
    pub timestamp: u64,
    /// State snapshot
    pub state: Vec<u8>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
    /// Parent checkpoint
    pub parent: Option<CheckpointId>,
    /// Is valid
    pub valid: bool,
}

/// Checkpoint manager
pub struct CheckpointManager {
    /// Checkpoints
    checkpoints: BTreeMap<CheckpointId, Checkpoint>,
    /// Latest checkpoint
    latest: Option<CheckpointId>,
    /// Checkpoint interval (ticks)
    interval: u64,
    /// Max checkpoints to keep
    max_checkpoints: usize,
    /// Current tick
    tick: AtomicU64,
    /// Last checkpoint tick
    last_checkpoint_tick: AtomicU64,
}

impl CheckpointManager {
    pub fn new(interval: u64, max_checkpoints: usize) -> Self {
        Self {
            checkpoints: BTreeMap::new(),
            latest: None,
            interval,
            max_checkpoints,
            tick: AtomicU64::new(0),
            last_checkpoint_tick: AtomicU64::new(0),
        }
    }

    /// Should checkpoint?
    #[inline]
    pub fn should_checkpoint(&self) -> bool {
        let tick = self.tick.load(Ordering::Relaxed);
        let last = self.last_checkpoint_tick.load(Ordering::Relaxed);
        tick - last >= self.interval
    }

    /// Create checkpoint
    pub fn create(&mut self, state: Vec<u8>, metadata: BTreeMap<String, String>) -> CheckpointId {
        let tick = self.tick.load(Ordering::Relaxed);
        let id = CheckpointId::generate();

        let checkpoint = Checkpoint {
            id,
            timestamp: tick,
            state,
            metadata,
            parent: self.latest,
            valid: true,
        };

        self.checkpoints.insert(id, checkpoint);
        self.latest = Some(id);
        self.last_checkpoint_tick.store(tick, Ordering::Relaxed);

        // Garbage collect old checkpoints
        while self.checkpoints.len() > self.max_checkpoints {
            let oldest = *self.checkpoints.keys().next().unwrap();
            if Some(oldest) != self.latest {
                self.checkpoints.remove(&oldest);
            } else {
                break;
            }
        }

        id
    }

    /// Restore from checkpoint
    #[inline(always)]
    pub fn restore(&self, id: CheckpointId) -> Option<&Checkpoint> {
        self.checkpoints.get(&id).filter(|c| c.valid)
    }

    /// Restore latest
    #[inline(always)]
    pub fn restore_latest(&self) -> Option<&Checkpoint> {
        self.latest.and_then(|id| self.restore(id))
    }

    /// Invalidate checkpoint
    #[inline]
    pub fn invalidate(&mut self, id: CheckpointId) {
        if let Some(checkpoint) = self.checkpoints.get_mut(&id) {
            checkpoint.valid = false;
        }
    }

    /// Tick
    #[inline(always)]
    pub fn tick(&mut self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }

    /// Get checkpoint
    #[inline(always)]
    pub fn get(&self, id: CheckpointId) -> Option<&Checkpoint> {
        self.checkpoints.get(&id)
    }

    /// List checkpoints
    #[inline(always)]
    pub fn list(&self) -> Vec<CheckpointId> {
        self.checkpoints.keys().copied().collect()
    }
}

// ============================================================================
// WATCHDOG
// ============================================================================

/// Watchdog timer
pub struct Watchdog {
    /// Timeout (ticks)
    timeout: u64,
    /// Last kick
    last_kick: AtomicU64,
    /// Current tick
    tick: AtomicU64,
    /// Is triggered
    triggered: AtomicBool,
    /// On timeout action
    action: WatchdogAction,
}

/// Watchdog action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogAction {
    Reset,
    Panic,
    Callback,
    Nothing,
}

impl Watchdog {
    pub fn new(timeout: u64, action: WatchdogAction) -> Self {
        Self {
            timeout,
            last_kick: AtomicU64::new(0),
            tick: AtomicU64::new(0),
            triggered: AtomicBool::new(false),
            action,
        }
    }

    /// Kick the watchdog
    #[inline]
    pub fn kick(&self) {
        let tick = self.tick.load(Ordering::Relaxed);
        self.last_kick.store(tick, Ordering::Relaxed);
        self.triggered.store(false, Ordering::Relaxed);
    }

    /// Check watchdog
    #[inline]
    pub fn check(&self) -> Option<WatchdogAction> {
        let tick = self.tick.load(Ordering::Relaxed);
        let last = self.last_kick.load(Ordering::Relaxed);

        if tick - last >= self.timeout && !self.triggered.load(Ordering::Relaxed) {
            self.triggered.store(true, Ordering::Relaxed);
            return Some(self.action);
        }

        None
    }

    /// Tick
    #[inline(always)]
    pub fn tick(&self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }

    /// Is triggered?
    #[inline(always)]
    pub fn is_triggered(&self) -> bool {
        self.triggered.load(Ordering::Relaxed)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker() {
        let mut cb = CircuitBreaker::new(3, 2, 10);

        assert!(cb.allow());
        assert_eq!(cb.state(), CircuitState::Closed);

        // Record failures
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.allow());

        // Wait for timeout
        for _ in 0..10 {
            cb.tick();
        }

        assert!(cb.allow());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        cb.record_success();

        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::exponential(5, 10, 1000);

        assert_eq!(policy.delay_for_attempt(0, 0.5), 10);
        assert_eq!(policy.delay_for_attempt(1, 0.5), 20);
        assert_eq!(policy.delay_for_attempt(2, 0.5), 40);
    }

    #[test]
    fn test_failover() {
        let mut fm = FailoverManager::new(NodeId(1), vec![NodeId(2), NodeId(3)]);

        // Simulate primary failure
        fm.health_check_result(NodeId(1), false);
        fm.health_check_result(NodeId(1), false);
        let action = fm.health_check_result(NodeId(1), false);

        assert!(action.is_some());
        if let Some(FailoverAction::Initiate { new_primary, .. }) = action {
            assert_eq!(new_primary, NodeId(2));
        }

        fm.complete_failover(true);
        assert_eq!(fm.primary(), Some(NodeId(2)));
    }

    #[test]
    fn test_checkpoint() {
        let mut cm = CheckpointManager::new(100, 5);

        let id1 = cm.create(vec![1, 2, 3], BTreeMap::new());
        let id2 = cm.create(vec![4, 5, 6], BTreeMap::new());

        assert_eq!(cm.latest, Some(id2));

        let restored = cm.restore_latest().unwrap();
        assert_eq!(restored.state, vec![4, 5, 6]);
        assert_eq!(restored.parent, Some(id1));
    }

    #[test]
    fn test_watchdog() {
        let wd = Watchdog::new(10, WatchdogAction::Reset);

        wd.kick();

        for _ in 0..5 {
            wd.tick();
        }

        assert!(wd.check().is_none());

        for _ in 0..10 {
            wd.tick();
        }

        let action = wd.check();
        assert_eq!(action, Some(WatchdogAction::Reset));
    }
}

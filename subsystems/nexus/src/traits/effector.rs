//! Effector Traits
//!
//! Traits for the ACT domain - execution and effect management.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use super::component::NexusComponent;
use super::decider::ValidationResult;
use crate::types::{
    ActionId, ActionType, CheckpointId, Duration, Effect, Intent, IntentId, NexusResult, TimeRange,
    TransactionId,
};

// ============================================================================
// EFFECTOR TRAIT
// ============================================================================

/// Trait for execution effectors
pub trait Effector: NexusComponent {
    /// Intent type (input)
    type Intent;
    /// Effect type (output)
    type Effect;

    /// Execute an intent
    fn execute(&mut self, intent: Self::Intent) -> NexusResult<Self::Effect>;

    /// Check if intent can be executed
    fn can_execute(&self, intent: &Self::Intent) -> bool;

    /// Validate preconditions
    fn validate_preconditions(&self, intent: &Self::Intent) -> ValidationResult;

    /// Get rate limit configuration
    fn rate_limit(&self) -> RateLimit;

    /// Is currently rate limited?
    fn is_rate_limited(&self) -> bool;

    /// Dry run (simulate without applying)
    fn dry_run(&self, intent: &Self::Intent) -> NexusResult<Self::Effect>;
}

// ============================================================================
// RATE LIMIT
// ============================================================================

/// Rate limit configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimit {
    /// Maximum operations per period
    pub max_ops: u32,
    /// Period duration
    pub period: Duration,
    /// Current count in period
    pub current: u32,
    /// Burst allowance
    pub burst: u32,
}

impl RateLimit {
    /// Create new rate limit
    pub fn new(max_ops: u32, period: Duration) -> Self {
        Self {
            max_ops,
            period,
            current: 0,
            burst: 0,
        }
    }

    /// With burst allowance
    pub fn with_burst(mut self, burst: u32) -> Self {
        self.burst = burst;
        self
    }

    /// Is at limit?
    pub fn is_at_limit(&self) -> bool {
        self.current >= self.max_ops + self.burst
    }

    /// Remaining capacity
    pub fn remaining(&self) -> u32 {
        (self.max_ops + self.burst).saturating_sub(self.current)
    }

    /// Record an operation
    pub fn record(&mut self) {
        self.current = self.current.saturating_add(1);
    }

    /// Reset counter (on period rollover)
    pub fn reset(&mut self) {
        self.current = 0;
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self::new(100, Duration::SECOND)
    }
}

// ============================================================================
// TRANSACTION MANAGER TRAIT
// ============================================================================

/// Transaction manager trait
pub trait TransactionManager: NexusComponent {
    /// Begin a transaction
    fn begin(&mut self) -> NexusResult<TransactionId>;

    /// Commit a transaction
    fn commit(&mut self, txn: TransactionId) -> NexusResult<()>;

    /// Rollback a transaction
    fn rollback(&mut self, txn: TransactionId) -> NexusResult<()>;

    /// Create a checkpoint
    fn checkpoint(&mut self) -> NexusResult<CheckpointId>;

    /// Restore to checkpoint
    fn restore(&mut self, checkpoint: CheckpointId) -> NexusResult<()>;

    /// Get active transactions
    fn active_transactions(&self) -> Vec<TransactionId>;

    /// Get transaction state
    fn transaction_state(&self, txn: TransactionId) -> Option<TransactionState>;
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionState {
    /// Transaction is active
    Active,
    /// Transaction is committing
    Committing,
    /// Transaction is committed
    Committed,
    /// Transaction is rolling back
    RollingBack,
    /// Transaction is rolled back
    RolledBack,
    /// Transaction failed
    Failed,
}

impl TransactionState {
    /// Is transaction still active?
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Is transaction completed (committed or rolled back)?
    pub const fn is_completed(&self) -> bool {
        matches!(self, Self::Committed | Self::RolledBack)
    }

    /// Is transaction in terminal state?
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Committed | Self::RolledBack | Self::Failed)
    }
}

// ============================================================================
// AUDIT LOGGER TRAIT
// ============================================================================

/// Audit logger trait
pub trait AuditLogger: NexusComponent {
    /// Log an action execution
    fn log_action(&mut self, action: &ActionId, intent: &Intent, effect: &Effect);

    /// Log a decision
    fn log_decision(&mut self, decision: &IntentId, context: &str, rationale: &str);

    /// Log a rollback
    fn log_rollback(&mut self, txn: TransactionId, reason: &str);

    /// Query audit log
    fn query(&self, filter: AuditFilter) -> Vec<AuditEntry>;

    /// Get entry by ID
    fn get(&self, id: u64) -> Option<&AuditEntry>;

    /// Get entry count
    fn count(&self) -> usize;
}

/// Audit log filter
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    /// Time range
    pub time_range: Option<TimeRange>,
    /// Action types to include
    pub action_types: Vec<ActionType>,
    /// Entry types to include
    pub entry_types: Vec<AuditEntryType>,
    /// Maximum results
    pub limit: Option<usize>,
}

impl AuditFilter {
    /// Create empty filter (match all)
    pub fn all() -> Self {
        Self::default()
    }

    /// Filter by time range
    pub fn in_range(mut self, range: TimeRange) -> Self {
        self.time_range = Some(range);
        self
    }

    /// Filter by entry type
    pub fn of_type(mut self, entry_type: AuditEntryType) -> Self {
        self.entry_types.push(entry_type);
        self
    }

    /// Limit results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Entry ID
    pub id: u64,
    /// Timestamp
    pub timestamp: crate::types::Timestamp,
    /// Entry type
    pub entry_type: AuditEntryType,
    /// Description
    pub description: String,
    /// Associated entity IDs
    pub associations: Vec<String>,
    /// Additional metadata
    pub metadata: Vec<(String, String)>,
}

impl AuditEntry {
    /// Create new audit entry
    pub fn new(entry_type: AuditEntryType, description: impl Into<String>) -> Self {
        static COUNTER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);
        Self {
            id: COUNTER.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
            timestamp: crate::types::Timestamp::now(),
            entry_type,
            description: description.into(),
            associations: Vec::new(),
            metadata: Vec::new(),
        }
    }

    /// Add association
    pub fn with_association(mut self, association: impl Into<String>) -> Self {
        self.associations.push(association.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }
}

/// Audit entry types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditEntryType {
    /// Decision made
    Decision,
    /// Action executed
    Action,
    /// Rollback performed
    Rollback,
    /// Policy violation
    PolicyViolation,
    /// Error occurred
    Error,
    /// Configuration change
    ConfigChange,
    /// State change
    StateChange,
}

impl AuditEntryType {
    /// Get type name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Action => "action",
            Self::Rollback => "rollback",
            Self::PolicyViolation => "policy_violation",
            Self::Error => "error",
            Self::ConfigChange => "config_change",
            Self::StateChange => "state_change",
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit() {
        let mut rl = RateLimit::new(10, Duration::SECOND);
        assert!(!rl.is_at_limit());
        assert_eq!(rl.remaining(), 10);

        for _ in 0..10 {
            rl.record();
        }
        assert!(rl.is_at_limit());
        assert_eq!(rl.remaining(), 0);

        rl.reset();
        assert!(!rl.is_at_limit());
    }

    #[test]
    fn test_rate_limit_with_burst() {
        let mut rl = RateLimit::new(10, Duration::SECOND).with_burst(5);
        assert_eq!(rl.remaining(), 15);

        for _ in 0..12 {
            rl.record();
        }
        assert!(!rl.is_at_limit());
        assert_eq!(rl.remaining(), 3);
    }

    #[test]
    fn test_transaction_state() {
        assert!(TransactionState::Active.is_active());
        assert!(TransactionState::Committed.is_completed());
        assert!(TransactionState::Failed.is_terminal());
    }

    #[test]
    fn test_audit_entry() {
        let entry = AuditEntry::new(AuditEntryType::Action, "Executed scaling")
            .with_association("action:123")
            .with_metadata("target", "scheduler");

        assert_eq!(entry.entry_type, AuditEntryType::Action);
        assert_eq!(entry.associations.len(), 1);
        assert_eq!(entry.metadata.len(), 1);
    }
}

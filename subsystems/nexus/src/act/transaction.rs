//! Transaction Manager — Atomicity and rollback
//!
//! The transaction manager provides ACID-like semantics for kernel
//! actions, allowing rollback on failure and atomic state changes.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::effect::{Change, ChangeValue};
// Re-export TransactionId from types for backward compatibility
pub use crate::types::TransactionId;
use crate::types::*;

// ============================================================================
// TRANSACTION
// ============================================================================

/// A transaction
#[derive(Debug)]
pub struct Transaction {
    /// Transaction ID
    pub id: TransactionId,
    /// Intent being executed
    pub intent_id: IntentId,
    /// State before action
    pub rollback_state: RollbackState,
    /// Changes made
    pub changes: Vec<Change>,
    /// Started at
    pub started_at: Timestamp,
    /// Status
    pub status: TransactionStatus,
}

impl Transaction {
    /// Create new transaction
    pub fn new(intent_id: IntentId, now: Timestamp) -> Self {
        Self {
            id: TransactionId::generate(),
            intent_id,
            rollback_state: RollbackState::new(now),
            changes: Vec::new(),
            started_at: now,
            status: TransactionStatus::Active,
        }
    }

    /// Get transaction age
    pub fn age(&self, now: Timestamp) -> Duration {
        now.elapsed_since(self.started_at)
    }

    /// Is active?
    pub fn is_active(&self) -> bool {
        self.status == TransactionStatus::Active
    }

    /// Is completed?
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            TransactionStatus::Committed
                | TransactionStatus::RolledBack
                | TransactionStatus::Failed
        )
    }

    /// Get change count
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }
}

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction is active
    Active,
    /// Transaction is committing
    Committing,
    /// Transaction is rolling back
    RollingBack,
    /// Transaction committed
    Committed,
    /// Transaction rolled back
    RolledBack,
    /// Transaction failed
    Failed,
}

impl TransactionStatus {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Committing => "Committing",
            Self::RollingBack => "Rolling Back",
            Self::Committed => "Committed",
            Self::RolledBack => "Rolled Back",
            Self::Failed => "Failed",
        }
    }
}

// ============================================================================
// ROLLBACK STATE
// ============================================================================

/// Rollback state
#[derive(Debug, Clone)]
pub struct RollbackState {
    /// Captured values
    pub captured: Vec<CapturedValue>,
    /// Capture timestamp
    pub captured_at: Timestamp,
}

impl RollbackState {
    /// Create new rollback state
    pub fn new(now: Timestamp) -> Self {
        Self {
            captured: Vec::new(),
            captured_at: now,
        }
    }

    /// Add captured value
    pub fn capture(&mut self, key: impl Into<String>, value: ChangeValue) {
        self.captured.push(CapturedValue {
            key: key.into(),
            value,
        });
    }

    /// Get captured value
    pub fn get(&self, key: &str) -> Option<&ChangeValue> {
        self.captured
            .iter()
            .find(|c| c.key == key)
            .map(|c| &c.value)
    }

    /// Capture count
    pub fn len(&self) -> usize {
        self.captured.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.captured.is_empty()
    }
}

/// Captured value for rollback
#[derive(Debug, Clone)]
pub struct CapturedValue {
    /// Key/path
    pub key: String,
    /// Value
    pub value: ChangeValue,
}

// ============================================================================
// TRANSACTION MANAGER
// ============================================================================

/// Transaction manager
pub struct TransactionManager {
    /// Active transactions
    active: BTreeMap<TransactionId, Transaction>,
    /// Completed transactions
    completed: Vec<TransactionId>,
    /// Maximum completed history
    max_completed: usize,
    /// Total transactions
    total: AtomicU64,
    /// Rollbacks
    rollbacks: AtomicU64,
}

impl TransactionManager {
    /// Create new transaction manager
    pub fn new() -> Self {
        Self {
            active: BTreeMap::new(),
            completed: Vec::new(),
            max_completed: 1000,
            total: AtomicU64::new(0),
            rollbacks: AtomicU64::new(0),
        }
    }

    /// Create with custom history size
    pub fn with_history_size(max_completed: usize) -> Self {
        let mut manager = Self::new();
        manager.max_completed = max_completed;
        manager
    }

    /// Begin a transaction
    pub fn begin(&mut self, intent_id: IntentId, now: Timestamp) -> TransactionId {
        let id = TransactionId::generate();
        self.total.fetch_add(1, Ordering::Relaxed);

        let transaction = Transaction {
            id,
            intent_id,
            rollback_state: RollbackState {
                captured: Vec::new(),
                captured_at: now,
            },
            changes: Vec::new(),
            started_at: now,
            status: TransactionStatus::Active,
        };

        self.active.insert(id, transaction);
        id
    }

    /// Get active transaction
    pub fn get(&self, tx_id: TransactionId) -> Option<&Transaction> {
        self.active.get(&tx_id)
    }

    /// Get mutable active transaction
    pub fn get_mut(&mut self, tx_id: TransactionId) -> Option<&mut Transaction> {
        self.active.get_mut(&tx_id)
    }

    /// Capture rollback state
    pub fn capture_state(&mut self, tx_id: TransactionId, key: String, value: ChangeValue) -> bool {
        if let Some(tx) = self.active.get_mut(&tx_id) {
            tx.rollback_state
                .captured
                .push(CapturedValue { key, value });
            true
        } else {
            false
        }
    }

    /// Record a change
    pub fn record_change(&mut self, tx_id: TransactionId, change: Change) -> bool {
        if let Some(tx) = self.active.get_mut(&tx_id) {
            tx.changes.push(change);
            true
        } else {
            false
        }
    }

    /// Commit a transaction
    pub fn commit(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        if let Some(mut tx) = self.active.remove(&tx_id) {
            tx.status = TransactionStatus::Committed;
            self.add_completed(tx_id);
            Ok(())
        } else {
            Err(TransactionError::NotFound)
        }
    }

    /// Rollback a transaction
    pub fn rollback(&mut self, tx_id: TransactionId) -> Result<RollbackState, TransactionError> {
        if let Some(mut tx) = self.active.remove(&tx_id) {
            tx.status = TransactionStatus::RolledBack;
            self.rollbacks.fetch_add(1, Ordering::Relaxed);
            self.add_completed(tx_id);
            Ok(tx.rollback_state)
        } else {
            Err(TransactionError::NotFound)
        }
    }

    /// Fail a transaction
    pub fn fail(&mut self, tx_id: TransactionId) -> Result<(), TransactionError> {
        if let Some(mut tx) = self.active.remove(&tx_id) {
            tx.status = TransactionStatus::Failed;
            self.add_completed(tx_id);
            Ok(())
        } else {
            Err(TransactionError::NotFound)
        }
    }

    /// Add to completed list
    fn add_completed(&mut self, tx_id: TransactionId) {
        self.completed.push(tx_id);
        if self.completed.len() > self.max_completed {
            self.completed.remove(0);
        }
    }

    /// Get active transaction count
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Get completed count
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Cleanup old transactions
    pub fn cleanup_stale(&mut self, max_age: Duration, now: Timestamp) -> usize {
        let stale: Vec<_> = self
            .active
            .iter()
            .filter(|(_, tx)| tx.age(now).as_nanos() > max_age.as_nanos())
            .map(|(id, _)| *id)
            .collect();

        let count = stale.len();
        for id in stale {
            let _ = self.fail(id);
        }
        count
    }

    /// Get statistics
    pub fn stats(&self) -> TransactionStats {
        TransactionStats {
            total: self.total.load(Ordering::Relaxed),
            active: self.active.len(),
            completed: self.completed.len(),
            rollbacks: self.rollbacks.load(Ordering::Relaxed),
        }
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionError {
    /// Transaction not found
    NotFound,
    /// Transaction already completed
    AlreadyCompleted,
    /// Rollback failed
    RollbackFailed,
}

impl TransactionError {
    /// Get error message
    pub fn message(&self) -> &'static str {
        match self {
            Self::NotFound => "Transaction not found",
            Self::AlreadyCompleted => "Transaction already completed",
            Self::RollbackFailed => "Rollback failed",
        }
    }
}

/// Transaction statistics
#[derive(Debug, Clone)]
pub struct TransactionStats {
    /// Total transactions created
    pub total: u64,
    /// Currently active
    pub active: usize,
    /// Completed count
    pub completed: usize,
    /// Rollback count
    pub rollbacks: u64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_manager() {
        let mut manager = TransactionManager::new();
        let tx = manager.begin(IntentId::generate(), Timestamp::now());

        assert_eq!(manager.active_count(), 1);

        manager.commit(tx).unwrap();
        assert_eq!(manager.active_count(), 0);
        assert_eq!(manager.completed_count(), 1);
    }

    #[test]
    fn test_rollback() {
        let mut manager = TransactionManager::new();
        let tx = manager.begin(IntentId::generate(), Timestamp::now());

        manager.capture_state(tx, String::from("key"), ChangeValue::Integer(42));

        let state = manager.rollback(tx).unwrap();
        assert_eq!(state.len(), 1);
        assert_eq!(manager.stats().rollbacks, 1);
    }

    #[test]
    fn test_transaction_not_found() {
        let mut manager = TransactionManager::new();
        let result = manager.commit(TransactionId::new(999));
        assert_eq!(result, Err(TransactionError::NotFound));
    }

    #[test]
    fn test_rollback_state() {
        let mut state = RollbackState::new(Timestamp::now());
        state.capture("test", ChangeValue::Boolean(true));

        assert_eq!(state.len(), 1);
        assert!(state.get("test").is_some());
    }
}

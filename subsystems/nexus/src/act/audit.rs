//! Audit Logger — Action traceability
//!
//! The audit logger records all actions executed by the ACT domain,
//! providing full traceability for debugging and forensics.

use alloc::format;
use alloc::string::String;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::effect::{ActionOutcome, Effect};
use super::limiter::target_to_string;
use super::transaction::TransactionId;
use crate::types::*;
// ActionType now comes from types::* above

// ============================================================================
// AUDIT ENTRY
// ============================================================================

/// Audit entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Entry ID
    pub id: AuditId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Intent that triggered this
    pub intent_id: IntentId,
    /// Action type
    pub action_type: ActionType,
    /// Target
    pub target: String,
    /// Outcome
    pub outcome: AuditOutcome,
    /// Duration
    pub duration: Duration,
    /// Changes made
    pub changes: Vec<String>,
    /// Effector used
    pub effector: String,
    /// Transaction ID
    pub transaction_id: Option<TransactionId>,
    /// Was rolled back
    pub rolled_back: bool,
}

impl AuditEntry {
    /// Create new audit entry
    pub fn new(
        intent_id: IntentId,
        action_type: ActionType,
        target: impl Into<String>,
        outcome: AuditOutcome,
    ) -> Self {
        Self {
            id: AuditId::generate(),
            timestamp: Timestamp::now(),
            intent_id,
            action_type,
            target: target.into(),
            outcome,
            duration: Duration::ZERO,
            changes: Vec::new(),
            effector: String::new(),
            transaction_id: None,
            rolled_back: false,
        }
    }

    /// Set duration
    #[inline(always)]
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set effector
    #[inline(always)]
    pub fn with_effector(mut self, effector: impl Into<String>) -> Self {
        self.effector = effector.into();
        self
    }

    /// Set transaction
    #[inline(always)]
    pub fn with_transaction(mut self, tx_id: TransactionId) -> Self {
        self.transaction_id = Some(tx_id);
        self
    }

    /// Add change description
    #[inline(always)]
    pub fn add_change(&mut self, change: impl Into<String>) {
        self.changes.push(change.into());
    }

    /// Mark as rolled back
    #[inline(always)]
    pub fn mark_rolled_back(&mut self) {
        self.rolled_back = true;
        self.outcome = AuditOutcome::RolledBack;
    }

    /// Is success?
    #[inline(always)]
    pub fn is_success(&self) -> bool {
        self.outcome == AuditOutcome::Success
    }

    /// Is failure?
    #[inline]
    pub fn is_failure(&self) -> bool {
        matches!(
            self.outcome,
            AuditOutcome::Failed | AuditOutcome::RolledBack
        )
    }
}

// ============================================================================
// AUDIT OUTCOME
// ============================================================================

/// Audit outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    /// Action succeeded
    Success,
    /// Action partially succeeded
    Partial,
    /// Action failed
    Failed,
    /// Action was skipped
    Skipped,
    /// Action was rejected
    Rejected,
    /// Action was rolled back
    RolledBack,
}

impl AuditOutcome {
    /// Get display name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::Partial => "Partial",
            Self::Failed => "Failed",
            Self::Skipped => "Skipped",
            Self::Rejected => "Rejected",
            Self::RolledBack => "Rolled Back",
        }
    }

    /// From action outcome
    #[inline]
    pub fn from_action_outcome(outcome: &ActionOutcome) -> Self {
        match outcome {
            ActionOutcome::Success { .. } => Self::Success,
            ActionOutcome::Partial { .. } => Self::Partial,
            ActionOutcome::Failed { .. } => Self::Failed,
            ActionOutcome::Skipped { .. } => Self::Skipped,
            ActionOutcome::Rejected { .. } => Self::Rejected,
        }
    }
}

// ============================================================================
// AUDIT LOGGER
// ============================================================================

/// Audit logger
pub struct AuditLogger {
    /// Audit entries
    entries: VecDeque<AuditEntry>,
    /// Maximum entries
    max_entries: usize,
    /// Entries written
    total_entries: AtomicU64,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
            total_entries: AtomicU64::new(0),
        }
    }

    /// Log an entry
    pub fn log(&mut self, entry: AuditEntry) -> AuditId {
        let id = entry.id;
        self.entries.push_back(entry);
        self.total_entries.fetch_add(1, Ordering::Relaxed);

        // Trim if necessary
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        id
    }

    /// Create entry for effect
    pub fn log_effect(&mut self, effect: &Effect, effector_name: &str) -> AuditId {
        let outcome = if effect.rolled_back {
            AuditOutcome::RolledBack
        } else {
            AuditOutcome::from_action_outcome(&effect.outcome)
        };

        let entry = AuditEntry {
            id: AuditId::generate(),
            timestamp: effect.ended_at,
            intent_id: effect.intent_id,
            action_type: effect.action_type,
            target: target_to_string(&effect.target),
            outcome,
            duration: effect.duration,
            changes: effect
                .changes
                .iter()
                .map(|c| format!("{:?}: {}", c.change_type, c.target))
                .collect(),
            effector: String::from(effector_name),
            transaction_id: if effect.transactional {
                Some(TransactionId::generate())
            } else {
                None
            },
            rolled_back: effect.rolled_back,
        };

        self.log(entry)
    }

    /// Get all entries
    #[inline(always)]
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Get recent entries
    #[inline(always)]
    pub fn recent(&self, count: usize) -> &[AuditEntry] {
        let start = self.entries.len().saturating_sub(count);
        &self.entries[start..]
    }

    /// Get entries by outcome
    #[inline]
    pub fn by_outcome(&self, outcome: AuditOutcome) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.outcome == outcome)
            .collect()
    }

    /// Get entries by action type
    #[inline]
    pub fn by_action(&self, action_type: ActionType) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.action_type == action_type)
            .collect()
    }

    /// Get entries for intent
    #[inline]
    pub fn for_intent(&self, intent_id: IntentId) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.intent_id == intent_id)
            .collect()
    }

    /// Get entry count
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> AuditStats {
        let successes = self.by_outcome(AuditOutcome::Success).len();
        let failures = self.by_outcome(AuditOutcome::Failed).len();
        let rollbacks = self.by_outcome(AuditOutcome::RolledBack).len();

        AuditStats {
            total_entries: self.total_entries.load(Ordering::Relaxed),
            current_entries: self.entries.len(),
            successes: successes as u64,
            failures: failures as u64,
            rollbacks: rollbacks as u64,
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Audit statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AuditStats {
    /// Total entries logged
    pub total_entries: u64,
    /// Current entries in buffer
    pub current_entries: usize,
    /// Successful actions
    pub successes: u64,
    /// Failed actions
    pub failures: u64,
    /// Rolled back actions
    pub rollbacks: u64,
}

impl AuditStats {
    /// Success rate
    #[inline]
    pub fn success_rate(&self) -> f32 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.successes as f32 / self.total_entries as f32
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
    fn test_audit_logger() {
        let mut logger = AuditLogger::new(100);
        let entry = AuditEntry::new(
            IntentId::generate(),
            ActionType::Log,
            "test",
            AuditOutcome::Success,
        );

        logger.log(entry);
        assert_eq!(logger.len(), 1);
    }

    #[test]
    fn test_audit_entry() {
        let entry = AuditEntry::new(
            IntentId::generate(),
            ActionType::Restart,
            "system",
            AuditOutcome::Success,
        )
        .with_duration(Duration::from_millis(100))
        .with_effector("process_effector");

        assert!(entry.is_success());
        assert!(!entry.effector.is_empty());
    }

    #[test]
    fn test_audit_stats() {
        let mut logger = AuditLogger::new(100);

        // Add some entries
        logger.log(AuditEntry::new(
            IntentId::generate(),
            ActionType::Log,
            "test",
            AuditOutcome::Success,
        ));
        logger.log(AuditEntry::new(
            IntentId::generate(),
            ActionType::Kill,
            "test",
            AuditOutcome::Failed,
        ));

        let stats = logger.stats();
        assert_eq!(stats.current_entries, 2);
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.failures, 1);
    }

    #[test]
    fn test_recent_entries() {
        let mut logger = AuditLogger::new(100);

        for i in 0..10 {
            logger.log(AuditEntry::new(
                IntentId::new(i),
                ActionType::Log,
                "test",
                AuditOutcome::Success,
            ));
        }

        let recent = logger.recent(5);
        assert_eq!(recent.len(), 5);
    }
}

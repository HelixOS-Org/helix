//! Effect — Action execution results
//!
//! Effects represent the results of executed actions, including
//! the changes made, timing information, and transaction status.

use alloc::string::String;
use alloc::vec::Vec;

use crate::types::*;
use crate::decide::{ActionType, ActionTarget};

// ============================================================================
// EFFECT
// ============================================================================

/// An effect - the result of an action
#[derive(Debug, Clone)]
pub struct Effect {
    /// Effect ID
    pub id: EffectId,
    /// Source intent
    pub intent_id: IntentId,
    /// Action type that was executed
    pub action_type: ActionType,
    /// Target that was affected
    pub target: ActionTarget,
    /// Outcome
    pub outcome: ActionOutcome,
    /// Start time
    pub started_at: Timestamp,
    /// End time
    pub ended_at: Timestamp,
    /// Duration
    pub duration: Duration,
    /// Was transaction used
    pub transactional: bool,
    /// Was rolled back
    pub rolled_back: bool,
    /// Changes made
    pub changes: Vec<Change>,
}

impl Effect {
    /// Create a new successful effect
    pub fn success(
        intent_id: IntentId,
        action_type: ActionType,
        target: ActionTarget,
        changes: Vec<Change>,
        started_at: Timestamp,
        ended_at: Timestamp,
    ) -> Self {
        Self {
            id: EffectId::generate(),
            intent_id,
            action_type,
            target,
            outcome: ActionOutcome::Success {
                summary: alloc::format!("{:?} executed successfully", action_type),
            },
            started_at,
            ended_at,
            duration: ended_at.elapsed_since(started_at),
            transactional: false,
            rolled_back: false,
            changes,
        }
    }

    /// Create a failed effect
    pub fn failed(
        intent_id: IntentId,
        action_type: ActionType,
        target: ActionTarget,
        error_code: ErrorCode,
        message: impl Into<String>,
        started_at: Timestamp,
        ended_at: Timestamp,
    ) -> Self {
        Self {
            id: EffectId::generate(),
            intent_id,
            action_type,
            target,
            outcome: ActionOutcome::Failed {
                error_code,
                message: message.into(),
            },
            started_at,
            ended_at,
            duration: ended_at.elapsed_since(started_at),
            transactional: false,
            rolled_back: false,
            changes: Vec::new(),
        }
    }

    /// Create a rejected effect
    pub fn rejected(
        intent_id: IntentId,
        action_type: ActionType,
        target: ActionTarget,
        reason: impl Into<String>,
        now: Timestamp,
    ) -> Self {
        Self {
            id: EffectId::generate(),
            intent_id,
            action_type,
            target,
            outcome: ActionOutcome::Rejected {
                reason: reason.into(),
            },
            started_at: now,
            ended_at: now,
            duration: Duration::ZERO,
            transactional: false,
            rolled_back: false,
            changes: Vec::new(),
        }
    }

    /// Set transactional flag
    pub fn with_transaction(mut self) -> Self {
        self.transactional = true;
        self
    }

    /// Mark as rolled back
    pub fn mark_rolled_back(&mut self) {
        self.rolled_back = true;
    }

    /// Is successful?
    pub fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Is failure?
    pub fn is_failure(&self) -> bool {
        self.outcome.is_failure()
    }

    /// Get change count
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// Has reversible changes?
    pub fn has_reversible_changes(&self) -> bool {
        self.changes.iter().any(|c| c.reversible)
    }
}

// ============================================================================
// ACTION OUTCOME
// ============================================================================

/// Action outcome
#[derive(Debug, Clone)]
pub enum ActionOutcome {
    /// Action succeeded
    Success {
        /// Summary of what was done
        summary: String,
    },
    /// Action partially succeeded
    Partial {
        /// What succeeded
        succeeded: String,
        /// What failed
        failed: String,
    },
    /// Action failed
    Failed {
        /// Error code
        error_code: ErrorCode,
        /// Error message
        message: String,
    },
    /// Action was skipped
    Skipped {
        /// Reason for skipping
        reason: String,
    },
    /// Action was rejected
    Rejected {
        /// Rejection reason
        reason: String,
    },
}

impl ActionOutcome {
    /// Is this a success
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Is this a partial success
    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial { .. })
    }

    /// Is this a failure
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Is this skipped
    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }

    /// Is this rejected
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }

    /// Get summary/message
    pub fn message(&self) -> &str {
        match self {
            Self::Success { summary } => summary,
            Self::Partial { succeeded, .. } => succeeded,
            Self::Failed { message, .. } => message,
            Self::Skipped { reason } => reason,
            Self::Rejected { reason } => reason,
        }
    }
}

// ============================================================================
// CHANGE
// ============================================================================

/// A change made by an action
#[derive(Debug, Clone)]
pub struct Change {
    /// Change ID
    pub id: ChangeId,
    /// What was changed
    pub target: String,
    /// Change type
    pub change_type: ChangeType,
    /// Previous value (for rollback)
    pub previous_value: Option<ChangeValue>,
    /// New value
    pub new_value: ChangeValue,
    /// Is reversible
    pub reversible: bool,
}

impl Change {
    /// Create a new change
    pub fn new(target: impl Into<String>, change_type: ChangeType, new_value: ChangeValue) -> Self {
        Self {
            id: ChangeId::generate(),
            target: target.into(),
            change_type,
            previous_value: None,
            new_value,
            reversible: true,
        }
    }

    /// Set previous value
    pub fn with_previous(mut self, value: ChangeValue) -> Self {
        self.previous_value = Some(value);
        self
    }

    /// Mark as irreversible
    pub fn irreversible(mut self) -> Self {
        self.reversible = false;
        self
    }

    /// Can be rolled back?
    pub fn can_rollback(&self) -> bool {
        self.reversible && self.previous_value.is_some()
    }
}

/// Change ID type
define_id!(ChangeId, "Change identifier");

/// Change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Value was created
    Create,
    /// Value was modified
    Modify,
    /// Value was deleted
    Delete,
    /// State was changed
    StateChange,
    /// Resource was allocated
    Allocate,
    /// Resource was deallocated
    Deallocate,
}

impl ChangeType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Modify => "Modify",
            Self::Delete => "Delete",
            Self::StateChange => "State Change",
            Self::Allocate => "Allocate",
            Self::Deallocate => "Deallocate",
        }
    }

    /// Is destructive?
    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::Delete | Self::Deallocate)
    }
}

// ============================================================================
// CHANGE VALUE
// ============================================================================

/// Change value
#[derive(Debug, Clone)]
pub enum ChangeValue {
    /// No value
    None,
    /// Boolean value
    Boolean(bool),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Bytes value
    Bytes(Vec<u8>),
    /// State value
    State(u32),
}

impl ChangeValue {
    /// Is none?
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// As boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// As integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// As string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }
}

impl Default for ChangeValue {
    fn default() -> Self {
        Self::None
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decide::ActionTarget;

    #[test]
    fn test_effect_success() {
        let now = Timestamp::now();
        let effect = Effect::success(
            IntentId::generate(),
            ActionType::Log,
            ActionTarget::System,
            Vec::new(),
            now,
            now,
        );

        assert!(effect.is_success());
        assert!(!effect.rolled_back);
    }

    #[test]
    fn test_change_creation() {
        let change = Change::new("test", ChangeType::Modify, ChangeValue::Integer(42))
            .with_previous(ChangeValue::Integer(0));

        assert!(change.can_rollback());
    }

    #[test]
    fn test_action_outcome() {
        let success = ActionOutcome::Success {
            summary: String::from("Done"),
        };
        assert!(success.is_success());
        assert!(!success.is_failure());
    }
}

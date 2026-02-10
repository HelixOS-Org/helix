//! # Action Rollback
//!
//! Manages action rollback and state restoration.
//! Implements checkpointing and undo mechanisms.
//!
//! Part of Year 2 COGNITION - Action Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// ROLLBACK TYPES
// ============================================================================

/// Checkpoint
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Checkpoint ID
    pub id: u64,
    /// Name
    pub name: String,
    /// State snapshot
    pub state: StateSnapshot,
    /// Created timestamp
    pub created: Timestamp,
    /// Parent checkpoint
    pub parent: Option<u64>,
}

/// State snapshot
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StateSnapshot {
    /// State values
    pub values: BTreeMap<String, StateValue>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// State value
#[derive(Debug, Clone)]
pub enum StateValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<StateValue>),
    Map(BTreeMap<String, StateValue>),
}

/// Action record
#[derive(Debug, Clone)]
pub struct ActionRecord {
    /// Record ID
    pub id: u64,
    /// Action name
    pub action: String,
    /// Before state (delta)
    pub before: BTreeMap<String, StateValue>,
    /// After state (delta)
    pub after: BTreeMap<String, StateValue>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Is reversible
    pub reversible: bool,
}

/// Rollback result
#[derive(Debug, Clone)]
pub struct RollbackResult {
    /// Success
    pub success: bool,
    /// Rolled back actions
    pub rolled_back: Vec<u64>,
    /// New state
    pub new_state: StateSnapshot,
    /// Errors
    pub errors: Vec<String>,
}

/// Undo operation
#[derive(Debug, Clone)]
pub struct UndoOperation {
    /// Operation ID
    pub id: u64,
    /// Target action
    pub target_action: u64,
    /// Reverse delta
    pub reverse_delta: BTreeMap<String, StateValue>,
    /// Status
    pub status: UndoStatus,
}

/// Undo status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoStatus {
    Pending,
    Applied,
    Failed,
    Skipped,
}

// ============================================================================
// ROLLBACK MANAGER
// ============================================================================

/// Rollback manager
pub struct RollbackManager {
    /// Current state
    current_state: StateSnapshot,
    /// Checkpoints
    checkpoints: BTreeMap<u64, Checkpoint>,
    /// Action history
    history: VecDeque<ActionRecord>,
    /// Undo stack
    undo_stack: Vec<UndoOperation>,
    /// Redo stack
    redo_stack: Vec<UndoOperation>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: RollbackConfig,
    /// Statistics
    stats: RollbackStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct RollbackConfig {
    /// Maximum checkpoints
    pub max_checkpoints: usize,
    /// Maximum history
    pub max_history: usize,
    /// Auto checkpoint interval
    pub auto_checkpoint_interval: usize,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            max_checkpoints: 50,
            max_history: 1000,
            auto_checkpoint_interval: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RollbackStats {
    /// Checkpoints created
    pub checkpoints_created: u64,
    /// Actions recorded
    pub actions_recorded: u64,
    /// Rollbacks performed
    pub rollbacks_performed: u64,
    /// Undos performed
    pub undos_performed: u64,
}

impl RollbackManager {
    /// Create new manager
    pub fn new(config: RollbackConfig) -> Self {
        Self {
            current_state: StateSnapshot {
                values: BTreeMap::new(),
                metadata: BTreeMap::new(),
            },
            checkpoints: BTreeMap::new(),
            history: VecDeque::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: RollbackStats::default(),
        }
    }

    /// Create checkpoint
    pub fn checkpoint(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let parent = self.checkpoints.keys().max().copied();

        let checkpoint = Checkpoint {
            id,
            name: name.into(),
            state: self.current_state.clone(),
            created: Timestamp::now(),
            parent,
        };

        self.checkpoints.insert(id, checkpoint);
        self.stats.checkpoints_created += 1;

        // Cleanup old checkpoints
        while self.checkpoints.len() > self.config.max_checkpoints {
            if let Some(oldest) = self.checkpoints.keys().next().copied() {
                self.checkpoints.remove(&oldest);
            }
        }

        id
    }

    /// Record action
    pub fn record(&mut self, action: &str, before: BTreeMap<String, StateValue>, after: BTreeMap<String, StateValue>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let record = ActionRecord {
            id,
            action: action.into(),
            before: before.clone(),
            after: after.clone(),
            timestamp: Timestamp::now(),
            reversible: true,
        };

        self.history.push_back(record);
        self.stats.actions_recorded += 1;

        // Apply to current state
        for (key, value) in after {
            self.current_state.values.insert(key, value);
        }

        // Clear redo stack on new action
        self.redo_stack.clear();

        // Cleanup old history
        while self.history.len() > self.config.max_history {
            self.history.remove(0);
        }

        // Auto checkpoint
        if self.stats.actions_recorded % self.config.auto_checkpoint_interval as u64 == 0 {
            self.checkpoint("auto");
        }

        id
    }

    /// Undo last action
    pub fn undo(&mut self) -> Option<u64> {
        let record = self.history.pop()?;

        if !record.reversible {
            // Put it back
            self.history.push_back(record);
            return None;
        }

        let undo_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Apply reverse delta
        for (key, value) in &record.before {
            self.current_state.values.insert(key.clone(), value.clone());
        }

        // Create undo operation for redo
        let undo_op = UndoOperation {
            id: undo_id,
            target_action: record.id,
            reverse_delta: record.after.clone(),
            status: UndoStatus::Applied,
        };

        self.redo_stack.push(undo_op);
        self.stats.undos_performed += 1;

        Some(record.id)
    }

    /// Redo last undone action
    #[inline]
    pub fn redo(&mut self) -> Option<u64> {
        let undo_op = self.redo_stack.pop()?;

        // Apply the after state
        for (key, value) in &undo_op.reverse_delta {
            self.current_state.values.insert(key.clone(), value.clone());
        }

        Some(undo_op.target_action)
    }

    /// Rollback to checkpoint
    pub fn rollback_to(&mut self, checkpoint_id: u64) -> RollbackResult {
        let checkpoint = match self.checkpoints.get(&checkpoint_id) {
            Some(cp) => cp.clone(),
            None => {
                return RollbackResult {
                    success: false,
                    rolled_back: Vec::new(),
                    new_state: self.current_state.clone(),
                    errors: vec!["Checkpoint not found".into()],
                };
            }
        };

        // Find actions to rollback
        let rolled_back: Vec<u64> = self.history.iter()
            .filter(|r| r.timestamp.0 > checkpoint.created.0)
            .map(|r| r.id)
            .collect();

        // Remove rolled back actions from history
        self.history.retain(|r| r.timestamp.0 <= checkpoint.created.0);

        // Restore state
        self.current_state = checkpoint.state.clone();

        self.stats.rollbacks_performed += 1;

        RollbackResult {
            success: true,
            rolled_back,
            new_state: self.current_state.clone(),
            errors: Vec::new(),
        }
    }

    /// Rollback N actions
    pub fn rollback_n(&mut self, n: usize) -> RollbackResult {
        let mut rolled_back = Vec::new();
        let mut errors = Vec::new();

        for _ in 0..n {
            match self.undo() {
                Some(id) => rolled_back.push(id),
                None => {
                    errors.push("No more actions to undo".into());
                    break;
                }
            }
        }

        RollbackResult {
            success: errors.is_empty(),
            rolled_back,
            new_state: self.current_state.clone(),
            errors,
        }
    }

    /// Get current state
    #[inline(always)]
    pub fn current_state(&self) -> &StateSnapshot {
        &self.current_state
    }

    /// Set state value
    #[inline]
    pub fn set(&mut self, key: &str, value: StateValue) {
        let before = self.current_state.values.get(key).cloned().unwrap_or(StateValue::Null);

        let mut before_map = BTreeMap::new();
        before_map.insert(key.into(), before);

        let mut after_map = BTreeMap::new();
        after_map.insert(key.into(), value);

        self.record("set", before_map, after_map);
    }

    /// Get state value
    #[inline(always)]
    pub fn get(&self, key: &str) -> Option<&StateValue> {
        self.current_state.values.get(key)
    }

    /// Get checkpoint
    #[inline(always)]
    pub fn get_checkpoint(&self, id: u64) -> Option<&Checkpoint> {
        self.checkpoints.get(&id)
    }

    /// List checkpoints
    #[inline(always)]
    pub fn list_checkpoints(&self) -> Vec<&Checkpoint> {
        self.checkpoints.values().collect()
    }

    /// Get history
    #[inline(always)]
    pub fn history(&self) -> &[ActionRecord] {
        &self.history
    }

    /// Can undo
    #[inline(always)]
    pub fn can_undo(&self) -> bool {
        self.history.iter().any(|r| r.reversible)
    }

    /// Can redo
    #[inline(always)]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &RollbackStats {
        &self.stats
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new(RollbackConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint() {
        let mut manager = RollbackManager::default();

        let id = manager.checkpoint("test");
        assert!(manager.get_checkpoint(id).is_some());
    }

    #[test]
    fn test_record() {
        let mut manager = RollbackManager::default();

        let mut before = BTreeMap::new();
        before.insert("x".into(), StateValue::Integer(0));

        let mut after = BTreeMap::new();
        after.insert("x".into(), StateValue::Integer(1));

        manager.record("set_x", before, after);

        let val = manager.get("x").unwrap();
        assert!(matches!(val, StateValue::Integer(1)));
    }

    #[test]
    fn test_undo() {
        let mut manager = RollbackManager::default();

        manager.set("x", StateValue::Integer(1));
        manager.set("x", StateValue::Integer(2));

        let val = manager.get("x").unwrap();
        assert!(matches!(val, StateValue::Integer(2)));

        manager.undo();

        let val = manager.get("x").unwrap();
        assert!(matches!(val, StateValue::Integer(1)));
    }

    #[test]
    fn test_redo() {
        let mut manager = RollbackManager::default();

        manager.set("x", StateValue::Integer(1));
        manager.set("x", StateValue::Integer(2));

        manager.undo();
        manager.redo();

        let val = manager.get("x").unwrap();
        assert!(matches!(val, StateValue::Integer(2)));
    }

    #[test]
    fn test_rollback_to() {
        let mut manager = RollbackManager::default();

        manager.set("x", StateValue::Integer(1));
        let cp = manager.checkpoint("before_change");

        manager.set("x", StateValue::Integer(2));
        manager.set("x", StateValue::Integer(3));

        let result = manager.rollback_to(cp);
        assert!(result.success);
        assert_eq!(result.rolled_back.len(), 2);

        let val = manager.get("x").unwrap();
        assert!(matches!(val, StateValue::Integer(1)));
    }

    #[test]
    fn test_rollback_n() {
        let mut manager = RollbackManager::default();

        manager.set("x", StateValue::Integer(1));
        manager.set("x", StateValue::Integer(2));
        manager.set("x", StateValue::Integer(3));

        let result = manager.rollback_n(2);
        assert!(result.success);
        assert_eq!(result.rolled_back.len(), 2);
    }
}

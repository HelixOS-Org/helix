//! Effector — Action execution interface
//!
//! Effectors are the components that actually execute kernel actions.
//! Each effector handles a specific subset of action types.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::effect::Change;
use super::transaction::{RollbackState, TransactionId};
use crate::types::*;
// ActionParameters, ActionTarget not in types - only ActionType available
// use crate::decide::{ActionParameters, ActionTarget, ActionType};

// ============================================================================
// EFFECTOR TRAIT
// ============================================================================

/// Effector trait - executes actual kernel actions
pub trait Effector: Send + Sync {
    /// Get effector ID
    fn id(&self) -> EffectorId;

    /// Get effector name
    fn name(&self) -> &str;

    /// Action types this effector can handle
    fn handles(&self) -> &[ActionType];

    /// Execute an action
    fn execute(
        &mut self,
        action_type: ActionType,
        target: &ActionTarget,
        parameters: &ActionParameters,
        tx: Option<TransactionId>,
    ) -> EffectorResult;

    /// Rollback an action
    fn rollback(&mut self, state: &RollbackState) -> EffectorResult;

    /// Check if action is supported
    fn supports(&self, action_type: ActionType, target: &ActionTarget) -> bool;
}

// ============================================================================
// EFFECTOR RESULT
// ============================================================================

/// Effector result
#[derive(Debug, Clone)]
pub struct EffectorResult {
    /// Success
    pub success: bool,
    /// Changes made
    pub changes: Vec<Change>,
    /// Error message
    pub error: Option<String>,
    /// Duration
    pub duration: Duration,
}

impl EffectorResult {
    /// Success result
    pub fn success(changes: Vec<Change>, duration: Duration) -> Self {
        Self {
            success: true,
            changes,
            error: None,
            duration,
        }
    }

    /// Failure result
    pub fn failure(message: impl Into<String>, duration: Duration) -> Self {
        Self {
            success: false,
            changes: Vec::new(),
            error: Some(message.into()),
            duration,
        }
    }

    /// Empty success
    pub fn ok() -> Self {
        Self {
            success: true,
            changes: Vec::new(),
            error: None,
            duration: Duration::ZERO,
        }
    }

    /// Has changes?
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }
}

// ============================================================================
// EFFECTOR REGISTRY
// ============================================================================

/// Effector registry
pub struct EffectorRegistry {
    /// Registered effectors
    effectors: BTreeMap<EffectorId, Box<dyn Effector>>,
    /// Effectors by action type
    by_action: BTreeMap<ActionType, Vec<EffectorId>>,
}

impl EffectorRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            effectors: BTreeMap::new(),
            by_action: BTreeMap::new(),
        }
    }

    /// Create with default effectors
    pub fn with_defaults() -> Self {
        use super::effectors::*;

        let mut registry = Self::new();
        registry.register(Box::new(NoOpEffector::new()));
        registry.register(Box::new(ProcessEffector::new()));
        registry.register(Box::new(MemoryEffector::new()));
        registry
    }

    /// Register an effector
    pub fn register(&mut self, effector: Box<dyn Effector>) {
        let id = effector.id();
        let handles = effector.handles().to_vec();

        for action_type in handles {
            self.by_action.entry(action_type).or_default().push(id);
        }

        self.effectors.insert(id, effector);
    }

    /// Unregister an effector
    pub fn unregister(&mut self, id: EffectorId) -> Option<Box<dyn Effector>> {
        if let Some(effector) = self.effectors.remove(&id) {
            // Remove from action type mappings
            for ids in self.by_action.values_mut() {
                ids.retain(|&eid| eid != id);
            }
            Some(effector)
        } else {
            None
        }
    }

    /// Find effector for action
    pub fn find(&self, action_type: ActionType, target: &ActionTarget) -> Option<EffectorId> {
        self.by_action
            .get(&action_type)?
            .iter()
            .find(|&id| {
                self.effectors
                    .get(id)
                    .map(|e| e.supports(action_type, target))
                    .unwrap_or(false)
            })
            .copied()
    }

    /// Find all effectors for action type
    pub fn find_all(&self, action_type: ActionType) -> Vec<EffectorId> {
        self.by_action
            .get(&action_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Get effector by ID
    pub fn get(&self, id: EffectorId) -> Option<&dyn Effector> {
        self.effectors.get(&id).map(|e| e.as_ref())
    }

    /// Get mutable effector by ID
    pub fn get_mut(&mut self, id: EffectorId) -> Option<&mut (dyn Effector + 'static)> {
        self.effectors.get_mut(&id).map(|e| &mut **e)
    }

    /// Count effectors
    pub fn count(&self) -> usize {
        self.effectors.len()
    }

    /// List all effector IDs
    pub fn list(&self) -> impl Iterator<Item = EffectorId> + '_ {
        self.effectors.keys().copied()
    }

    /// Get effector names
    pub fn names(&self) -> Vec<&str> {
        self.effectors.values().map(|e| e.name()).collect()
    }
}

impl Default for EffectorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effector_result() {
        let result = EffectorResult::success(Vec::new(), Duration::from_millis(10));
        assert!(result.success);
        assert!(!result.has_changes());
    }

    #[test]
    fn test_effector_failure() {
        let result = EffectorResult::failure("Error", Duration::ZERO);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_effector_registry() {
        let registry = EffectorRegistry::with_defaults();
        assert!(registry.count() > 0);

        let effector = registry.find(ActionType::NoOp, &ActionTarget::System);
        assert!(effector.is_some());
    }
}

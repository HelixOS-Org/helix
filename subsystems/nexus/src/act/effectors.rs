//! Effector Implementations
//!
//! Concrete effector implementations for various action types.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::effect::{Change, ChangeId, ChangeType, ChangeValue};
use super::effector::{Effector, EffectorId, EffectorResult};
use super::limiter::target_to_string;
use super::transaction::{RollbackState, TransactionId};
use crate::decide::{ActionParameters, ActionTarget, ActionType};
use crate::types::*;

// ============================================================================
// NOOP EFFECTOR
// ============================================================================

/// NoOp effector - handles do-nothing actions
pub struct NoOpEffector {
    id: EffectorId,
}

impl NoOpEffector {
    /// Create new NoOp effector
    pub fn new() -> Self {
        Self {
            id: EffectorId::generate(),
        }
    }
}

impl Effector for NoOpEffector {
    fn id(&self) -> EffectorId {
        self.id
    }

    fn name(&self) -> &str {
        "noop_effector"
    }

    fn handles(&self) -> &[ActionType] {
        &[ActionType::NoOp, ActionType::Log, ActionType::Alert]
    }

    fn execute(
        &mut self,
        action_type: ActionType,
        _target: &ActionTarget,
        _parameters: &ActionParameters,
        _tx: Option<TransactionId>,
    ) -> EffectorResult {
        match action_type {
            ActionType::NoOp => EffectorResult::success(Vec::new(), Duration::ZERO),
            ActionType::Log => EffectorResult::success(
                vec![Change {
                    id: ChangeId::generate(),
                    target: String::from("log"),
                    change_type: ChangeType::Create,
                    previous_value: None,
                    new_value: ChangeValue::String(String::from("logged")),
                    reversible: false,
                }],
                Duration::from_micros(10),
            ),
            ActionType::Alert => EffectorResult::success(
                vec![Change {
                    id: ChangeId::generate(),
                    target: String::from("alert"),
                    change_type: ChangeType::Create,
                    previous_value: None,
                    new_value: ChangeValue::Boolean(true),
                    reversible: false,
                }],
                Duration::from_micros(100),
            ),
            _ => EffectorResult::failure("Unsupported action", Duration::ZERO),
        }
    }

    fn rollback(&mut self, _state: &RollbackState) -> EffectorResult {
        EffectorResult::success(Vec::new(), Duration::ZERO)
    }

    fn supports(&self, action_type: ActionType, _target: &ActionTarget) -> bool {
        self.handles().contains(&action_type)
    }
}

impl Default for NoOpEffector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PROCESS EFFECTOR
// ============================================================================

/// Process effector - handles process-related actions
pub struct ProcessEffector {
    id: EffectorId,
    actions_executed: AtomicU64,
}

impl ProcessEffector {
    /// Create new process effector
    pub fn new() -> Self {
        Self {
            id: EffectorId::generate(),
            actions_executed: AtomicU64::new(0),
        }
    }

    /// Get actions executed count
    pub fn actions_executed(&self) -> u64 {
        self.actions_executed.load(Ordering::Relaxed)
    }
}

impl Effector for ProcessEffector {
    fn id(&self) -> EffectorId {
        self.id
    }

    fn name(&self) -> &str {
        "process_effector"
    }

    fn handles(&self) -> &[ActionType] {
        &[ActionType::Restart, ActionType::Kill, ActionType::Throttle]
    }

    fn execute(
        &mut self,
        action_type: ActionType,
        target: &ActionTarget,
        _parameters: &ActionParameters,
        _tx: Option<TransactionId>,
    ) -> EffectorResult {
        self.actions_executed.fetch_add(1, Ordering::Relaxed);
        let start = Timestamp::now();

        let changes = match action_type {
            ActionType::Kill => {
                if let ActionTarget::Process(pid) = target {
                    vec![Change {
                        id: ChangeId::generate(),
                        target: format!("process:{}", pid),
                        change_type: ChangeType::Delete,
                        previous_value: Some(ChangeValue::State(1)), // Running
                        new_value: ChangeValue::State(0),            // Terminated
                        reversible: false,
                    }]
                } else {
                    return EffectorResult::failure(
                        "Kill requires process target",
                        Timestamp::now().elapsed_since(start),
                    );
                }
            },
            ActionType::Restart => {
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::StateChange,
                    previous_value: Some(ChangeValue::State(1)),
                    new_value: ChangeValue::State(1),
                    reversible: true,
                }]
            },
            ActionType::Throttle => {
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::Modify,
                    previous_value: Some(ChangeValue::Integer(100)),
                    new_value: ChangeValue::Integer(50),
                    reversible: true,
                }]
            },
            _ => Vec::new(),
        };

        EffectorResult::success(changes, Timestamp::now().elapsed_since(start))
    }

    fn rollback(&mut self, _state: &RollbackState) -> EffectorResult {
        EffectorResult::success(Vec::new(), Duration::ZERO)
    }

    fn supports(&self, action_type: ActionType, target: &ActionTarget) -> bool {
        self.handles().contains(&action_type)
            && matches!(target, ActionTarget::Process(_) | ActionTarget::System)
    }
}

impl Default for ProcessEffector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MEMORY EFFECTOR
// ============================================================================

/// Memory effector - handles memory-related actions
pub struct MemoryEffector {
    id: EffectorId,
    actions_executed: AtomicU64,
}

impl MemoryEffector {
    /// Create new memory effector
    pub fn new() -> Self {
        Self {
            id: EffectorId::generate(),
            actions_executed: AtomicU64::new(0),
        }
    }

    /// Get actions executed count
    pub fn actions_executed(&self) -> u64 {
        self.actions_executed.load(Ordering::Relaxed)
    }
}

impl Effector for MemoryEffector {
    fn id(&self) -> EffectorId {
        self.id
    }

    fn name(&self) -> &str {
        "memory_effector"
    }

    fn handles(&self) -> &[ActionType] {
        &[
            ActionType::Allocate,
            ActionType::Deallocate,
            ActionType::Repair,
        ]
    }

    fn execute(
        &mut self,
        action_type: ActionType,
        target: &ActionTarget,
        parameters: &ActionParameters,
        _tx: Option<TransactionId>,
    ) -> EffectorResult {
        self.actions_executed.fetch_add(1, Ordering::Relaxed);
        let start = Timestamp::now();

        let changes = match action_type {
            ActionType::Allocate => {
                let size = parameters.get_int("size").unwrap_or(4096);
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::Allocate,
                    previous_value: None,
                    new_value: ChangeValue::Integer(size),
                    reversible: true,
                }]
            },
            ActionType::Deallocate => {
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::Deallocate,
                    previous_value: Some(ChangeValue::Integer(4096)),
                    new_value: ChangeValue::None,
                    reversible: false,
                }]
            },
            ActionType::Repair => {
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::Modify,
                    previous_value: Some(ChangeValue::String(String::from("corrupted"))),
                    new_value: ChangeValue::String(String::from("repaired")),
                    reversible: false,
                }]
            },
            _ => Vec::new(),
        };

        EffectorResult::success(changes, Timestamp::now().elapsed_since(start))
    }

    fn rollback(&mut self, _state: &RollbackState) -> EffectorResult {
        EffectorResult::success(Vec::new(), Duration::ZERO)
    }

    fn supports(&self, action_type: ActionType, target: &ActionTarget) -> bool {
        self.handles().contains(&action_type)
            && matches!(target, ActionTarget::Memory { .. } | ActionTarget::System)
    }
}

impl Default for MemoryEffector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MODULE EFFECTOR
// ============================================================================

/// Module effector - handles module-related actions
pub struct ModuleEffector {
    id: EffectorId,
    actions_executed: AtomicU64,
}

impl ModuleEffector {
    /// Create new module effector
    pub fn new() -> Self {
        Self {
            id: EffectorId::generate(),
            actions_executed: AtomicU64::new(0),
        }
    }
}

impl Effector for ModuleEffector {
    fn id(&self) -> EffectorId {
        self.id
    }

    fn name(&self) -> &str {
        "module_effector"
    }

    fn handles(&self) -> &[ActionType] {
        &[
            ActionType::Enable,
            ActionType::Disable,
            ActionType::Reconfigure,
        ]
    }

    fn execute(
        &mut self,
        action_type: ActionType,
        target: &ActionTarget,
        _parameters: &ActionParameters,
        _tx: Option<TransactionId>,
    ) -> EffectorResult {
        self.actions_executed.fetch_add(1, Ordering::Relaxed);
        let start = Timestamp::now();

        let changes = match action_type {
            ActionType::Enable => {
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::StateChange,
                    previous_value: Some(ChangeValue::Boolean(false)),
                    new_value: ChangeValue::Boolean(true),
                    reversible: true,
                }]
            },
            ActionType::Disable => {
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::StateChange,
                    previous_value: Some(ChangeValue::Boolean(true)),
                    new_value: ChangeValue::Boolean(false),
                    reversible: true,
                }]
            },
            ActionType::Reconfigure => {
                vec![Change {
                    id: ChangeId::generate(),
                    target: target_to_string(target),
                    change_type: ChangeType::Modify,
                    previous_value: None,
                    new_value: ChangeValue::String(String::from("reconfigured")),
                    reversible: true,
                }]
            },
            _ => Vec::new(),
        };

        EffectorResult::success(changes, Timestamp::now().elapsed_since(start))
    }

    fn rollback(&mut self, _state: &RollbackState) -> EffectorResult {
        EffectorResult::success(Vec::new(), Duration::ZERO)
    }

    fn supports(&self, action_type: ActionType, target: &ActionTarget) -> bool {
        self.handles().contains(&action_type)
            && matches!(target, ActionTarget::Module(_) | ActionTarget::System)
    }
}

impl Default for ModuleEffector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_effector() {
        let mut effector = NoOpEffector::new();
        let result = effector.execute(
            ActionType::NoOp,
            &ActionTarget::System,
            &ActionParameters::new(),
            None,
        );
        assert!(result.success);
    }

    #[test]
    fn test_process_effector() {
        let mut effector = ProcessEffector::new();
        let result = effector.execute(
            ActionType::Restart,
            &ActionTarget::System,
            &ActionParameters::new(),
            None,
        );
        assert!(result.success);
        assert_eq!(effector.actions_executed(), 1);
    }

    #[test]
    fn test_memory_effector() {
        let mut effector = MemoryEffector::new();
        let mut params = ActionParameters::new();
        params.set_int("size", 8192);

        let result = effector.execute(ActionType::Allocate, &ActionTarget::System, &params, None);
        assert!(result.success);
        assert!(result.has_changes());
    }

    #[test]
    fn test_module_effector() {
        let mut effector = ModuleEffector::new();
        let result = effector.execute(
            ActionType::Enable,
            &ActionTarget::Module(String::from("test")),
            &ActionParameters::new(),
            None,
        );
        assert!(result.success);
    }
}

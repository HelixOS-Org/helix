//! # Cognitive Transition Manager
//!
//! State transition management for cognitive systems.
//! Handles complex multi-phase transitions with rollback support.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// TRANSITION TYPES
// ============================================================================

/// A state transition definition
#[derive(Debug, Clone)]
pub struct Transition<S: Clone + Eq> {
    /// Transition ID
    pub id: u64,
    /// From state
    pub from: S,
    /// To state
    pub to: S,
    /// Conditions required
    pub conditions: Vec<TransitionCondition>,
    /// Actions to execute
    pub actions: Vec<TransitionAction>,
    /// Rollback actions
    pub rollback: Vec<TransitionAction>,
    /// Priority
    pub priority: u32,
    /// Timeout (ns)
    pub timeout_ns: Option<u64>,
}

/// Transition condition
#[derive(Debug, Clone)]
pub struct TransitionCondition {
    /// Condition name
    pub name: String,
    /// Condition type
    pub condition_type: ConditionType,
}

/// Condition type
#[derive(Debug, Clone)]
pub enum ConditionType {
    /// Boolean flag must be true
    Flag(String),
    /// Metric must be above threshold
    MetricAbove(String, f64),
    /// Metric must be below threshold
    MetricBelow(String, f64),
    /// Time elapsed since event
    TimeElapsed(u64),
    /// Custom predicate
    Custom(String),
    /// All of these conditions
    All(Vec<ConditionType>),
    /// Any of these conditions
    Any(Vec<ConditionType>),
}

/// Transition action
#[derive(Debug, Clone)]
pub struct TransitionAction {
    /// Action name
    pub name: String,
    /// Action type
    pub action_type: ActionType,
    /// Required for transition
    pub required: bool,
}

/// Action type
#[derive(Debug, Clone)]
pub enum ActionType {
    /// Set a flag
    SetFlag(String, bool),
    /// Emit an event
    EmitEvent(String),
    /// Call a handler
    CallHandler(String),
    /// Wait for duration (ns)
    Wait(u64),
    /// Notify domain
    Notify(DomainId),
    /// Custom action
    Custom(String),
}

// ============================================================================
// TRANSITION EXECUTION
// ============================================================================

/// Active transition execution
#[derive(Debug)]
pub struct TransitionExecution<S: Clone + Eq> {
    /// Execution ID
    pub id: u64,
    /// Transition being executed
    pub transition: Transition<S>,
    /// Phase
    pub phase: ExecutionPhase,
    /// Actions completed
    pub actions_completed: Vec<usize>,
    /// Current action index
    pub current_action: usize,
    /// Started time
    pub started: Timestamp,
    /// Error if failed
    pub error: Option<String>,
    /// Context data
    pub context: BTreeMap<String, String>,
}

/// Execution phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPhase {
    /// Checking conditions
    Checking,
    /// Executing actions
    Executing,
    /// Committing transition
    Committing,
    /// Completed successfully
    Completed,
    /// Rolling back
    RollingBack,
    /// Failed
    Failed,
}

impl<S: Clone + Eq> TransitionExecution<S> {
    /// Create new execution
    pub fn new(id: u64, transition: Transition<S>) -> Self {
        Self {
            id,
            transition,
            phase: ExecutionPhase::Checking,
            actions_completed: Vec::new(),
            current_action: 0,
            started: Timestamp::now(),
            error: None,
            context: BTreeMap::new(),
        }
    }

    /// Check if timed out
    #[inline]
    pub fn is_timed_out(&self) -> bool {
        if let Some(timeout) = self.transition.timeout_ns {
            let now = Timestamp::now();
            now.elapsed_since(self.started) > timeout
        } else {
            false
        }
    }

    /// Get progress
    #[inline]
    pub fn progress(&self) -> f64 {
        if self.transition.actions.is_empty() {
            1.0
        } else {
            self.actions_completed.len() as f64 / self.transition.actions.len() as f64
        }
    }
}

// ============================================================================
// STATE MACHINE
// ============================================================================

/// State machine with transition management
#[repr(align(64))]
pub struct StateMachine<S: Clone + Eq + Ord> {
    /// Current state
    current_state: S,
    /// Available transitions
    transitions: BTreeMap<u64, Transition<S>>,
    /// Active executions
    executions: BTreeMap<u64, TransitionExecution<S>>,
    /// State history
    history: VecDeque<StateChange<S>>,
    /// Next transition ID
    next_transition_id: AtomicU64,
    /// Next execution ID
    next_execution_id: AtomicU64,
    /// Flags
    flags: BTreeMap<String, bool>,
    /// Metrics
    metrics: BTreeMap<String, f64>,
    /// Configuration
    config: StateMachineConfig,
    /// Statistics
    stats: StateMachineStats,
}

/// State change record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StateChange<S: Clone> {
    /// From state
    pub from: S,
    /// To state
    pub to: S,
    /// Transition ID
    pub transition_id: u64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Duration (ns)
    pub duration_ns: u64,
}

/// Configuration
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StateMachineConfig {
    /// Maximum history
    pub max_history: usize,
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    /// Default timeout (ns)
    pub default_timeout_ns: u64,
    /// Enable auto-rollback on failure
    pub auto_rollback: bool,
}

impl Default for StateMachineConfig {
    fn default() -> Self {
        Self {
            max_history: 1000,
            max_concurrent: 10,
            default_timeout_ns: 30_000_000_000, // 30 seconds
            auto_rollback: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct StateMachineStats {
    /// Total transitions
    pub total_transitions: u64,
    /// Successful transitions
    pub successful: u64,
    /// Failed transitions
    pub failed: u64,
    /// Rollbacks
    pub rollbacks: u64,
    /// Average transition time (ns)
    pub avg_duration_ns: f64,
}

impl<S: Clone + Eq + Ord + core::fmt::Debug> StateMachine<S> {
    /// Create new state machine
    pub fn new(initial_state: S, config: StateMachineConfig) -> Self {
        Self {
            current_state: initial_state,
            transitions: BTreeMap::new(),
            executions: BTreeMap::new(),
            history: VecDeque::new(),
            next_transition_id: AtomicU64::new(1),
            next_execution_id: AtomicU64::new(1),
            flags: BTreeMap::new(),
            metrics: BTreeMap::new(),
            config,
            stats: StateMachineStats::default(),
        }
    }

    /// Get current state
    #[inline(always)]
    pub fn state(&self) -> &S {
        &self.current_state
    }

    /// Add transition
    pub fn add_transition(
        &mut self,
        from: S,
        to: S,
        conditions: Vec<TransitionCondition>,
        actions: Vec<TransitionAction>,
        rollback: Vec<TransitionAction>,
    ) -> u64 {
        let id = self.next_transition_id.fetch_add(1, Ordering::Relaxed);

        let transition = Transition {
            id,
            from,
            to,
            conditions,
            actions,
            rollback,
            priority: 0,
            timeout_ns: Some(self.config.default_timeout_ns),
        };

        self.transitions.insert(id, transition);
        id
    }

    /// Set flag
    #[inline(always)]
    pub fn set_flag(&mut self, name: &str, value: bool) {
        self.flags.insert(name.into(), value);
    }

    /// Get flag
    #[inline(always)]
    pub fn get_flag(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }

    /// Set metric
    #[inline(always)]
    pub fn set_metric(&mut self, name: &str, value: f64) {
        self.metrics.insert(name.into(), value);
    }

    /// Get metric
    #[inline(always)]
    pub fn get_metric(&self, name: &str) -> f64 {
        self.metrics.get(name).copied().unwrap_or(0.0)
    }

    /// Check condition
    fn check_condition(&self, condition: &ConditionType) -> bool {
        match condition {
            ConditionType::Flag(name) => self.get_flag(name),
            ConditionType::MetricAbove(name, threshold) => self.get_metric(name) > *threshold,
            ConditionType::MetricBelow(name, threshold) => self.get_metric(name) < *threshold,
            ConditionType::TimeElapsed(_) => true, // Would need event tracking
            ConditionType::Custom(_) => true,
            ConditionType::All(conditions) => conditions.iter().all(|c| self.check_condition(c)),
            ConditionType::Any(conditions) => conditions.iter().any(|c| self.check_condition(c)),
        }
    }

    /// Get valid transitions from current state
    #[inline]
    pub fn available_transitions(&self) -> Vec<&Transition<S>> {
        self.transitions
            .values()
            .filter(|t| t.from == self.current_state)
            .filter(|t| {
                t.conditions
                    .iter()
                    .all(|c| self.check_condition(&c.condition_type))
            })
            .collect()
    }

    /// Start a transition
    pub fn start_transition(&mut self, transition_id: u64) -> Result<u64, &'static str> {
        let transition = self
            .transitions
            .get(&transition_id)
            .ok_or("Unknown transition")?
            .clone();

        if transition.from != self.current_state {
            return Err("Invalid source state");
        }

        // Check conditions
        for condition in &transition.conditions {
            if !self.check_condition(&condition.condition_type) {
                return Err("Conditions not met");
            }
        }

        // Check concurrent limit
        if self.executions.len() >= self.config.max_concurrent {
            return Err("Too many concurrent executions");
        }

        let exec_id = self.next_execution_id.fetch_add(1, Ordering::Relaxed);
        let execution = TransitionExecution::new(exec_id, transition);

        self.executions.insert(exec_id, execution);
        self.stats.total_transitions += 1;

        Ok(exec_id)
    }

    /// Step execution forward
    pub fn step_execution(&mut self, exec_id: u64) -> Result<ExecutionPhase, &'static str> {
        let execution = self
            .executions
            .get_mut(&exec_id)
            .ok_or("Unknown execution")?;

        // Check timeout
        if execution.is_timed_out() {
            execution.phase = ExecutionPhase::Failed;
            execution.error = Some("Timeout".into());

            if self.config.auto_rollback {
                execution.phase = ExecutionPhase::RollingBack;
            }

            return Ok(execution.phase);
        }

        match execution.phase {
            ExecutionPhase::Checking => {
                // Verify conditions still hold
                for condition in &execution.transition.conditions {
                    if !self.check_condition(&condition.condition_type) {
                        execution.phase = ExecutionPhase::Failed;
                        execution.error = Some("Condition no longer met".into());
                        return Ok(execution.phase);
                    }
                }
                execution.phase = ExecutionPhase::Executing;
            },
            ExecutionPhase::Executing => {
                // Execute next action
                if execution.current_action < execution.transition.actions.len() {
                    // Simulate action execution
                    let action = &execution.transition.actions[execution.current_action];

                    match &action.action_type {
                        ActionType::SetFlag(name, value) => {
                            self.flags.insert(name.clone(), *value);
                        },
                        ActionType::EmitEvent(_) => {
                            // Event emission would be handled externally
                        },
                        _ => {},
                    }

                    execution.actions_completed.push(execution.current_action);
                    execution.current_action += 1;
                } else {
                    execution.phase = ExecutionPhase::Committing;
                }
            },
            ExecutionPhase::Committing => {
                // Commit the transition
                let from = execution.transition.from.clone();
                let to = execution.transition.to.clone();
                let duration = Timestamp::now().elapsed_since(execution.started);

                self.current_state = to.clone();
                execution.phase = ExecutionPhase::Completed;

                // Record history
                if self.history.len() >= self.config.max_history {
                    self.history.pop_front();
                }
                self.history.push_back(StateChange {
                    from,
                    to,
                    transition_id: execution.transition.id,
                    timestamp: Timestamp::now(),
                    duration_ns: duration,
                });

                self.stats.successful += 1;
                self.stats.avg_duration_ns = (self.stats.avg_duration_ns
                    * (self.stats.successful - 1) as f64
                    + duration as f64)
                    / self.stats.successful as f64;
            },
            ExecutionPhase::RollingBack => {
                // Execute rollback actions in reverse
                let rollback_index = execution.actions_completed.len();
                if rollback_index > 0 {
                    let action_index = execution.actions_completed.pop().unwrap();
                    if action_index < execution.transition.rollback.len() {
                        let action = &execution.transition.rollback[action_index];

                        match &action.action_type {
                            ActionType::SetFlag(name, value) => {
                                self.flags.insert(name.clone(), !*value);
                            },
                            _ => {},
                        }
                    }
                } else {
                    execution.phase = ExecutionPhase::Failed;
                    self.stats.rollbacks += 1;
                }
            },
            ExecutionPhase::Completed | ExecutionPhase::Failed => {
                // Terminal states
            },
        }

        Ok(execution.phase)
    }

    /// Complete execution (run all steps)
    pub fn complete_execution(&mut self, exec_id: u64) -> Result<bool, &'static str> {
        loop {
            let phase = self.step_execution(exec_id)?;
            match phase {
                ExecutionPhase::Completed => return Ok(true),
                ExecutionPhase::Failed => {
                    self.stats.failed += 1;
                    return Ok(false);
                },
                _ => continue,
            }
        }
    }

    /// Trigger transition by finding matching one
    pub fn trigger(&mut self, target: &S) -> Result<u64, &'static str> {
        let transition = self
            .transitions
            .values()
            .find(|t| t.from == self.current_state && t.to == *target)
            .ok_or("No transition to target state")?
            .id;

        let exec_id = self.start_transition(transition)?;
        self.complete_execution(exec_id)?;
        Ok(exec_id)
    }

    /// Get execution
    #[inline(always)]
    pub fn get_execution(&self, id: u64) -> Option<&TransitionExecution<S>> {
        self.executions.get(&id)
    }

    /// Get history
    #[inline(always)]
    pub fn history(&self) -> &[StateChange<S>] {
        &self.history
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &StateMachineStats {
        &self.stats
    }

    /// Cleanup completed/failed executions
    #[inline]
    pub fn cleanup(&mut self) {
        self.executions.retain(|_, e| {
            e.phase != ExecutionPhase::Completed && e.phase != ExecutionPhase::Failed
        });
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum TestState {
        Initial,
        Running,
        Stopped,
    }

    #[test]
    fn test_state_machine() {
        let mut sm = StateMachine::new(TestState::Initial, StateMachineConfig::default());

        sm.add_transition(
            TestState::Initial,
            TestState::Running,
            vec![],
            vec![],
            vec![],
        );

        sm.add_transition(
            TestState::Running,
            TestState::Stopped,
            vec![],
            vec![],
            vec![],
        );

        assert_eq!(*sm.state(), TestState::Initial);

        sm.trigger(&TestState::Running).unwrap();
        assert_eq!(*sm.state(), TestState::Running);

        sm.trigger(&TestState::Stopped).unwrap();
        assert_eq!(*sm.state(), TestState::Stopped);
    }

    #[test]
    fn test_conditions() {
        let mut sm = StateMachine::new(TestState::Initial, StateMachineConfig::default());

        sm.add_transition(
            TestState::Initial,
            TestState::Running,
            vec![TransitionCondition {
                name: "ready".into(),
                condition_type: ConditionType::Flag("ready".into()),
            }],
            vec![],
            vec![],
        );

        // Should fail - condition not met
        assert!(sm.trigger(&TestState::Running).is_err());

        // Set flag
        sm.set_flag("ready", true);

        // Should succeed now
        sm.trigger(&TestState::Running).unwrap();
        assert_eq!(*sm.state(), TestState::Running);
    }

    #[test]
    fn test_history() {
        let mut sm = StateMachine::new(TestState::Initial, StateMachineConfig::default());

        sm.add_transition(
            TestState::Initial,
            TestState::Running,
            vec![],
            vec![],
            vec![],
        );
        sm.add_transition(
            TestState::Running,
            TestState::Stopped,
            vec![],
            vec![],
            vec![],
        );

        sm.trigger(&TestState::Running).unwrap();
        sm.trigger(&TestState::Stopped).unwrap();

        let history = sm.history();
        assert_eq!(history.len(), 2);
    }
}

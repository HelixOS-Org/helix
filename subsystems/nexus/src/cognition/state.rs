//! # Cognitive State Machine
//!
//! Manages the state of cognitive domains and transitions.
//! Provides state persistence and recovery.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// STATE TYPES
// ============================================================================

/// Cognitive state
#[derive(Debug, Clone)]
pub struct CognitiveState {
    /// State ID
    pub id: u64,
    /// State name
    pub name: String,
    /// State type
    pub state_type: StateType,
    /// State data
    pub data: StateData,
    /// Entry conditions
    pub entry_conditions: Vec<StateCondition>,
    /// Exit conditions
    pub exit_conditions: Vec<StateCondition>,
    /// Transitions
    pub transitions: Vec<StateTransition>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// State type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateType {
    /// Initial state
    Initial,
    /// Normal processing state
    Normal,
    /// Transient state
    Transient,
    /// Final state
    Final,
    /// Error state
    Error,
    /// Recovery state
    Recovery,
}

/// State data
#[derive(Debug, Clone)]
pub enum StateData {
    /// Empty state
    Empty,
    /// Numeric values
    Numeric(BTreeMap<String, f64>),
    /// String values
    Text(BTreeMap<String, String>),
    /// Mixed values
    Mixed(BTreeMap<String, StateValue>),
    /// Binary data
    Binary(Vec<u8>),
}

/// State value
#[derive(Debug, Clone)]
pub enum StateValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    Array(Vec<StateValue>),
}

/// State condition
#[derive(Debug, Clone)]
pub struct StateCondition {
    /// Condition ID
    pub id: u64,
    /// Variable to check
    pub variable: String,
    /// Operator
    pub operator: ConditionOperator,
    /// Value to compare
    pub value: StateValue,
}

/// Condition operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOperator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Contains,
    StartsWith,
    EndsWith,
}

/// State transition
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// Transition ID
    pub id: u64,
    /// Source state
    pub from: u64,
    /// Target state
    pub to: u64,
    /// Trigger event
    pub trigger: TransitionTrigger,
    /// Guard conditions
    pub guards: Vec<StateCondition>,
    /// Actions to execute
    pub actions: Vec<TransitionAction>,
    /// Priority
    pub priority: u32,
}

/// Transition trigger
#[derive(Debug, Clone)]
pub enum TransitionTrigger {
    /// Event trigger
    Event(String),
    /// Timeout trigger
    Timeout(u64),
    /// Condition trigger
    Condition(StateCondition),
    /// Always (auto-transition)
    Auto,
}

/// Transition action
#[derive(Debug, Clone)]
pub struct TransitionAction {
    /// Action type
    pub action_type: ActionType,
    /// Parameters
    pub params: BTreeMap<String, String>,
}

/// Action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    /// Set variable
    SetVariable,
    /// Emit event
    EmitEvent,
    /// Log message
    Log,
    /// Call function
    Call,
    /// Reset timer
    ResetTimer,
}

// ============================================================================
// STATE MACHINE
// ============================================================================

/// State machine for a cognitive domain
pub struct StateMachine {
    /// Machine ID
    id: u64,
    /// Domain ID
    domain_id: DomainId,
    /// All states
    states: BTreeMap<u64, CognitiveState>,
    /// Current state ID
    current_state: u64,
    /// Previous state ID
    previous_state: Option<u64>,
    /// Variables
    variables: BTreeMap<String, StateValue>,
    /// Timers
    timers: BTreeMap<String, Timer>,
    /// History
    history: Vec<StateHistoryEntry>,
    /// Configuration
    config: StateMachineConfig,
    /// Statistics
    stats: StateMachineStats,
}

/// Timer
#[derive(Debug, Clone)]
pub struct Timer {
    /// Timer name
    pub name: String,
    /// Start time
    pub started: Timestamp,
    /// Duration
    pub duration: u64,
    /// Repeating
    pub repeating: bool,
}

/// History entry
#[derive(Debug, Clone)]
pub struct StateHistoryEntry {
    /// Timestamp
    pub timestamp: Timestamp,
    /// From state
    pub from: u64,
    /// To state
    pub to: u64,
    /// Trigger
    pub trigger: String,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct StateMachineConfig {
    /// Maximum history entries
    pub max_history: usize,
    /// Enable auto-recovery
    pub auto_recovery: bool,
    /// Transition timeout
    pub transition_timeout_ns: u64,
}

impl Default for StateMachineConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            auto_recovery: true,
            transition_timeout_ns: 1_000_000_000, // 1 second
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct StateMachineStats {
    /// Total transitions
    pub total_transitions: u64,
    /// Successful transitions
    pub successful: u64,
    /// Failed transitions
    pub failed: u64,
    /// Error recoveries
    pub recoveries: u64,
    /// Time in current state
    pub time_in_state_ns: u64,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new(id: u64, domain_id: DomainId, config: StateMachineConfig) -> Self {
        Self {
            id,
            domain_id,
            states: BTreeMap::new(),
            current_state: 0,
            previous_state: None,
            variables: BTreeMap::new(),
            timers: BTreeMap::new(),
            history: Vec::new(),
            config,
            stats: StateMachineStats::default(),
        }
    }

    /// Add a state
    pub fn add_state(&mut self, state: CognitiveState) -> u64 {
        let id = state.id;
        self.states.insert(id, state);
        id
    }

    /// Get current state
    pub fn current_state(&self) -> Option<&CognitiveState> {
        self.states.get(&self.current_state)
    }

    /// Get state by ID
    pub fn get_state(&self, id: u64) -> Option<&CognitiveState> {
        self.states.get(&id)
    }

    /// Set initial state
    pub fn set_initial_state(&mut self, state_id: u64) {
        self.current_state = state_id;
    }

    /// Set variable
    pub fn set_variable(&mut self, name: &str, value: StateValue) {
        self.variables.insert(name.into(), value);
    }

    /// Get variable
    pub fn get_variable(&self, name: &str) -> Option<&StateValue> {
        self.variables.get(name)
    }

    /// Trigger an event
    pub fn trigger_event(&mut self, event: &str) -> Result<bool, String> {
        let current = self.current_state;
        let state = match self.states.get(&current) {
            Some(s) => s,
            None => return Err("No current state".into()),
        };

        // Find matching transitions
        let mut matching: Vec<_> = state
            .transitions
            .iter()
            .filter(|t| {
                if let TransitionTrigger::Event(ref e) = t.trigger {
                    e == event && self.check_guards(&t.guards)
                } else {
                    false
                }
            })
            .collect();

        // Sort by priority
        matching.sort_by_key(|t| core::cmp::Reverse(t.priority));

        if let Some(transition) = matching.first() {
            self.execute_transition(transition.clone())
        } else {
            Ok(false)
        }
    }

    /// Execute a transition
    fn execute_transition(&mut self, transition: &StateTransition) -> Result<bool, String> {
        self.stats.total_transitions += 1;

        // Execute entry actions of target state
        let target = match self.states.get(&transition.to) {
            Some(s) => s.clone(),
            None => {
                self.stats.failed += 1;
                return Err("Target state not found".into());
            },
        };

        // Check entry conditions
        for condition in &target.entry_conditions {
            if !self.check_condition(condition) {
                self.stats.failed += 1;
                return Err("Entry condition not met".into());
            }
        }

        // Execute transition actions
        for action in &transition.actions {
            self.execute_action(action)?;
        }

        // Update state
        let from = self.current_state;
        self.previous_state = Some(from);
        self.current_state = transition.to;

        // Record history
        if self.history.len() >= self.config.max_history {
            self.history.remove(0);
        }
        self.history.push(StateHistoryEntry {
            timestamp: Timestamp::now(),
            from,
            to: transition.to,
            trigger: format!("{:?}", transition.trigger),
        });

        self.stats.successful += 1;
        Ok(true)
    }

    /// Check guard conditions
    fn check_guards(&self, guards: &[StateCondition]) -> bool {
        guards.iter().all(|g| self.check_condition(g))
    }

    /// Check a single condition
    fn check_condition(&self, condition: &StateCondition) -> bool {
        let var = match self.variables.get(&condition.variable) {
            Some(v) => v,
            None => return false,
        };

        match (&condition.operator, var, &condition.value) {
            (ConditionOperator::Equal, StateValue::Int(a), StateValue::Int(b)) => a == b,
            (ConditionOperator::NotEqual, StateValue::Int(a), StateValue::Int(b)) => a != b,
            (ConditionOperator::GreaterThan, StateValue::Int(a), StateValue::Int(b)) => a > b,
            (ConditionOperator::LessThan, StateValue::Int(a), StateValue::Int(b)) => a < b,
            (ConditionOperator::Equal, StateValue::Float(a), StateValue::Float(b)) => {
                (a - b).abs() < f64::EPSILON
            },
            (ConditionOperator::GreaterThan, StateValue::Float(a), StateValue::Float(b)) => a > b,
            (ConditionOperator::LessThan, StateValue::Float(a), StateValue::Float(b)) => a < b,
            (ConditionOperator::Equal, StateValue::Bool(a), StateValue::Bool(b)) => a == b,
            (ConditionOperator::Equal, StateValue::Text(a), StateValue::Text(b)) => a == b,
            (ConditionOperator::Contains, StateValue::Text(a), StateValue::Text(b)) => {
                a.contains(b.as_str())
            },
            (ConditionOperator::StartsWith, StateValue::Text(a), StateValue::Text(b)) => {
                a.starts_with(b.as_str())
            },
            (ConditionOperator::EndsWith, StateValue::Text(a), StateValue::Text(b)) => {
                a.ends_with(b.as_str())
            },
            _ => false,
        }
    }

    /// Execute an action
    fn execute_action(&mut self, action: &TransitionAction) -> Result<(), String> {
        match action.action_type {
            ActionType::SetVariable => {
                if let (Some(name), Some(value)) =
                    (action.params.get("name"), action.params.get("value"))
                {
                    // Parse value - simplified
                    let state_value = if let Ok(n) = value.parse::<i64>() {
                        StateValue::Int(n)
                    } else if let Ok(f) = value.parse::<f64>() {
                        StateValue::Float(f)
                    } else if value == "true" || value == "false" {
                        StateValue::Bool(value == "true")
                    } else {
                        StateValue::Text(value.clone())
                    };
                    self.variables.insert(name.clone(), state_value);
                }
                Ok(())
            },
            ActionType::ResetTimer => {
                if let Some(name) = action.params.get("name") {
                    let duration = action
                        .params
                        .get("duration")
                        .and_then(|d| d.parse().ok())
                        .unwrap_or(0);
                    self.timers.insert(name.clone(), Timer {
                        name: name.clone(),
                        started: Timestamp::now(),
                        duration,
                        repeating: false,
                    });
                }
                Ok(())
            },
            ActionType::Log => {
                // Logging - no-op in kernel
                Ok(())
            },
            ActionType::EmitEvent => {
                // Event emission handled elsewhere
                Ok(())
            },
            ActionType::Call => {
                // Function call - would need callback registry
                Ok(())
            },
        }
    }

    /// Process timeouts
    pub fn process_timeouts(&mut self) {
        let now = Timestamp::now();

        // Check timers
        let expired: Vec<_> = self
            .timers
            .iter()
            .filter(|(_, t)| now.elapsed_since(t.started) >= t.duration)
            .map(|(name, _)| name.clone())
            .collect();

        for timer_name in expired {
            // Trigger timeout event
            let _ = self.trigger_event(&format!("timeout_{}", timer_name));

            if let Some(timer) = self.timers.get_mut(&timer_name) {
                if timer.repeating {
                    timer.started = now;
                } else {
                    self.timers.remove(&timer_name);
                }
            }
        }

        // Check state timeout transitions
        if let Some(state) = self.states.get(&self.current_state).cloned() {
            for transition in &state.transitions {
                if let TransitionTrigger::Timeout(duration) = transition.trigger {
                    // Check if we've been in state long enough
                    if let Some(last_entry) = self.history.last() {
                        if last_entry.to == self.current_state {
                            let time_in_state = now.elapsed_since(last_entry.timestamp);
                            if time_in_state >= duration {
                                let _ = self.execute_transition(transition);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get history
    pub fn history(&self) -> &[StateHistoryEntry] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> &StateMachineStats {
        &self.stats
    }

    /// Get machine ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get domain ID
    pub fn domain_id(&self) -> DomainId {
        self.domain_id
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        // Find initial state
        let initial = self
            .states
            .values()
            .find(|s| s.state_type == StateType::Initial)
            .map(|s| s.id)
            .unwrap_or(0);

        self.current_state = initial;
        self.previous_state = None;
        self.variables.clear();
        self.timers.clear();
        self.history.clear();
    }
}

// ============================================================================
// STATE MACHINE MANAGER
// ============================================================================

/// Manages multiple state machines
pub struct StateMachineManager {
    /// State machines by domain
    machines: BTreeMap<DomainId, StateMachine>,
    /// Next machine ID
    next_id: AtomicU64,
}

impl StateMachineManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            machines: BTreeMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Create a state machine for a domain
    pub fn create_machine(&mut self, domain_id: DomainId, config: StateMachineConfig) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let machine = StateMachine::new(id, domain_id, config);
        self.machines.insert(domain_id, machine);
        id
    }

    /// Get machine for domain
    pub fn get(&self, domain_id: DomainId) -> Option<&StateMachine> {
        self.machines.get(&domain_id)
    }

    /// Get mutable machine
    pub fn get_mut(&mut self, domain_id: DomainId) -> Option<&mut StateMachine> {
        self.machines.get_mut(&domain_id)
    }

    /// Process all timeouts
    pub fn process_all_timeouts(&mut self) {
        for machine in self.machines.values_mut() {
            machine.process_timeouts();
        }
    }

    /// Get all statistics
    pub fn all_stats(&self) -> BTreeMap<DomainId, &StateMachineStats> {
        self.machines
            .iter()
            .map(|(id, m)| (*id, m.stats()))
            .collect()
    }
}

impl Default for StateMachineManager {
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
    fn test_state_machine() {
        let config = StateMachineConfig::default();
        let mut machine = StateMachine::new(1, DomainId::new(1), config);

        // Create states
        let initial = CognitiveState {
            id: 1,
            name: "initial".into(),
            state_type: StateType::Initial,
            data: StateData::Empty,
            entry_conditions: Vec::new(),
            exit_conditions: Vec::new(),
            transitions: vec![StateTransition {
                id: 1,
                from: 1,
                to: 2,
                trigger: TransitionTrigger::Event("start".into()),
                guards: Vec::new(),
                actions: Vec::new(),
                priority: 100,
            }],
            metadata: BTreeMap::new(),
        };

        let running = CognitiveState {
            id: 2,
            name: "running".into(),
            state_type: StateType::Normal,
            data: StateData::Empty,
            entry_conditions: Vec::new(),
            exit_conditions: Vec::new(),
            transitions: Vec::new(),
            metadata: BTreeMap::new(),
        };

        machine.add_state(initial);
        machine.add_state(running);
        machine.set_initial_state(1);

        assert_eq!(machine.current_state().unwrap().name, "initial");

        let result = machine.trigger_event("start");
        assert!(result.is_ok());
        assert!(result.unwrap());

        assert_eq!(machine.current_state().unwrap().name, "running");
    }

    #[test]
    fn test_guarded_transition() {
        let config = StateMachineConfig::default();
        let mut machine = StateMachine::new(1, DomainId::new(1), config);

        let state1 = CognitiveState {
            id: 1,
            name: "state1".into(),
            state_type: StateType::Initial,
            data: StateData::Empty,
            entry_conditions: Vec::new(),
            exit_conditions: Vec::new(),
            transitions: vec![StateTransition {
                id: 1,
                from: 1,
                to: 2,
                trigger: TransitionTrigger::Event("go".into()),
                guards: vec![StateCondition {
                    id: 1,
                    variable: "ready".into(),
                    operator: ConditionOperator::Equal,
                    value: StateValue::Bool(true),
                }],
                actions: Vec::new(),
                priority: 100,
            }],
            metadata: BTreeMap::new(),
        };

        let state2 = CognitiveState {
            id: 2,
            name: "state2".into(),
            state_type: StateType::Normal,
            data: StateData::Empty,
            entry_conditions: Vec::new(),
            exit_conditions: Vec::new(),
            transitions: Vec::new(),
            metadata: BTreeMap::new(),
        };

        machine.add_state(state1);
        machine.add_state(state2);
        machine.set_initial_state(1);

        // Try transition without guard satisfied
        let result = machine.trigger_event("go");
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Didn't transition

        // Set variable and try again
        machine.set_variable("ready", StateValue::Bool(true));
        let result = machine.trigger_event("go");
        assert!(result.is_ok());
        assert!(result.unwrap()); // Transitioned
    }
}

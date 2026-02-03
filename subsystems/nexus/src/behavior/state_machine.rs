//! NEXUS Year 2: Hierarchical State Machines
//!
//! Advanced state machine implementation with hierarchical states,
//! transitions, guards, and actions for kernel AI behavior modeling.

#![allow(dead_code)]

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    vec::Vec,
};

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for states
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(pub u64);

impl StateId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub const NONE: StateId = StateId(0);
}

/// Unique identifier for transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionId(pub u64);

/// Unique identifier for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

/// Event that can trigger transitions
#[derive(Debug, Clone)]
pub struct StateEvent {
    pub id: EventId,
    pub name: String,
    pub data: Option<EventData>,
    pub timestamp: u64,
}

impl StateEvent {
    pub fn new(id: EventId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            data: None,
            timestamp: 0,
        }
    }

    pub fn with_data(mut self, data: EventData) -> Self {
        self.data = Some(data);
        self
    }

    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// Data associated with an event
#[derive(Debug, Clone)]
pub enum EventData {
    None,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
}

impl EventData {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

/// Context for state machine execution
pub struct StateContext {
    pub time: u64,
    pub delta_time: u64,
    pub variables: BTreeMap<String, EventData>,
    pub pending_events: Vec<StateEvent>,
}

impl StateContext {
    pub fn new(time: u64, delta_time: u64) -> Self {
        Self {
            time,
            delta_time,
            variables: BTreeMap::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn get_var(&self, name: &str) -> Option<&EventData> {
        self.variables.get(name)
    }

    pub fn set_var(&mut self, name: String, value: EventData) {
        self.variables.insert(name, value);
    }

    pub fn emit_event(&mut self, event: StateEvent) {
        self.pending_events.push(event);
    }

    pub fn take_events(&mut self) -> Vec<StateEvent> {
        core::mem::take(&mut self.pending_events)
    }
}

// ============================================================================
// State Definition
// ============================================================================

/// State type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateType {
    /// Normal state
    Normal,
    /// Initial state (entry point)
    Initial,
    /// Final state (exit point)
    Final,
    /// Composite state containing sub-states
    Composite,
    /// History state (remembers last active sub-state)
    History,
    /// Deep history (remembers full nested state)
    DeepHistory,
}

/// A state in the state machine
pub struct State {
    pub id: StateId,
    pub name: String,
    pub state_type: StateType,
    pub parent: Option<StateId>,
    pub children: Vec<StateId>,
    pub initial_child: Option<StateId>,

    // Callbacks
    on_enter: Option<Box<dyn Fn(&mut StateContext) + Send + Sync>>,
    on_exit: Option<Box<dyn Fn(&mut StateContext) + Send + Sync>>,
    on_update: Option<Box<dyn Fn(&mut StateContext) + Send + Sync>>,

    // Internal state
    time_in_state: u64,
}

impl State {
    pub fn new(id: StateId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            state_type: StateType::Normal,
            parent: None,
            children: Vec::new(),
            initial_child: None,
            on_enter: None,
            on_exit: None,
            on_update: None,
            time_in_state: 0,
        }
    }

    pub fn with_type(mut self, state_type: StateType) -> Self {
        self.state_type = state_type;
        self
    }

    pub fn with_parent(mut self, parent: StateId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_on_enter<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut StateContext) + Send + Sync + 'static,
    {
        self.on_enter = Some(Box::new(f));
        self
    }

    pub fn with_on_exit<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut StateContext) + Send + Sync + 'static,
    {
        self.on_exit = Some(Box::new(f));
        self
    }

    pub fn with_on_update<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut StateContext) + Send + Sync + 'static,
    {
        self.on_update = Some(Box::new(f));
        self
    }

    pub fn enter(&mut self, ctx: &mut StateContext) {
        self.time_in_state = 0;
        if let Some(ref f) = self.on_enter {
            f(ctx);
        }
    }

    pub fn exit(&mut self, ctx: &mut StateContext) {
        if let Some(ref f) = self.on_exit {
            f(ctx);
        }
    }

    pub fn update(&mut self, ctx: &mut StateContext) {
        self.time_in_state += ctx.delta_time;
        if let Some(ref f) = self.on_update {
            f(ctx);
        }
    }

    pub fn time_in_state(&self) -> u64 {
        self.time_in_state
    }

    pub fn is_composite(&self) -> bool {
        self.state_type == StateType::Composite
    }

    pub fn is_final(&self) -> bool {
        self.state_type == StateType::Final
    }
}

// ============================================================================
// Transition Definition
// ============================================================================

/// Condition for transition guards
pub enum TransitionCondition {
    /// Always true
    Always,
    /// Event must match
    OnEvent(EventId),
    /// Variable comparison
    VarEquals(String, EventData),
    /// Variable greater than
    VarGreater(String, f64),
    /// Variable less than
    VarLess(String, f64),
    /// Time in state exceeds threshold
    TimeInState(u64),
    /// Custom condition function
    Custom(Box<dyn Fn(&StateContext, Option<&StateEvent>) -> bool + Send + Sync>),
    /// All conditions must be true
    And(Vec<TransitionCondition>),
    /// Any condition must be true
    Or(Vec<TransitionCondition>),
    /// Negation
    Not(Box<TransitionCondition>),
}

impl TransitionCondition {
    pub fn evaluate(&self, ctx: &StateContext, event: Option<&StateEvent>, time_in_state: u64) -> bool {
        match self {
            Self::Always => true,
            Self::OnEvent(id) => {
                event.map(|e| e.id == *id).unwrap_or(false)
            }
            Self::VarEquals(name, expected) => {
                ctx.get_var(name).map(|v| match (v, expected) {
                    (EventData::Int(a), EventData::Int(b)) => a == b,
                    (EventData::Float(a), EventData::Float(b)) => (a - b).abs() < 1e-9,
                    (EventData::Bool(a), EventData::Bool(b)) => a == b,
                    _ => false,
                }).unwrap_or(false)
            }
            Self::VarGreater(name, threshold) => {
                ctx.get_var(name).map(|v| match v {
                    EventData::Float(f) => *f > *threshold,
                    EventData::Int(i) => (*i as f64) > *threshold,
                    _ => false,
                }).unwrap_or(false)
            }
            Self::VarLess(name, threshold) => {
                ctx.get_var(name).map(|v| match v {
                    EventData::Float(f) => *f < *threshold,
                    EventData::Int(i) => (*i as f64) < *threshold,
                    _ => false,
                }).unwrap_or(false)
            }
            Self::TimeInState(threshold) => time_in_state >= *threshold,
            Self::Custom(f) => f(ctx, event),
            Self::And(conditions) => {
                conditions.iter().all(|c| c.evaluate(ctx, event, time_in_state))
            }
            Self::Or(conditions) => {
                conditions.iter().any(|c| c.evaluate(ctx, event, time_in_state))
            }
            Self::Not(condition) => !condition.evaluate(ctx, event, time_in_state),
        }
    }
}

/// Transition between states
pub struct Transition {
    pub id: TransitionId,
    pub name: String,
    pub source: StateId,
    pub target: StateId,
    pub condition: TransitionCondition,
    pub priority: i32,
    pub action: Option<Box<dyn Fn(&mut StateContext) + Send + Sync>>,
}

impl Transition {
    pub fn new(
        id: TransitionId,
        name: impl Into<String>,
        source: StateId,
        target: StateId,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            source,
            target,
            condition: TransitionCondition::Always,
            priority: 0,
            action: None,
        }
    }

    pub fn with_condition(mut self, condition: TransitionCondition) -> Self {
        self.condition = condition;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_action<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut StateContext) + Send + Sync + 'static,
    {
        self.action = Some(Box::new(f));
        self
    }

    pub fn can_fire(&self, ctx: &StateContext, event: Option<&StateEvent>, time_in_state: u64) -> bool {
        self.condition.evaluate(ctx, event, time_in_state)
    }

    pub fn execute(&self, ctx: &mut StateContext) {
        if let Some(ref action) = self.action {
            action(ctx);
        }
    }
}

// ============================================================================
// State Machine
// ============================================================================

/// Flat state machine
pub struct StateMachine {
    name: String,
    states: BTreeMap<StateId, State>,
    transitions: Vec<Transition>,
    initial_state: StateId,
    current_state: StateId,
    history: Vec<StateId>,
}

impl StateMachine {
    pub fn new(name: impl Into<String>, initial_state: StateId) -> Self {
        Self {
            name: name.into(),
            states: BTreeMap::new(),
            transitions: Vec::new(),
            initial_state,
            current_state: initial_state,
            history: Vec::new(),
        }
    }

    pub fn add_state(&mut self, state: State) {
        self.states.insert(state.id, state);
    }

    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
        // Sort by priority (higher first)
        self.transitions.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn current_state(&self) -> StateId {
        self.current_state
    }

    pub fn current_state_name(&self) -> Option<&str> {
        self.states.get(&self.current_state).map(|s| s.name.as_str())
    }

    pub fn history(&self) -> &[StateId] {
        &self.history
    }

    /// Initialize the state machine
    pub fn initialize(&mut self, ctx: &mut StateContext) {
        self.current_state = self.initial_state;
        if let Some(state) = self.states.get_mut(&self.current_state) {
            state.enter(ctx);
        }
    }

    /// Process an event
    pub fn process_event(&mut self, event: &StateEvent, ctx: &mut StateContext) -> bool {
        self.try_transition(Some(event), ctx)
    }

    /// Update the state machine
    pub fn update(&mut self, ctx: &mut StateContext) {
        // Update current state
        if let Some(state) = self.states.get_mut(&self.current_state) {
            state.update(ctx);
        }

        // Try automatic transitions (without event)
        self.try_transition(None, ctx);
    }

    fn try_transition(&mut self, event: Option<&StateEvent>, ctx: &mut StateContext) -> bool {
        let time_in_state = self.states.get(&self.current_state)
            .map(|s| s.time_in_state())
            .unwrap_or(0);

        // Find first matching transition
        let matching_transition = self.transitions.iter()
            .find(|t| t.source == self.current_state && t.can_fire(ctx, event, time_in_state));

        if let Some(transition) = matching_transition {
            let target = transition.target;

            // Exit current state
            if let Some(state) = self.states.get_mut(&self.current_state) {
                state.exit(ctx);
            }

            // Execute transition action
            // Note: We can't call transition.execute directly due to borrow checker
            // So we store the action separately
            let has_action = self.transitions.iter()
                .find(|t| t.source == self.current_state && t.target == target)
                .and_then(|t| t.action.as_ref())
                .is_some();

            if has_action {
                // Re-find and execute
                for t in &self.transitions {
                    if t.source == self.current_state && t.target == target {
                        if let Some(ref action) = t.action {
                            action(ctx);
                        }
                        break;
                    }
                }
            }

            // Update history
            self.history.push(self.current_state);
            if self.history.len() > 100 {
                self.history.remove(0);
            }

            // Enter new state
            self.current_state = target;
            if let Some(state) = self.states.get_mut(&self.current_state) {
                state.enter(ctx);
            }

            return true;
        }

        false
    }

    /// Check if in a final state
    pub fn is_finished(&self) -> bool {
        self.states.get(&self.current_state)
            .map(|s| s.is_final())
            .unwrap_or(false)
    }

    /// Reset to initial state
    pub fn reset(&mut self, ctx: &mut StateContext) {
        // Exit current state
        if let Some(state) = self.states.get_mut(&self.current_state) {
            state.exit(ctx);
        }

        self.current_state = self.initial_state;
        self.history.clear();

        // Enter initial state
        if let Some(state) = self.states.get_mut(&self.current_state) {
            state.enter(ctx);
        }
    }
}

// ============================================================================
// Hierarchical State Machine
// ============================================================================

/// Entry for hierarchical state tracking
#[derive(Debug, Clone)]
struct HierarchyEntry {
    state_id: StateId,
    depth: usize,
}

/// Hierarchical state machine with nested states
pub struct HierarchicalStateMachine {
    name: String,
    states: BTreeMap<StateId, State>,
    transitions: Vec<Transition>,
    root_state: StateId,
    active_states: Vec<StateId>, // Stack of active states (root to leaf)
    history: BTreeMap<StateId, StateId>, // History for composite states
}

impl HierarchicalStateMachine {
    pub fn new(name: impl Into<String>, root_state: StateId) -> Self {
        Self {
            name: name.into(),
            states: BTreeMap::new(),
            transitions: Vec::new(),
            root_state,
            active_states: Vec::new(),
            history: BTreeMap::new(),
        }
    }

    pub fn add_state(&mut self, state: State) {
        let id = state.id;
        let parent = state.parent;

        self.states.insert(id, state);

        // Update parent's children list
        if let Some(parent_id) = parent {
            if let Some(parent_state) = self.states.get_mut(&parent_id) {
                parent_state.children.push(id);
            }
        }
    }

    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
        self.transitions.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn active_states(&self) -> &[StateId] {
        &self.active_states
    }

    pub fn current_leaf(&self) -> Option<StateId> {
        self.active_states.last().copied()
    }

    pub fn is_in_state(&self, state_id: StateId) -> bool {
        self.active_states.contains(&state_id)
    }

    /// Initialize the hierarchical state machine
    pub fn initialize(&mut self, ctx: &mut StateContext) {
        self.active_states.clear();
        self.enter_state(self.root_state, ctx);
    }

    fn enter_state(&mut self, state_id: StateId, ctx: &mut StateContext) {
        self.active_states.push(state_id);

        if let Some(state) = self.states.get_mut(&state_id) {
            state.enter(ctx);

            // If composite, enter initial child
            if state.is_composite() {
                if let Some(initial) = state.initial_child {
                    self.enter_state(initial, ctx);
                } else if !state.children.is_empty() {
                    let first_child = state.children[0];
                    self.enter_state(first_child, ctx);
                }
            }
        }
    }

    fn exit_state(&mut self, state_id: StateId, ctx: &mut StateContext) {
        // Exit children first (leaf to root)
        if let Some(state) = self.states.get(&state_id) {
            let children = state.children.clone();
            for child in children {
                if self.active_states.contains(&child) {
                    self.exit_state(child, ctx);
                }
            }
        }

        // Save history for parent
        if let Some(state) = self.states.get(&state_id) {
            if let Some(parent) = state.parent {
                self.history.insert(parent, state_id);
            }
        }

        // Exit this state
        if let Some(state) = self.states.get_mut(&state_id) {
            state.exit(ctx);
        }

        self.active_states.retain(|&s| s != state_id);
    }

    /// Process an event
    pub fn process_event(&mut self, event: &StateEvent, ctx: &mut StateContext) -> bool {
        // Check transitions from leaf to root
        for &state_id in self.active_states.iter().rev() {
            let time_in_state = self.states.get(&state_id)
                .map(|s| s.time_in_state())
                .unwrap_or(0);

            let matching = self.transitions.iter()
                .find(|t| t.source == state_id && t.can_fire(ctx, Some(event), time_in_state))
                .map(|t| (t.target, t.id));

            if let Some((target, _)) = matching {
                return self.transition_to(state_id, target, ctx);
            }
        }

        false
    }

    /// Update the hierarchical state machine
    pub fn update(&mut self, ctx: &mut StateContext) {
        // Update all active states (root to leaf)
        for &state_id in &self.active_states.clone() {
            if let Some(state) = self.states.get_mut(&state_id) {
                state.update(ctx);
            }
        }

        // Try automatic transitions from leaf to root
        for &state_id in self.active_states.clone().iter().rev() {
            let time_in_state = self.states.get(&state_id)
                .map(|s| s.time_in_state())
                .unwrap_or(0);

            let matching = self.transitions.iter()
                .find(|t| t.source == state_id && t.can_fire(ctx, None, time_in_state))
                .map(|t| t.target);

            if let Some(target) = matching {
                if self.transition_to(state_id, target, ctx) {
                    break;
                }
            }
        }
    }

    fn transition_to(&mut self, source: StateId, target: StateId, ctx: &mut StateContext) -> bool {
        // Find LCA (Lowest Common Ancestor)
        let source_ancestors = self.get_ancestors(source);
        let target_ancestors = self.get_ancestors(target);

        let lca = source_ancestors.iter()
            .find(|s| target_ancestors.contains(s))
            .copied();

        // Exit states up to LCA
        let mut states_to_exit = Vec::new();
        for &state_id in self.active_states.iter().rev() {
            if Some(state_id) == lca {
                break;
            }
            states_to_exit.push(state_id);
            if state_id == source {
                break;
            }
        }

        for state_id in states_to_exit {
            self.exit_state(state_id, ctx);
        }

        // Execute transition action
        for t in &self.transitions {
            if t.source == source && t.target == target {
                if let Some(ref action) = t.action {
                    action(ctx);
                }
                break;
            }
        }

        // Enter states from LCA to target
        let mut states_to_enter = Vec::new();
        let mut current = target;
        loop {
            states_to_enter.push(current);
            if let Some(state) = self.states.get(&current) {
                if Some(current) == lca || state.parent.is_none() {
                    break;
                }
                if let Some(parent) = state.parent {
                    current = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        states_to_enter.reverse();

        for state_id in states_to_enter {
            if !self.active_states.contains(&state_id) {
                self.enter_state(state_id, ctx);
            }
        }

        true
    }

    fn get_ancestors(&self, state_id: StateId) -> Vec<StateId> {
        let mut ancestors = vec![state_id];
        let mut current = state_id;

        while let Some(state) = self.states.get(&current) {
            if let Some(parent) = state.parent {
                ancestors.push(parent);
                current = parent;
            } else {
                break;
            }
        }

        ancestors
    }

    /// Get history state for a composite state
    pub fn get_history(&self, composite: StateId) -> Option<StateId> {
        self.history.get(&composite).copied()
    }

    /// Reset the state machine
    pub fn reset(&mut self, ctx: &mut StateContext) {
        // Exit all active states
        for &state_id in self.active_states.clone().iter().rev() {
            if let Some(state) = self.states.get_mut(&state_id) {
                state.exit(ctx);
            }
        }

        self.active_states.clear();
        self.history.clear();

        // Re-initialize
        self.initialize(ctx);
    }
}

// ============================================================================
// Kernel State Machines
// ============================================================================

/// Predefined kernel states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelState {
    Idle,
    Low,
    Medium,
    High,
    Critical,
    Emergency,
    Recovery,
    Shutdown,
}

impl KernelState {
    pub fn as_state_id(&self) -> StateId {
        StateId(*self as u64 + 1)
    }
}

/// Builder for kernel state machines
pub struct KernelStateMachineBuilder {
    name: String,
    next_id: u64,
    states: Vec<State>,
    transitions: Vec<Transition>,
}

impl KernelStateMachineBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            next_id: 1,
            states: Vec::new(),
            transitions: Vec::new(),
        }
    }

    fn next_state_id(&mut self) -> StateId {
        let id = StateId(self.next_id);
        self.next_id += 1;
        id
    }

    fn next_transition_id(&mut self) -> TransitionId {
        let id = TransitionId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn add_state(mut self, name: impl Into<String>, state_type: StateType) -> (Self, StateId) {
        let id = self.next_state_id();
        let state = State::new(id, name).with_type(state_type);
        self.states.push(state);
        (self, id)
    }

    pub fn add_transition(
        mut self,
        name: impl Into<String>,
        source: StateId,
        target: StateId,
        condition: TransitionCondition,
    ) -> Self {
        let id = self.next_transition_id();
        let transition = Transition::new(id, name, source, target).with_condition(condition);
        self.transitions.push(transition);
        self
    }

    pub fn build(self, initial: StateId) -> StateMachine {
        let mut sm = StateMachine::new(self.name, initial);
        for state in self.states {
            sm.add_state(state);
        }
        for transition in self.transitions {
            sm.add_transition(transition);
        }
        sm
    }
}

/// Create a kernel load management state machine
pub fn create_kernel_load_fsm() -> StateMachine {
    let builder = KernelStateMachineBuilder::new("KernelLoad");

    let (builder, idle) = builder.add_state("Idle", StateType::Initial);
    let (builder, low) = builder.add_state("Low", StateType::Normal);
    let (builder, medium) = builder.add_state("Medium", StateType::Normal);
    let (builder, high) = builder.add_state("High", StateType::Normal);
    let (builder, critical) = builder.add_state("Critical", StateType::Normal);

    let builder = builder
        .add_transition("to_low", idle, low,
            TransitionCondition::VarGreater("cpu_load".into(), 0.1))
        .add_transition("to_medium", low, medium,
            TransitionCondition::VarGreater("cpu_load".into(), 0.4))
        .add_transition("to_high", medium, high,
            TransitionCondition::VarGreater("cpu_load".into(), 0.7))
        .add_transition("to_critical", high, critical,
            TransitionCondition::VarGreater("cpu_load".into(), 0.9))
        .add_transition("from_low", low, idle,
            TransitionCondition::VarLess("cpu_load".into(), 0.1))
        .add_transition("from_medium", medium, low,
            TransitionCondition::VarLess("cpu_load".into(), 0.4))
        .add_transition("from_high", high, medium,
            TransitionCondition::VarLess("cpu_load".into(), 0.7))
        .add_transition("from_critical", critical, high,
            TransitionCondition::VarLess("cpu_load".into(), 0.9));

    builder.build(idle)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_id() {
        let id = StateId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_event_data() {
        let data = EventData::Int(42);
        assert_eq!(data.as_int(), Some(42));
        assert_eq!(data.as_float(), None);
    }

    #[test]
    fn test_state_context() {
        let mut ctx = StateContext::new(1000, 16);
        ctx.set_var("test".into(), EventData::Bool(true));
        assert_eq!(ctx.get_var("test").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_transition_condition_always() {
        let ctx = StateContext::new(0, 0);
        assert!(TransitionCondition::Always.evaluate(&ctx, None, 0));
    }

    #[test]
    fn test_state_machine_creation() {
        let sm = StateMachine::new("test", StateId(1));
        assert_eq!(sm.current_state(), StateId(1));
    }
}

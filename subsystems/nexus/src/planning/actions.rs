//! # Action Definitions for NEXUS Planning
//!
//! Action representation with preconditions and effects.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// ACTION TYPES
// ============================================================================

/// Action identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub u32);

/// State variable identifier
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateVar(pub String);

/// State value
#[derive(Debug, Clone, PartialEq)]
pub enum StateValue {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
}

impl StateValue {
    /// Get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            StateValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as int
    pub fn as_int(&self) -> Option<i64> {
        match self {
            StateValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            StateValue::Float(f) => Some(*f),
            StateValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
}

/// World state
#[derive(Debug, Clone, Default)]
pub struct WorldState {
    /// State variables
    variables: BTreeMap<StateVar, StateValue>,
}

impl WorldState {
    /// Create empty state
    pub fn new() -> Self {
        Self {
            variables: BTreeMap::new(),
        }
    }

    /// Set variable
    pub fn set(&mut self, var: StateVar, value: StateValue) {
        self.variables.insert(var, value);
    }

    /// Get variable
    pub fn get(&self, var: &StateVar) -> Option<&StateValue> {
        self.variables.get(var)
    }

    /// Check if variable is set
    pub fn has(&self, var: &StateVar) -> bool {
        self.variables.contains_key(var)
    }

    /// Set bool variable
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.set(StateVar(String::from(name)), StateValue::Bool(value));
    }

    /// Set int variable
    pub fn set_int(&mut self, name: &str, value: i64) {
        self.set(StateVar(String::from(name)), StateValue::Int(value));
    }

    /// Get bool variable
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.get(&StateVar(String::from(name)))?.as_bool()
    }

    /// Get int variable
    pub fn get_int(&self, name: &str) -> Option<i64> {
        self.get(&StateVar(String::from(name)))?.as_int()
    }

    /// Apply changes from another state
    pub fn apply(&mut self, changes: &WorldState) {
        for (var, value) in &changes.variables {
            self.variables.insert(var.clone(), value.clone());
        }
    }

    /// Clone with changes
    pub fn with_changes(&self, changes: &WorldState) -> Self {
        let mut new_state = self.clone();
        new_state.apply(changes);
        new_state
    }
}

// ============================================================================
// PRECONDITIONS
// ============================================================================

/// Comparison operator for preconditions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
}

/// Action precondition
#[derive(Debug, Clone)]
pub struct ActionPrecondition {
    /// Variable to check
    pub variable: StateVar,
    /// Comparison operator
    pub comparison: Comparison,
    /// Expected value
    pub value: StateValue,
}

impl ActionPrecondition {
    /// Create new precondition
    pub fn new(variable: StateVar, comparison: Comparison, value: StateValue) -> Self {
        Self {
            variable,
            comparison,
            value,
        }
    }

    /// Create equality precondition
    pub fn equals(name: &str, value: StateValue) -> Self {
        Self::new(StateVar(String::from(name)), Comparison::Eq, value)
    }

    /// Create bool true precondition
    pub fn is_true(name: &str) -> Self {
        Self::equals(name, StateValue::Bool(true))
    }

    /// Create bool false precondition
    pub fn is_false(name: &str) -> Self {
        Self::equals(name, StateValue::Bool(false))
    }

    /// Check if precondition is satisfied
    pub fn check(&self, state: &WorldState) -> bool {
        let actual = match state.get(&self.variable) {
            Some(v) => v,
            None => return false,
        };

        match (&self.value, actual) {
            (StateValue::Bool(expected), StateValue::Bool(actual)) => {
                self.compare_bool(*expected, *actual)
            }
            (StateValue::Int(expected), StateValue::Int(actual)) => {
                self.compare_ord(*expected, *actual)
            }
            (StateValue::Float(expected), StateValue::Float(actual)) => {
                self.compare_float(*expected, *actual)
            }
            (StateValue::String(expected), StateValue::String(actual)) => {
                self.compare_ord(expected, actual)
            }
            _ => false, // Type mismatch
        }
    }

    fn compare_bool(&self, expected: bool, actual: bool) -> bool {
        match self.comparison {
            Comparison::Eq => expected == actual,
            Comparison::Ne => expected != actual,
            _ => false,
        }
    }

    fn compare_ord<T: Ord>(&self, expected: T, actual: T) -> bool {
        match self.comparison {
            Comparison::Eq => expected == actual,
            Comparison::Ne => expected != actual,
            Comparison::Lt => actual < expected,
            Comparison::Le => actual <= expected,
            Comparison::Gt => actual > expected,
            Comparison::Ge => actual >= expected,
        }
    }

    fn compare_float(&self, expected: f64, actual: f64) -> bool {
        match self.comparison {
            Comparison::Eq => (expected - actual).abs() < 0.0001,
            Comparison::Ne => (expected - actual).abs() >= 0.0001,
            Comparison::Lt => actual < expected,
            Comparison::Le => actual <= expected,
            Comparison::Gt => actual > expected,
            Comparison::Ge => actual >= expected,
        }
    }
}

// ============================================================================
// EFFECTS
// ============================================================================

/// Effect type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    /// Set variable to value
    Set,
    /// Add value to variable
    Add,
    /// Subtract value from variable
    Subtract,
    /// Toggle boolean variable
    Toggle,
    /// Delete variable
    Delete,
}

/// Action effect
#[derive(Debug, Clone)]
pub struct ActionEffect {
    /// Variable to modify
    pub variable: StateVar,
    /// Effect type
    pub effect_type: EffectType,
    /// Value for the effect
    pub value: StateValue,
    /// Probability of effect occurring (for stochastic actions)
    pub probability: f64,
}

impl ActionEffect {
    /// Create new effect
    pub fn new(variable: StateVar, effect_type: EffectType, value: StateValue) -> Self {
        Self {
            variable,
            effect_type,
            value,
            probability: 1.0,
        }
    }

    /// Create set effect
    pub fn set(name: &str, value: StateValue) -> Self {
        Self::new(StateVar(String::from(name)), EffectType::Set, value)
    }

    /// Create set true effect
    pub fn set_true(name: &str) -> Self {
        Self::set(name, StateValue::Bool(true))
    }

    /// Create set false effect
    pub fn set_false(name: &str) -> Self {
        Self::set(name, StateValue::Bool(false))
    }

    /// Apply effect to state
    pub fn apply(&self, state: &mut WorldState) {
        match self.effect_type {
            EffectType::Set => {
                state.set(self.variable.clone(), self.value.clone());
            }
            EffectType::Add => {
                if let Some(current) = state.get(&self.variable).cloned() {
                    let new_value = match (&current, &self.value) {
                        (StateValue::Int(c), StateValue::Int(v)) => StateValue::Int(c + v),
                        (StateValue::Float(c), StateValue::Float(v)) => StateValue::Float(c + v),
                        _ => return,
                    };
                    state.set(self.variable.clone(), new_value);
                }
            }
            EffectType::Subtract => {
                if let Some(current) = state.get(&self.variable).cloned() {
                    let new_value = match (&current, &self.value) {
                        (StateValue::Int(c), StateValue::Int(v)) => StateValue::Int(c - v),
                        (StateValue::Float(c), StateValue::Float(v)) => StateValue::Float(c - v),
                        _ => return,
                    };
                    state.set(self.variable.clone(), new_value);
                }
            }
            EffectType::Toggle => {
                if let Some(StateValue::Bool(b)) = state.get(&self.variable).cloned() {
                    state.set(self.variable.clone(), StateValue::Bool(!b));
                }
            }
            EffectType::Delete => {
                state.variables.remove(&self.variable);
            }
        }
    }
}

// ============================================================================
// ACTION
// ============================================================================

/// An action that can be executed
#[derive(Debug, Clone)]
pub struct Action {
    /// Action ID
    pub id: ActionId,
    /// Action name
    pub name: String,
    /// Description
    pub description: String,
    /// Preconditions (all must be satisfied)
    pub preconditions: Vec<ActionPrecondition>,
    /// Effects (applied when action executes)
    pub effects: Vec<ActionEffect>,
    /// Cost of executing action
    pub cost: f64,
    /// Duration of action (time units)
    pub duration: u64,
    /// Is action deterministic?
    pub deterministic: bool,
}

impl Action {
    /// Create new action
    pub fn new(id: ActionId, name: String) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            preconditions: Vec::new(),
            effects: Vec::new(),
            cost: 1.0,
            duration: 1,
            deterministic: true,
        }
    }

    /// Add precondition
    pub fn with_precondition(mut self, precondition: ActionPrecondition) -> Self {
        self.preconditions.push(precondition);
        self
    }

    /// Add effect
    pub fn with_effect(mut self, effect: ActionEffect) -> Self {
        self.effects.push(effect);
        self
    }

    /// Set cost
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost = cost.max(0.0);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    /// Check if action is applicable in state
    pub fn is_applicable(&self, state: &WorldState) -> bool {
        self.preconditions.iter().all(|p| p.check(state))
    }

    /// Apply action to state (returns new state)
    pub fn apply(&self, state: &WorldState) -> WorldState {
        let mut new_state = state.clone();
        for effect in &self.effects {
            effect.apply(&mut new_state);
        }
        new_state
    }

    /// Get variables read by this action
    pub fn reads(&self) -> BTreeSet<StateVar> {
        self.preconditions.iter().map(|p| p.variable.clone()).collect()
    }

    /// Get variables written by this action
    pub fn writes(&self) -> BTreeSet<StateVar> {
        self.effects.iter().map(|e| e.variable.clone()).collect()
    }
}

// ============================================================================
// ACTION SPACE
// ============================================================================

/// Collection of available actions
pub struct ActionSpace {
    /// Actions by ID
    actions: BTreeMap<ActionId, Action>,
    /// Actions by name
    by_name: BTreeMap<String, ActionId>,
    /// Next action ID
    next_id: u32,
}

impl ActionSpace {
    /// Create new action space
    pub fn new() -> Self {
        Self {
            actions: BTreeMap::new(),
            by_name: BTreeMap::new(),
            next_id: 0,
        }
    }

    /// Add action
    pub fn add(&mut self, action: Action) -> ActionId {
        let id = action.id;
        self.by_name.insert(action.name.clone(), id);
        self.actions.insert(id, action);
        id
    }

    /// Create and add action
    pub fn create(&mut self, name: String) -> ActionId {
        let id = ActionId(self.next_id);
        self.next_id += 1;
        let action = Action::new(id, name);
        self.add(action)
    }

    /// Get action by ID
    pub fn get(&self, id: ActionId) -> Option<&Action> {
        self.actions.get(&id)
    }

    /// Get action by name
    pub fn get_by_name(&self, name: &str) -> Option<&Action> {
        self.by_name.get(name).and_then(|id| self.actions.get(id))
    }

    /// Get applicable actions for state
    pub fn get_applicable(&self, state: &WorldState) -> Vec<&Action> {
        self.actions.values()
            .filter(|a| a.is_applicable(state))
            .collect()
    }

    /// Get all actions
    pub fn all(&self) -> Vec<&Action> {
        self.actions.values().collect()
    }

    /// Get action count
    pub fn count(&self) -> usize {
        self.actions.len()
    }
}

impl Default for ActionSpace {
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
    fn test_world_state() {
        let mut state = WorldState::new();
        state.set_bool("has_key", true);
        state.set_int("coins", 10);

        assert_eq!(state.get_bool("has_key"), Some(true));
        assert_eq!(state.get_int("coins"), Some(10));
    }

    #[test]
    fn test_precondition() {
        let mut state = WorldState::new();
        state.set_bool("door_open", false);
        state.set_int("energy", 50);

        let precond1 = ActionPrecondition::is_false("door_open");
        assert!(precond1.check(&state));

        let precond2 = ActionPrecondition::new(
            StateVar(String::from("energy")),
            Comparison::Ge,
            StateValue::Int(30),
        );
        assert!(precond2.check(&state));
    }

    #[test]
    fn test_action_effect() {
        let mut state = WorldState::new();
        state.set_bool("light", false);
        state.set_int("count", 5);

        let effect1 = ActionEffect::set_true("light");
        effect1.apply(&mut state);
        assert_eq!(state.get_bool("light"), Some(true));

        let effect2 = ActionEffect::new(
            StateVar(String::from("count")),
            EffectType::Add,
            StateValue::Int(3),
        );
        effect2.apply(&mut state);
        assert_eq!(state.get_int("count"), Some(8));
    }

    #[test]
    fn test_action_application() {
        let mut state = WorldState::new();
        state.set_bool("door_locked", true);
        state.set_bool("has_key", true);

        let unlock = Action::new(ActionId(0), String::from("unlock_door"))
            .with_precondition(ActionPrecondition::is_true("has_key"))
            .with_precondition(ActionPrecondition::is_true("door_locked"))
            .with_effect(ActionEffect::set_false("door_locked"));

        assert!(unlock.is_applicable(&state));

        let new_state = unlock.apply(&state);
        assert_eq!(new_state.get_bool("door_locked"), Some(false));
    }

    #[test]
    fn test_action_space() {
        let mut space = ActionSpace::new();

        let action = Action::new(ActionId(0), String::from("walk"))
            .with_cost(1.0);
        space.add(action);

        let action2 = Action::new(ActionId(1), String::from("run"))
            .with_cost(2.0);
        space.add(action2);

        assert_eq!(space.count(), 2);
        assert!(space.get_by_name("walk").is_some());
    }
}

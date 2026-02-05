//! # Event Calculus Engine for NEXUS
//!
//! Year 2 "COGNITION" - Event Calculus implementation for reasoning about
//! actions, events, and their effects over time. Complements Allen's Interval
//! Algebra in temporal.rs with action-based temporal reasoning.
//!
//! ## Features
//!
//! - Fluent tracking (time-varying properties)
//! - Action/event effects
//! - Inertia and persistence reasoning
//! - Narrative construction
//! - Temporal projection
//! - Abductive reasoning about missing events

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Time point (discrete)
pub type Time = u64;

/// Fluent identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluentId(pub u32);

/// Event identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u32);

/// Action identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub u32);

/// A fluent (time-varying property)
#[derive(Debug, Clone)]
pub struct Fluent {
    /// Fluent ID
    pub id: FluentId,
    /// Fluent name
    pub name: String,
    /// Type of fluent
    pub fluent_type: FluentType,
    /// Current value (at latest known time)
    pub current_value: FluentValue,
    /// Initial value
    pub initial_value: FluentValue,
}

/// Type of fluent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluentType {
    /// Boolean fluent (true/false)
    Boolean,
    /// Integer fluent
    Integer,
    /// Real-valued fluent
    Real,
    /// Symbolic fluent (from a finite domain)
    Symbolic,
}

/// Fluent value
#[derive(Debug, Clone, PartialEq)]
pub enum FluentValue {
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Real value
    Real(f64),
    /// Symbolic value (string)
    Symbol(String),
    /// Unknown value
    Unknown,
}

impl FluentValue {
    /// Check if value is true (for boolean fluents)
    pub fn is_true(&self) -> bool {
        matches!(self, FluentValue::Bool(true))
    }

    /// Check if value is false (for boolean fluents)
    pub fn is_false(&self) -> bool {
        matches!(self, FluentValue::Bool(false))
    }

    /// Get as boolean
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            FluentValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as integer
    pub fn as_int(&self) -> Option<i64> {
        match self {
            FluentValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get as real
    pub fn as_real(&self) -> Option<f64> {
        match self {
            FluentValue::Real(r) => Some(*r),
            FluentValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
}

/// An event occurrence
#[derive(Debug, Clone)]
pub struct Event {
    /// Event ID
    pub id: EventId,
    /// Action that generated this event
    pub action: ActionId,
    /// Time of occurrence
    pub time: Time,
    /// Arguments
    pub args: Vec<FluentValue>,
    /// Was this event observed or inferred?
    pub observed: bool,
}

/// An action definition
#[derive(Debug, Clone)]
pub struct Action {
    /// Action ID
    pub id: ActionId,
    /// Action name
    pub name: String,
    /// Preconditions (fluents that must hold)
    pub preconditions: Vec<Precondition>,
    /// Initiation effects (what becomes true)
    pub initiates: Vec<Effect>,
    /// Termination effects (what becomes false)
    pub terminates: Vec<Effect>,
    /// Release effects (what becomes unknown/non-inertial)
    pub releases: Vec<Effect>,
}

/// Precondition for action
#[derive(Debug, Clone)]
pub struct Precondition {
    /// Fluent ID
    pub fluent: FluentId,
    /// Required value
    pub required_value: FluentValue,
    /// Is this a negative precondition?
    pub negated: bool,
}

/// Effect of an action
#[derive(Debug, Clone)]
pub struct Effect {
    /// Fluent affected
    pub fluent: FluentId,
    /// New value
    pub value: FluentValue,
    /// Condition for effect (optional)
    pub condition: Option<Precondition>,
}

// ============================================================================
// FLUENT TIMELINE
// ============================================================================

/// A timeline tracking fluent values over time
#[derive(Debug, Clone)]
pub struct FluentTimeline {
    /// Fluent ID
    pub fluent: FluentId,
    /// Changes over time: (time, value, caused_by_event)
    changes: Vec<(Time, FluentValue, Option<EventId>)>,
    /// Maximum validity period (inertia)
    max_validity: Option<Time>,
}

impl FluentTimeline {
    /// Create new timeline
    pub fn new(fluent: FluentId, initial_value: FluentValue) -> Self {
        Self {
            fluent,
            changes: vec![(0, initial_value, None)],
            max_validity: None,
        }
    }

    /// Record a change
    pub fn record_change(&mut self, time: Time, value: FluentValue, caused_by: Option<EventId>) {
        // Insert in order
        let pos = self.changes.partition_point(|(t, _, _)| *t <= time);
        self.changes.insert(pos, (time, value, caused_by));
    }

    /// Get value at time (with inertia)
    pub fn value_at(&self, time: Time) -> &FluentValue {
        // Find the last change at or before this time
        for (t, v, _) in self.changes.iter().rev() {
            if *t <= time {
                // Check validity
                if let Some(max_v) = self.max_validity {
                    if time - *t > max_v {
                        return &FluentValue::Unknown;
                    }
                }
                return v;
            }
        }

        &FluentValue::Unknown
    }

    /// Get the event that caused the value at time
    pub fn caused_by_at(&self, time: Time) -> Option<EventId> {
        for (t, _, e) in self.changes.iter().rev() {
            if *t <= time {
                return *e;
            }
        }
        None
    }

    /// Check if fluent holds at time
    pub fn holds_at(&self, time: Time) -> bool {
        self.value_at(time).is_true()
    }

    /// Get all change times
    pub fn change_times(&self) -> Vec<Time> {
        self.changes.iter().map(|(t, _, _)| *t).collect()
    }

    /// Get interval where fluent holds
    pub fn holds_interval(&self, from: Time, to: Time) -> Vec<(Time, Time)> {
        let mut intervals = Vec::new();
        let mut start: Option<Time> = None;

        for t in from..=to {
            let holds = self.value_at(t).is_true();

            match (start, holds) {
                (None, true) => start = Some(t),
                (Some(s), false) => {
                    intervals.push((s, t - 1));
                    start = None;
                },
                _ => {},
            }
        }

        if let Some(s) = start {
            intervals.push((s, to));
        }

        intervals
    }
}

// ============================================================================
// NARRATIVE
// ============================================================================

/// A narrative (sequence of events)
#[derive(Debug, Clone)]
pub struct Narrative {
    /// Narrative ID
    pub id: u32,
    /// Name
    pub name: String,
    /// Events in chronological order
    pub events: Vec<Event>,
    /// Initial state
    pub initial_state: BTreeMap<FluentId, FluentValue>,
}

impl Narrative {
    /// Create new narrative
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            events: Vec::new(),
            initial_state: BTreeMap::new(),
        }
    }

    /// Add event
    pub fn add_event(&mut self, event: Event) {
        // Insert in chronological order
        let pos = self.events.partition_point(|e| e.time <= event.time);
        self.events.insert(pos, event);
    }

    /// Get event at time
    pub fn event_at(&self, time: Time) -> Option<&Event> {
        self.events.iter().find(|e| e.time == time)
    }

    /// Get events in time range
    pub fn events_in_range(&self, from: Time, to: Time) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.time >= from && e.time <= to)
            .collect()
    }

    /// Set initial value
    pub fn set_initial(&mut self, fluent: FluentId, value: FluentValue) {
        self.initial_state.insert(fluent, value);
    }
}

// ============================================================================
// EVENT CALCULUS ENGINE
// ============================================================================

/// Event Calculus reasoning engine
pub struct EventCalculus {
    /// Fluent definitions
    fluents: BTreeMap<FluentId, Fluent>,
    /// Action definitions
    actions: BTreeMap<ActionId, Action>,
    /// Current narrative
    narrative: Narrative,
    /// Fluent timelines
    timelines: BTreeMap<FluentId, FluentTimeline>,
    /// Released fluents (non-inertial)
    released: BTreeSet<(FluentId, Time)>,
    /// Next IDs
    next_fluent_id: u32,
    next_action_id: u32,
    next_event_id: u32,
    /// Current time
    current_time: Time,
}

impl EventCalculus {
    /// Create new event calculus engine
    pub fn new() -> Self {
        Self {
            fluents: BTreeMap::new(),
            actions: BTreeMap::new(),
            narrative: Narrative::new(0, String::from("default")),
            timelines: BTreeMap::new(),
            released: BTreeSet::new(),
            next_fluent_id: 0,
            next_action_id: 0,
            next_event_id: 0,
            current_time: 0,
        }
    }

    /// Define a fluent
    pub fn define_fluent(&mut self, name: String, fluent_type: FluentType) -> FluentId {
        let id = FluentId(self.next_fluent_id);
        self.next_fluent_id += 1;

        let initial = match fluent_type {
            FluentType::Boolean => FluentValue::Bool(false),
            FluentType::Integer => FluentValue::Int(0),
            FluentType::Real => FluentValue::Real(0.0),
            FluentType::Symbolic => FluentValue::Unknown,
        };

        let fluent = Fluent {
            id,
            name,
            fluent_type,
            current_value: initial.clone(),
            initial_value: initial.clone(),
        };

        self.fluents.insert(id, fluent);
        self.timelines.insert(id, FluentTimeline::new(id, initial));

        id
    }

    /// Define an action
    pub fn define_action(&mut self, name: String) -> ActionId {
        let id = ActionId(self.next_action_id);
        self.next_action_id += 1;

        let action = Action {
            id,
            name,
            preconditions: Vec::new(),
            initiates: Vec::new(),
            terminates: Vec::new(),
            releases: Vec::new(),
        };

        self.actions.insert(id, action);
        id
    }

    /// Add precondition to action
    pub fn add_precondition(
        &mut self,
        action_id: ActionId,
        fluent: FluentId,
        value: FluentValue,
        negated: bool,
    ) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.preconditions.push(Precondition {
                fluent,
                required_value: value,
                negated,
            });
        }
    }

    /// Add initiates effect
    pub fn add_initiates(&mut self, action_id: ActionId, fluent: FluentId, value: FluentValue) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.initiates.push(Effect {
                fluent,
                value,
                condition: None,
            });
        }
    }

    /// Add terminates effect
    pub fn add_terminates(&mut self, action_id: ActionId, fluent: FluentId) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.terminates.push(Effect {
                fluent,
                value: FluentValue::Bool(false),
                condition: None,
            });
        }
    }

    /// Add releases effect
    pub fn add_releases(&mut self, action_id: ActionId, fluent: FluentId) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.releases.push(Effect {
                fluent,
                value: FluentValue::Unknown,
                condition: None,
            });
        }
    }

    /// Set initial value of fluent
    pub fn initially(&mut self, fluent: FluentId, value: FluentValue) {
        if let Some(f) = self.fluents.get_mut(&fluent) {
            f.initial_value = value.clone();
            f.current_value = value.clone();
        }
        if let Some(timeline) = self.timelines.get_mut(&fluent) {
            timeline.record_change(0, value.clone(), None);
        }
        self.narrative.set_initial(fluent, value);
    }

    /// Check if action's preconditions are satisfied at time
    pub fn can_happen(&self, action_id: ActionId, time: Time) -> bool {
        let action = match self.actions.get(&action_id) {
            Some(a) => a,
            None => return false,
        };

        for pre in &action.preconditions {
            let timeline = match self.timelines.get(&pre.fluent) {
                Some(t) => t,
                None => return false,
            };

            let current = timeline.value_at(time);
            let matches = *current == pre.required_value;

            if pre.negated && matches {
                return false;
            }
            if !pre.negated && !matches {
                return false;
            }
        }

        true
    }

    /// Execute an event (action occurrence)
    pub fn happens(
        &mut self,
        action_id: ActionId,
        time: Time,
        args: Vec<FluentValue>,
    ) -> Option<EventId> {
        if !self.can_happen(action_id, time) {
            return None;
        }

        let event_id = EventId(self.next_event_id);
        self.next_event_id += 1;

        let event = Event {
            id: event_id,
            action: action_id,
            time,
            args,
            observed: true,
        };

        // Apply effects
        self.apply_effects(action_id, time, event_id);

        // Record event
        self.narrative.add_event(event);

        // Advance time
        if time > self.current_time {
            self.current_time = time;
        }

        Some(event_id)
    }

    /// Apply effects of an action
    fn apply_effects(&mut self, action_id: ActionId, time: Time, event_id: EventId) {
        let action = match self.actions.get(&action_id) {
            Some(a) => a.clone(),
            None => return,
        };

        // Apply initiates
        for effect in &action.initiates {
            if self.check_effect_condition(&effect.condition, time) {
                if let Some(timeline) = self.timelines.get_mut(&effect.fluent) {
                    timeline.record_change(time, effect.value.clone(), Some(event_id));
                }
                if let Some(f) = self.fluents.get_mut(&effect.fluent) {
                    f.current_value = effect.value.clone();
                }
            }
        }

        // Apply terminates
        for effect in &action.terminates {
            if self.check_effect_condition(&effect.condition, time) {
                if let Some(timeline) = self.timelines.get_mut(&effect.fluent) {
                    timeline.record_change(time, FluentValue::Bool(false), Some(event_id));
                }
                if let Some(f) = self.fluents.get_mut(&effect.fluent) {
                    f.current_value = FluentValue::Bool(false);
                }
            }
        }

        // Apply releases
        for effect in &action.releases {
            if self.check_effect_condition(&effect.condition, time) {
                self.released.insert((effect.fluent, time));
            }
        }
    }

    /// Check effect condition
    fn check_effect_condition(&self, condition: &Option<Precondition>, time: Time) -> bool {
        match condition {
            None => true,
            Some(pre) => {
                let timeline = match self.timelines.get(&pre.fluent) {
                    Some(t) => t,
                    None => return false,
                };

                let current = timeline.value_at(time);
                let matches = *current == pre.required_value;

                if pre.negated { !matches } else { matches }
            },
        }
    }

    /// Query: HoldsAt(fluent, time)
    pub fn holds_at(&self, fluent: FluentId, time: Time) -> bool {
        match self.timelines.get(&fluent) {
            Some(timeline) => timeline.holds_at(time),
            None => false,
        }
    }

    /// Query: value of fluent at time
    pub fn value_at(&self, fluent: FluentId, time: Time) -> FluentValue {
        match self.timelines.get(&fluent) {
            Some(timeline) => timeline.value_at(time).clone(),
            None => FluentValue::Unknown,
        }
    }

    /// Query: what event initiated fluent at time?
    pub fn initiated_by(&self, fluent: FluentId, time: Time) -> Option<EventId> {
        self.timelines.get(&fluent)?.caused_by_at(time)
    }

    /// Query: when did fluent become true?
    pub fn when_initiated(&self, fluent: FluentId) -> Option<Time> {
        let timeline = self.timelines.get(&fluent)?;

        for (t, v, _) in &timeline.changes {
            if v.is_true() {
                return Some(*t);
            }
        }

        None
    }

    /// Query: is fluent released (non-inertial) at time?
    pub fn is_released(&self, fluent: FluentId, time: Time) -> bool {
        // Check if there's a release at or before this time
        for (f, t) in &self.released {
            if *f == fluent && *t <= time {
                return true;
            }
        }
        false
    }

    /// Project: compute fluent values at future time
    pub fn project(&self, time: Time) -> BTreeMap<FluentId, FluentValue> {
        let mut state = BTreeMap::new();

        for (&fluent_id, timeline) in &self.timelines {
            let value = if self.is_released(fluent_id, time) {
                FluentValue::Unknown
            } else {
                timeline.value_at(time).clone()
            };
            state.insert(fluent_id, value);
        }

        state
    }

    /// Abduction: find what events must have happened to explain current state
    pub fn abduce_events(
        &self,
        observed_state: &BTreeMap<FluentId, FluentValue>,
        max_time: Time,
    ) -> Vec<(ActionId, Time)> {
        let mut explanations = Vec::new();

        for (&fluent_id, observed_value) in observed_state {
            let current = self.value_at(fluent_id, max_time);

            if current != *observed_value {
                // Need to find an action that could have caused this change
                for (&action_id, action) in &self.actions {
                    // Check if action initiates this fluent to the observed value
                    for effect in &action.initiates {
                        if effect.fluent == fluent_id && effect.value == *observed_value {
                            // This action could explain the observation
                            // Find a time when it could have happened
                            for t in 1..=max_time {
                                if self.can_happen(action_id, t) {
                                    explanations.push((action_id, t));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        explanations
    }

    /// Get the narrative
    pub fn narrative(&self) -> &Narrative {
        &self.narrative
    }

    /// Get fluent by ID
    pub fn get_fluent(&self, id: FluentId) -> Option<&Fluent> {
        self.fluents.get(&id)
    }

    /// Get action by ID
    pub fn get_action(&self, id: ActionId) -> Option<&Action> {
        self.actions.get(&id)
    }

    /// Get current time
    pub fn current_time(&self) -> Time {
        self.current_time
    }
}

impl Default for EventCalculus {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// KERNEL-SPECIFIC EVENT CALCULUS
// ============================================================================

/// Kernel-specific event calculus for reasoning about OS state
pub struct KernelEventCalculus {
    /// Base event calculus
    ec: EventCalculus,
    /// Predefined fluents
    fluent_process_running: FluentId,
    fluent_memory_allocated: FluentId,
    fluent_resource_locked: FluentId,
    fluent_device_ready: FluentId,
    fluent_interrupt_pending: FluentId,
    /// Predefined actions
    action_start_process: ActionId,
    action_stop_process: ActionId,
    action_allocate_memory: ActionId,
    action_free_memory: ActionId,
    action_acquire_lock: ActionId,
    action_release_lock: ActionId,
    action_enable_interrupt: ActionId,
    action_disable_interrupt: ActionId,
}

impl KernelEventCalculus {
    /// Create kernel event calculus with predefined fluents and actions
    pub fn new() -> Self {
        let mut ec = EventCalculus::new();

        // Define kernel fluents
        let fluent_process_running =
            ec.define_fluent(String::from("process_running"), FluentType::Boolean);
        let fluent_memory_allocated =
            ec.define_fluent(String::from("memory_allocated"), FluentType::Boolean);
        let fluent_resource_locked =
            ec.define_fluent(String::from("resource_locked"), FluentType::Boolean);
        let fluent_device_ready =
            ec.define_fluent(String::from("device_ready"), FluentType::Boolean);
        let fluent_interrupt_pending =
            ec.define_fluent(String::from("interrupt_pending"), FluentType::Boolean);

        // Define kernel actions
        let action_start_process = ec.define_action(String::from("start_process"));
        ec.add_initiates(
            action_start_process,
            fluent_process_running,
            FluentValue::Bool(true),
        );

        let action_stop_process = ec.define_action(String::from("stop_process"));
        ec.add_precondition(
            action_stop_process,
            fluent_process_running,
            FluentValue::Bool(true),
            false,
        );
        ec.add_terminates(action_stop_process, fluent_process_running);

        let action_allocate_memory = ec.define_action(String::from("allocate_memory"));
        ec.add_initiates(
            action_allocate_memory,
            fluent_memory_allocated,
            FluentValue::Bool(true),
        );

        let action_free_memory = ec.define_action(String::from("free_memory"));
        ec.add_precondition(
            action_free_memory,
            fluent_memory_allocated,
            FluentValue::Bool(true),
            false,
        );
        ec.add_terminates(action_free_memory, fluent_memory_allocated);

        let action_acquire_lock = ec.define_action(String::from("acquire_lock"));
        ec.add_precondition(
            action_acquire_lock,
            fluent_resource_locked,
            FluentValue::Bool(true),
            true,
        ); // Not already locked
        ec.add_initiates(
            action_acquire_lock,
            fluent_resource_locked,
            FluentValue::Bool(true),
        );

        let action_release_lock = ec.define_action(String::from("release_lock"));
        ec.add_precondition(
            action_release_lock,
            fluent_resource_locked,
            FluentValue::Bool(true),
            false,
        );
        ec.add_terminates(action_release_lock, fluent_resource_locked);

        let action_enable_interrupt = ec.define_action(String::from("enable_interrupt"));
        ec.add_terminates(action_enable_interrupt, fluent_interrupt_pending);

        let action_disable_interrupt = ec.define_action(String::from("disable_interrupt"));
        ec.add_initiates(
            action_disable_interrupt,
            fluent_interrupt_pending,
            FluentValue::Bool(true),
        );

        Self {
            ec,
            fluent_process_running,
            fluent_memory_allocated,
            fluent_resource_locked,
            fluent_device_ready,
            fluent_interrupt_pending,
            action_start_process,
            action_stop_process,
            action_allocate_memory,
            action_free_memory,
            action_acquire_lock,
            action_release_lock,
            action_enable_interrupt,
            action_disable_interrupt,
        }
    }

    /// Start a process
    pub fn start_process(&mut self, time: Time) -> Option<EventId> {
        self.ec.happens(self.action_start_process, time, vec![])
    }

    /// Stop a process
    pub fn stop_process(&mut self, time: Time) -> Option<EventId> {
        self.ec.happens(self.action_stop_process, time, vec![])
    }

    /// Allocate memory
    pub fn allocate_memory(&mut self, time: Time) -> Option<EventId> {
        self.ec.happens(self.action_allocate_memory, time, vec![])
    }

    /// Free memory
    pub fn free_memory(&mut self, time: Time) -> Option<EventId> {
        self.ec.happens(self.action_free_memory, time, vec![])
    }

    /// Acquire lock
    pub fn acquire_lock(&mut self, time: Time) -> Option<EventId> {
        self.ec.happens(self.action_acquire_lock, time, vec![])
    }

    /// Release lock
    pub fn release_lock(&mut self, time: Time) -> Option<EventId> {
        self.ec.happens(self.action_release_lock, time, vec![])
    }

    /// Check if process is running at time
    pub fn is_process_running(&self, time: Time) -> bool {
        self.ec.holds_at(self.fluent_process_running, time)
    }

    /// Check if memory is allocated at time
    pub fn is_memory_allocated(&self, time: Time) -> bool {
        self.ec.holds_at(self.fluent_memory_allocated, time)
    }

    /// Check if resource is locked at time
    pub fn is_locked(&self, time: Time) -> bool {
        self.ec.holds_at(self.fluent_resource_locked, time)
    }

    /// Get full state projection at time
    pub fn state_at(&self, time: Time) -> KernelState {
        KernelState {
            process_running: self.ec.holds_at(self.fluent_process_running, time),
            memory_allocated: self.ec.holds_at(self.fluent_memory_allocated, time),
            resource_locked: self.ec.holds_at(self.fluent_resource_locked, time),
            device_ready: self.ec.holds_at(self.fluent_device_ready, time),
            interrupt_pending: self.ec.holds_at(self.fluent_interrupt_pending, time),
            time,
        }
    }

    /// Detect potential deadlock
    pub fn detect_deadlock(&self, time: Time) -> bool {
        // Simple deadlock detection: locked but process not running
        self.is_locked(time) && !self.is_process_running(time)
    }

    /// Detect memory leak
    pub fn detect_memory_leak(&self, time: Time) -> bool {
        // Memory allocated but process not running
        self.is_memory_allocated(time) && !self.is_process_running(time)
    }

    /// Get underlying engine
    pub fn engine(&self) -> &EventCalculus {
        &self.ec
    }
}

impl Default for KernelEventCalculus {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel state at a point in time
#[derive(Debug, Clone)]
pub struct KernelState {
    /// Is a process running?
    pub process_running: bool,
    /// Is memory allocated?
    pub memory_allocated: bool,
    /// Is a resource locked?
    pub resource_locked: bool,
    /// Is device ready?
    pub device_ready: bool,
    /// Is interrupt pending?
    pub interrupt_pending: bool,
    /// Time
    pub time: Time,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fluent_timeline() {
        let fluent = FluentId(0);
        let mut timeline = FluentTimeline::new(fluent, FluentValue::Bool(false));

        timeline.record_change(5, FluentValue::Bool(true), Some(EventId(0)));
        timeline.record_change(10, FluentValue::Bool(false), Some(EventId(1)));

        assert!(!timeline.holds_at(3));
        assert!(timeline.holds_at(5));
        assert!(timeline.holds_at(7));
        assert!(!timeline.holds_at(10));
        assert!(!timeline.holds_at(15));
    }

    #[test]
    fn test_event_calculus() {
        let mut ec = EventCalculus::new();

        // Define fluent and action
        let light = ec.define_fluent(String::from("light_on"), FluentType::Boolean);
        let switch = ec.define_action(String::from("flip_switch"));

        ec.add_initiates(switch, light, FluentValue::Bool(true));

        // Initially light is off
        assert!(!ec.holds_at(light, 0));

        // Flip switch at time 5
        ec.happens(switch, 5, vec![]);

        // Now light is on
        assert!(!ec.holds_at(light, 3));
        assert!(ec.holds_at(light, 5));
        assert!(ec.holds_at(light, 10)); // Inertia!
    }

    #[test]
    fn test_preconditions() {
        let mut ec = EventCalculus::new();

        let door_open = ec.define_fluent(String::from("door_open"), FluentType::Boolean);
        let enter = ec.define_action(String::from("enter"));

        // Can only enter if door is open
        ec.add_precondition(enter, door_open, FluentValue::Bool(true), false);

        // Door is initially closed
        assert!(!ec.can_happen(enter, 0));

        // Open door
        let open = ec.define_action(String::from("open_door"));
        ec.add_initiates(open, door_open, FluentValue::Bool(true));
        ec.happens(open, 5, vec![]);

        // Now can enter
        assert!(ec.can_happen(enter, 5));
    }

    #[test]
    fn test_kernel_event_calculus() {
        let mut kec = KernelEventCalculus::new();

        // Initially nothing running
        assert!(!kec.is_process_running(0));
        assert!(!kec.is_locked(0));

        // Start process
        kec.start_process(10);
        assert!(kec.is_process_running(10));
        assert!(kec.is_process_running(15));

        // Acquire lock
        kec.acquire_lock(20);
        assert!(kec.is_locked(20));

        // Stop process
        kec.stop_process(30);
        assert!(!kec.is_process_running(30));

        // Deadlock detected (lock held but no process)
        assert!(kec.detect_deadlock(30));
    }

    #[test]
    fn test_narrative() {
        let mut narrative = Narrative::new(0, String::from("test"));

        narrative.add_event(Event {
            id: EventId(0),
            action: ActionId(0),
            time: 10,
            args: vec![],
            observed: true,
        });

        narrative.add_event(Event {
            id: EventId(1),
            action: ActionId(0),
            time: 5, // Earlier!
            args: vec![],
            observed: true,
        });

        // Should be in chronological order
        assert_eq!(narrative.events[0].time, 5);
        assert_eq!(narrative.events[1].time, 10);
    }

    #[test]
    fn test_fluent_value() {
        assert!(FluentValue::Bool(true).is_true());
        assert!(FluentValue::Bool(false).is_false());
        assert_eq!(FluentValue::Int(42).as_int(), Some(42));
        assert_eq!(FluentValue::Real(3.14).as_real(), Some(3.14));
    }
}

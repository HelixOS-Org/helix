//! # Reactive Planning for NEXUS
//!
//! Reactive planning with triggers, replanning, and adaptation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::actions::{ActionId, WorldState};
use super::goals::GoalId;

// ============================================================================
// TRIGGER TYPES
// ============================================================================

/// Trigger identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TriggerId(pub u32);

/// Response identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResponseId(pub u32);

/// Trigger condition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerConditionType {
    /// State variable changed
    StateChanged,
    /// State variable equals value
    StateEquals,
    /// State variable threshold crossed
    ThresholdCrossed,
    /// Goal status changed
    GoalStatusChanged,
    /// Time elapsed
    TimeElapsed,
    /// Event occurred
    EventOccurred,
    /// Always true (polling)
    Always,
}

/// A trigger condition
#[derive(Debug, Clone)]
pub struct TriggerCondition {
    /// Condition type
    pub condition_type: TriggerConditionType,
    /// Variable to monitor
    pub variable: String,
    /// Threshold value
    pub threshold: f64,
    /// Check function (optional custom logic)
    check_fn: Option<fn(&WorldState) -> bool>,
}

impl TriggerCondition {
    /// Create state changed condition
    pub fn state_changed(variable: String) -> Self {
        Self {
            condition_type: TriggerConditionType::StateChanged,
            variable,
            threshold: 0.0,
            check_fn: None,
        }
    }

    /// Create threshold condition
    pub fn threshold(variable: String, threshold: f64) -> Self {
        Self {
            condition_type: TriggerConditionType::ThresholdCrossed,
            variable,
            threshold,
            check_fn: None,
        }
    }

    /// Create always-true condition
    pub fn always() -> Self {
        Self {
            condition_type: TriggerConditionType::Always,
            variable: String::new(),
            threshold: 0.0,
            check_fn: None,
        }
    }

    /// Check condition
    pub fn check(&self, state: &WorldState, prev_state: Option<&WorldState>) -> bool {
        if let Some(f) = self.check_fn {
            return f(state);
        }

        match self.condition_type {
            TriggerConditionType::Always => true,
            TriggerConditionType::StateChanged => {
                if let Some(prev) = prev_state {
                    let curr = state.get_bool(&self.variable);
                    let old = prev.get_bool(&self.variable);
                    curr != old
                } else {
                    false
                }
            },
            TriggerConditionType::ThresholdCrossed => {
                if let Some(prev) = prev_state {
                    let curr = state.get_int(&self.variable).map(|v| v as f64);
                    let old = prev.get_int(&self.variable).map(|v| v as f64);

                    match (curr, old) {
                        (Some(c), Some(o)) => {
                            (c >= self.threshold && o < self.threshold)
                                || (c < self.threshold && o >= self.threshold)
                        },
                        _ => false,
                    }
                } else {
                    false
                }
            },
            _ => false,
        }
    }
}

/// A trigger for reactive behavior
#[derive(Debug, Clone)]
pub struct Trigger {
    /// Trigger ID
    pub id: TriggerId,
    /// Trigger name
    pub name: String,
    /// Condition
    pub condition: TriggerCondition,
    /// Priority
    pub priority: u32,
    /// Is enabled
    pub enabled: bool,
    /// Fire count
    pub fire_count: u64,
    /// Last fired timestamp
    pub last_fired: u64,
    /// Minimum time between firings
    pub cooldown: u64,
}

impl Trigger {
    /// Create new trigger
    pub fn new(id: TriggerId, name: String, condition: TriggerCondition) -> Self {
        Self {
            id,
            name,
            condition,
            priority: 50,
            enabled: true,
            fire_count: 0,
            last_fired: 0,
            cooldown: 0,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set cooldown
    pub fn with_cooldown(mut self, cooldown: u64) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Check if can fire
    pub fn can_fire(&self, current_time: u64) -> bool {
        self.enabled && (current_time >= self.last_fired + self.cooldown)
    }

    /// Fire trigger
    pub fn fire(&mut self, current_time: u64) {
        self.fire_count += 1;
        self.last_fired = current_time;
    }
}

// ============================================================================
// RESPONSE
// ============================================================================

/// Response type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    /// Execute action
    ExecuteAction,
    /// Adopt goal
    AdoptGoal,
    /// Abandon goal
    AbandonGoal,
    /// Replan
    Replan,
    /// Notify
    Notify,
    /// Custom handler
    Custom,
}

/// A response to a trigger
#[derive(Debug, Clone)]
pub struct Response {
    /// Response ID
    pub id: ResponseId,
    /// Response name
    pub name: String,
    /// Response type
    pub response_type: ResponseType,
    /// Action to execute (if ExecuteAction)
    pub action: Option<ActionId>,
    /// Goal to adopt/abandon (if goal-related)
    pub goal: Option<GoalId>,
    /// Parameters
    pub params: BTreeMap<String, f64>,
}

impl Response {
    /// Create execute action response
    pub fn execute(id: ResponseId, name: String, action: ActionId) -> Self {
        Self {
            id,
            name,
            response_type: ResponseType::ExecuteAction,
            action: Some(action),
            goal: None,
            params: BTreeMap::new(),
        }
    }

    /// Create adopt goal response
    pub fn adopt_goal(id: ResponseId, name: String, goal: GoalId) -> Self {
        Self {
            id,
            name,
            response_type: ResponseType::AdoptGoal,
            action: None,
            goal: Some(goal),
            params: BTreeMap::new(),
        }
    }

    /// Create replan response
    pub fn replan(id: ResponseId, name: String) -> Self {
        Self {
            id,
            name,
            response_type: ResponseType::Replan,
            action: None,
            goal: None,
            params: BTreeMap::new(),
        }
    }
}

// ============================================================================
// REACTIVE PLANNER
// ============================================================================

/// Rule linking trigger to response
#[derive(Debug, Clone)]
pub struct ReactiveRule {
    /// Trigger ID
    pub trigger: TriggerId,
    /// Response ID
    pub response: ResponseId,
    /// Is enabled
    pub enabled: bool,
}

/// Reactive planner
pub struct ReactivePlanner {
    /// Triggers
    triggers: BTreeMap<TriggerId, Trigger>,
    /// Responses
    responses: BTreeMap<ResponseId, Response>,
    /// Rules
    rules: Vec<ReactiveRule>,
    /// Previous state (for change detection)
    prev_state: Option<WorldState>,
    /// Current time
    current_time: u64,
    /// Next IDs
    next_trigger_id: u32,
    next_response_id: u32,
    /// Fired triggers history
    fire_history: Vec<(u64, TriggerId)>,
    /// Max history size
    max_history: usize,
}

impl ReactivePlanner {
    /// Create new reactive planner
    pub fn new() -> Self {
        Self {
            triggers: BTreeMap::new(),
            responses: BTreeMap::new(),
            rules: Vec::new(),
            prev_state: None,
            current_time: 0,
            next_trigger_id: 0,
            next_response_id: 0,
            fire_history: Vec::new(),
            max_history: 100,
        }
    }

    /// Add trigger
    pub fn add_trigger(&mut self, trigger: Trigger) -> TriggerId {
        let id = trigger.id;
        self.triggers.insert(id, trigger);
        id
    }

    /// Create trigger
    pub fn create_trigger(&mut self, name: String, condition: TriggerCondition) -> TriggerId {
        let id = TriggerId(self.next_trigger_id);
        self.next_trigger_id += 1;
        let trigger = Trigger::new(id, name, condition);
        self.add_trigger(trigger)
    }

    /// Add response
    pub fn add_response(&mut self, response: Response) -> ResponseId {
        let id = response.id;
        self.responses.insert(id, response);
        id
    }

    /// Create response
    pub fn create_response(&mut self, name: String, response_type: ResponseType) -> ResponseId {
        let id = ResponseId(self.next_response_id);
        self.next_response_id += 1;
        let response = Response {
            id,
            name,
            response_type,
            action: None,
            goal: None,
            params: BTreeMap::new(),
        };
        self.add_response(response)
    }

    /// Link trigger to response
    pub fn link(&mut self, trigger: TriggerId, response: ResponseId) {
        self.rules.push(ReactiveRule {
            trigger,
            response,
            enabled: true,
        });
    }

    /// Update with new state
    pub fn update(&mut self, state: &WorldState) -> Vec<&Response> {
        let mut responses = Vec::new();
        let mut fired_triggers = Vec::new();

        // Check all triggers
        for (&id, trigger) in &mut self.triggers {
            if !trigger.can_fire(self.current_time) {
                continue;
            }

            if trigger.condition.check(state, self.prev_state.as_ref()) {
                fired_triggers.push(id);
            }
        }

        // Sort by priority
        fired_triggers.sort_by(|a, b| {
            let pa = self.triggers.get(a).map(|t| t.priority).unwrap_or(0);
            let pb = self.triggers.get(b).map(|t| t.priority).unwrap_or(0);
            pb.cmp(&pa) // Higher priority first
        });

        // Fire triggers and collect responses
        for trigger_id in fired_triggers {
            // Update trigger
            if let Some(trigger) = self.triggers.get_mut(&trigger_id) {
                trigger.fire(self.current_time);
            }

            // Record history
            if self.fire_history.len() >= self.max_history {
                self.fire_history.remove(0);
            }
            self.fire_history.push((self.current_time, trigger_id));

            // Find matching rules
            for rule in &self.rules {
                if rule.enabled && rule.trigger == trigger_id {
                    if let Some(response) = self.responses.get(&rule.response) {
                        responses.push(response);
                    }
                }
            }
        }

        // Update previous state
        self.prev_state = Some(state.clone());

        responses
    }

    /// Advance time
    pub fn advance_time(&mut self, delta: u64) {
        self.current_time += delta;
    }

    /// Get trigger
    pub fn get_trigger(&self, id: TriggerId) -> Option<&Trigger> {
        self.triggers.get(&id)
    }

    /// Enable/disable trigger
    pub fn set_trigger_enabled(&mut self, id: TriggerId, enabled: bool) {
        if let Some(trigger) = self.triggers.get_mut(&id) {
            trigger.enabled = enabled;
        }
    }

    /// Get fire history
    pub fn fire_history(&self) -> &[(u64, TriggerId)] {
        &self.fire_history
    }

    /// Get trigger count
    pub fn trigger_count(&self) -> usize {
        self.triggers.len()
    }
}

impl Default for ReactivePlanner {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REPLANNING
// ============================================================================

/// Replan trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplanReason {
    /// Action failed
    ActionFailed,
    /// State changed unexpectedly
    StateChanged,
    /// New goal adopted
    NewGoal,
    /// Goal abandoned
    GoalAbandoned,
    /// Timeout
    Timeout,
    /// Forced
    Forced,
}

/// Replan event
#[derive(Debug, Clone)]
pub struct ReplanEvent {
    /// Reason for replanning
    pub reason: ReplanReason,
    /// Timestamp
    pub timestamp: u64,
    /// Failed action (if applicable)
    pub failed_action: Option<ActionId>,
    /// New goal (if applicable)
    pub new_goal: Option<GoalId>,
}

/// Replan manager
pub struct ReplanManager {
    /// Replan events
    events: Vec<ReplanEvent>,
    /// Last replan time
    last_replan: u64,
    /// Minimum time between replans
    min_interval: u64,
    /// Replan count
    replan_count: u64,
}

impl ReplanManager {
    /// Create new replan manager
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            last_replan: 0,
            min_interval: 10,
            replan_count: 0,
        }
    }

    /// Request replan
    pub fn request_replan(&mut self, reason: ReplanReason, timestamp: u64) -> bool {
        if timestamp < self.last_replan + self.min_interval {
            return false; // Too soon
        }

        self.events.push(ReplanEvent {
            reason,
            timestamp,
            failed_action: None,
            new_goal: None,
        });

        self.last_replan = timestamp;
        self.replan_count += 1;
        true
    }

    /// Request replan due to action failure
    pub fn action_failed(&mut self, action: ActionId, timestamp: u64) -> bool {
        if timestamp < self.last_replan + self.min_interval {
            return false;
        }

        self.events.push(ReplanEvent {
            reason: ReplanReason::ActionFailed,
            timestamp,
            failed_action: Some(action),
            new_goal: None,
        });

        self.last_replan = timestamp;
        self.replan_count += 1;
        true
    }

    /// Get pending replan events
    pub fn pending_events(&mut self) -> Vec<ReplanEvent> {
        core::mem::take(&mut self.events)
    }

    /// Get replan count
    pub fn replan_count(&self) -> u64 {
        self.replan_count
    }

    /// Set minimum interval
    pub fn set_min_interval(&mut self, interval: u64) {
        self.min_interval = interval;
    }
}

impl Default for ReplanManager {
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
    fn test_trigger_condition() {
        let cond = TriggerCondition::always();
        let state = WorldState::new();
        assert!(cond.check(&state, None));
    }

    #[test]
    fn test_trigger() {
        let cond = TriggerCondition::always();
        let mut trigger = Trigger::new(TriggerId(0), String::from("test"), cond).with_cooldown(10);

        assert!(trigger.can_fire(0));
        trigger.fire(0);
        assert!(!trigger.can_fire(5));
        assert!(trigger.can_fire(10));
    }

    #[test]
    fn test_reactive_planner() {
        let mut planner = ReactivePlanner::new();

        let trigger =
            planner.create_trigger(String::from("always_trigger"), TriggerCondition::always());

        let response = planner.create_response(String::from("replan"), ResponseType::Replan);

        planner.link(trigger, response);

        let state = WorldState::new();
        let responses = planner.update(&state);

        assert!(!responses.is_empty());
        assert_eq!(responses[0].response_type, ResponseType::Replan);
    }

    #[test]
    fn test_state_change_trigger() {
        let mut planner = ReactivePlanner::new();

        let trigger = planner.create_trigger(
            String::from("state_change"),
            TriggerCondition::state_changed(String::from("alarm")),
        );

        let response = planner.create_response(String::from("handle_alarm"), ResponseType::Notify);

        planner.link(trigger, response);

        let mut state1 = WorldState::new();
        state1.set_bool("alarm", false);

        let responses1 = planner.update(&state1);
        assert!(responses1.is_empty()); // First update, no previous state

        let mut state2 = WorldState::new();
        state2.set_bool("alarm", true);

        planner.advance_time(1);
        let responses2 = planner.update(&state2);
        assert!(!responses2.is_empty()); // State changed!
    }

    #[test]
    fn test_replan_manager() {
        let mut manager = ReplanManager::new();
        manager.set_min_interval(5);

        assert!(manager.request_replan(ReplanReason::StateChanged, 0));
        assert!(!manager.request_replan(ReplanReason::StateChanged, 3)); // Too soon
        assert!(manager.request_replan(ReplanReason::StateChanged, 10));

        assert_eq!(manager.replan_count(), 2);
    }
}

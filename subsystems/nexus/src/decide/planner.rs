//! # Decision Planning
//!
//! Plans sequences of actions to achieve goals.
//! Supports hierarchical and reactive planning.
//!
//! Part of Year 2 COGNITION - Decision-Making Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// PLANNING TYPES
// ============================================================================

/// Goal
#[derive(Debug, Clone)]
pub struct Goal {
    /// Goal ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Priority
    pub priority: Priority,
    /// Status
    pub status: GoalStatus,
    /// Deadline
    pub deadline: Option<Timestamp>,
    /// Preconditions
    pub preconditions: Vec<Condition>,
    /// Success conditions
    pub success_conditions: Vec<Condition>,
    /// Sub-goals
    pub sub_goals: Vec<u64>,
}

/// Priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Goal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalStatus {
    Pending,
    Active,
    Achieved,
    Failed,
    Abandoned,
}

/// Condition
#[derive(Debug, Clone)]
pub struct Condition {
    /// Condition ID
    pub id: u64,
    /// Variable
    pub variable: String,
    /// Operator
    pub operator: Operator,
    /// Value
    pub value: Value,
}

/// Operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    Contains,
    Exists,
}

/// Value
#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// Action plan
#[derive(Debug, Clone)]
pub struct Plan {
    /// Plan ID
    pub id: u64,
    /// Goal ID
    pub goal_id: u64,
    /// Actions
    pub actions: Vec<PlannedAction>,
    /// Status
    pub status: PlanStatus,
    /// Created
    pub created: Timestamp,
    /// Cost estimate
    pub estimated_cost: f64,
    /// Duration estimate (ns)
    pub estimated_duration_ns: u64,
}

/// Plan status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    Draft,
    Ready,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

/// Planned action
#[derive(Debug, Clone)]
pub struct PlannedAction {
    /// Action ID
    pub id: u64,
    /// Action type
    pub action_type: ActionType,
    /// Parameters
    pub parameters: BTreeMap<String, Value>,
    /// Preconditions
    pub preconditions: Vec<Condition>,
    /// Effects
    pub effects: Vec<Effect>,
    /// Cost
    pub cost: f64,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Status
    pub status: ActionStatus,
}

/// Action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Compute,
    Query,
    Transform,
    Store,
    Retrieve,
    Communicate,
    Wait,
    Branch,
    Loop,
}

/// Action status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Effect
#[derive(Debug, Clone)]
pub struct Effect {
    /// Variable
    pub variable: String,
    /// Operation
    pub operation: EffectOp,
    /// Value
    pub value: Value,
}

/// Effect operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectOp {
    Set,
    Add,
    Remove,
    Clear,
}

// ============================================================================
// PLANNER
// ============================================================================

/// Planner
pub struct Planner {
    /// Goals
    goals: BTreeMap<u64, Goal>,
    /// Plans
    plans: BTreeMap<u64, Plan>,
    /// Action templates
    action_templates: BTreeMap<String, ActionTemplate>,
    /// World state
    world_state: BTreeMap<String, Value>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: PlannerConfig,
    /// Statistics
    stats: PlannerStats,
}

/// Action template
#[derive(Debug, Clone)]
pub struct ActionTemplate {
    /// Name
    pub name: String,
    /// Action type
    pub action_type: ActionType,
    /// Required parameters
    pub parameters: Vec<String>,
    /// Preconditions
    pub preconditions: Vec<Condition>,
    /// Effects
    pub effects: Vec<Effect>,
    /// Base cost
    pub cost: f64,
    /// Base duration (ns)
    pub duration_ns: u64,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Maximum plan depth
    pub max_depth: usize,
    /// Maximum actions per plan
    pub max_actions: usize,
    /// Enable optimization
    pub optimize: bool,
    /// Timeout (ns)
    pub timeout_ns: u64,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            max_actions: 50,
            optimize: true,
            timeout_ns: 5_000_000_000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct PlannerStats {
    /// Goals created
    pub goals_created: u64,
    /// Plans created
    pub plans_created: u64,
    /// Plans executed
    pub plans_executed: u64,
    /// Plans succeeded
    pub plans_succeeded: u64,
}

impl Planner {
    /// Create new planner
    pub fn new(config: PlannerConfig) -> Self {
        Self {
            goals: BTreeMap::new(),
            plans: BTreeMap::new(),
            action_templates: BTreeMap::new(),
            world_state: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: PlannerStats::default(),
        }
    }

    /// Create goal
    pub fn create_goal(
        &mut self,
        name: &str,
        description: &str,
        priority: Priority,
        success_conditions: Vec<Condition>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let goal = Goal {
            id,
            name: name.into(),
            description: description.into(),
            priority,
            status: GoalStatus::Pending,
            deadline: None,
            preconditions: Vec::new(),
            success_conditions,
            sub_goals: Vec::new(),
        };

        self.goals.insert(id, goal);
        self.stats.goals_created += 1;

        id
    }

    /// Register action template
    pub fn register_action(&mut self, template: ActionTemplate) {
        self.action_templates.insert(template.name.clone(), template);
    }

    /// Set world state
    pub fn set_state(&mut self, variable: &str, value: Value) {
        self.world_state.insert(variable.into(), value);
    }

    /// Get world state
    pub fn get_state(&self, variable: &str) -> Option<&Value> {
        self.world_state.get(variable)
    }

    /// Plan for goal
    pub fn plan(&mut self, goal_id: u64) -> Option<u64> {
        let goal = self.goals.get(&goal_id)?.clone();

        // Check if goal already achieved
        if self.conditions_satisfied(&goal.success_conditions) {
            if let Some(g) = self.goals.get_mut(&goal_id) {
                g.status = GoalStatus::Achieved;
            }
            return None;
        }

        // Simple forward planning
        let actions = self.find_actions_for_goal(&goal);

        if actions.is_empty() {
            return None;
        }

        let plan_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let total_cost: f64 = actions.iter().map(|a| a.cost).sum();
        let total_duration: u64 = actions.iter().map(|a| a.duration_ns).sum();

        let plan = Plan {
            id: plan_id,
            goal_id,
            actions,
            status: PlanStatus::Ready,
            created: Timestamp::now(),
            estimated_cost: total_cost,
            estimated_duration_ns: total_duration,
        };

        self.plans.insert(plan_id, plan);
        self.stats.plans_created += 1;

        if let Some(g) = self.goals.get_mut(&goal_id) {
            g.status = GoalStatus::Active;
        }

        Some(plan_id)
    }

    fn conditions_satisfied(&self, conditions: &[Condition]) -> bool {
        conditions.iter().all(|c| self.evaluate_condition(c))
    }

    fn evaluate_condition(&self, condition: &Condition) -> bool {
        let current = match self.world_state.get(&condition.variable) {
            Some(v) => v,
            None => return condition.operator == Operator::Exists && matches!(condition.value, Value::Bool(false)),
        };

        match (&condition.operator, current, &condition.value) {
            (Operator::Equal, Value::Bool(a), Value::Bool(b)) => a == b,
            (Operator::Equal, Value::Int(a), Value::Int(b)) => a == b,
            (Operator::Equal, Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Operator::Equal, Value::String(a), Value::String(b)) => a == b,

            (Operator::NotEqual, Value::Bool(a), Value::Bool(b)) => a != b,
            (Operator::NotEqual, Value::Int(a), Value::Int(b)) => a != b,

            (Operator::GreaterThan, Value::Int(a), Value::Int(b)) => a > b,
            (Operator::GreaterThan, Value::Float(a), Value::Float(b)) => a > b,

            (Operator::LessThan, Value::Int(a), Value::Int(b)) => a < b,
            (Operator::LessThan, Value::Float(a), Value::Float(b)) => a < b,

            (Operator::Exists, _, Value::Bool(true)) => true,

            _ => false,
        }
    }

    fn find_actions_for_goal(&mut self, goal: &Goal) -> Vec<PlannedAction> {
        let mut actions = Vec::new();
        let mut simulated_state = self.world_state.clone();

        // For each success condition, find actions that achieve it
        for condition in &goal.success_conditions {
            if let Some(template) = self.find_action_for_condition(condition) {
                let action_id = self.next_id.fetch_add(1, Ordering::Relaxed);

                let action = PlannedAction {
                    id: action_id,
                    action_type: template.action_type,
                    parameters: BTreeMap::new(),
                    preconditions: template.preconditions.clone(),
                    effects: template.effects.clone(),
                    cost: template.cost,
                    duration_ns: template.duration_ns,
                    status: ActionStatus::Pending,
                };

                // Simulate effects
                for effect in &action.effects {
                    simulated_state.insert(effect.variable.clone(), effect.value.clone());
                }

                actions.push(action);

                if actions.len() >= self.config.max_actions {
                    break;
                }
            }
        }

        // Optimize if enabled
        if self.config.optimize {
            self.optimize_actions(&mut actions);
        }

        actions
    }

    fn find_action_for_condition(&self, condition: &Condition) -> Option<&ActionTemplate> {
        // Find template with effect that satisfies condition
        self.action_templates.values().find(|t| {
            t.effects.iter().any(|e| {
                e.variable == condition.variable &&
                self.effect_satisfies_condition(e, condition)
            })
        })
    }

    fn effect_satisfies_condition(&self, effect: &Effect, condition: &Condition) -> bool {
        match (&effect.operation, &condition.operator) {
            (EffectOp::Set, Operator::Equal) => {
                self.values_match(&effect.value, &condition.value)
            }
            _ => false,
        }
    }

    fn values_match(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => (x - y).abs() < f64::EPSILON,
            (Value::String(x), Value::String(y)) => x == y,
            _ => false,
        }
    }

    fn optimize_actions(&self, actions: &mut Vec<PlannedAction>) {
        // Remove duplicate effects
        let mut seen_effects: alloc::collections::BTreeSet<String> = alloc::collections::BTreeSet::new();

        actions.retain(|action| {
            let mut dominated = true;
            for effect in &action.effects {
                if !seen_effects.contains(&effect.variable) {
                    seen_effects.insert(effect.variable.clone());
                    dominated = false;
                }
            }
            !dominated
        });

        // Sort by cost
        actions.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap());
    }

    /// Execute plan
    pub fn execute(&mut self, plan_id: u64) -> Result<(), String> {
        let plan = self.plans.get_mut(&plan_id)
            .ok_or("Plan not found")?;

        plan.status = PlanStatus::Executing;
        self.stats.plans_executed += 1;

        for action in &mut plan.actions {
            // Check preconditions
            if !self.conditions_satisfied(&action.preconditions) {
                action.status = ActionStatus::Failed;
                plan.status = PlanStatus::Failed;
                return Err("Precondition not satisfied".into());
            }

            action.status = ActionStatus::Running;

            // Apply effects
            for effect in &action.effects {
                match effect.operation {
                    EffectOp::Set => {
                        self.world_state.insert(effect.variable.clone(), effect.value.clone());
                    }
                    EffectOp::Clear => {
                        self.world_state.remove(&effect.variable);
                    }
                    _ => {}
                }
            }

            action.status = ActionStatus::Completed;
        }

        plan.status = PlanStatus::Completed;
        self.stats.plans_succeeded += 1;

        // Update goal
        if let Some(goal) = self.goals.get_mut(&plan.goal_id) {
            if self.conditions_satisfied(&goal.success_conditions) {
                goal.status = GoalStatus::Achieved;
            }
        }

        Ok(())
    }

    /// Get goal
    pub fn get_goal(&self, id: u64) -> Option<&Goal> {
        self.goals.get(&id)
    }

    /// Get plan
    pub fn get_plan(&self, id: u64) -> Option<&Plan> {
        self.plans.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &PlannerStats {
        &self.stats
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self::new(PlannerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_goal() {
        let mut planner = Planner::default();

        let id = planner.create_goal(
            "test",
            "Test goal",
            Priority::Normal,
            vec![],
        );

        assert!(planner.get_goal(id).is_some());
    }

    #[test]
    fn test_register_action() {
        let mut planner = Planner::default();

        planner.register_action(ActionTemplate {
            name: "set_flag".into(),
            action_type: ActionType::Compute,
            parameters: vec![],
            preconditions: vec![],
            effects: vec![Effect {
                variable: "flag".into(),
                operation: EffectOp::Set,
                value: Value::Bool(true),
            }],
            cost: 1.0,
            duration_ns: 1000,
        });

        assert!(planner.action_templates.contains_key("set_flag"));
    }

    #[test]
    fn test_plan_and_execute() {
        let mut planner = Planner::default();

        // Register action
        planner.register_action(ActionTemplate {
            name: "enable".into(),
            action_type: ActionType::Compute,
            parameters: vec![],
            preconditions: vec![],
            effects: vec![Effect {
                variable: "enabled".into(),
                operation: EffectOp::Set,
                value: Value::Bool(true),
            }],
            cost: 1.0,
            duration_ns: 1000,
        });

        // Create goal
        let goal_id = planner.create_goal(
            "enable_feature",
            "Enable the feature",
            Priority::Normal,
            vec![Condition {
                id: 1,
                variable: "enabled".into(),
                operator: Operator::Equal,
                value: Value::Bool(true),
            }],
        );

        // Plan
        let plan_id = planner.plan(goal_id);
        assert!(plan_id.is_some());

        // Execute
        let result = planner.execute(plan_id.unwrap());
        assert!(result.is_ok());

        // Check state
        assert!(matches!(planner.get_state("enabled"), Some(Value::Bool(true))));
    }

    #[test]
    fn test_goal_already_achieved() {
        let mut planner = Planner::default();

        planner.set_state("done", Value::Bool(true));

        let goal_id = planner.create_goal(
            "check",
            "Already done",
            Priority::Normal,
            vec![Condition {
                id: 1,
                variable: "done".into(),
                operator: Operator::Equal,
                value: Value::Bool(true),
            }],
        );

        // Should return None since already achieved
        let plan_id = planner.plan(goal_id);
        assert!(plan_id.is_none());

        let goal = planner.get_goal(goal_id).unwrap();
        assert_eq!(goal.status, GoalStatus::Achieved);
    }
}

//! # Action Executor
//!
//! Executes planned actions with monitoring.
//! Manages action lifecycle and coordination.
//!
//! Part of Year 2 COGNITION - Act/Execute

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// EXECUTION TYPES
// ============================================================================

/// Action
#[derive(Debug, Clone)]
pub struct Action {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Type
    pub action_type: ActionType,
    /// Parameters
    pub params: BTreeMap<String, ParamValue>,
    /// Priority
    pub priority: u32,
    /// Status
    pub status: ActionStatus,
    /// Created
    pub created: Timestamp,
}

/// Action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    Compute,
    Store,
    Retrieve,
    Send,
    Receive,
    Transform,
    Validate,
    Coordinate,
}

/// Parameter value
#[derive(Debug, Clone)]
pub enum ParamValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
}

/// Action status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Pending,
    Ready,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Action ID
    pub action_id: u64,
    /// Success
    pub success: bool,
    /// Output
    pub output: Option<ParamValue>,
    /// Duration ns
    pub duration_ns: u64,
    /// Error
    pub error: Option<String>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Context ID
    pub id: u64,
    /// Variables
    pub variables: BTreeMap<String, ParamValue>,
    /// Resources
    pub resources: BTreeMap<String, ResourceState>,
}

/// Resource state
#[derive(Debug, Clone)]
pub struct ResourceState {
    /// Name
    pub name: String,
    /// Available
    pub available: bool,
    /// Capacity
    pub capacity: u64,
    /// Used
    pub used: u64,
}

/// Action plan
#[derive(Debug, Clone)]
pub struct ActionPlan {
    /// Plan ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Steps
    pub steps: Vec<PlanStep>,
    /// Current step
    pub current: usize,
    /// Status
    pub status: PlanStatus,
}

/// Plan step
#[derive(Debug, Clone)]
pub struct PlanStep {
    /// Step index
    pub index: usize,
    /// Action IDs
    pub actions: Vec<u64>,
    /// Parallel
    pub parallel: bool,
    /// Completed
    pub completed: bool,
}

/// Plan status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    Created,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ============================================================================
// ACTION EXECUTOR
// ============================================================================

/// Action executor
pub struct ActionExecutor {
    /// Actions
    actions: BTreeMap<u64, Action>,
    /// Results
    results: BTreeMap<u64, ExecutionResult>,
    /// Context
    context: ExecutionContext,
    /// Plans
    plans: BTreeMap<u64, ActionPlan>,
    /// Queue
    queue: Vec<u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ExecutorConfig,
    /// Statistics
    stats: ExecutorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum queue size
    pub max_queue: usize,
    /// Default timeout ns
    pub default_timeout_ns: u64,
    /// Retry count
    pub retry_count: usize,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_queue: 1000,
            default_timeout_ns: 1_000_000_000,
            retry_count: 3,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ExecutorStats {
    /// Actions created
    pub actions_created: u64,
    /// Actions executed
    pub actions_executed: u64,
    /// Actions succeeded
    pub actions_succeeded: u64,
    /// Actions failed
    pub actions_failed: u64,
    /// Total duration ns
    pub total_duration_ns: u64,
}

impl ActionExecutor {
    /// Create new executor
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            actions: BTreeMap::new(),
            results: BTreeMap::new(),
            context: ExecutionContext {
                id: 1,
                variables: BTreeMap::new(),
                resources: BTreeMap::new(),
            },
            plans: BTreeMap::new(),
            queue: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ExecutorStats::default(),
        }
    }

    /// Create action
    pub fn create_action(
        &mut self,
        name: &str,
        action_type: ActionType,
        params: BTreeMap<String, ParamValue>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let action = Action {
            id,
            name: name.into(),
            action_type,
            params,
            priority: 0,
            status: ActionStatus::Pending,
            created: Timestamp::now(),
        };

        self.actions.insert(id, action);
        self.stats.actions_created += 1;

        id
    }

    /// Set action priority
    pub fn set_priority(&mut self, action_id: u64, priority: u32) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.priority = priority;
        }
    }

    /// Queue action
    pub fn queue(&mut self, action_id: u64) -> bool {
        if self.queue.len() >= self.config.max_queue {
            return false;
        }

        if let Some(action) = self.actions.get_mut(&action_id) {
            action.status = ActionStatus::Ready;
            self.queue.push(action_id);

            // Sort by priority
            self.queue.sort_by(|a, b| {
                let pa = self.actions.get(a).map(|x| x.priority).unwrap_or(0);
                let pb = self.actions.get(b).map(|x| x.priority).unwrap_or(0);
                pb.cmp(&pa)
            });

            true
        } else {
            false
        }
    }

    /// Execute next action
    pub fn execute_next(&mut self) -> Option<ExecutionResult> {
        let action_id = self.queue.pop()?;

        self.execute(action_id)
    }

    /// Execute specific action
    pub fn execute(&mut self, action_id: u64) -> Option<ExecutionResult> {
        let action = self.actions.get_mut(&action_id)?;
        action.status = ActionStatus::Running;

        let start = Timestamp::now();

        // Simulate execution
        let (success, output, error) = self.run_action(action);

        let end = Timestamp::now();
        let duration_ns = end.0 - start.0;

        // Update status
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.status = if success {
                ActionStatus::Completed
            } else {
                ActionStatus::Failed
            };
        }

        let result = ExecutionResult {
            action_id,
            success,
            output,
            duration_ns,
            error,
            timestamp: end,
        };

        // Update stats
        self.stats.actions_executed += 1;
        self.stats.total_duration_ns += duration_ns;

        if success {
            self.stats.actions_succeeded += 1;
        } else {
            self.stats.actions_failed += 1;
        }

        self.results.insert(action_id, result.clone());

        Some(result)
    }

    fn run_action(&self, action: &Action) -> (bool, Option<ParamValue>, Option<String>) {
        // Check resources
        for (resource_name, _) in &action.params {
            if resource_name.starts_with("resource:") {
                let name = &resource_name[9..];
                if let Some(res) = self.context.resources.get(name) {
                    if !res.available || res.used >= res.capacity {
                        return (false, None, Some("Resource unavailable".into()));
                    }
                }
            }
        }

        // Execute based on type
        match action.action_type {
            ActionType::Compute => {
                (true, Some(ParamValue::Int(0)), None)
            }
            ActionType::Store => {
                (true, Some(ParamValue::Bool(true)), None)
            }
            ActionType::Retrieve => {
                if let Some(key) = action.params.get("key") {
                    if let ParamValue::Text(k) = key {
                        let val = self.context.variables.get(k).cloned();
                        (true, val, None)
                    } else {
                        (false, None, Some("Invalid key type".into()))
                    }
                } else {
                    (false, None, Some("Missing key".into()))
                }
            }
            ActionType::Transform => {
                (true, Some(ParamValue::Bool(true)), None)
            }
            ActionType::Validate => {
                (true, Some(ParamValue::Bool(true)), None)
            }
            _ => (true, None, None)
        }
    }

    /// Create plan
    pub fn create_plan(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let plan = ActionPlan {
            id,
            name: name.into(),
            steps: Vec::new(),
            current: 0,
            status: PlanStatus::Created,
        };

        self.plans.insert(id, plan);
        id
    }

    /// Add plan step
    pub fn add_step(&mut self, plan_id: u64, actions: Vec<u64>, parallel: bool) {
        if let Some(plan) = self.plans.get_mut(&plan_id) {
            let step = PlanStep {
                index: plan.steps.len(),
                actions,
                parallel,
                completed: false,
            };
            plan.steps.push(step);
        }
    }

    /// Run plan step
    pub fn run_plan_step(&mut self, plan_id: u64) -> Vec<ExecutionResult> {
        let mut results = Vec::new();

        let step = {
            let plan = match self.plans.get_mut(&plan_id) {
                Some(p) => p,
                None => return results,
            };

            if plan.status != PlanStatus::Created && plan.status != PlanStatus::Running {
                return results;
            }

            plan.status = PlanStatus::Running;

            if plan.current >= plan.steps.len() {
                plan.status = PlanStatus::Completed;
                return results;
            }

            plan.steps[plan.current].clone()
        };

        // Execute actions
        for action_id in &step.actions {
            if let Some(result) = self.execute(*action_id) {
                if !result.success && !step.parallel {
                    if let Some(plan) = self.plans.get_mut(&plan_id) {
                        plan.status = PlanStatus::Failed;
                    }
                    results.push(result);
                    return results;
                }
                results.push(result);
            }
        }

        // Mark complete and advance
        if let Some(plan) = self.plans.get_mut(&plan_id) {
            plan.steps[plan.current].completed = true;
            plan.current += 1;

            if plan.current >= plan.steps.len() {
                plan.status = PlanStatus::Completed;
            }
        }

        results
    }

    /// Set variable
    pub fn set_variable(&mut self, name: &str, value: ParamValue) {
        self.context.variables.insert(name.into(), value);
    }

    /// Add resource
    pub fn add_resource(&mut self, name: &str, capacity: u64) {
        self.context.resources.insert(name.into(), ResourceState {
            name: name.into(),
            available: true,
            capacity,
            used: 0,
        });
    }

    /// Get action
    pub fn get(&self, id: u64) -> Option<&Action> {
        self.actions.get(&id)
    }

    /// Get result
    pub fn get_result(&self, id: u64) -> Option<&ExecutionResult> {
        self.results.get(&id)
    }

    /// Get queue length
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Get statistics
    pub fn stats(&self) -> &ExecutorStats {
        &self.stats
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new(ExecutorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_action() {
        let mut executor = ActionExecutor::default();

        let id = executor.create_action("test", ActionType::Compute, BTreeMap::new());
        assert!(executor.get(id).is_some());
    }

    #[test]
    fn test_queue_execute() {
        let mut executor = ActionExecutor::default();

        let id = executor.create_action("compute", ActionType::Compute, BTreeMap::new());
        executor.queue(id);

        let result = executor.execute_next();
        assert!(result.is_some());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_priority() {
        let mut executor = ActionExecutor::default();

        let low = executor.create_action("low", ActionType::Compute, BTreeMap::new());
        let high = executor.create_action("high", ActionType::Compute, BTreeMap::new());

        executor.set_priority(low, 1);
        executor.set_priority(high, 10);

        executor.queue(low);
        executor.queue(high);

        // High priority should be first
        let result = executor.execute_next().unwrap();
        assert_eq!(result.action_id, high);
    }

    #[test]
    fn test_plan() {
        let mut executor = ActionExecutor::default();

        let a1 = executor.create_action("a1", ActionType::Compute, BTreeMap::new());
        let a2 = executor.create_action("a2", ActionType::Compute, BTreeMap::new());

        let plan = executor.create_plan("test");
        executor.add_step(plan, vec![a1], false);
        executor.add_step(plan, vec![a2], false);

        let results = executor.run_plan_step(plan);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);

        let results2 = executor.run_plan_step(plan);
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_retrieve_action() {
        let mut executor = ActionExecutor::default();

        executor.set_variable("key1", ParamValue::Int(42));

        let mut params = BTreeMap::new();
        params.insert("key".into(), ParamValue::Text("key1".into()));

        let id = executor.create_action("retrieve", ActionType::Retrieve, params);
        let result = executor.execute(id).unwrap();

        assert!(result.success);
    }
}

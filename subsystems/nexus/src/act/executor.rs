//! # Action Executor
//!
//! Executes planned actions with proper sequencing and error handling.
//! Supports parallel and sequential execution.
//!
//! Part of Year 2 COGNITION - Action Execution Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// EXECUTOR TYPES
// ============================================================================

/// Action
#[derive(Debug, Clone)]
pub struct Action {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Action type
    pub action_type: ExecutableActionType,
    /// Parameters
    pub params: BTreeMap<String, ActionValue>,
    /// Dependencies (action IDs that must complete first)
    pub dependencies: Vec<u64>,
    /// Timeout (ns)
    pub timeout_ns: u64,
    /// Retry policy
    pub retry_policy: RetryPolicy,
    /// Status
    pub status: ActionStatus,
    /// Result
    pub result: Option<ActionResult>,
}

/// Executable action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableActionType {
    Compute,
    Store,
    Retrieve,
    Transform,
    Send,
    Receive,
    Wait,
    Conditional,
    Parallel,
    Sequential,
}

/// Action value
#[derive(Debug, Clone)]
pub enum ActionValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<ActionValue>),
    Map(BTreeMap<String, ActionValue>),
}

/// Action status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
    Retrying,
}

/// Action result
#[derive(Debug, Clone)]
pub struct ActionResult {
    /// Success
    pub success: bool,
    /// Output
    pub output: Option<ActionValue>,
    /// Error
    pub error: Option<String>,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Retries used
    pub retries_used: u32,
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retries
    pub max_retries: u32,
    /// Delay between retries (ns)
    pub retry_delay_ns: u64,
    /// Exponential backoff
    pub exponential_backoff: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ns: 1_000_000_000, // 1 second
            exponential_backoff: true,
        }
    }
}

/// Execution batch
#[derive(Debug, Clone)]
pub struct ExecutionBatch {
    /// Batch ID
    pub id: u64,
    /// Actions
    pub actions: Vec<u64>,
    /// Parallel execution
    pub parallel: bool,
    /// Status
    pub status: BatchStatus,
    /// Started
    pub started: Option<Timestamp>,
    /// Completed
    pub completed: Option<Timestamp>,
}

/// Batch status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStatus {
    Pending,
    Running,
    Completed,
    PartiallyFailed,
    Failed,
}

// ============================================================================
// ACTION EXECUTOR
// ============================================================================

/// Action executor
pub struct ActionExecutor {
    /// Actions
    actions: BTreeMap<u64, Action>,
    /// Batches
    batches: BTreeMap<u64, ExecutionBatch>,
    /// Execution queue
    queue: Vec<u64>,
    /// Handlers
    handlers: BTreeMap<ExecutableActionType, ActionHandler>,
    /// Context
    context: ExecutionContext,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ExecutorConfig,
    /// Statistics
    stats: ExecutorStats,
}

/// Action handler
type ActionHandler = fn(&Action, &mut ExecutionContext) -> ActionResult;

/// Execution context
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// Variables
    pub variables: BTreeMap<String, ActionValue>,
    /// Output from previous action
    pub last_output: Option<ActionValue>,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum parallel actions
    pub max_parallel: usize,
    /// Default timeout (ns)
    pub default_timeout_ns: u64,
    /// Enable logging
    pub enable_logging: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            default_timeout_ns: 30_000_000_000, // 30 seconds
            enable_logging: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ExecutorStats {
    /// Actions executed
    pub actions_executed: u64,
    /// Actions succeeded
    pub actions_succeeded: u64,
    /// Actions failed
    pub actions_failed: u64,
    /// Total retries
    pub total_retries: u64,
    /// Average duration (ns)
    pub avg_duration_ns: f64,
}

impl ActionExecutor {
    /// Create new executor
    pub fn new(config: ExecutorConfig) -> Self {
        let mut executor = Self {
            actions: BTreeMap::new(),
            batches: BTreeMap::new(),
            queue: Vec::new(),
            handlers: BTreeMap::new(),
            context: ExecutionContext::default(),
            next_id: AtomicU64::new(1),
            config,
            stats: ExecutorStats::default(),
        };

        // Register default handlers
        executor.register_default_handlers();

        executor
    }

    fn register_default_handlers(&mut self) {
        self.handlers
            .insert(ExecutableActionType::Compute, |action, ctx| {
                // Simple compute handler
                let output = ctx
                    .variables
                    .get("compute_result")
                    .cloned()
                    .unwrap_or(ActionValue::Null);

                ActionResult {
                    success: true,
                    output: Some(output),
                    error: None,
                    duration_ns: 1000,
                    retries_used: 0,
                }
            });

        self.handlers
            .insert(ExecutableActionType::Store, |action, ctx| {
                // Store handler
                if let Some(ActionValue::String(key)) = action.params.get("key") {
                    if let Some(value) = action.params.get("value") {
                        ctx.variables.insert(key.clone(), value.clone());
                    }
                }

                ActionResult {
                    success: true,
                    output: None,
                    error: None,
                    duration_ns: 500,
                    retries_used: 0,
                }
            });

        self.handlers
            .insert(ExecutableActionType::Retrieve, |action, ctx| {
                // Retrieve handler
                let output = action.params.get("key").and_then(|k| {
                    if let ActionValue::String(key) = k {
                        ctx.variables.get(key).cloned()
                    } else {
                        None
                    }
                });

                ActionResult {
                    success: output.is_some(),
                    output,
                    error: None,
                    duration_ns: 500,
                    retries_used: 0,
                }
            });

        self.handlers
            .insert(ExecutableActionType::Wait, |action, _ctx| {
                // Wait handler (simulated)
                let duration = action
                    .params
                    .get("duration_ns")
                    .and_then(|v| {
                        if let ActionValue::Int(n) = v {
                            Some(*n as u64)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(1000);

                ActionResult {
                    success: true,
                    output: None,
                    error: None,
                    duration_ns: duration,
                    retries_used: 0,
                }
            });
    }

    /// Register custom handler
    pub fn register_handler(&mut self, action_type: ExecutableActionType, handler: ActionHandler) {
        self.handlers.insert(action_type, handler);
    }

    /// Create action
    pub fn create_action(
        &mut self,
        name: &str,
        action_type: ExecutableActionType,
        params: BTreeMap<String, ActionValue>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let action = Action {
            id,
            name: name.into(),
            action_type,
            params,
            dependencies: Vec::new(),
            timeout_ns: self.config.default_timeout_ns,
            retry_policy: RetryPolicy::default(),
            status: ActionStatus::Pending,
            result: None,
        };

        self.actions.insert(id, action);
        id
    }

    /// Add dependency
    pub fn add_dependency(&mut self, action_id: u64, depends_on: u64) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.dependencies.push(depends_on);
        }
    }

    /// Queue action
    pub fn queue(&mut self, action_id: u64) {
        if !self.queue.contains(&action_id) {
            self.queue.push(action_id);
        }
    }

    /// Create batch
    pub fn create_batch(&mut self, action_ids: Vec<u64>, parallel: bool) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let batch = ExecutionBatch {
            id,
            actions: action_ids,
            parallel,
            status: BatchStatus::Pending,
            started: None,
            completed: None,
        };

        self.batches.insert(id, batch);
        id
    }

    /// Execute single action
    pub fn execute(&mut self, action_id: u64) -> Option<ActionResult> {
        // Check dependencies
        let action = self.actions.get(&action_id)?.clone();

        for dep_id in &action.dependencies {
            if let Some(dep) = self.actions.get(dep_id) {
                if dep.status != ActionStatus::Completed {
                    return None; // Dependency not ready
                }
            }
        }

        // Get handler
        let handler = self.handlers.get(&action.action_type)?;

        // Update status
        if let Some(a) = self.actions.get_mut(&action_id) {
            a.status = ActionStatus::Running;
        }

        // Execute with retry
        let mut retries = 0;
        let result = loop {
            let result = handler(&action, &mut self.context);

            if result.success || retries >= action.retry_policy.max_retries {
                break result;
            }

            retries += 1;
            self.stats.total_retries += 1;

            if let Some(a) = self.actions.get_mut(&action_id) {
                a.status = ActionStatus::Retrying;
            }
        };

        // Update action
        if let Some(a) = self.actions.get_mut(&action_id) {
            a.status = if result.success {
                ActionStatus::Completed
            } else {
                ActionStatus::Failed
            };
            a.result = Some(result.clone());
        }

        // Update context
        if let Some(output) = &result.output {
            self.context.last_output = Some(output.clone());
        }

        // Update stats
        self.stats.actions_executed += 1;
        if result.success {
            self.stats.actions_succeeded += 1;
        } else {
            self.stats.actions_failed += 1;
        }

        let n = self.stats.actions_executed as f64;
        self.stats.avg_duration_ns =
            (self.stats.avg_duration_ns * (n - 1.0) + result.duration_ns as f64) / n;

        Some(result)
    }

    /// Execute batch
    pub fn execute_batch(&mut self, batch_id: u64) -> BatchStatus {
        let batch = match self.batches.get_mut(&batch_id) {
            Some(b) => {
                b.status = BatchStatus::Running;
                b.started = Some(Timestamp::now());
                b.clone()
            },
            None => return BatchStatus::Failed,
        };

        let mut all_success = true;
        let mut any_success = false;

        if batch.parallel {
            // Parallel execution (simplified - actually sequential here)
            for &action_id in &batch.actions {
                if let Some(result) = self.execute(action_id) {
                    if result.success {
                        any_success = true;
                    } else {
                        all_success = false;
                    }
                } else {
                    all_success = false;
                }
            }
        } else {
            // Sequential execution
            for &action_id in &batch.actions {
                if let Some(result) = self.execute(action_id) {
                    if result.success {
                        any_success = true;
                    } else {
                        all_success = false;
                        break; // Stop on first failure
                    }
                } else {
                    all_success = false;
                    break;
                }
            }
        }

        let status = if all_success {
            BatchStatus::Completed
        } else if any_success {
            BatchStatus::PartiallyFailed
        } else {
            BatchStatus::Failed
        };

        if let Some(b) = self.batches.get_mut(&batch_id) {
            b.status = status;
            b.completed = Some(Timestamp::now());
        }

        status
    }

    /// Process queue
    pub fn process_queue(&mut self) -> usize {
        let ready: Vec<u64> = self
            .queue
            .iter()
            .copied()
            .filter(|&id| self.is_ready(id))
            .collect();

        let mut processed = 0;

        for action_id in ready {
            if self.execute(action_id).is_some() {
                processed += 1;
            }

            self.queue.retain(|&id| id != action_id);
        }

        processed
    }

    fn is_ready(&self, action_id: u64) -> bool {
        if let Some(action) = self.actions.get(&action_id) {
            action.dependencies.iter().all(|&dep_id| {
                self.actions
                    .get(&dep_id)
                    .map(|a| a.status == ActionStatus::Completed)
                    .unwrap_or(false)
            })
        } else {
            false
        }
    }

    /// Set context variable
    pub fn set_variable(&mut self, name: &str, value: ActionValue) {
        self.context.variables.insert(name.into(), value);
    }

    /// Get context variable
    pub fn get_variable(&self, name: &str) -> Option<&ActionValue> {
        self.context.variables.get(name)
    }

    /// Get action
    pub fn get_action(&self, id: u64) -> Option<&Action> {
        self.actions.get(&id)
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

        let id = executor.create_action("test", ExecutableActionType::Compute, BTreeMap::new());

        assert!(executor.get_action(id).is_some());
    }

    #[test]
    fn test_execute_action() {
        let mut executor = ActionExecutor::default();

        let id = executor.create_action("compute", ExecutableActionType::Compute, BTreeMap::new());

        let result = executor.execute(id);
        assert!(result.is_some());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_store_and_retrieve() {
        let mut executor = ActionExecutor::default();

        let mut store_params = BTreeMap::new();
        store_params.insert("key".into(), ActionValue::String("test_key".into()));
        store_params.insert("value".into(), ActionValue::Int(42));

        let store_id = executor.create_action("store", ExecutableActionType::Store, store_params);

        executor.execute(store_id);

        let mut retrieve_params = BTreeMap::new();
        retrieve_params.insert("key".into(), ActionValue::String("test_key".into()));

        let retrieve_id =
            executor.create_action("retrieve", ExecutableActionType::Retrieve, retrieve_params);

        let result = executor.execute(retrieve_id).unwrap();
        assert!(result.success);
        assert!(matches!(result.output, Some(ActionValue::Int(42))));
    }

    #[test]
    fn test_dependencies() {
        let mut executor = ActionExecutor::default();

        let a1 = executor.create_action("first", ExecutableActionType::Compute, BTreeMap::new());
        let a2 = executor.create_action("second", ExecutableActionType::Compute, BTreeMap::new());

        executor.add_dependency(a2, a1);

        // Can't execute a2 before a1
        assert!(executor.execute(a2).is_none());

        // Execute a1
        executor.execute(a1);

        // Now a2 should work
        assert!(executor.execute(a2).is_some());
    }

    #[test]
    fn test_batch_execution() {
        let mut executor = ActionExecutor::default();

        let a1 = executor.create_action("a1", ExecutableActionType::Compute, BTreeMap::new());
        let a2 = executor.create_action("a2", ExecutableActionType::Compute, BTreeMap::new());

        let batch_id = executor.create_batch(vec![a1, a2], false);
        let status = executor.execute_batch(batch_id);

        assert_eq!(status, BatchStatus::Completed);
    }
}

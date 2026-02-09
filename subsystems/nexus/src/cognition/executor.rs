//! # Cognitive Executor
//!
//! Task execution framework for cognitive operations.
//! Manages execution contexts, parallelism, and completion.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// EXECUTION TYPES
// ============================================================================

/// Executable task
#[derive(Debug)]
pub struct ExecutableTask {
    /// Task ID
    pub id: u64,
    /// Task name
    pub name: String,
    /// Task type
    pub task_type: TaskType,
    /// Priority
    pub priority: TaskPriority,
    /// Owner domain
    pub owner: DomainId,
    /// Input data
    pub input: TaskInput,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Timeout (ns)
    pub timeout_ns: Option<u64>,
    /// Retry policy
    pub retry: RetryPolicy,
    /// Created at
    pub created_at: Timestamp,
}

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// Computation task
    Compute,
    /// I/O task
    Io,
    /// Network task
    Network,
    /// Inference task
    Inference,
    /// Training task
    Training,
    /// Pipeline task
    Pipeline,
    /// Maintenance task
    Maintenance,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Lowest,
    Low,
    Normal,
    High,
    Highest,
    Critical,
}

/// Task input
#[derive(Debug, Clone)]
pub struct TaskInput {
    /// Input type
    pub input_type: String,
    /// Data
    pub data: InputData,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Input data
#[derive(Debug, Clone)]
pub enum InputData {
    Empty,
    Binary(Vec<u8>),
    Text(String),
    Structured(BTreeMap<String, InputValue>),
    Reference(u64),
}

/// Input value
#[derive(Debug, Clone)]
pub enum InputValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<InputValue>),
    Object(BTreeMap<String, InputValue>),
}

/// Retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum retries
    pub max_retries: u32,
    /// Backoff type
    pub backoff: BackoffType,
    /// Retry on these errors
    pub retry_on: Vec<String>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff: BackoffType::Exponential {
                base_ns: 100_000_000,
            }, // 100ms
            retry_on: Vec::new(),
        }
    }
}

/// Backoff type
#[derive(Debug, Clone)]
pub enum BackoffType {
    /// Constant backoff
    Constant(u64),
    /// Linear backoff
    Linear { base_ns: u64 },
    /// Exponential backoff
    Exponential { base_ns: u64 },
}

// ============================================================================
// EXECUTION STATE
// ============================================================================

/// Execution state
#[derive(Debug)]
#[repr(align(64))]
pub struct ExecutionState {
    /// Task ID
    pub task_id: u64,
    /// Current status
    pub status: ExecutionStatus,
    /// Progress (0-100)
    pub progress: u32,
    /// Retries attempted
    pub retries: u32,
    /// Started at
    pub started_at: Option<Timestamp>,
    /// Finished at
    pub finished_at: Option<Timestamp>,
    /// Result
    pub result: Option<ExecutionResult>,
    /// Error
    pub error: Option<ExecutionError>,
    /// Metrics
    pub metrics: ExecutionMetrics,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// Pending execution
    Pending,
    /// Waiting for dependencies
    Waiting,
    /// Running
    Running,
    /// Paused
    Paused,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
    /// Timed out
    TimedOut,
}

/// Execution result
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Result type
    pub result_type: String,
    /// Output data
    pub output: OutputData,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Output data
#[derive(Debug, Clone)]
pub enum OutputData {
    Empty,
    Binary(Vec<u8>),
    Text(String),
    Structured(BTreeMap<String, InputValue>),
    Reference(u64),
}

/// Execution error
#[derive(Debug, Clone)]
pub struct ExecutionError {
    /// Error code
    pub code: String,
    /// Message
    pub message: String,
    /// Retryable
    pub retryable: bool,
    /// Details
    pub details: Option<String>,
}

/// Execution metrics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ExecutionMetrics {
    /// CPU time (ns)
    pub cpu_time_ns: u64,
    /// Wall time (ns)
    pub wall_time_ns: u64,
    /// Memory peak (bytes)
    pub memory_peak: u64,
    /// I/O bytes read
    pub io_read: u64,
    /// I/O bytes written
    pub io_write: u64,
}

// ============================================================================
// EXECUTOR
// ============================================================================

/// Cognitive executor
pub struct CognitiveExecutor {
    /// Pending tasks
    pending: VecDeque<ExecutableTask>,
    /// Waiting tasks (for dependencies)
    waiting: BTreeMap<u64, ExecutableTask>,
    /// Running tasks
    running: BTreeMap<u64, ExecutionState>,
    /// Completed tasks
    completed: BTreeMap<u64, ExecutionState>,
    /// Task outputs (for dependencies)
    outputs: BTreeMap<u64, ExecutionResult>,
    /// Next task ID
    next_id: AtomicU64,
    /// Running flag
    running_flag: AtomicBool,
    /// Configuration
    config: ExecutorConfig,
    /// Statistics
    stats: ExecutorStats,
}

/// Executor configuration
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum concurrent tasks
    pub max_concurrent: usize,
    /// Maximum pending tasks
    pub max_pending: usize,
    /// Maximum completed history
    pub max_completed: usize,
    /// Default timeout (ns)
    pub default_timeout_ns: u64,
    /// Enable preemption
    pub enable_preemption: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            max_pending: 1000,
            max_completed: 1000,
            default_timeout_ns: 60_000_000_000, // 1 minute
            enable_preemption: true,
        }
    }
}

/// Executor statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ExecutorStats {
    /// Tasks submitted
    pub submitted: u64,
    /// Tasks completed
    pub completed: u64,
    /// Tasks failed
    pub failed: u64,
    /// Tasks cancelled
    pub cancelled: u64,
    /// Tasks timed out
    pub timed_out: u64,
    /// Total retries
    pub retries: u64,
    /// Average execution time (ns)
    pub avg_execution_ns: f64,
}

impl CognitiveExecutor {
    /// Create new executor
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            pending: VecDeque::new(),
            waiting: BTreeMap::new(),
            running: BTreeMap::new(),
            completed: BTreeMap::new(),
            outputs: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            running_flag: AtomicBool::new(true),
            config,
            stats: ExecutorStats::default(),
        }
    }

    /// Submit task
    pub fn submit(
        &mut self,
        name: &str,
        task_type: TaskType,
        priority: TaskPriority,
        owner: DomainId,
        input: TaskInput,
        dependencies: Vec<u64>,
    ) -> Result<u64, &'static str> {
        if self.pending.len() >= self.config.max_pending {
            return Err("Pending queue full");
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let task = ExecutableTask {
            id,
            name: name.into(),
            task_type,
            priority,
            owner,
            input,
            dependencies: dependencies.clone(),
            timeout_ns: Some(self.config.default_timeout_ns),
            retry: RetryPolicy::default(),
            created_at: Timestamp::now(),
        };

        // Check if dependencies are satisfied
        let deps_satisfied = dependencies.iter().all(|d| self.outputs.contains_key(d));

        if deps_satisfied {
            self.pending.push_back(task);
        } else {
            self.waiting.insert(id, task);
        }

        self.stats.submitted += 1;
        Ok(id)
    }

    /// Schedule next task for execution
    pub fn schedule_next(&mut self) -> Option<u64> {
        if self.running.len() >= self.config.max_concurrent {
            return None;
        }

        if self.pending.is_empty() {
            return None;
        }

        // Sort by priority
        self.pending.sort_by(|a, b| b.priority.cmp(&a.priority));

        let task = self.pending.pop_front().unwrap();
        let id = task.id;

        let state = ExecutionState {
            task_id: id,
            status: ExecutionStatus::Running,
            progress: 0,
            retries: 0,
            started_at: Some(Timestamp::now()),
            finished_at: None,
            result: None,
            error: None,
            metrics: ExecutionMetrics::default(),
        };

        self.running.insert(id, state);
        Some(id)
    }

    /// Update task progress
    #[inline]
    pub fn update_progress(&mut self, task_id: u64, progress: u32) -> Result<(), &'static str> {
        let state = self.running.get_mut(&task_id).ok_or("Task not running")?;

        state.progress = progress.min(100);
        Ok(())
    }

    /// Complete task successfully
    pub fn complete(&mut self, task_id: u64, result: ExecutionResult) -> Result<(), &'static str> {
        let mut state = self.running.remove(&task_id).ok_or("Task not running")?;

        let now = Timestamp::now();
        if let Some(started) = state.started_at {
            state.metrics.wall_time_ns = now.elapsed_since(started);
        }

        state.status = ExecutionStatus::Completed;
        state.progress = 100;
        state.finished_at = Some(now);
        state.result = Some(result.clone());

        // Store output for dependencies
        self.outputs.insert(task_id, result);

        // Move to completed
        self.move_to_completed(task_id, state);

        // Check waiting tasks
        self.check_waiting_tasks();

        self.stats.completed += 1;
        self.update_avg_execution(state.metrics.wall_time_ns);

        Ok(())
    }

    /// Fail task
    pub fn fail(&mut self, task_id: u64, error: ExecutionError) -> Result<bool, &'static str> {
        let mut state = self.running.remove(&task_id).ok_or("Task not running")?;

        let now = Timestamp::now();
        if let Some(started) = state.started_at {
            state.metrics.wall_time_ns = now.elapsed_since(started);
        }

        // Check if should retry
        let task = self.find_original_task(task_id);
        let should_retry =
            error.retryable && state.retries < task.map(|t| t.retry.max_retries).unwrap_or(0);

        if should_retry {
            state.retries += 1;
            state.status = ExecutionStatus::Pending;
            state.started_at = None;
            state.progress = 0;
            self.stats.retries += 1;

            // Re-queue
            if let Some(task) = task {
                self.pending.push_back(task.clone());
            }

            self.running.insert(task_id, state);
            Ok(true)
        } else {
            state.status = ExecutionStatus::Failed;
            state.finished_at = Some(now);
            state.error = Some(error);

            self.move_to_completed(task_id, state);
            self.stats.failed += 1;

            Ok(false)
        }
    }

    /// Cancel task
    pub fn cancel(&mut self, task_id: u64) -> Result<(), &'static str> {
        // Try running
        if let Some(mut state) = self.running.remove(&task_id) {
            state.status = ExecutionStatus::Cancelled;
            state.finished_at = Some(Timestamp::now());
            self.move_to_completed(task_id, state);
            self.stats.cancelled += 1;
            return Ok(());
        }

        // Try pending
        if let Some(pos) = self.pending.iter().position(|t| t.id == task_id) {
            self.pending.remove(pos);
            self.stats.cancelled += 1;
            return Ok(());
        }

        // Try waiting
        if self.waiting.remove(&task_id).is_some() {
            self.stats.cancelled += 1;
            return Ok(());
        }

        Err("Task not found")
    }

    /// Check for timeouts
    pub fn check_timeouts(&mut self) -> Vec<u64> {
        let now = Timestamp::now();
        let mut timed_out = Vec::new();

        for (id, state) in &self.running {
            if let Some(started) = state.started_at {
                let elapsed = now.elapsed_since(started);

                // Get task timeout
                if let Some(task) = self.find_original_task(*id) {
                    if let Some(timeout) = task.timeout_ns {
                        if elapsed > timeout {
                            timed_out.push(*id);
                        }
                    }
                }
            }
        }

        for id in &timed_out {
            if let Some(mut state) = self.running.remove(id) {
                state.status = ExecutionStatus::TimedOut;
                state.finished_at = Some(now);
                self.move_to_completed(*id, state);
                self.stats.timed_out += 1;
            }
        }

        timed_out
    }

    fn find_original_task(&self, _id: u64) -> Option<&ExecutableTask> {
        // In a real implementation, would keep original tasks
        None
    }

    fn move_to_completed(&mut self, id: u64, state: ExecutionState) {
        // Limit completed history
        while self.completed.len() >= self.config.max_completed {
            if let Some(oldest) = self.completed.keys().next().copied() {
                self.completed.remove(&oldest);
                self.outputs.remove(&oldest);
            }
        }
        self.completed.insert(id, state);
    }

    fn check_waiting_tasks(&mut self) {
        let ready: Vec<u64> = self
            .waiting
            .iter()
            .filter(|(_, task)| {
                task.dependencies
                    .iter()
                    .all(|d| self.outputs.contains_key(d))
            })
            .map(|(id, _)| *id)
            .collect();

        for id in ready {
            if let Some(task) = self.waiting.remove(&id) {
                self.pending.push_back(task);
            }
        }
    }

    fn update_avg_execution(&mut self, new_time: u64) {
        let n = self.stats.completed as f64;
        self.stats.avg_execution_ns =
            (self.stats.avg_execution_ns * (n - 1.0) + new_time as f64) / n;
    }

    /// Get task state
    #[inline]
    pub fn get_state(&self, task_id: u64) -> Option<&ExecutionState> {
        self.running
            .get(&task_id)
            .or_else(|| self.completed.get(&task_id))
    }

    /// Get running tasks
    #[inline(always)]
    pub fn running_tasks(&self) -> Vec<u64> {
        self.running.keys().copied().collect()
    }

    /// Get pending count
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get running count
    #[inline(always)]
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Pause executor
    #[inline(always)]
    pub fn pause(&self) {
        self.running_flag.store(false, Ordering::Release);
    }

    /// Resume executor
    #[inline(always)]
    pub fn resume(&self) {
        self.running_flag.store(true, Ordering::Release);
    }

    /// Is running
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.running_flag.load(Ordering::Acquire)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ExecutorStats {
        &self.stats
    }
}

impl Default for CognitiveExecutor {
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
    fn test_task_submission() {
        let mut executor = CognitiveExecutor::default();
        let owner = DomainId::new(1);

        let id = executor
            .submit(
                "test_task",
                TaskType::Compute,
                TaskPriority::Normal,
                owner,
                TaskInput {
                    input_type: "test".into(),
                    data: InputData::Empty,
                    metadata: BTreeMap::new(),
                },
                vec![],
            )
            .unwrap();

        assert!(executor.pending_count() == 1);
        assert!(executor.running_count() == 0);
    }

    #[test]
    fn test_task_execution() {
        let mut executor = CognitiveExecutor::default();
        let owner = DomainId::new(1);

        let id = executor
            .submit(
                "test",
                TaskType::Compute,
                TaskPriority::Normal,
                owner,
                TaskInput {
                    input_type: "test".into(),
                    data: InputData::Empty,
                    metadata: BTreeMap::new(),
                },
                vec![],
            )
            .unwrap();

        // Schedule
        let scheduled = executor.schedule_next();
        assert_eq!(scheduled, Some(id));
        assert!(executor.running_count() == 1);

        // Complete
        executor
            .complete(id, ExecutionResult {
                result_type: "test".into(),
                output: OutputData::Empty,
                metadata: BTreeMap::new(),
            })
            .unwrap();

        let state = executor.get_state(id).unwrap();
        assert_eq!(state.status, ExecutionStatus::Completed);
    }

    #[test]
    fn test_dependencies() {
        let mut executor = CognitiveExecutor::default();
        let owner = DomainId::new(1);

        // Submit first task
        let id1 = executor
            .submit(
                "first",
                TaskType::Compute,
                TaskPriority::Normal,
                owner,
                TaskInput {
                    input_type: "test".into(),
                    data: InputData::Empty,
                    metadata: BTreeMap::new(),
                },
                vec![],
            )
            .unwrap();

        // Submit dependent task
        let id2 = executor
            .submit(
                "second",
                TaskType::Compute,
                TaskPriority::Normal,
                owner,
                TaskInput {
                    input_type: "test".into(),
                    data: InputData::Empty,
                    metadata: BTreeMap::new(),
                },
                vec![id1],
            )
            .unwrap();

        // Second should be waiting
        assert_eq!(executor.pending_count(), 1);
        assert!(executor.waiting.contains_key(&id2));

        // Execute first
        executor.schedule_next();
        executor
            .complete(id1, ExecutionResult {
                result_type: "test".into(),
                output: OutputData::Empty,
                metadata: BTreeMap::new(),
            })
            .unwrap();

        // Second should now be pending
        assert_eq!(executor.pending_count(), 1);
        assert!(!executor.waiting.contains_key(&id2));
    }

    #[test]
    fn test_priority_scheduling() {
        let mut executor = CognitiveExecutor::default();
        let owner = DomainId::new(1);

        // Submit low priority
        executor
            .submit(
                "low",
                TaskType::Compute,
                TaskPriority::Low,
                owner,
                TaskInput {
                    input_type: "".into(),
                    data: InputData::Empty,
                    metadata: BTreeMap::new(),
                },
                vec![],
            )
            .unwrap();

        // Submit high priority
        executor
            .submit(
                "high",
                TaskType::Compute,
                TaskPriority::High,
                owner,
                TaskInput {
                    input_type: "".into(),
                    data: InputData::Empty,
                    metadata: BTreeMap::new(),
                },
                vec![],
            )
            .unwrap();

        // High priority should be scheduled first
        let scheduled = executor.schedule_next().unwrap();
        let state = executor.get_state(scheduled).unwrap();
        // The task should be running now
        assert_eq!(state.status, ExecutionStatus::Running);
    }
}

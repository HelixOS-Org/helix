//! # Cognitive Scheduler
//!
//! Advanced scheduling for cognitive tasks.
//! Multi-level scheduling with priority, fairness, and deadlines.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// TASK TYPES
// ============================================================================

/// A cognitive task
#[derive(Debug, Clone)]
pub struct CognitiveTask {
    /// Task ID
    pub id: u64,
    /// Task name
    pub name: String,
    /// Owner domain
    pub owner: DomainId,
    /// Priority
    pub priority: TaskPriority,
    /// State
    pub state: TaskState,
    /// Task type
    pub task_type: TaskType,
    /// Created time
    pub created: Timestamp,
    /// Started time
    pub started: Option<Timestamp>,
    /// Completed time
    pub completed: Option<Timestamp>,
    /// Deadline
    pub deadline: Option<Timestamp>,
    /// CPU time used (ns)
    pub cpu_time_ns: u64,
    /// Wall time used (ns)
    pub wall_time_ns: u64,
    /// Dependencies
    pub dependencies: Vec<u64>,
    /// Priority boost
    pub boost: i8,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TaskPriority {
    /// Idle
    Idle     = 0,
    /// Low
    Low      = 1,
    /// Normal
    Normal   = 2,
    /// High
    High     = 3,
    /// Critical
    Critical = 4,
    /// Realtime
    Realtime = 5,
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Pending
    Pending,
    /// Ready to run
    Ready,
    /// Running
    Running,
    /// Blocked
    Blocked,
    /// Waiting on dependencies
    WaitingDeps,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
}

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// Compute task
    Compute,
    /// I/O task
    Io,
    /// Memory task
    Memory,
    /// Communication task
    Communication,
    /// Learning task
    Learning,
    /// Inference task
    Inference,
}

// ============================================================================
// SCHEDULING POLICIES
// ============================================================================

/// Scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// First-come first-served
    Fifo,
    /// Round robin
    RoundRobin,
    /// Priority-based
    Priority,
    /// Earliest deadline first
    EarliestDeadline,
    /// Multi-level feedback queue
    Mlfq,
    /// Completely fair scheduler
    Cfs,
    /// Real-time
    Realtime,
}

/// Time slice configuration
#[derive(Debug, Clone)]
pub struct TimeSlice {
    /// Base quantum (ns)
    pub base_ns: u64,
    /// Minimum quantum (ns)
    pub min_ns: u64,
    /// Maximum quantum (ns)
    pub max_ns: u64,
    /// Priority multipliers
    pub priority_mult: [f64; 6],
}

impl Default for TimeSlice {
    fn default() -> Self {
        Self {
            base_ns: 10_000_000, // 10ms
            min_ns: 1_000_000,   // 1ms
            max_ns: 100_000_000, // 100ms
            priority_mult: [0.25, 0.5, 1.0, 2.0, 4.0, 8.0],
        }
    }
}

impl TimeSlice {
    /// Get quantum for priority
    pub fn quantum_for(&self, priority: TaskPriority) -> u64 {
        let mult = self.priority_mult[priority as usize];
        let quantum = (self.base_ns as f64 * mult) as u64;
        quantum.clamp(self.min_ns, self.max_ns)
    }
}

// ============================================================================
// RUN QUEUE
// ============================================================================

/// Run queue for ready tasks
#[derive(Debug)]
pub struct RunQueue {
    /// Queue name
    pub name: String,
    /// Priority level
    pub level: usize,
    /// Tasks in queue
    tasks: Vec<u64>,
    /// Time slice for this queue
    pub time_slice: u64,
    /// Total run time
    pub total_run_time: u64,
    /// Tasks processed
    pub tasks_processed: u64,
}

impl RunQueue {
    /// Create a new run queue
    pub fn new(name: &str, level: usize, time_slice: u64) -> Self {
        Self {
            name: name.into(),
            level,
            tasks: Vec::new(),
            time_slice,
            total_run_time: 0,
            tasks_processed: 0,
        }
    }

    /// Enqueue a task
    pub fn enqueue(&mut self, task_id: u64) {
        self.tasks.push(task_id);
    }

    /// Dequeue next task
    pub fn dequeue(&mut self) -> Option<u64> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks.remove(0))
        }
    }

    /// Peek next task
    pub fn peek(&self) -> Option<u64> {
        self.tasks.first().copied()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Remove task
    pub fn remove(&mut self, task_id: u64) -> bool {
        if let Some(pos) = self.tasks.iter().position(|&t| t == task_id) {
            self.tasks.remove(pos);
            true
        } else {
            false
        }
    }
}

// ============================================================================
// SCHEDULER
// ============================================================================

/// Cognitive task scheduler
pub struct CognitiveScheduler {
    /// All tasks
    tasks: BTreeMap<u64, CognitiveTask>,
    /// Run queues (one per priority level)
    run_queues: Vec<RunQueue>,
    /// Blocked tasks
    blocked: Vec<u64>,
    /// Waiting on dependencies
    waiting_deps: Vec<u64>,
    /// Current task
    current: Option<u64>,
    /// Next task ID
    next_task_id: AtomicU64,
    /// Scheduling policy
    policy: SchedulingPolicy,
    /// Time slice config
    time_slice: TimeSlice,
    /// Configuration
    config: SchedulerConfig,
    /// Statistics
    stats: SchedulerStats,
}

/// Scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum tasks
    pub max_tasks: usize,
    /// Queue levels (for MLFQ)
    pub queue_levels: usize,
    /// Preemption enabled
    pub preemption: bool,
    /// Priority aging
    pub priority_aging: bool,
    /// Aging interval (ns)
    pub aging_interval_ns: u64,
    /// Deadline tolerance (ns)
    pub deadline_tolerance_ns: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_tasks: 10000,
            queue_levels: 8,
            preemption: true,
            priority_aging: true,
            aging_interval_ns: 1_000_000_000,   // 1 second
            deadline_tolerance_ns: 100_000_000, // 100ms
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Total tasks created
    pub total_created: u64,
    /// Total tasks completed
    pub total_completed: u64,
    /// Total tasks failed
    pub total_failed: u64,
    /// Context switches
    pub context_switches: u64,
    /// Preemptions
    pub preemptions: u64,
    /// Average wait time (ns)
    pub avg_wait_ns: f64,
    /// Average turnaround time (ns)
    pub avg_turnaround_ns: f64,
    /// Deadline misses
    pub deadline_misses: u64,
}

impl CognitiveScheduler {
    /// Create a new scheduler
    pub fn new(policy: SchedulingPolicy, config: SchedulerConfig) -> Self {
        let time_slice = TimeSlice::default();
        let mut run_queues = Vec::new();

        // Create run queues for each priority level
        for level in 0..config.queue_levels {
            let quantum = time_slice.base_ns * (1 << level.min(4));
            run_queues.push(RunQueue::new(&format!("queue_{}", level), level, quantum));
        }

        Self {
            tasks: BTreeMap::new(),
            run_queues,
            blocked: Vec::new(),
            waiting_deps: Vec::new(),
            current: None,
            next_task_id: AtomicU64::new(1),
            policy,
            time_slice,
            config,
            stats: SchedulerStats::default(),
        }
    }

    /// Create a task
    pub fn create_task(
        &mut self,
        name: &str,
        owner: DomainId,
        priority: TaskPriority,
        task_type: TaskType,
        deadline: Option<Timestamp>,
        dependencies: Vec<u64>,
    ) -> u64 {
        let id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let task = CognitiveTask {
            id,
            name: name.into(),
            owner,
            priority,
            state: TaskState::Pending,
            task_type,
            created: now,
            started: None,
            completed: None,
            deadline,
            cpu_time_ns: 0,
            wall_time_ns: 0,
            dependencies,
            boost: 0,
        };

        self.tasks.insert(id, task);
        self.stats.total_created += 1;

        // Check dependencies
        self.check_ready(id);

        id
    }

    /// Check if task is ready
    fn check_ready(&mut self, task_id: u64) {
        let task = match self.tasks.get_mut(&task_id) {
            Some(t) => t,
            None => return,
        };

        if task.state != TaskState::Pending {
            return;
        }

        // Check dependencies
        let deps_satisfied = task.dependencies.iter().all(|dep_id| {
            self.tasks
                .get(dep_id)
                .map(|t| t.state == TaskState::Completed)
                .unwrap_or(true)
        });

        if deps_satisfied {
            task.state = TaskState::Ready;
            let priority = task.priority;
            let id = task.id;
            self.enqueue(id, priority);
        } else {
            task.state = TaskState::WaitingDeps;
            self.waiting_deps.push(task_id);
        }
    }

    /// Enqueue task to run queue
    fn enqueue(&mut self, task_id: u64, priority: TaskPriority) {
        let level = match self.policy {
            SchedulingPolicy::Priority | SchedulingPolicy::Realtime => priority as usize,
            SchedulingPolicy::Mlfq => {
                // MLFQ: start at highest priority
                0
            },
            _ => 0,
        };

        let level = level.min(self.run_queues.len() - 1);
        self.run_queues[level].enqueue(task_id);
    }

    /// Select next task to run
    pub fn schedule(&mut self) -> Option<u64> {
        match self.policy {
            SchedulingPolicy::Fifo => self.schedule_fifo(),
            SchedulingPolicy::RoundRobin => self.schedule_round_robin(),
            SchedulingPolicy::Priority => self.schedule_priority(),
            SchedulingPolicy::EarliestDeadline => self.schedule_edf(),
            SchedulingPolicy::Mlfq => self.schedule_mlfq(),
            SchedulingPolicy::Cfs => self.schedule_cfs(),
            SchedulingPolicy::Realtime => self.schedule_realtime(),
        }
    }

    /// FIFO scheduling
    fn schedule_fifo(&mut self) -> Option<u64> {
        for queue in &mut self.run_queues {
            if let Some(id) = queue.dequeue() {
                return Some(id);
            }
        }
        None
    }

    /// Round robin scheduling
    fn schedule_round_robin(&mut self) -> Option<u64> {
        // Simple round robin: take from first non-empty queue
        for queue in &mut self.run_queues {
            if let Some(id) = queue.dequeue() {
                return Some(id);
            }
        }
        None
    }

    /// Priority scheduling
    fn schedule_priority(&mut self) -> Option<u64> {
        // Check from highest priority queue first
        for queue in self.run_queues.iter_mut().rev() {
            if let Some(id) = queue.dequeue() {
                return Some(id);
            }
        }
        None
    }

    /// Earliest deadline first scheduling
    fn schedule_edf(&mut self) -> Option<u64> {
        // Find task with earliest deadline
        let mut earliest_id = None;
        let mut earliest_deadline = u64::MAX;
        let mut earliest_queue = 0;

        for (qi, queue) in self.run_queues.iter().enumerate() {
            for &task_id in &queue.tasks {
                if let Some(task) = self.tasks.get(&task_id) {
                    let deadline = task.deadline.map(|d| d.raw()).unwrap_or(u64::MAX);
                    if deadline < earliest_deadline {
                        earliest_deadline = deadline;
                        earliest_id = Some(task_id);
                        earliest_queue = qi;
                    }
                }
            }
        }

        if let Some(id) = earliest_id {
            self.run_queues[earliest_queue].remove(id);
            Some(id)
        } else {
            None
        }
    }

    /// MLFQ scheduling
    fn schedule_mlfq(&mut self) -> Option<u64> {
        // Check from highest priority (level 0) first
        for queue in &mut self.run_queues {
            if let Some(id) = queue.dequeue() {
                return Some(id);
            }
        }
        None
    }

    /// CFS scheduling
    fn schedule_cfs(&mut self) -> Option<u64> {
        // Find task with minimum virtual runtime (approximated by cpu_time)
        let mut min_id = None;
        let mut min_vruntime = u64::MAX;
        let mut min_queue = 0;

        for (qi, queue) in self.run_queues.iter().enumerate() {
            for &task_id in &queue.tasks {
                if let Some(task) = self.tasks.get(&task_id) {
                    // Virtual runtime adjusted by priority
                    let weight = self.time_slice.priority_mult[task.priority as usize];
                    let vruntime = (task.cpu_time_ns as f64 / weight) as u64;

                    if vruntime < min_vruntime {
                        min_vruntime = vruntime;
                        min_id = Some(task_id);
                        min_queue = qi;
                    }
                }
            }
        }

        if let Some(id) = min_id {
            self.run_queues[min_queue].remove(id);
            Some(id)
        } else {
            None
        }
    }

    /// Realtime scheduling
    fn schedule_realtime(&mut self) -> Option<u64> {
        self.schedule_priority()
    }

    /// Start running a task
    pub fn start(&mut self, task_id: u64) {
        let now = Timestamp::now();

        if let Some(task) = self.tasks.get_mut(&task_id) {
            if task.state == TaskState::Ready {
                task.state = TaskState::Running;
                task.started = Some(now);
                self.current = Some(task_id);
                self.stats.context_switches += 1;
            }
        }
    }

    /// Complete a task
    pub fn complete(&mut self, task_id: u64) {
        let now = Timestamp::now();

        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.state = TaskState::Completed;
            task.completed = Some(now);

            if let Some(started) = task.started {
                task.wall_time_ns = now.elapsed_since(started);
            }

            // Update statistics
            self.stats.total_completed += 1;
            if let Some(created) = Some(task.created) {
                let turnaround = now.elapsed_since(created);
                self.stats.avg_turnaround_ns = (self.stats.avg_turnaround_ns
                    * (self.stats.total_completed - 1) as f64
                    + turnaround as f64)
                    / self.stats.total_completed as f64;
            }

            // Check deadline
            if let Some(deadline) = task.deadline {
                if now.raw() > deadline.raw() + self.config.deadline_tolerance_ns {
                    self.stats.deadline_misses += 1;
                }
            }
        }

        if self.current == Some(task_id) {
            self.current = None;
        }

        // Wake up dependent tasks
        self.wake_dependents(task_id);
    }

    /// Wake up tasks waiting on completed dependency
    fn wake_dependents(&mut self, completed_id: u64) {
        let waiting: Vec<u64> = self.waiting_deps.clone();

        for task_id in waiting {
            if let Some(task) = self.tasks.get(&task_id) {
                let deps_satisfied = task.dependencies.iter().all(|dep_id| {
                    if *dep_id == completed_id {
                        true
                    } else {
                        self.tasks
                            .get(dep_id)
                            .map(|t| t.state == TaskState::Completed)
                            .unwrap_or(true)
                    }
                });

                if deps_satisfied {
                    self.waiting_deps.retain(|&id| id != task_id);
                    let priority = task.priority;
                    if let Some(t) = self.tasks.get_mut(&task_id) {
                        t.state = TaskState::Ready;
                    }
                    self.enqueue(task_id, priority);
                }
            }
        }
    }

    /// Fail a task
    pub fn fail(&mut self, task_id: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.state = TaskState::Failed;
            task.completed = Some(Timestamp::now());
            self.stats.total_failed += 1;
        }

        if self.current == Some(task_id) {
            self.current = None;
        }
    }

    /// Block a task
    pub fn block(&mut self, task_id: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.state = TaskState::Blocked;
            self.blocked.push(task_id);
        }

        if self.current == Some(task_id) {
            self.current = None;
        }
    }

    /// Unblock a task
    pub fn unblock(&mut self, task_id: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            if task.state == TaskState::Blocked {
                task.state = TaskState::Ready;
                self.blocked.retain(|&id| id != task_id);
                self.enqueue(task_id, task.priority);
            }
        }
    }

    /// Preempt current task
    pub fn preempt(&mut self) -> Option<u64> {
        if !self.config.preemption {
            return None;
        }

        if let Some(current_id) = self.current.take() {
            if let Some(task) = self.tasks.get_mut(&current_id) {
                if task.state == TaskState::Running {
                    task.state = TaskState::Ready;
                    self.enqueue(current_id, task.priority);
                    self.stats.preemptions += 1;
                    return Some(current_id);
                }
            }
        }

        None
    }

    /// Update task CPU time
    pub fn account_time(&mut self, task_id: u64, time_ns: u64) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.cpu_time_ns += time_ns;

            // MLFQ: demote if used too much time
            if self.policy == SchedulingPolicy::Mlfq {
                let level = self
                    .run_queues
                    .iter()
                    .find(|q| q.tasks.contains(&task_id))
                    .map(|q| q.level)
                    .unwrap_or(0);

                if level < self.run_queues.len() - 1 {
                    let quota = self.run_queues[level].time_slice;
                    if task.cpu_time_ns > quota {
                        // Demote
                        self.run_queues[level].remove(task_id);
                        self.run_queues[level + 1].enqueue(task_id);
                    }
                }
            }
        }
    }

    /// Boost priority
    pub fn boost(&mut self, task_id: u64, amount: i8) {
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.boost = task.boost.saturating_add(amount);
        }
    }

    /// Get task
    pub fn get_task(&self, id: u64) -> Option<&CognitiveTask> {
        self.tasks.get(&id)
    }

    /// Get current task
    pub fn current_task(&self) -> Option<&CognitiveTask> {
        self.current.and_then(|id| self.tasks.get(&id))
    }

    /// Get tasks by owner
    pub fn tasks_by_owner(&self, owner: DomainId) -> Vec<&CognitiveTask> {
        self.tasks.values().filter(|t| t.owner == owner).collect()
    }

    /// Get quantum for current task
    pub fn current_quantum(&self) -> u64 {
        self.current
            .and_then(|id| self.tasks.get(&id))
            .map(|t| self.time_slice.quantum_for(t.priority))
            .unwrap_or(self.time_slice.base_ns)
    }

    /// Get statistics
    pub fn stats(&self) -> &SchedulerStats {
        &self.stats
    }
}

impl Default for CognitiveScheduler {
    fn default() -> Self {
        Self::new(SchedulingPolicy::Mlfq, SchedulerConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let mut scheduler = CognitiveScheduler::default();
        let domain = DomainId::new(1);

        let id = scheduler.create_task(
            "test_task",
            domain,
            TaskPriority::Normal,
            TaskType::Compute,
            None,
            vec![],
        );

        let task = scheduler.get_task(id).unwrap();
        assert_eq!(task.name, "test_task");
        assert_eq!(task.state, TaskState::Ready);
    }

    #[test]
    fn test_scheduling() {
        let mut scheduler =
            CognitiveScheduler::new(SchedulingPolicy::Priority, SchedulerConfig::default());
        let domain = DomainId::new(1);

        let low_id = scheduler.create_task(
            "low",
            domain,
            TaskPriority::Low,
            TaskType::Compute,
            None,
            vec![],
        );
        let high_id = scheduler.create_task(
            "high",
            domain,
            TaskPriority::High,
            TaskType::Compute,
            None,
            vec![],
        );

        // High priority should be scheduled first
        let next = scheduler.schedule().unwrap();
        assert_eq!(next, high_id);
    }

    #[test]
    fn test_dependencies() {
        let mut scheduler = CognitiveScheduler::default();
        let domain = DomainId::new(1);

        let dep_id = scheduler.create_task(
            "dep",
            domain,
            TaskPriority::Normal,
            TaskType::Compute,
            None,
            vec![],
        );
        let main_id = scheduler.create_task(
            "main",
            domain,
            TaskPriority::Normal,
            TaskType::Compute,
            None,
            vec![dep_id],
        );

        // Main should be waiting
        assert_eq!(
            scheduler.get_task(main_id).unwrap().state,
            TaskState::WaitingDeps
        );

        // Complete dependency
        scheduler.complete(dep_id);

        // Main should now be ready
        assert_eq!(scheduler.get_task(main_id).unwrap().state, TaskState::Ready);
    }

    #[test]
    fn test_preemption() {
        let mut scheduler = CognitiveScheduler::default();
        let domain = DomainId::new(1);

        let id = scheduler.create_task(
            "task",
            domain,
            TaskPriority::Normal,
            TaskType::Compute,
            None,
            vec![],
        );

        scheduler.schedule();
        scheduler.start(id);

        let preempted = scheduler.preempt();
        assert_eq!(preempted, Some(id));
        assert_eq!(scheduler.get_task(id).unwrap().state, TaskState::Ready);
    }
}

//! # Scheduler Subsystem
//!
//! Process/task scheduler initialization and management.
//! Core phase subsystem that enables preemptive multitasking.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// =============================================================================
// TASK TYPES
// =============================================================================

/// Task ID type
pub type TaskId = u64;

/// Task priority (0 = highest, 255 = lowest)
pub type Priority = u8;

/// Task state
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Created   = 0,
    Ready     = 1,
    Running   = 2,
    Blocked   = 3,
    Sleeping  = 4,
    Suspended = 5,
    Zombie    = 6,
    Dead      = 7,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Created
    }
}

/// Task flags
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskFlags {
    pub kernel: bool,      // Kernel task (ring 0)
    pub idle: bool,        // Idle task
    pub realtime: bool,    // Real-time priority
    pub affinity: u64,     // CPU affinity mask
    pub preemptible: bool, // Can be preempted
}

/// Task control block
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub state: TaskState,
    pub priority: Priority,
    pub flags: TaskFlags,

    // Scheduling stats
    pub runtime_ns: u64,
    pub switches: u64,
    pub last_run: u64,
    pub time_slice: u64,
    pub remaining_slice: u64,

    // Context
    pub stack_ptr: u64,
    pub stack_base: u64,
    pub stack_size: usize,

    // Parent/children
    pub parent_id: Option<TaskId>,
    pub exit_code: i32,
}

impl Task {
    /// Create new task
    pub fn new(id: TaskId, name: String, priority: Priority) -> Self {
        Self {
            id,
            name,
            state: TaskState::Created,
            priority,
            flags: TaskFlags::default(),
            runtime_ns: 0,
            switches: 0,
            last_run: 0,
            time_slice: 10_000_000, // 10ms default
            remaining_slice: 10_000_000,
            stack_ptr: 0,
            stack_base: 0,
            stack_size: 0,
            parent_id: None,
            exit_code: 0,
        }
    }

    /// Is task runnable?
    pub fn is_runnable(&self) -> bool {
        matches!(self.state, TaskState::Ready | TaskState::Running)
    }
}

// =============================================================================
// SCHEDULING ALGORITHMS
// =============================================================================

/// Scheduling algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerAlgorithm {
    /// Round-robin with fixed time slices
    RoundRobin,
    /// Priority-based preemptive
    Priority,
    /// Completely Fair Scheduler (Linux-style)
    Cfs,
    /// Multi-Level Feedback Queue
    Mlfq,
    /// Earliest Deadline First (real-time)
    Edf,
    /// Rate Monotonic (real-time)
    RateMonotonic,
}

impl Default for SchedulerAlgorithm {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Run queue for round-robin
pub struct RunQueue {
    tasks: VecDeque<TaskId>,
}

impl RunQueue {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    pub fn push(&mut self, id: TaskId) {
        self.tasks.push_back(id);
    }

    pub fn pop(&mut self) -> Option<TaskId> {
        self.tasks.pop_front()
    }

    pub fn remove(&mut self, id: TaskId) -> bool {
        if let Some(pos) = self.tasks.iter().position(|&t| t == id) {
            self.tasks.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

impl Default for RunQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority queue for priority scheduling
pub struct PriorityQueue {
    // 256 priority levels
    queues: [VecDeque<TaskId>; 256],
    bitmap: [u64; 4], // 256 bits for quick lookup
}

impl PriorityQueue {
    pub fn new() -> Self {
        Self {
            queues: core::array::from_fn(|_| VecDeque::new()),
            bitmap: [0; 4],
        }
    }

    pub fn push(&mut self, id: TaskId, priority: Priority) {
        let idx = priority as usize;
        self.queues[idx].push_back(id);
        self.bitmap[idx / 64] |= 1 << (idx % 64);
    }

    pub fn pop_highest(&mut self) -> Option<(TaskId, Priority)> {
        // Find highest priority (lowest number)
        for (word_idx, &word) in self.bitmap.iter().enumerate() {
            if word != 0 {
                let bit = word.trailing_zeros() as usize;
                let priority = (word_idx * 64 + bit) as Priority;

                if let Some(id) = self.queues[priority as usize].pop_front() {
                    if self.queues[priority as usize].is_empty() {
                        self.bitmap[word_idx] &= !(1 << bit);
                    }
                    return Some((id, priority));
                }
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.bitmap.iter().all(|&w| w == 0)
    }
}

impl Default for PriorityQueue {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// SCHEDULER SUBSYSTEM
// =============================================================================

/// Scheduler subsystem
///
/// Manages task scheduling and context switching.
pub struct SchedulerSubsystem {
    info: SubsystemInfo,
    algorithm: SchedulerAlgorithm,

    // Task management
    tasks: Vec<Task>,
    next_task_id: AtomicU64,
    current_task: AtomicU64,
    idle_task_id: TaskId,

    // Run queues
    run_queue: RunQueue,
    priority_queue: PriorityQueue,

    // Scheduler state
    enabled: AtomicBool,
    preempt_count: AtomicU32,
    need_resched: AtomicBool,

    // Statistics
    context_switches: AtomicU64,
    total_runtime: AtomicU64,

    // Configuration
    default_time_slice: u64,
    min_time_slice: u64,
    max_time_slice: u64,
}

static SCHEDULER_DEPS: [Dependency; 2] = [
    Dependency::required("timers"),
    Dependency::required("interrupts"),
];

impl SchedulerSubsystem {
    /// Create new scheduler subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("scheduler", InitPhase::Core)
                .with_priority(800)
                .with_description("Task scheduler")
                .with_dependencies(&SCHEDULER_DEPS)
                .provides(PhaseCapabilities::SCHEDULER)
                .essential(),
            algorithm: SchedulerAlgorithm::RoundRobin,
            tasks: Vec::new(),
            next_task_id: AtomicU64::new(1),
            current_task: AtomicU64::new(0),
            idle_task_id: 0,
            run_queue: RunQueue::new(),
            priority_queue: PriorityQueue::new(),
            enabled: AtomicBool::new(false),
            preempt_count: AtomicU32::new(0),
            need_resched: AtomicBool::new(false),
            context_switches: AtomicU64::new(0),
            total_runtime: AtomicU64::new(0),
            default_time_slice: 10_000_000, // 10ms
            min_time_slice: 1_000_000,      // 1ms
            max_time_slice: 100_000_000,    // 100ms
        }
    }

    /// Get scheduling algorithm
    pub fn algorithm(&self) -> SchedulerAlgorithm {
        self.algorithm
    }

    /// Set scheduling algorithm
    pub fn set_algorithm(&mut self, algo: SchedulerAlgorithm) {
        self.algorithm = algo;
    }

    /// Is scheduler enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Enable scheduler
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable scheduler
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Get current task ID
    pub fn current(&self) -> TaskId {
        self.current_task.load(Ordering::SeqCst)
    }

    /// Get current task
    pub fn current_task(&self) -> Option<&Task> {
        let id = self.current();
        self.get_task(id)
    }

    /// Get task by ID
    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// Get task by ID (mutable)
    pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Create new task
    pub fn create_task(&mut self, name: &str, priority: Priority) -> TaskId {
        let id = self.next_task_id.fetch_add(1, Ordering::SeqCst);

        let task = Task::new(id, String::from(name), priority);
        self.tasks.push(task);

        id
    }

    /// Make task ready
    pub fn make_ready(&mut self, id: TaskId) {
        if let Some(task) = self.get_task_mut(id) {
            task.state = TaskState::Ready;

            match self.algorithm {
                SchedulerAlgorithm::RoundRobin => {
                    self.run_queue.push(id);
                },
                SchedulerAlgorithm::Priority | SchedulerAlgorithm::Cfs => {
                    let priority = task.priority;
                    self.priority_queue.push(id, priority);
                },
                _ => {
                    self.run_queue.push(id);
                },
            }
        }
    }

    /// Block current task
    pub fn block_current(&mut self) {
        let id = self.current();
        if let Some(task) = self.get_task_mut(id) {
            task.state = TaskState::Blocked;
        }
        self.need_resched.store(true, Ordering::SeqCst);
    }

    /// Wake up task
    pub fn wake(&mut self, id: TaskId) {
        if let Some(task) = self.get_task_mut(id) {
            if task.state == TaskState::Blocked || task.state == TaskState::Sleeping {
                self.make_ready(id);
            }
        }
    }

    /// Yield current task
    pub fn yield_current(&mut self) {
        let id = self.current();
        if let Some(task) = self.get_task_mut(id) {
            task.state = TaskState::Ready;
            self.make_ready(id);
        }
        self.need_resched.store(true, Ordering::SeqCst);
    }

    /// Exit current task
    pub fn exit(&mut self, code: i32) {
        let id = self.current();
        if let Some(task) = self.get_task_mut(id) {
            task.state = TaskState::Zombie;
            task.exit_code = code;
        }
        self.need_resched.store(true, Ordering::SeqCst);
    }

    /// Get number of runnable tasks
    pub fn runnable_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.is_runnable()).count()
    }

    /// Get total task count
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Disable preemption
    pub fn preempt_disable(&self) {
        self.preempt_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Enable preemption
    pub fn preempt_enable(&self) {
        let prev = self.preempt_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 && self.need_resched.load(Ordering::SeqCst) {
            // Would trigger reschedule here
        }
    }

    /// Is preemption enabled?
    pub fn is_preemptible(&self) -> bool {
        self.preempt_count.load(Ordering::SeqCst) == 0
    }

    /// Pick next task (round-robin)
    fn pick_next_rr(&mut self) -> Option<TaskId> {
        self.run_queue.pop()
    }

    /// Pick next task (priority)
    fn pick_next_priority(&mut self) -> Option<TaskId> {
        self.priority_queue.pop_highest().map(|(id, _)| id)
    }

    /// Pick next task
    pub fn pick_next(&mut self) -> Option<TaskId> {
        match self.algorithm {
            SchedulerAlgorithm::RoundRobin => self.pick_next_rr(),
            SchedulerAlgorithm::Priority | SchedulerAlgorithm::Cfs => self.pick_next_priority(),
            _ => self.pick_next_rr(),
        }
    }

    /// Schedule (called from timer interrupt)
    pub fn schedule(&mut self) {
        if !self.is_enabled() || !self.is_preemptible() {
            return;
        }

        let current_id = self.current();

        // Put current back if still runnable
        if let Some(current) = self.get_task(current_id) {
            if current.state == TaskState::Running {
                match self.algorithm {
                    SchedulerAlgorithm::RoundRobin => {
                        self.run_queue.push(current_id);
                    },
                    SchedulerAlgorithm::Priority => {
                        let priority = current.priority;
                        self.priority_queue.push(current_id, priority);
                    },
                    _ => {},
                }
            }
        }

        // Pick next
        if let Some(next_id) = self.pick_next() {
            if next_id != current_id {
                self.switch_to(next_id);
            }
        } else if current_id != self.idle_task_id {
            // No runnable tasks, switch to idle
            self.switch_to(self.idle_task_id);
        }

        self.need_resched.store(false, Ordering::SeqCst);
    }

    /// Context switch to task
    fn switch_to(&mut self, next_id: TaskId) {
        let current_id = self.current();

        // Update states
        if let Some(current) = self.get_task_mut(current_id) {
            if current.state == TaskState::Running {
                current.state = TaskState::Ready;
            }
        }

        if let Some(next) = self.get_task_mut(next_id) {
            next.state = TaskState::Running;
            next.switches += 1;
        }

        self.current_task.store(next_id, Ordering::SeqCst);
        self.context_switches.fetch_add(1, Ordering::Relaxed);

        // In real code: perform actual context switch via assembly
    }

    /// Timer tick handler
    pub fn timer_tick(&mut self) {
        let id = self.current();
        if let Some(task) = self.get_task_mut(id) {
            if task.remaining_slice > 0 {
                task.remaining_slice -= 1;
            }

            if task.remaining_slice == 0 {
                task.remaining_slice = task.time_slice;
                self.need_resched.store(true, Ordering::SeqCst);
            }
        }
    }

    /// Create idle task
    fn create_idle_task(&mut self) -> TaskId {
        let id = self.create_task("idle", 255); // Lowest priority

        if let Some(task) = self.get_task_mut(id) {
            task.flags.idle = true;
            task.flags.kernel = true;
            task.state = TaskState::Ready;
        }

        id
    }

    /// Get context switch count
    pub fn context_switches(&self) -> u64 {
        self.context_switches.load(Ordering::Relaxed)
    }
}

impl Default for SchedulerSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for SchedulerSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing scheduler");

        // Get algorithm from config
        let algo_name = ctx.config().get_str("scheduler_algorithm", "round_robin");
        self.algorithm = match algo_name.as_str() {
            "priority" => SchedulerAlgorithm::Priority,
            "cfs" => SchedulerAlgorithm::Cfs,
            "mlfq" => SchedulerAlgorithm::Mlfq,
            "edf" => SchedulerAlgorithm::Edf,
            _ => SchedulerAlgorithm::RoundRobin,
        };

        ctx.info(alloc::format!("Scheduler algorithm: {:?}", self.algorithm));

        // Get time slice config
        self.default_time_slice = ctx.config().get_uint("time_slice_ms", 10) as u64 * 1_000_000;

        // Create idle task
        self.idle_task_id = self.create_idle_task();
        ctx.debug(alloc::format!("Idle task ID: {}", self.idle_task_id));

        // Create kernel main task (current context)
        let main_id = self.create_task("kernel_main", 0);
        if let Some(task) = self.get_task_mut(main_id) {
            task.flags.kernel = true;
            task.state = TaskState::Running;
        }
        self.current_task.store(main_id, Ordering::SeqCst);

        ctx.info(alloc::format!(
            "Scheduler initialized: {} tasks, time slice: {} ms",
            self.task_count(),
            self.default_time_slice / 1_000_000
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Scheduler shutdown");

        // Disable scheduler
        self.disable();

        // Log stats
        ctx.info(alloc::format!(
            "Context switches: {}, tasks created: {}",
            self.context_switches(),
            self.task_count()
        ));

        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_subsystem() {
        let sub = SchedulerSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Core);
        assert!(sub.info().provides.contains(PhaseCapabilities::SCHEDULER));
    }

    #[test]
    fn test_task_creation() {
        let mut sched = SchedulerSubsystem::new();

        let id = sched.create_task("test", 128);
        assert!(id > 0);

        let task = sched.get_task(id).unwrap();
        assert_eq!(task.name, "test");
        assert_eq!(task.priority, 128);
        assert_eq!(task.state, TaskState::Created);
    }

    #[test]
    fn test_run_queue() {
        let mut queue = RunQueue::new();

        queue.push(1);
        queue.push(2);
        queue.push(3);

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(3));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_priority_queue() {
        let mut queue = PriorityQueue::new();

        queue.push(1, 100);
        queue.push(2, 50);
        queue.push(3, 0);

        assert!(!queue.is_empty());
        assert_eq!(queue.pop_highest(), Some((3, 0))); // Highest priority
        assert_eq!(queue.pop_highest(), Some((2, 50)));
        assert_eq!(queue.pop_highest(), Some((1, 100)));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_preempt_count() {
        let sched = SchedulerSubsystem::new();

        assert!(sched.is_preemptible());

        sched.preempt_disable();
        assert!(!sched.is_preemptible());

        sched.preempt_enable();
        assert!(sched.is_preemptible());
    }
}

//! # Scheduler Traits
//!
//! Defines the traits that scheduler implementations must implement.

use crate::{ThreadId, ProcessId, ExecResult};
use super::Priority;
use alloc::string::String;
use alloc::vec::Vec;

/// Thread information for scheduling
#[derive(Debug, Clone)]
pub struct SchedulableThread {
    /// Thread ID
    pub id: ThreadId,
    /// Process ID
    pub process: ProcessId,
    /// Thread priority
    pub priority: Priority,
    /// CPU affinity mask (bit per CPU)
    pub affinity: u64,
    /// Thread name (for debugging)
    pub name: String,
    /// Is this a kernel thread?
    pub is_kernel: bool,
    /// Is this a real-time thread?
    pub is_realtime: bool,
}

impl SchedulableThread {
    /// Create a new schedulable thread
    pub fn new(id: ThreadId, process: ProcessId, priority: Priority) -> Self {
        Self {
            id,
            process,
            priority,
            affinity: u64::MAX, // All CPUs
            name: String::new(),
            is_kernel: false,
            is_realtime: false,
        }
    }

    /// Set the thread name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Mark as kernel thread
    pub fn kernel(mut self) -> Self {
        self.is_kernel = true;
        self
    }

    /// Mark as real-time thread
    pub fn realtime(mut self) -> Self {
        self.is_realtime = true;
        self
    }

    /// Set CPU affinity
    pub fn with_affinity(mut self, affinity: u64) -> Self {
        self.affinity = affinity;
        self
    }
}

/// The core scheduler trait
///
/// All scheduler implementations must implement this trait.
pub trait Scheduler: Send + Sync {
    /// Get the scheduler name
    fn name(&self) -> &'static str;

    /// Get the scheduler version
    fn version(&self) -> &'static str;

    /// Initialize the scheduler
    fn init(&mut self, cpu_count: usize) -> ExecResult<()>;

    /// Pick the next thread to run on a CPU
    fn pick_next(&self, cpu: usize) -> Option<ThreadId>;

    /// Add a thread to the scheduler
    fn add_thread(&self, thread: SchedulableThread) -> ExecResult<()>;

    /// Remove a thread from the scheduler
    fn remove_thread(&self, id: ThreadId) -> ExecResult<()>;

    /// Mark a thread as ready
    fn thread_ready(&self, id: ThreadId) -> ExecResult<()>;

    /// Mark a thread as blocked
    fn thread_block(&self, id: ThreadId) -> ExecResult<()>;

    /// Yield the current thread (voluntary preemption)
    fn yield_thread(&self, cpu: usize);

    /// Timer tick - called periodically
    fn tick(&self, cpu: usize);

    /// Set thread priority
    fn set_priority(&self, id: ThreadId, priority: Priority) -> ExecResult<()>;

    /// Get thread priority
    fn get_priority(&self, id: ThreadId) -> Option<Priority>;

    /// Check if a reschedule is needed
    fn needs_reschedule(&self, cpu: usize) -> bool;

    /// Get scheduler statistics
    fn stats(&self) -> SchedulerStats;

    /// Set scheduling policy for a thread
    fn set_policy(&self, _id: ThreadId, _policy: SchedulingPolicy) -> ExecResult<()> {
        Err(crate::ExecError::InvalidArgument)
    }

    /// Migrate a thread to another CPU
    fn migrate_thread(&self, _id: ThreadId, _target_cpu: usize) -> ExecResult<()> {
        Err(crate::ExecError::InvalidArgument)
    }
}

/// Scheduling policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Normal time-sharing
    Normal,
    /// First-in-first-out (for real-time)
    Fifo,
    /// Round-robin (for real-time)
    RoundRobin,
    /// Batch scheduling
    Batch,
    /// Idle (only runs when nothing else to do)
    Idle,
    /// Deadline scheduling
    Deadline {
        /// Runtime in nanoseconds
        runtime: u64,
        /// Period in nanoseconds
        period: u64,
        /// Deadline in nanoseconds
        deadline: u64,
    },
}

/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    /// Total number of context switches
    pub context_switches: u64,
    /// Number of threads currently runnable
    pub runnable_threads: usize,
    /// Number of threads currently blocked
    pub blocked_threads: usize,
    /// Average wait time (nanoseconds)
    pub avg_wait_time: u64,
    /// Average run time (nanoseconds)
    pub avg_run_time: u64,
    /// Load per CPU
    pub cpu_load: Vec<u8>,
}

/// Load balancer trait for SMP systems
pub trait LoadBalancer: Send + Sync {
    /// Balance load across CPUs
    fn balance(&self, scheduler: &dyn Scheduler);

    /// Get load for a CPU
    fn cpu_load(&self, cpu: usize) -> u32;

    /// Suggest migration for a thread
    fn suggest_migration(&self, thread: ThreadId, current_cpu: usize) -> Option<usize>;
}

/// Per-CPU scheduler data
pub trait PerCpuScheduler: Send + Sync {
    /// Get the current thread on this CPU
    fn current_thread(&self) -> Option<ThreadId>;

    /// Set the current thread on this CPU
    fn set_current_thread(&self, id: Option<ThreadId>);

    /// Get the run queue for this CPU
    fn run_queue(&self) -> &dyn RunQueue;

    /// Get the run queue for this CPU (mutable)
    fn run_queue_mut(&mut self) -> &mut dyn RunQueue;
}

/// Run queue trait
pub trait RunQueue: Send + Sync {
    /// Add a thread to the queue
    fn enqueue(&mut self, id: ThreadId, priority: Priority);

    /// Remove and return the next thread
    fn dequeue(&mut self) -> Option<ThreadId>;

    /// Peek at the next thread without removing
    fn peek(&self) -> Option<ThreadId>;

    /// Check if the queue is empty
    fn is_empty(&self) -> bool;

    /// Get the number of threads in the queue
    fn len(&self) -> usize;

    /// Remove a specific thread
    fn remove(&mut self, id: ThreadId) -> bool;
}

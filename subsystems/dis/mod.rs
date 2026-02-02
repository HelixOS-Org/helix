//! # Dynamic Intent Scheduling (DIS) - Revolutionary Scheduling Subsystem
//!
//! DIS is a next-generation task scheduling system that goes beyond traditional
//! priority-based scheduling. It introduces **intent-aware scheduling** where
//! tasks declare their intentions and the scheduler optimizes globally.
//!
//! ## Key Innovations
//!
//! 1. **Intent-Based Scheduling**: Tasks declare what they want to achieve,
//!    not just their priority. The scheduler understands semantics.
//!
//! 2. **Adaptive Policies**: Scheduling policies evolve at runtime based on
//!    observed patterns and system load.
//!
//! 3. **Statistical Learning**: The scheduler learns from execution patterns
//!    to predict and optimize future scheduling decisions.
//!
//! 4. **Security Isolation**: Each scheduling domain is isolated with
//!    capability-based access control.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    DIS - Dynamic Intent Scheduling               │
//! │                                                                  │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
//! │  │   Intent    │  │   Policy    │  │  Adaptive   │              │
//! │  │   Engine    │──│   Engine    │──│  Optimizer  │              │
//! │  └─────────────┘  └─────────────┘  └─────────────┘              │
//! │         │                │                │                      │
//! │         ▼                ▼                ▼                      │
//! │  ┌──────────────────────────────────────────────────────────┐   │
//! │  │              DIS SCHEDULER CORE                           │   │
//! │  │  [RT Queue] [Interactive] [Batch] [Background] [Idle]    │   │
//! │  └──────────────────────────────────────────────────────────┘   │
//! │         │                │                │                      │
//! │         ▼                ▼                ▼                      │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
//! │  │   Stats     │  │  Execution  │  │  Security   │              │
//! │  │  Collector  │  │   Engine    │  │  Isolator   │              │
//! │  └─────────────┘  └─────────────┘  └─────────────┘              │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use helix_dis::{Intent, IntentClass, DISScheduler};
//!
//! // Create a scheduler instance
//! let mut scheduler = DISScheduler::new();
//!
//! // Spawn a task with intent
//! let intent = Intent::new()
//!     .class(IntentClass::Interactive)
//!     .latency_target(Duration::from_millis(16))  // 60 FPS target
//!     .cpu_budget(CpuBudget::Percent(30))
//!     .build();
//!
//! scheduler.spawn_with_intent("ui_render", task_fn, intent);
//!
//! // The scheduler will automatically:
//! // - Prioritize this task for low latency
//! // - Ensure it gets ~30% CPU time
//! // - Learn its execution pattern
//! // - Adapt scheduling based on actual behavior
//! ```
//!
//! ## Revolutionary Aspects
//!
//! | Feature | Traditional | DIS |
//! |---------|-------------|-----|
//! | Priority | Static number | Dynamic intent |
//! | Policy | Fixed algorithm | Adaptive |
//! | Learning | None | Statistical |
//! | Isolation | Process-level | Capability-based |
//! | Optimization | Local | Global |

#![no_std]
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate alloc;

// =============================================================================
// Module Declarations
// =============================================================================

pub mod api;
pub mod executor;
pub mod intent;
pub mod ipc;
pub mod isolation;
pub mod optimizer;
pub mod policy;
pub mod queues;
pub mod scheduler;
pub mod stats;

// =============================================================================
// Re-exports
// =============================================================================

// =============================================================================
// Core Types
// =============================================================================
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub use api::{DISEvent, TaskHandle, DIS};
pub use executor::{ExecutionContext, Executor};
pub use intent::{Intent, IntentBuilder, IntentClass, IntentFlags, IntentId};
pub use ipc::{IPCManager, Message, MessagePayload, Request, Response};
pub use isolation::{Capability, IsolationLevel, SecurityDomain};
pub use optimizer::{AdaptiveOptimizer, OptimizationHint};
pub use policy::{Policy, PolicyAction, PolicyEngine, PolicyId, PolicyRule};
pub use queues::{MultiLevelQueue, QueueManager};
pub use scheduler::{DISScheduler, SchedulerConfig, SchedulerStats};
use spin::RwLock;
pub use stats::{StatsCollector, StatsSnapshot, SystemStats, TaskStats};

/// Unique identifier for a task in DIS
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TaskId(pub u64);

impl TaskId {
    /// Create a new task ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Invalid/null task ID
    pub const NULL: Self = Self(0);
}

/// CPU core identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct CpuId(pub u32);

impl CpuId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }

    pub fn id(&self) -> u32 {
        self.0
    }
}

/// Time measurement in nanoseconds
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Nanoseconds(pub u64);

impl Nanoseconds {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(u64::MAX);

    pub const fn new(ns: u64) -> Self {
        Self(ns)
    }

    /// Alias for ZERO
    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn from_micros(us: u64) -> Self {
        Self(us * 1_000)
    }

    pub const fn from_millis(ms: u64) -> Self {
        Self(ms * 1_000_000)
    }

    pub const fn from_secs(s: u64) -> Self {
        Self(s * 1_000_000_000)
    }

    pub const fn as_micros(&self) -> u64 {
        self.0 / 1_000
    }

    pub const fn as_millis(&self) -> u64 {
        self.0 / 1_000_000
    }

    pub const fn as_secs(&self) -> u64 {
        self.0 / 1_000_000_000
    }

    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl core::ops::Add for Nanoseconds {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl core::ops::Sub for Nanoseconds {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl core::ops::AddAssign for Nanoseconds {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.saturating_add(rhs.0);
    }
}

/// CPU budget specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuBudget {
    /// No limit
    Unlimited,
    /// Percentage of CPU time (0-100)
    Percent(u8),
    /// Absolute time per period
    Quota {
        quota: Nanoseconds,
        period: Nanoseconds,
    },
    /// Shares-based (like Linux cgroups)
    Shares(u32),
}

impl Default for CpuBudget {
    fn default() -> Self {
        Self::Unlimited
    }
}

/// Memory budget specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryBudget {
    /// No limit
    Unlimited,
    /// Hard limit in bytes
    Limit(u64),
    /// Soft limit with eviction priority
    Soft { limit: u64, priority: u8 },
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::Unlimited
    }
}

/// I/O budget specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoBudget {
    /// No limit
    Unlimited,
    /// Bandwidth limit in bytes per second
    Bandwidth(u64),
    /// IOPS limit
    Iops(u32),
    /// Combined limits
    Combined { bandwidth: u64, iops: u32 },
}

impl Default for IoBudget {
    fn default() -> Self {
        Self::Unlimited
    }
}

/// Task state in DIS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskState {
    /// Task is ready to run
    Ready     = 0,
    /// Task is currently running
    Running   = 1,
    /// Task is blocked waiting for something
    Blocked   = 2,
    /// Task is sleeping
    Sleeping  = 3,
    /// Task is suspended
    Suspended = 4,
    /// Task has completed
    Completed = 5,
    /// Task was killed
    Killed    = 6,
    /// Task is being migrated between CPUs
    Migrating = 7,
}

impl TaskState {
    /// Check if task is runnable
    pub fn is_runnable(&self) -> bool {
        matches!(self, Self::Ready | Self::Running)
    }

    /// Check if task is terminated
    pub fn is_terminated(&self) -> bool {
        matches!(self, Self::Completed | Self::Killed)
    }
}

/// A task descriptor in DIS
#[derive(Debug, Clone)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,
    /// Task name for debugging
    pub name: String,
    /// Current state
    pub state: TaskState,
    /// Associated intent
    pub intent: Intent,
    /// Execution statistics
    pub stats: TaskStats,
    /// Security domain
    pub domain: SecurityDomain,
    /// Assigned CPU (if any)
    pub cpu: Option<CpuId>,
    /// Parent task (if any)
    pub parent: Option<TaskId>,
    /// Child tasks
    pub children: Vec<TaskId>,
    /// Creation timestamp
    pub created_at: Nanoseconds,
    /// Last scheduled timestamp
    pub last_scheduled: Nanoseconds,
    /// Accumulated runtime
    pub runtime: Nanoseconds,
    /// Wait time accumulator
    pub wait_time: Nanoseconds,
    /// Current effective priority (computed)
    pub effective_priority: i32,
    /// Base priority from intent
    pub base_priority: i32,
    /// Priority boost/penalty from optimizer
    pub priority_adjustment: i32,
    /// Deadline (if real-time)
    pub deadline: Option<Nanoseconds>,
    /// Time slice remaining
    pub time_slice: Nanoseconds,
    /// Preemption count
    pub preempt_count: u32,
    /// Context switch count
    pub context_switches: u64,
    /// Voluntary yields
    pub voluntary_yields: u64,
    /// Involuntary preemptions
    pub involuntary_preemptions: u64,
    /// CPU affinity mask
    pub affinity: u64,
    /// CPU affinity (alternative name)
    pub cpu_affinity: u64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Flags
    pub flags: TaskFlags,
}

bitflags::bitflags! {
    /// Task flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TaskFlags: u32 {
        /// Task is a kernel thread
        const KERNEL = 1 << 0;
        /// Task is idle
        const IDLE = 1 << 1;
        /// Task should not be preempted
        const NO_PREEMPT = 1 << 2;
        /// Task is real-time
        const REALTIME = 1 << 3;
        /// Task is pinned to a CPU
        const PINNED = 1 << 4;
        /// Task is interactive
        const INTERACTIVE = 1 << 5;
        /// Task is batch processing
        const BATCH = 1 << 6;
        /// Task is I/O bound
        const IO_BOUND = 1 << 7;
        /// Task is CPU bound
        const CPU_BOUND = 1 << 8;
        /// Task needs low latency
        const LOW_LATENCY = 1 << 9;
        /// Task is being debugged
        const DEBUGGED = 1 << 10;
        /// Task is frozen
        const FROZEN = 1 << 11;
        /// Task is migrating
        const MIGRATING = 1 << 12;
        /// Task has CPU affinity
        const CPU_AFFINITY = 1 << 13;
        /// Task has deadline
        const DEADLINE = 1 << 14;
    }
}

impl Default for TaskFlags {
    fn default() -> Self {
        Self::empty()
    }
}

impl Task {
    /// Create a new task with the given intent
    pub fn new(id: TaskId, name: String, intent: Intent) -> Self {
        let base_priority = intent.compute_base_priority();
        let flags = intent.compute_flags();
        let deadline = intent.deadline();
        let time_slice = intent.compute_time_slice();

        Self {
            id,
            name,
            state: TaskState::Ready,
            intent,
            stats: TaskStats::new(),
            domain: SecurityDomain::default(),
            cpu: None,
            parent: None,
            children: Vec::new(),
            created_at: Nanoseconds::ZERO,
            last_scheduled: Nanoseconds::ZERO,
            runtime: Nanoseconds::ZERO,
            wait_time: Nanoseconds::ZERO,
            effective_priority: base_priority,
            base_priority,
            priority_adjustment: 0,
            deadline,
            time_slice,
            preempt_count: 0,
            context_switches: 0,
            voluntary_yields: 0,
            involuntary_preemptions: 0,
            affinity: u64::MAX, // All CPUs
            cpu_affinity: u64::MAX,
            memory_usage: 0,
            flags,
        }
    }

    /// Recalculate effective priority
    pub fn recalculate_priority(&mut self) {
        self.effective_priority = self.base_priority.saturating_add(self.priority_adjustment);

        // Apply wait time boost (tasks waiting longer get priority boost)
        let wait_boost = (self.wait_time.as_millis() / 100) as i32;
        self.effective_priority = self.effective_priority.saturating_sub(wait_boost.min(20));
        // Max 20 points boost
    }

    /// Check if this task should preempt another
    pub fn should_preempt(&self, other: &Task) -> bool {
        // Real-time always preempts non-real-time
        if self.flags.contains(TaskFlags::REALTIME) && !other.flags.contains(TaskFlags::REALTIME) {
            return true;
        }

        // Deadline-based for real-time tasks
        if let (Some(my_deadline), Some(other_deadline)) = (self.deadline, other.deadline) {
            return my_deadline < other_deadline;
        }

        // Priority-based for normal tasks
        self.effective_priority < other.effective_priority
    }

    /// Update runtime statistics
    pub fn update_runtime(&mut self, delta: Nanoseconds) {
        self.runtime += delta;
        self.stats.update_runtime(delta);
    }

    /// Update wait time
    pub fn update_wait_time(&mut self, delta: Nanoseconds) {
        self.wait_time += delta;
        self.stats.update_wait_time(delta);
    }
}

// =============================================================================
// DIS Error Types
// =============================================================================

/// Errors that can occur in DIS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DISError {
    /// Task not found
    TaskNotFound(TaskId),
    /// Invalid intent specification
    InvalidIntent,
    /// Policy violation
    PolicyViolation,
    /// Security violation
    SecurityViolation,
    /// Resource exhausted
    ResourceExhausted,
    /// Invalid state transition
    InvalidState,
    /// CPU not found
    CpuNotFound,
    /// No CPU available
    NoCpuAvailable,
    /// Queue full
    QueueFull,
    /// Deadline missed
    DeadlineMissed,
    /// Permission denied
    PermissionDenied,
    /// Already exists
    AlreadyExists,
    /// Not supported
    NotSupported,
    /// Internal error
    Internal,
    // IPC-related errors
    /// No recipient specified
    NoRecipient,
    /// Channel not found
    ChannelNotFound,
    /// Channel is closed
    ChannelClosed,
    /// Not a channel endpoint
    NotChannelEndpoint,
    /// Message queue full
    MessageQueueFull,
    /// IPC timeout
    IpcTimeout,
    // Security domain errors
    /// Domain not found
    DomainNotFound(u64),
    /// Domain not empty
    DomainNotEmpty,
    /// Capability exceeds max
    CapabilityExceedsMax,
}

/// Result type for DIS operations
pub type DISResult<T> = Result<T, DISError>;

// =============================================================================
// Global DIS Instance
// =============================================================================

/// Global DIS scheduler instance
static DIS_SCHEDULER: RwLock<Option<DISScheduler>> = RwLock::new(None);

/// Global ID counter for tasks
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

/// Global ID counter for intents
static NEXT_INTENT_ID: AtomicU64 = AtomicU64::new(1);

/// Global ID counter for policies
static NEXT_POLICY_ID: AtomicU64 = AtomicU64::new(1);

/// Is DIS initialized?
static DIS_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Generate a new unique task ID
pub fn generate_task_id() -> TaskId {
    TaskId::new(NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst))
}

/// Generate a new unique intent ID
pub fn generate_intent_id() -> IntentId {
    IntentId::new(NEXT_INTENT_ID.fetch_add(1, Ordering::SeqCst))
}

/// Generate a new unique policy ID
pub fn generate_policy_id() -> PolicyId {
    PolicyId::new(NEXT_POLICY_ID.fetch_add(1, Ordering::SeqCst))
}

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the DIS subsystem
pub fn init() {
    if DIS_INITIALIZED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }

    log_dis("[DIS] Initializing Dynamic Intent Scheduling subsystem...");

    // Create the scheduler
    let config = SchedulerConfig::default();
    let scheduler = DISScheduler::new(config);

    *DIS_SCHEDULER.write() = Some(scheduler);

    log_dis("[DIS] ✓ Scheduler core initialized");
    log_dis("[DIS] ✓ Intent engine ready");
    log_dis("[DIS] ✓ Policy engine ready");
    log_dis("[DIS] ✓ Statistics collector ready");
    log_dis("[DIS] ✓ Adaptive optimizer ready");
    log_dis("[DIS] ✓ Security isolator ready");
    log_dis("[DIS] Dynamic Intent Scheduling is ACTIVE\n");
}

/// Check if DIS is initialized
pub fn is_initialized() -> bool {
    DIS_INITIALIZED.load(Ordering::SeqCst)
}

/// Get access to the global scheduler
pub fn scheduler() -> spin::RwLockReadGuard<'static, Option<DISScheduler>> {
    DIS_SCHEDULER.read()
}

/// Get mutable access to the global scheduler
pub fn scheduler_mut() -> spin::RwLockWriteGuard<'static, Option<DISScheduler>> {
    DIS_SCHEDULER.write()
}

// =============================================================================
// Logging Helper
// =============================================================================

fn log_dis(msg: &str) {
    for &c in msg.as_bytes() {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3F8u16,
                in("al") c,
                options(nomem, nostack)
            );
        }
    }
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3F8u16,
            in("al") b'\n',
            options(nomem, nostack)
        );
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id() {
        let id1 = generate_task_id();
        let id2 = generate_task_id();
        assert_ne!(id1, id2);
        assert!(id2.raw() > id1.raw());
    }

    #[test]
    fn test_nanoseconds() {
        let ns = Nanoseconds::from_millis(1000);
        assert_eq!(ns.as_secs(), 1);
        assert_eq!(ns.as_millis(), 1000);
        assert_eq!(ns.as_micros(), 1_000_000);
    }

    #[test]
    fn test_task_state() {
        assert!(TaskState::Ready.is_runnable());
        assert!(TaskState::Running.is_runnable());
        assert!(!TaskState::Blocked.is_runnable());
        assert!(TaskState::Completed.is_terminated());
        assert!(TaskState::Killed.is_terminated());
    }
}

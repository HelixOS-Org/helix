//! # Task System
//!
//! The core task abstraction for processes and threads.
//! Supports both kernel tasks (ring 0) and user tasks (ring 3).
//!
//! ## Design
//!
//! - Tasks are the fundamental unit of execution
//! - Each task has its own stack and register context
//! - Context switches are done cooperatively (yield) or preemptively (timer)
//! - Tasks can be kernel-only or have userspace components

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use spin::RwLock;

/// Task ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TaskId(u64);

impl TaskId {
    /// Create a new unique task ID
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Kernel idle task ID
    pub const IDLE: Self = Self(0);
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskState {
    /// Task is ready to run
    Ready   = 0,
    /// Task is currently running
    Running = 1,
    /// Task is blocked waiting for something
    Blocked = 2,
    /// Task has exited
    Dead    = 3,
}

/// Task privilege level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskPrivilege {
    /// Kernel task (ring 0)
    Kernel = 0,
    /// User task (ring 3)
    User   = 3,
}

/// CPU register context saved during context switch
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct CpuContext {
    // Callee-saved registers (must be preserved across function calls)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,

    // Return address (rip when we switch back)
    pub rip: u64,

    // Stack pointer
    pub rsp: u64,

    // Flags
    pub rflags: u64,

    // For userspace tasks
    pub cs: u64,
    pub ss: u64,
}

impl CpuContext {
    /// Create a new context for a kernel task
    pub fn new_kernel(entry_point: u64, stack_top: u64) -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rip: entry_point,
            rsp: stack_top,
            rflags: 0x202, // Interrupts enabled
            cs: 0x08,      // Kernel code segment
            ss: 0x10,      // Kernel data segment
        }
    }

    /// Create a new context for a user task
    pub fn new_user(entry_point: u64, stack_top: u64, kernel_stack: u64) -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rip: entry_point,
            rsp: stack_top,
            rflags: 0x202, // Interrupts enabled
            cs: 0x1B,      // User code segment (ring 3) | 0x18 | 3
            ss: 0x23,      // User data segment (ring 3) | 0x20 | 3
        }
    }
}

/// Stack size for kernel tasks
pub const KERNEL_STACK_SIZE: usize = 16 * 1024; // 16 KB

/// Stack size for user tasks
pub const USER_STACK_SIZE: usize = 64 * 1024; // 64 KB

/// A kernel stack
pub struct KernelStack {
    data: Box<[u8; KERNEL_STACK_SIZE]>,
}

impl KernelStack {
    /// Allocate a new kernel stack
    pub fn new() -> Self {
        Self {
            data: Box::new([0u8; KERNEL_STACK_SIZE]),
        }
    }

    /// Get the top of the stack (stacks grow downward)
    pub fn top(&self) -> u64 {
        let ptr = self.data.as_ptr() as u64;
        ptr + KERNEL_STACK_SIZE as u64
    }

    /// Get the bottom of the stack
    pub fn bottom(&self) -> u64 {
        self.data.as_ptr() as u64
    }
}

impl Default for KernelStack {
    fn default() -> Self {
        Self::new()
    }
}

/// A task (thread of execution)
pub struct Task {
    /// Unique identifier
    pub id: TaskId,
    /// Task name (for debugging)
    pub name: String,
    /// Current state
    pub state: TaskState,
    /// Privilege level
    pub privilege: TaskPrivilege,
    /// CPU context (registers)
    pub context: CpuContext,
    /// Kernel stack
    pub kernel_stack: KernelStack,
    /// User stack (if userspace task)
    pub user_stack: Option<Box<[u8; USER_STACK_SIZE]>>,
    /// Time slice remaining (in ticks)
    pub time_slice: u32,
    /// Total CPU time used (in ticks)
    pub cpu_time: u64,
    /// Exit code (if dead)
    pub exit_code: Option<i32>,
}

impl Task {
    /// Create a new kernel task
    pub fn new_kernel(name: impl Into<String>, entry: extern "C" fn()) -> Self {
        let kernel_stack = KernelStack::new();
        let stack_top = kernel_stack.top();

        // We need to set up the stack so context_switch can "return" to entry
        // The initial context switch will pop registers and ret to entry
        Self {
            id: TaskId::new(),
            name: name.into(),
            state: TaskState::Ready,
            privilege: TaskPrivilege::Kernel,
            context: CpuContext::new_kernel(entry as u64, stack_top),
            kernel_stack,
            user_stack: None,
            time_slice: 10, // 10 ticks default
            cpu_time: 0,
            exit_code: None,
        }
    }

    /// Create a new user task (for future use)
    pub fn new_user(name: impl Into<String>, entry: u64, user_stack_top: u64) -> Self {
        let kernel_stack = KernelStack::new();

        Self {
            id: TaskId::new(),
            name: name.into(),
            state: TaskState::Ready,
            privilege: TaskPrivilege::User,
            context: CpuContext::new_user(entry, user_stack_top, kernel_stack.top()),
            kernel_stack,
            user_stack: Some(Box::new([0u8; USER_STACK_SIZE])),
            time_slice: 10,
            cpu_time: 0,
            exit_code: None,
        }
    }

    /// Check if task is runnable
    pub fn is_runnable(&self) -> bool {
        matches!(self.state, TaskState::Ready | TaskState::Running)
    }
}

// =============================================================================
// Task Scheduler
// =============================================================================

/// The global task scheduler
pub struct Scheduler {
    /// All tasks
    tasks: RwLock<Vec<Task>>,
    /// Index of currently running task
    current: AtomicU32,
    /// Total context switches
    switches: AtomicU64,
    /// Is scheduler active?
    active: AtomicU32,
}

impl Scheduler {
    /// Create a new scheduler
    pub const fn new() -> Self {
        Self {
            tasks: RwLock::new(Vec::new()),
            current: AtomicU32::new(0),
            switches: AtomicU64::new(0),
            active: AtomicU32::new(0),
        }
    }

    /// Initialize with idle task
    pub fn init(&self) {
        let mut tasks = self.tasks.write();

        // Create the idle task (always task 0)
        // Give it a tiny time slice so we switch to real tasks quickly
        let idle = Task {
            id: TaskId::IDLE,
            name: String::from("idle"),
            state: TaskState::Running,
            privilege: TaskPrivilege::Kernel,
            context: CpuContext::default(),
            kernel_stack: KernelStack::new(),
            user_stack: None,
            time_slice: 1, // Switch away from idle ASAP!
            cpu_time: 0,
            exit_code: None,
        };
        tasks.push(idle);

        log::info!("Scheduler initialized with idle task");
    }

    /// Spawn a new kernel task
    pub fn spawn(&self, task: Task) -> TaskId {
        let id = task.id;
        let name = task.name.clone();

        let mut tasks = self.tasks.write();
        tasks.push(task);

        log::debug!("Spawned task '{}' (id={})", name, id.as_u64());
        id
    }

    /// Get the current task ID
    pub fn current_task_id(&self) -> TaskId {
        let idx = self.current.load(Ordering::Relaxed) as usize;
        let tasks = self.tasks.read();
        if idx < tasks.len() {
            tasks[idx].id
        } else {
            TaskId::IDLE
        }
    }

    /// Start the scheduler
    pub fn start(&self) {
        self.active.store(1, Ordering::SeqCst);
        log::info!("Scheduler started");
    }

    /// Stop the scheduler
    pub fn stop(&self) {
        self.active.store(0, Ordering::SeqCst);
    }

    /// Check if scheduler is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed) != 0
    }

    /// Timer tick handler - returns true if we should context switch
    pub fn tick(&self) -> bool {
        if !self.is_active() {
            return false;
        }

        let idx = self.current.load(Ordering::Relaxed) as usize;
        let mut tasks = self.tasks.write();

        if idx >= tasks.len() {
            return false;
        }

        // Increment CPU time
        tasks[idx].cpu_time += 1;

        // Decrement time slice
        if tasks[idx].time_slice > 0 {
            tasks[idx].time_slice -= 1;
        }

        // Switch if time slice expired
        tasks[idx].time_slice == 0
    }

    /// Schedule the next task
    ///
    /// Returns (old_context_ptr, new_context_ptr) if we should switch
    pub fn schedule(&self) -> Option<(*mut CpuContext, *const CpuContext)> {
        if !self.is_active() {
            return None;
        }

        let mut tasks = self.tasks.write();
        let num_tasks = tasks.len();

        if num_tasks <= 1 {
            return None; // Only idle task
        }

        let current_idx = self.current.load(Ordering::Relaxed) as usize;

        // Mark current task as ready (if it was running)
        if tasks[current_idx].state == TaskState::Running {
            tasks[current_idx].state = TaskState::Ready;
            tasks[current_idx].time_slice = 10; // Reset time slice
        }

        // Find next runnable task (round-robin)
        let mut next_idx = (current_idx + 1) % num_tasks;
        let start = next_idx;

        loop {
            if tasks[next_idx].is_runnable() && next_idx != 0 {
                // Found a non-idle runnable task
                break;
            }
            next_idx = (next_idx + 1) % num_tasks;
            if next_idx == start {
                // Wrapped around, run idle
                next_idx = 0;
                break;
            }
        }

        if next_idx == current_idx {
            return None; // Same task, no switch needed
        }

        // Prepare switch
        tasks[next_idx].state = TaskState::Running;
        self.current.store(next_idx as u32, Ordering::SeqCst);
        self.switches.fetch_add(1, Ordering::Relaxed);

        let old_ctx = &mut tasks[current_idx].context as *mut CpuContext;
        let new_ctx = &tasks[next_idx].context as *const CpuContext;

        log::trace!(
            "Switch: {} -> {}",
            tasks[current_idx].name,
            tasks[next_idx].name
        );

        Some((old_ctx, new_ctx))
    }

    /// Exit the current task
    pub fn exit(&self, code: i32) {
        let idx = self.current.load(Ordering::Relaxed) as usize;
        let mut tasks = self.tasks.write();

        if idx < tasks.len() && idx != 0 {
            tasks[idx].state = TaskState::Dead;
            tasks[idx].exit_code = Some(code);
            log::info!("Task '{}' exited with code {}", tasks[idx].name, code);
        }
    }

    /// Get number of tasks
    pub fn task_count(&self) -> usize {
        self.tasks.read().len()
    }

    /// Get number of context switches
    pub fn context_switches(&self) -> u64 {
        self.switches.load(Ordering::Relaxed)
    }

    /// Yield the current task
    pub fn yield_now(&self) {
        let idx = self.current.load(Ordering::Relaxed) as usize;
        let mut tasks = self.tasks.write();
        if idx < tasks.len() {
            tasks[idx].time_slice = 0; // Force reschedule
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Global Scheduler
// =============================================================================

use spin::Once;

static GLOBAL_SCHEDULER: Once<Scheduler> = Once::new();

/// Get the global scheduler
pub fn scheduler() -> &'static Scheduler {
    GLOBAL_SCHEDULER.call_once(|| {
        let sched = Scheduler::new();
        sched.init();
        sched
    })
}

/// Spawn a new kernel task
pub fn spawn(name: impl Into<String>, entry: extern "C" fn()) -> TaskId {
    let task = Task::new_kernel(name, entry);
    scheduler().spawn(task)
}

/// Exit the current task
pub fn exit(code: i32) -> ! {
    scheduler().exit(code);

    // Trigger reschedule
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Yield to other tasks
pub fn yield_now() {
    scheduler().yield_now();
}

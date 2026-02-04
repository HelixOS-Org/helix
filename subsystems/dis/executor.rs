//! # Execution Engine
//!
//! The Execution Engine handles the actual execution of scheduled tasks,
//! managing context switches, time accounting, and execution state.
//!
//! ## Responsibilities
//!
//! - Context switching between tasks
//! - Time slice management
//! - CPU state saving and restoration
//! - Execution statistics tracking
//! - Preemption handling
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                          EXECUTION ENGINE                                     │
//! │                                                                               │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐  │
//! │  │                        CONTEXT MANAGER                                   │  │
//! │  │                                                                          │  │
//! │  │   ┌──────────────────┐    ┌──────────────────┐    ┌─────────────────┐   │  │
//! │  │   │   SAVE STATE     │ -> │   SWITCH STACK   │ -> │  RESTORE STATE  │   │  │
//! │  │   │                  │    │                  │    │                 │   │  │
//! │  │   │  • Registers     │    │  • Stack ptr     │    │  • Registers    │   │  │
//! │  │   │  • FPU state     │    │  • Page table    │    │  • FPU state    │   │  │
//! │  │   │  • MMX/SSE       │    │  • TLS           │    │  • MMX/SSE      │   │  │
//! │  │   │  • Debug regs    │    │  • Segments      │    │  • Debug regs   │   │  │
//! │  │   └──────────────────┘    └──────────────────┘    └─────────────────┘   │  │
//! │  └─────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                               │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐  │
//! │  │                        TIME ACCOUNTANT                                   │  │
//! │  │                                                                          │  │
//! │  │   • Track user time                                                      │  │
//! │  │   • Track kernel time                                                    │  │
//! │  │   • Track wait time                                                      │  │
//! │  │   • Update statistics                                                    │  │
//! │  └─────────────────────────────────────────────────────────────────────────┘  │
//! │                                                                               │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐  │
//! │  │                        PREEMPTION MANAGER                                │  │
//! │  │                                                                          │  │
//! │  │   • Check preemption conditions                                          │  │
//! │  │   • Handle timer interrupts                                              │  │
//! │  │   • Manage preemption points                                             │  │
//! │  │   • Coordinate with scheduler                                            │  │
//! │  └─────────────────────────────────────────────────────────────────────────┘  │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::RwLock;

use super::{CpuId, DISError, DISResult, Nanoseconds, TaskId};

// =============================================================================
// CPU Context
// =============================================================================

/// CPU register state
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct CpuContext {
    // General purpose registers
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,

    // Instruction pointer
    pub rip: u64,

    // Flags
    pub rflags: u64,

    // Segment registers
    pub cs: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
    pub ss: u64,

    // Control registers
    pub cr3: u64, // Page table base

    // FS/GS base (for TLS)
    pub fs_base: u64,
    pub gs_base: u64,
}

/// FPU/SIMD state
#[derive(Clone)]
#[repr(C, align(64))]
pub struct FpuState {
    /// FXSAVE/XSAVE area (512-4096 bytes depending on features)
    pub data: [u8; 4096],
    /// Size of valid data
    pub size: usize,
    /// Features saved
    pub features: FpuFeatures,
}

impl Default for FpuState {
    fn default() -> Self {
        Self {
            data: [0; 4096],
            size: 512,
            features: FpuFeatures::FPU | FpuFeatures::SSE,
        }
    }
}

impl core::fmt::Debug for FpuState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FpuState")
            .field("size", &self.size)
            .field("features", &self.features)
            .finish()
    }
}

bitflags::bitflags! {
    /// FPU features
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FpuFeatures: u32 {
        const FPU = 1 << 0;
        const SSE = 1 << 1;
        const AVX = 1 << 2;
        const AVX512 = 1 << 3;
        const MPX = 1 << 4;
        const PKRU = 1 << 5;
    }
}

/// Complete execution context
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// CPU registers
    pub cpu: CpuContext,
    /// FPU state
    pub fpu: FpuState,
    /// Debug registers
    pub debug: DebugState,
    /// Context flags
    pub flags: ContextFlags,
    /// Last switch timestamp
    pub last_switch: Nanoseconds,
    /// Total context switches
    pub switches: u64,
}

/// Debug register state
#[derive(Debug, Clone, Default)]
pub struct DebugState {
    pub dr0: u64,
    pub dr1: u64,
    pub dr2: u64,
    pub dr3: u64,
    pub dr6: u64,
    pub dr7: u64,
}

bitflags::bitflags! {
    /// Context flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ContextFlags: u32 {
        /// Context is valid
        const VALID = 1 << 0;
        /// Context is dirty (needs save)
        const DIRTY = 1 << 1;
        /// FPU state is valid
        const FPU_VALID = 1 << 2;
        /// FPU was used
        const FPU_USED = 1 << 3;
        /// Debug registers active
        const DEBUG_ACTIVE = 1 << 4;
        /// Single step mode
        const SINGLE_STEP = 1 << 5;
        /// In kernel mode
        const KERNEL_MODE = 1 << 6;
        /// Preemption disabled
        const NO_PREEMPT = 1 << 7;
    }
}

impl ExecutionContext {
    /// Create new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize for new task
    pub fn init(&mut self, entry_point: u64, stack_ptr: u64, page_table: u64) {
        self.cpu.rip = entry_point;
        self.cpu.rsp = stack_ptr;
        self.cpu.cr3 = page_table;
        self.cpu.rflags = 0x202; // IF set
        self.cpu.cs = 0x23; // User code segment
        self.cpu.ss = 0x1b; // User data segment
        self.flags = ContextFlags::VALID;
    }

    /// Initialize for kernel thread
    pub fn init_kernel(&mut self, entry_point: u64, stack_ptr: u64) {
        self.cpu.rip = entry_point;
        self.cpu.rsp = stack_ptr;
        self.cpu.rflags = 0x202;
        self.cpu.cs = 0x08; // Kernel code segment
        self.cpu.ss = 0x10; // Kernel data segment
        self.flags = ContextFlags::VALID | ContextFlags::KERNEL_MODE;
    }

    /// Mark as dirty
    pub fn mark_dirty(&mut self) {
        self.flags.insert(ContextFlags::DIRTY);
    }

    /// Clear dirty flag
    pub fn clear_dirty(&mut self) {
        self.flags.remove(ContextFlags::DIRTY);
    }

    /// Check if valid
    pub fn is_valid(&self) -> bool {
        self.flags.contains(ContextFlags::VALID)
    }

    /// Mark FPU as used
    pub fn use_fpu(&mut self) {
        self.flags
            .insert(ContextFlags::FPU_USED | ContextFlags::FPU_VALID);
    }
}

// =============================================================================
// Time Accounting
// =============================================================================

/// Time accounting for a task
#[derive(Debug, Clone, Default)]
pub struct TimeAccounting {
    /// Time spent in user mode
    pub user_time: Nanoseconds,
    /// Time spent in kernel mode
    pub kernel_time: Nanoseconds,
    /// Time spent waiting for CPU
    pub wait_time: Nanoseconds,
    /// Time spent blocked
    pub blocked_time: Nanoseconds,
    /// Current state start time
    pub state_start: Nanoseconds,
    /// Current state
    pub current_state: AccountingState,
}

/// Accounting state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccountingState {
    #[default]
    Idle,
    Running,
    Waiting,
    Blocked,
}

impl TimeAccounting {
    /// Create new time accounting
    pub fn new() -> Self {
        Self::default()
    }

    /// Start running
    pub fn start_running(&mut self, now: Nanoseconds, kernel: bool) {
        self.update(now);
        self.current_state = AccountingState::Running;
        self.state_start = now;
    }

    /// Stop running
    pub fn stop_running(&mut self, now: Nanoseconds) {
        self.update(now);
    }

    /// Start waiting
    pub fn start_waiting(&mut self, now: Nanoseconds) {
        self.update(now);
        self.current_state = AccountingState::Waiting;
        self.state_start = now;
    }

    /// Start blocked
    pub fn start_blocked(&mut self, now: Nanoseconds) {
        self.update(now);
        self.current_state = AccountingState::Blocked;
        self.state_start = now;
    }

    /// Update accounting
    fn update(&mut self, now: Nanoseconds) {
        if self.state_start > Nanoseconds::zero() && now > self.state_start {
            let delta = now - self.state_start;

            match self.current_state {
                AccountingState::Running => {
                    self.user_time += delta;
                },
                AccountingState::Waiting => {
                    self.wait_time += delta;
                },
                AccountingState::Blocked => {
                    self.blocked_time += delta;
                },
                AccountingState::Idle => {},
            }
        }
    }

    /// Get total CPU time
    pub fn cpu_time(&self) -> Nanoseconds {
        self.user_time + self.kernel_time
    }

    /// Get total time
    pub fn total_time(&self) -> Nanoseconds {
        self.user_time + self.kernel_time + self.wait_time + self.blocked_time
    }
}

// =============================================================================
// Execution State
// =============================================================================

/// Execution state for a task
pub struct ExecutionState {
    /// Task ID
    pub task_id: TaskId,
    /// Execution context
    pub context: ExecutionContext,
    /// Time accounting
    pub accounting: TimeAccounting,
    /// Remaining time slice
    pub time_slice: Nanoseconds,
    /// Number of preemptions
    pub preemptions: u64,
    /// Number of context switches
    pub switches: u64,
    /// Last CPU
    pub last_cpu: Option<CpuId>,
    /// Execution flags
    pub flags: ExecutionFlags,
}

bitflags::bitflags! {
    /// Execution flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ExecutionFlags: u32 {
        /// Task is running
        const RUNNING = 1 << 0;
        /// Task was preempted
        const PREEMPTED = 1 << 1;
        /// Task yielded voluntarily
        const YIELDED = 1 << 2;
        /// Task exhausted time slice
        const SLICE_EXHAUSTED = 1 << 3;
        /// Task needs FPU restore
        const NEEDS_FPU = 1 << 4;
        /// Task needs TLS restore
        const NEEDS_TLS = 1 << 5;
        /// Lazy FPU restore pending
        const LAZY_FPU = 1 << 6;
    }
}

impl ExecutionState {
    /// Create new execution state
    pub fn new(task_id: TaskId, time_slice: Nanoseconds) -> Self {
        Self {
            task_id,
            context: ExecutionContext::new(),
            accounting: TimeAccounting::new(),
            time_slice,
            preemptions: 0,
            switches: 0,
            last_cpu: None,
            flags: ExecutionFlags::empty(),
        }
    }

    /// Initialize context
    pub fn init(&mut self, entry: u64, stack: u64, page_table: u64) {
        self.context.init(entry, stack, page_table);
    }

    /// Record switch in
    pub fn switch_in(&mut self, now: Nanoseconds, cpu: CpuId) {
        self.flags.insert(ExecutionFlags::RUNNING);
        self.accounting.start_running(now, false);
        self.last_cpu = Some(cpu);
        self.switches += 1;
    }

    /// Record switch out
    pub fn switch_out(&mut self, now: Nanoseconds, preempted: bool) {
        self.flags.remove(ExecutionFlags::RUNNING);
        self.accounting.stop_running(now);

        if preempted {
            self.flags.insert(ExecutionFlags::PREEMPTED);
            self.preemptions += 1;
        } else {
            self.flags.insert(ExecutionFlags::YIELDED);
        }
    }

    /// Update time slice
    pub fn update_time_slice(&mut self, delta: Nanoseconds) {
        if delta >= self.time_slice {
            self.time_slice = Nanoseconds::zero();
            self.flags.insert(ExecutionFlags::SLICE_EXHAUSTED);
        } else {
            self.time_slice = self.time_slice - delta;
        }
    }

    /// Reset time slice
    pub fn reset_time_slice(&mut self, slice: Nanoseconds) {
        self.time_slice = slice;
        self.flags.remove(ExecutionFlags::SLICE_EXHAUSTED);
    }

    /// Check if time slice exhausted
    pub fn slice_exhausted(&self) -> bool {
        self.flags.contains(ExecutionFlags::SLICE_EXHAUSTED)
    }
}

// =============================================================================
// Executor
// =============================================================================

/// The main executor
pub struct Executor {
    /// Per-task execution state
    states: RwLock<BTreeMap<TaskId, ExecutionState>>,
    /// Current task per CPU
    current: RwLock<BTreeMap<CpuId, TaskId>>,
    /// Idle tasks per CPU
    idle_tasks: RwLock<BTreeMap<CpuId, TaskId>>,
    /// Preemption enabled per CPU
    preempt_enabled: RwLock<BTreeMap<CpuId, bool>>,
    /// Current timestamp
    current_time: AtomicU64,
    /// Statistics
    stats: ExecutorStats,
}

/// Executor statistics
#[derive(Debug, Default)]
struct ExecutorStats {
    context_switches: AtomicU64,
    preemptions: AtomicU64,
    voluntary_switches: AtomicU64,
    fpu_switches: AtomicU64,
}

impl Executor {
    /// Create new executor
    pub fn new() -> Self {
        Self {
            states: RwLock::new(BTreeMap::new()),
            current: RwLock::new(BTreeMap::new()),
            idle_tasks: RwLock::new(BTreeMap::new()),
            preempt_enabled: RwLock::new(BTreeMap::new()),
            current_time: AtomicU64::new(0),
            stats: ExecutorStats::default(),
        }
    }

    /// Register task
    pub fn register_task(&self, task_id: TaskId, time_slice: Nanoseconds) {
        let state = ExecutionState::new(task_id, time_slice);
        self.states.write().insert(task_id, state);
    }

    /// Unregister task
    pub fn unregister_task(&self, task_id: TaskId) {
        self.states.write().remove(&task_id);
    }

    /// Initialize task context
    pub fn init_task(
        &self,
        task_id: TaskId,
        entry: u64,
        stack: u64,
        page_table: u64,
    ) -> DISResult<()> {
        if let Some(state) = self.states.write().get_mut(&task_id) {
            state.init(entry, stack, page_table);
            Ok(())
        } else {
            Err(DISError::TaskNotFound(task_id))
        }
    }

    /// Register idle task for CPU
    pub fn register_idle(&self, cpu: CpuId, task_id: TaskId) {
        self.idle_tasks.write().insert(cpu, task_id);
        self.preempt_enabled.write().insert(cpu, true);
    }

    /// Get current task on CPU
    pub fn current(&self, cpu: CpuId) -> Option<TaskId> {
        self.current.read().get(&cpu).copied()
    }

    /// Switch context
    pub fn switch_context(
        &self,
        cpu: CpuId,
        from: TaskId,
        to: TaskId,
    ) -> DISResult<ContextSwitchResult> {
        let now = Nanoseconds::new(self.current_time.load(Ordering::Relaxed));
        let mut states = self.states.write();

        // Save current context
        let preempted = if let Some(from_state) = states.get_mut(&from) {
            let preempted = !from_state.flags.contains(ExecutionFlags::YIELDED);
            from_state.switch_out(now, preempted);
            preempted
        } else {
            false
        };

        // Restore new context
        if let Some(to_state) = states.get_mut(&to) {
            to_state.switch_in(now, cpu);
        } else {
            return Err(DISError::TaskNotFound(to));
        }

        // Update current
        self.current.write().insert(cpu, to);

        // Update statistics
        self.stats.context_switches.fetch_add(1, Ordering::Relaxed);
        if preempted {
            self.stats.preemptions.fetch_add(1, Ordering::Relaxed);
        } else {
            self.stats
                .voluntary_switches
                .fetch_add(1, Ordering::Relaxed);
        }

        Ok(ContextSwitchResult {
            from_task: from,
            to_task: to,
            preempted,
            switch_time: now,
            cpu,
        })
    }

    /// Tick handler
    pub fn tick(&self, cpu: CpuId, delta: Nanoseconds) {
        self.current_time.fetch_add(delta.raw(), Ordering::Relaxed);

        // Update current task's time slice
        if let Some(task_id) = self.current(cpu) {
            if let Some(state) = self.states.write().get_mut(&task_id) {
                state.update_time_slice(delta);
            }
        }
    }

    /// Check if current task needs preemption
    pub fn needs_preemption(&self, cpu: CpuId) -> bool {
        if !self.is_preempt_enabled(cpu) {
            return false;
        }

        if let Some(task_id) = self.current(cpu) {
            if let Some(state) = self.states.read().get(&task_id) {
                return state.slice_exhausted();
            }
        }

        false
    }

    /// Check if preemption enabled
    pub fn is_preempt_enabled(&self, cpu: CpuId) -> bool {
        self.preempt_enabled
            .read()
            .get(&cpu)
            .copied()
            .unwrap_or(true)
    }

    /// Disable preemption
    pub fn disable_preemption(&self, cpu: CpuId) {
        self.preempt_enabled.write().insert(cpu, false);
    }

    /// Enable preemption
    pub fn enable_preemption(&self, cpu: CpuId) {
        self.preempt_enabled.write().insert(cpu, true);
    }

    /// Get execution context
    pub fn get_context(&self, task_id: TaskId) -> Option<ExecutionContext> {
        self.states.read().get(&task_id).map(|s| s.context.clone())
    }

    /// Set execution context
    pub fn set_context(&self, task_id: TaskId, context: ExecutionContext) -> DISResult<()> {
        if let Some(state) = self.states.write().get_mut(&task_id) {
            state.context = context;
            Ok(())
        } else {
            Err(DISError::TaskNotFound(task_id))
        }
    }

    /// Reset time slice
    pub fn reset_time_slice(&self, task_id: TaskId, slice: Nanoseconds) -> DISResult<()> {
        if let Some(state) = self.states.write().get_mut(&task_id) {
            state.reset_time_slice(slice);
            Ok(())
        } else {
            Err(DISError::TaskNotFound(task_id))
        }
    }

    /// Get statistics
    pub fn statistics(&self) -> ExecutorStatistics {
        ExecutorStatistics {
            context_switches: self.stats.context_switches.load(Ordering::Relaxed),
            preemptions: self.stats.preemptions.load(Ordering::Relaxed),
            voluntary_switches: self.stats.voluntary_switches.load(Ordering::Relaxed),
            fpu_switches: self.stats.fpu_switches.load(Ordering::Relaxed),
            active_tasks: self.states.read().len() as u64,
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

/// Context switch result
#[derive(Debug, Clone)]
pub struct ContextSwitchResult {
    pub from_task: TaskId,
    pub to_task: TaskId,
    pub preempted: bool,
    pub switch_time: Nanoseconds,
    pub cpu: CpuId,
}

/// Executor statistics
#[derive(Debug, Clone)]
pub struct ExecutorStatistics {
    pub context_switches: u64,
    pub preemptions: u64,
    pub voluntary_switches: u64,
    pub fpu_switches: u64,
    pub active_tasks: u64,
}

// =============================================================================
// Context Switch Operations (Platform-specific stubs)
// =============================================================================

/// Low-level context switch operations
pub mod arch {
    use super::*;

    /// Save CPU context (stub)
    #[inline(always)]
    pub fn save_context(ctx: &mut CpuContext) {
        // In real implementation, this would use inline assembly:
        // asm!("mov {}, rax", out(reg) ctx.rax);
        // etc.
        let _ = ctx;
    }

    /// Restore CPU context (stub)
    #[inline(always)]
    pub fn restore_context(ctx: &CpuContext) {
        // In real implementation:
        // asm!("mov rax, {}", in(reg) ctx.rax);
        // etc.
        let _ = ctx;
    }

    /// Save FPU state (stub)
    #[inline(always)]
    pub fn save_fpu(fpu: &mut FpuState) {
        // In real implementation:
        // asm!("fxsave [{}]", in(reg) fpu.data.as_mut_ptr());
        let _ = fpu;
    }

    /// Restore FPU state (stub)
    #[inline(always)]
    pub fn restore_fpu(fpu: &FpuState) {
        // In real implementation:
        // asm!("fxrstor [{}]", in(reg) fpu.data.as_ptr());
        let _ = fpu;
    }

    /// Switch page tables (stub)
    #[inline(always)]
    pub fn switch_page_table(cr3: u64) {
        // In real implementation:
        // asm!("mov cr3, {}", in(reg) cr3);
        let _ = cr3;
    }

    /// Set FS base (stub)
    #[inline(always)]
    pub fn set_fs_base(base: u64) {
        // In real implementation:
        // Write to MSR FS_BASE
        let _ = base;
    }

    /// Set GS base (stub)
    #[inline(always)]
    pub fn set_gs_base(base: u64) {
        // In real implementation:
        // Write to MSR GS_BASE
        let _ = base;
    }

    /// Full context switch (stub)
    #[inline(always)]
    pub fn context_switch(from: &mut ExecutionContext, to: &ExecutionContext) {
        save_context(&mut from.cpu);

        if from.flags.contains(ContextFlags::FPU_USED) {
            save_fpu(&mut from.fpu);
        }

        if from.cpu.cr3 != to.cpu.cr3 {
            switch_page_table(to.cpu.cr3);
        }

        if to.flags.contains(ContextFlags::FPU_USED) {
            restore_fpu(&to.fpu);
        }

        if from.cpu.fs_base != to.cpu.fs_base {
            set_fs_base(to.cpu.fs_base);
        }

        if from.cpu.gs_base != to.cpu.gs_base {
            set_gs_base(to.cpu.gs_base);
        }

        restore_context(&to.cpu);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context() {
        let mut ctx = ExecutionContext::new();
        ctx.init(0x1000, 0x2000, 0x3000);

        assert!(ctx.is_valid());
        assert_eq!(ctx.cpu.rip, 0x1000);
        assert_eq!(ctx.cpu.rsp, 0x2000);
        assert_eq!(ctx.cpu.cr3, 0x3000);
    }

    #[test]
    fn test_time_accounting() {
        let mut acc = TimeAccounting::new();

        acc.start_running(Nanoseconds::from_millis(0), false);
        acc.stop_running(Nanoseconds::from_millis(100));

        assert_eq!(acc.user_time.as_millis(), 100);
    }

    #[test]
    fn test_executor() {
        let executor = Executor::new();

        let task1 = TaskId::new(1);
        let task2 = TaskId::new(2);

        executor.register_task(task1, Nanoseconds::from_millis(10));
        executor.register_task(task2, Nanoseconds::from_millis(10));

        let cpu = CpuId::new(0);
        executor.register_idle(cpu, TaskId::new(0));

        // Switch to task1
        let _ = executor.switch_context(cpu, TaskId::new(0), task1);
        assert_eq!(executor.current(cpu), Some(task1));

        // Switch to task2
        let result = executor.switch_context(cpu, task1, task2).unwrap();
        assert_eq!(result.from_task, task1);
        assert_eq!(result.to_task, task2);
    }
}

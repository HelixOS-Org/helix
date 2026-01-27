//! # Thread Structure
//!
//! Core thread data structure.

use crate::{ThreadId, ProcessId, ExecResult, ExecError};
use super::ThreadState;
use crate::scheduler::Priority;
use helix_hal::VirtAddr;
use alloc::string::String;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};

/// Thread flags
pub mod flags {
    use bitflags::bitflags;
    
    bitflags! {
        /// Thread flags
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct ThreadFlags: u32 {
            /// Thread is a kernel thread
            const KERNEL = 1 << 0;
            /// Thread is the idle thread
            const IDLE = 1 << 1;
            /// Thread is the init thread
            const INIT = 1 << 2;
            /// Thread uses FPU
            const FPU = 1 << 3;
            /// Thread is being debugged
            const TRACED = 1 << 4;
            /// Thread should be killed
            const KILL_PENDING = 1 << 5;
            /// Thread is in a signal handler
            const IN_SIGNAL = 1 << 6;
            /// Thread should not be migrated
            const NO_MIGRATE = 1 << 7;
        }
    }
}

pub use flags::ThreadFlags;

/// Thread structure
pub struct Thread {
    /// Unique identifier
    id: ThreadId,
    /// Owning process
    process: ProcessId,
    /// Thread name
    name: String,
    /// Current state
    state: AtomicU32,
    /// Priority
    priority: spin::RwLock<Priority>,
    /// Flags
    flags: spin::RwLock<ThreadFlags>,
    /// CPU context
    context: spin::Mutex<ThreadContext>,
    /// Kernel stack
    kernel_stack: KernelStack,
    /// CPU affinity mask
    affinity: spin::RwLock<u64>,
    /// Current CPU (-1 if not running)
    cpu: spin::RwLock<Option<usize>>,
    /// Exit code
    exit_code: spin::RwLock<Option<i32>>,
    /// Thread-local storage pointer
    tls: spin::RwLock<Option<VirtAddr>>,
}

impl Thread {
    /// Create a new thread
    pub fn new(
        id: ThreadId,
        process: ProcessId,
        name: impl Into<String>,
        entry: VirtAddr,
        stack: VirtAddr,
    ) -> ExecResult<Self> {
        let kernel_stack = KernelStack::allocate()?;
        
        Ok(Self {
            id,
            process,
            name: name.into(),
            state: AtomicU32::new(ThreadState::Creating as u32),
            priority: spin::RwLock::new(Priority::DEFAULT),
            flags: spin::RwLock::new(ThreadFlags::empty()),
            context: spin::Mutex::new(ThreadContext::new(entry, stack)),
            kernel_stack,
            affinity: spin::RwLock::new(u64::MAX),
            cpu: spin::RwLock::new(None),
            exit_code: spin::RwLock::new(None),
            tls: spin::RwLock::new(None),
        })
    }

    /// Get thread ID
    pub fn id(&self) -> ThreadId {
        self.id
    }

    /// Get process ID
    pub fn process(&self) -> ProcessId {
        self.process
    }

    /// Get thread name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get current state
    pub fn state(&self) -> ThreadState {
        let val = self.state.load(Ordering::SeqCst);
        // Convert back from u32
        ThreadState::from_u32(val).unwrap_or(ThreadState::Dead)
    }

    /// Set thread state
    pub fn set_state(&self, state: ThreadState) {
        self.state.store(state.as_u32(), Ordering::SeqCst);
    }

    /// Get priority
    pub fn priority(&self) -> Priority {
        *self.priority.read()
    }

    /// Set priority
    pub fn set_priority(&self, priority: Priority) {
        *self.priority.write() = priority;
    }

    /// Get flags
    pub fn flags(&self) -> ThreadFlags {
        *self.flags.read()
    }

    /// Set a flag
    pub fn set_flag(&self, flag: ThreadFlags) {
        self.flags.write().insert(flag);
    }

    /// Clear a flag
    pub fn clear_flag(&self, flag: ThreadFlags) {
        self.flags.write().remove(flag);
    }

    /// Check if kernel thread
    pub fn is_kernel(&self) -> bool {
        self.flags.read().contains(ThreadFlags::KERNEL)
    }

    /// Get kernel stack top
    pub fn kernel_stack_top(&self) -> VirtAddr {
        self.kernel_stack.top()
    }

    /// Get CPU affinity
    pub fn affinity(&self) -> u64 {
        *self.affinity.read()
    }

    /// Set CPU affinity
    pub fn set_affinity(&self, mask: u64) {
        *self.affinity.write() = mask;
    }

    /// Get current CPU
    pub fn cpu(&self) -> Option<usize> {
        *self.cpu.read()
    }

    /// Set current CPU
    pub fn set_cpu(&self, cpu: Option<usize>) {
        *self.cpu.write() = cpu;
    }

    /// Save context
    pub fn save_context(&self, ctx: ThreadContext) {
        *self.context.lock() = ctx;
    }

    /// Get context (clone)
    pub fn context(&self) -> ThreadContext {
        self.context.lock().clone()
    }

    /// Exit the thread
    pub fn exit(&self, code: i32) {
        *self.exit_code.write() = Some(code);
        self.set_state(ThreadState::Zombie);
    }
}

/// Thread context (saved registers)
#[derive(Debug, Clone, Default)]
pub struct ThreadContext {
    /// Instruction pointer
    pub ip: u64,
    /// Stack pointer
    pub sp: u64,
    /// General purpose registers (architecture-specific count)
    pub regs: [u64; 16],
    /// Flags register
    pub flags: u64,
}

impl ThreadContext {
    /// Create a new context
    pub fn new(entry: VirtAddr, stack: VirtAddr) -> Self {
        Self {
            ip: entry.as_u64(),
            sp: stack.as_u64(),
            regs: [0; 16],
            flags: 0,
        }
    }
}

/// Kernel stack
pub struct KernelStack {
    /// Stack base address
    base: VirtAddr,
    /// Stack size
    size: usize,
}

impl KernelStack {
    /// Default kernel stack size (16 KiB)
    pub const DEFAULT_SIZE: usize = 16 * 1024;

    /// Allocate a kernel stack
    pub fn allocate() -> ExecResult<Self> {
        // TODO: Actually allocate memory
        // For now, just create a placeholder
        Ok(Self {
            base: VirtAddr::new(0),
            size: Self::DEFAULT_SIZE,
        })
    }

    /// Get stack top (highest address)
    pub fn top(&self) -> VirtAddr {
        VirtAddr::new(self.base.as_u64() + self.size as u64)
    }

    /// Get stack base (lowest address)
    pub fn base(&self) -> VirtAddr {
        self.base
    }

    /// Get stack size
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        // TODO: Deallocate the stack
    }
}

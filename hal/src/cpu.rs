//! # CPU Abstraction
//!
//! This module defines traits for CPU-level operations that are architecture-specific.

use core::fmt::Debug;

use crate::{HalResult, VirtAddr};

/// CPU abstraction trait
///
/// Provides architecture-independent access to CPU features and state.
pub trait CpuAbstraction: Send + Sync {
    /// CPU context type for saving/restoring state
    type Context: CpuContext;

    /// CPU ID type
    type CpuId: Copy + Eq + Debug;

    /// Get the current CPU ID
    fn current_cpu_id(&self) -> Self::CpuId;

    /// Get the number of CPUs available
    fn cpu_count(&self) -> usize;

    /// Check if this is the bootstrap processor
    fn is_bsp(&self) -> bool;

    /// Enable interrupts
    ///
    /// # Safety
    /// Enabling interrupts when the system is not ready can cause undefined behavior.
    unsafe fn enable_interrupts(&self);

    /// Disable interrupts
    ///
    /// # Safety
    /// Disabling interrupts for too long can cause system hangs.
    unsafe fn disable_interrupts(&self);

    /// Check if interrupts are enabled
    fn interrupts_enabled(&self) -> bool;

    /// Execute with interrupts disabled
    fn without_interrupts<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R;

    /// Halt the CPU until an interrupt occurs
    fn halt(&self);

    /// Pause the CPU (for spinlocks)
    fn pause(&self);

    /// Memory barrier (full fence)
    fn memory_barrier(&self);

    /// Read memory barrier
    fn read_barrier(&self);

    /// Write memory barrier
    fn write_barrier(&self);

    /// Invalidate instruction cache
    fn invalidate_icache(&self);

    /// Get the current stack pointer
    fn stack_pointer(&self) -> VirtAddr;

    /// Get the current instruction pointer
    fn instruction_pointer(&self) -> VirtAddr;

    /// Read a CPU-specific register by name
    fn read_register(&self, name: &str) -> HalResult<u64>;

    /// Write a CPU-specific register by name
    ///
    /// # Safety
    /// Writing to CPU registers can cause undefined behavior.
    unsafe fn write_register(&self, name: &str, value: u64) -> HalResult<()>;
}

/// CPU context trait
///
/// Represents the saved state of a CPU for context switching.
pub trait CpuContext: Clone + Send + Sync + Default {
    /// Create a new context for kernel execution
    fn new_kernel(entry: VirtAddr, stack: VirtAddr) -> Self;

    /// Create a new context for user execution
    fn new_user(entry: VirtAddr, stack: VirtAddr) -> Self;

    /// Get the instruction pointer from the context
    fn instruction_pointer(&self) -> VirtAddr;

    /// Set the instruction pointer in the context
    fn set_instruction_pointer(&mut self, ip: VirtAddr);

    /// Get the stack pointer from the context
    fn stack_pointer(&self) -> VirtAddr;

    /// Set the stack pointer in the context
    fn set_stack_pointer(&mut self, sp: VirtAddr);

    /// Get the return value register
    fn return_value(&self) -> u64;

    /// Set the return value register
    fn set_return_value(&mut self, value: u64);

    /// Get syscall argument by index (0-5)
    fn syscall_arg(&self, index: usize) -> u64;

    /// Set syscall argument by index (0-5)
    fn set_syscall_arg(&mut self, index: usize, value: u64);
}

/// FPU/SIMD state trait
pub trait FpuContext: Clone + Send + Sync {
    /// Save the current FPU state
    fn save(&mut self);

    /// Restore FPU state
    ///
    /// # Safety
    /// The state must have been previously saved.
    unsafe fn restore(&self);

    /// Reset to initial state
    fn reset(&mut self);
}

/// CPU features detection
#[derive(Debug, Clone, Default)]
pub struct CpuFeatures {
    /// Has floating point support
    pub has_fpu: bool,
    /// Has SIMD support (SSE/NEON/etc)
    pub has_simd: bool,
    /// Has advanced SIMD (AVX/SVE/etc)
    pub has_advanced_simd: bool,
    /// Has virtualization support
    pub has_virtualization: bool,
    /// Has memory protection keys
    pub has_memory_protection_keys: bool,
    /// Has transactional memory
    pub has_transactional_memory: bool,
    /// Has cryptographic extensions
    pub has_crypto: bool,
    /// Has atomic instructions
    pub has_atomics: bool,
    /// Number of breakpoint registers
    pub breakpoint_count: usize,
    /// Number of watchpoint registers
    pub watchpoint_count: usize,
}

/// CPU topology information
#[derive(Debug, Clone, Default)]
pub struct CpuTopology {
    /// Number of physical cores
    pub physical_cores: usize,
    /// Number of logical cores (with SMT)
    pub logical_cores: usize,
    /// Number of NUMA nodes
    pub numa_nodes: usize,
    /// Cache levels
    pub cache_levels: usize,
    /// L1 data cache size in bytes
    pub l1d_cache_size: usize,
    /// L1 instruction cache size in bytes
    pub l1i_cache_size: usize,
    /// L2 cache size in bytes
    pub l2_cache_size: usize,
    /// L3 cache size in bytes
    pub l3_cache_size: usize,
}

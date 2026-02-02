//! # AArch64 Memory Barriers
//!
//! This module provides memory barrier primitives for AArch64.
//! ARM memory model is weakly ordered, so barriers are essential for
//! correct multi-threaded and device I/O operations.

use core::arch::asm;

// =============================================================================
// Barrier Domain
// =============================================================================

/// Memory barrier domain/shareability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierDomain {
    /// Full system (all observers)
    FullSystem,
    /// Outer shareable domain
    OuterShareable,
    /// Inner shareable domain
    InnerShareable,
    /// Non-shareable (local CPU only)
    NonShareable,
}

/// Memory barrier type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierType {
    /// Full barrier (loads and stores)
    Full,
    /// Store barrier only
    Store,
    /// Load barrier only
    Load,
}

// =============================================================================
// Data Memory Barrier (DMB)
// =============================================================================

/// Data Memory Barrier - ensures ordering of memory accesses
///
/// DMB ensures that all explicit memory accesses that appear in program order
/// before the DMB instruction are observed before any explicit memory accesses
/// that appear in program order after the DMB instruction.

/// DMB - Full System, all accesses
#[inline(always)]
pub fn dmb_sy() {
    unsafe {
        asm!("dmb sy", options(nostack, preserves_flags));
    }
}

/// DMB - Full System, stores only
#[inline(always)]
pub fn dmb_st() {
    unsafe {
        asm!("dmb st", options(nostack, preserves_flags));
    }
}

/// DMB - Full System, loads only
#[inline(always)]
pub fn dmb_ld() {
    unsafe {
        asm!("dmb ld", options(nostack, preserves_flags));
    }
}

/// DMB - Inner Shareable, all accesses
#[inline(always)]
pub fn dmb_ish() {
    unsafe {
        asm!("dmb ish", options(nostack, preserves_flags));
    }
}

/// DMB - Inner Shareable, stores only
#[inline(always)]
pub fn dmb_ishst() {
    unsafe {
        asm!("dmb ishst", options(nostack, preserves_flags));
    }
}

/// DMB - Inner Shareable, loads only
#[inline(always)]
pub fn dmb_ishld() {
    unsafe {
        asm!("dmb ishld", options(nostack, preserves_flags));
    }
}

/// DMB - Outer Shareable, all accesses
#[inline(always)]
pub fn dmb_osh() {
    unsafe {
        asm!("dmb osh", options(nostack, preserves_flags));
    }
}

/// DMB - Outer Shareable, stores only
#[inline(always)]
pub fn dmb_oshst() {
    unsafe {
        asm!("dmb oshst", options(nostack, preserves_flags));
    }
}

/// DMB - Outer Shareable, loads only
#[inline(always)]
pub fn dmb_oshld() {
    unsafe {
        asm!("dmb oshld", options(nostack, preserves_flags));
    }
}

/// DMB - Non-shareable, all accesses
#[inline(always)]
pub fn dmb_nsh() {
    unsafe {
        asm!("dmb nsh", options(nostack, preserves_flags));
    }
}

/// DMB - Non-shareable, stores only
#[inline(always)]
pub fn dmb_nshst() {
    unsafe {
        asm!("dmb nshst", options(nostack, preserves_flags));
    }
}

/// DMB - Non-shareable, loads only
#[inline(always)]
pub fn dmb_nshld() {
    unsafe {
        asm!("dmb nshld", options(nostack, preserves_flags));
    }
}

// =============================================================================
// Data Synchronization Barrier (DSB)
// =============================================================================

/// Data Synchronization Barrier - ensures completion of memory accesses
///
/// DSB is a stronger barrier than DMB. It ensures that all memory accesses
/// that appear in program order before the DSB instruction are complete
/// before the DSB instruction completes.

/// DSB - Full System, all accesses
#[inline(always)]
pub fn dsb_sy() {
    unsafe {
        asm!("dsb sy", options(nostack, preserves_flags));
    }
}

/// DSB - Full System, stores only
#[inline(always)]
pub fn dsb_st() {
    unsafe {
        asm!("dsb st", options(nostack, preserves_flags));
    }
}

/// DSB - Full System, loads only
#[inline(always)]
pub fn dsb_ld() {
    unsafe {
        asm!("dsb ld", options(nostack, preserves_flags));
    }
}

/// DSB - Inner Shareable, all accesses
#[inline(always)]
pub fn dsb_ish() {
    unsafe {
        asm!("dsb ish", options(nostack, preserves_flags));
    }
}

/// DSB - Inner Shareable, stores only
#[inline(always)]
pub fn dsb_ishst() {
    unsafe {
        asm!("dsb ishst", options(nostack, preserves_flags));
    }
}

/// DSB - Inner Shareable, loads only
#[inline(always)]
pub fn dsb_ishld() {
    unsafe {
        asm!("dsb ishld", options(nostack, preserves_flags));
    }
}

/// DSB - Outer Shareable, all accesses
#[inline(always)]
pub fn dsb_osh() {
    unsafe {
        asm!("dsb osh", options(nostack, preserves_flags));
    }
}

/// DSB - Outer Shareable, stores only
#[inline(always)]
pub fn dsb_oshst() {
    unsafe {
        asm!("dsb oshst", options(nostack, preserves_flags));
    }
}

/// DSB - Outer Shareable, loads only
#[inline(always)]
pub fn dsb_oshld() {
    unsafe {
        asm!("dsb oshld", options(nostack, preserves_flags));
    }
}

/// DSB - Non-shareable, all accesses
#[inline(always)]
pub fn dsb_nsh() {
    unsafe {
        asm!("dsb nsh", options(nostack, preserves_flags));
    }
}

/// DSB - Non-shareable, stores only
#[inline(always)]
pub fn dsb_nshst() {
    unsafe {
        asm!("dsb nshst", options(nostack, preserves_flags));
    }
}

/// DSB - Non-shareable, loads only
#[inline(always)]
pub fn dsb_nshld() {
    unsafe {
        asm!("dsb nshld", options(nostack, preserves_flags));
    }
}

// =============================================================================
// Instruction Synchronization Barrier (ISB)
// =============================================================================

/// Instruction Synchronization Barrier
///
/// ISB flushes the pipeline and ensures that all instructions after the ISB
/// are fetched from cache or memory after the ISB has completed.
///
/// Use ISB after:
/// - Changing system registers
/// - Writing to instruction memory
/// - Enabling/disabling caches
/// - Installing exception handlers
#[inline(always)]
pub fn isb() {
    unsafe {
        asm!("isb", options(nostack, preserves_flags));
    }
}

/// Instruction Synchronization Barrier with full system scope
#[inline(always)]
pub fn isb_sy() {
    unsafe {
        asm!("isb sy", options(nostack, preserves_flags));
    }
}

// =============================================================================
// High-Level Barrier API
// =============================================================================

/// Generic data memory barrier
#[inline(always)]
pub fn dmb(domain: BarrierDomain, barrier_type: BarrierType) {
    match (domain, barrier_type) {
        (BarrierDomain::FullSystem, BarrierType::Full) => dmb_sy(),
        (BarrierDomain::FullSystem, BarrierType::Store) => dmb_st(),
        (BarrierDomain::FullSystem, BarrierType::Load) => dmb_ld(),
        (BarrierDomain::InnerShareable, BarrierType::Full) => dmb_ish(),
        (BarrierDomain::InnerShareable, BarrierType::Store) => dmb_ishst(),
        (BarrierDomain::InnerShareable, BarrierType::Load) => dmb_ishld(),
        (BarrierDomain::OuterShareable, BarrierType::Full) => dmb_osh(),
        (BarrierDomain::OuterShareable, BarrierType::Store) => dmb_oshst(),
        (BarrierDomain::OuterShareable, BarrierType::Load) => dmb_oshld(),
        (BarrierDomain::NonShareable, BarrierType::Full) => dmb_nsh(),
        (BarrierDomain::NonShareable, BarrierType::Store) => dmb_nshst(),
        (BarrierDomain::NonShareable, BarrierType::Load) => dmb_nshld(),
    }
}

/// Generic data synchronization barrier
#[inline(always)]
pub fn dsb(domain: BarrierDomain, barrier_type: BarrierType) {
    match (domain, barrier_type) {
        (BarrierDomain::FullSystem, BarrierType::Full) => dsb_sy(),
        (BarrierDomain::FullSystem, BarrierType::Store) => dsb_st(),
        (BarrierDomain::FullSystem, BarrierType::Load) => dsb_ld(),
        (BarrierDomain::InnerShareable, BarrierType::Full) => dsb_ish(),
        (BarrierDomain::InnerShareable, BarrierType::Store) => dsb_ishst(),
        (BarrierDomain::InnerShareable, BarrierType::Load) => dsb_ishld(),
        (BarrierDomain::OuterShareable, BarrierType::Full) => dsb_osh(),
        (BarrierDomain::OuterShareable, BarrierType::Store) => dsb_oshst(),
        (BarrierDomain::OuterShareable, BarrierType::Load) => dsb_oshld(),
        (BarrierDomain::NonShareable, BarrierType::Full) => dsb_nsh(),
        (BarrierDomain::NonShareable, BarrierType::Store) => dsb_nshst(),
        (BarrierDomain::NonShareable, BarrierType::Load) => dsb_nshld(),
    }
}

// =============================================================================
// Common Barrier Patterns
// =============================================================================

/// Memory fence - full ordering for multi-threaded code
///
/// Use for synchronization between threads on the same or different CPUs.
#[inline(always)]
pub fn memory_fence() {
    dmb_ish();
}

/// Store fence - ensures all prior stores are visible
#[inline(always)]
pub fn store_fence() {
    dmb_ishst();
}

/// Load fence - ensures all prior loads are complete
#[inline(always)]
pub fn load_fence() {
    dmb_ishld();
}

/// Device memory barrier - for MMIO ordering
///
/// Use when accessing memory-mapped device registers.
#[inline(always)]
pub fn device_memory_barrier() {
    dsb_sy();
}

/// Compiler barrier - prevents compiler reordering
///
/// Does not emit any CPU instructions, only prevents compiler
/// from reordering memory operations across this point.
#[inline(always)]
pub fn compiler_barrier() {
    unsafe {
        asm!("", options(nostack, preserves_flags));
    }
}

/// Full synchronization barrier
///
/// Ensures all memory accesses complete and pipeline is flushed.
/// Use after system register changes.
#[inline(always)]
pub fn full_barrier() {
    dsb_sy();
    isb();
}

/// Context synchronization
///
/// Use when changing translation tables, exception handlers, etc.
#[inline(always)]
pub fn context_synchronize() {
    dsb_ish();
    isb();
}

// =============================================================================
// Speculation Barriers
// =============================================================================

/// Speculation barrier
///
/// Prevents speculative execution of instructions after this point
/// based on data or address speculation before this point.
#[inline(always)]
pub fn speculation_barrier() {
    // SB (Speculation Barrier) instruction - ARMv8.5
    // For older cores, DSB+ISB provides similar protection
    dsb_sy();
    isb();
}

/// Consumption of Speculative Data Barrier (CSDB)
///
/// Controls when the result of a conditional select is available for
/// speculative execution. Part of Spectre mitigations.
#[inline(always)]
pub fn csdb() {
    unsafe {
        asm!("csdb", options(nostack, preserves_flags));
    }
}

// =============================================================================
// TLB Invalidation Barriers
// =============================================================================

/// TLB barrier - use after TLB invalidation
#[inline(always)]
pub fn tlb_barrier() {
    dsb_ish();
    isb();
}

/// Ensure TLB maintenance complete
#[inline(always)]
pub fn tlb_sync() {
    dsb_ish();
}

// =============================================================================
// Acquire/Release Semantics
// =============================================================================

/// Acquire barrier
///
/// Ensures that all memory accesses after this barrier are observed after
/// the barrier completes. Used for acquiring locks.
#[inline(always)]
pub fn acquire() {
    dmb_ishld();
}

/// Release barrier
///
/// Ensures that all memory accesses before this barrier complete before
/// the barrier completes. Used for releasing locks.
#[inline(always)]
pub fn release() {
    dmb_ishst();
}

/// Acquire-Release barrier
///
/// Full barrier for lock-protected critical sections.
#[inline(always)]
pub fn acquire_release() {
    dmb_ish();
}

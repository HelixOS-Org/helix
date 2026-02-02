//! # RISC-V Memory Barriers
//!
//! This module provides memory barrier and ordering primitives for RISC-V.
//!
//! ## RISC-V Memory Model (RVWMO)
//!
//! RISC-V uses a relaxed memory model called RVWMO (RISC-V Weak Memory Ordering).
//! The FENCE instruction is used to enforce ordering between memory operations.
//!
//! ## FENCE Instruction Format
//!
//! ```text
//! FENCE predecessor, successor
//! ```
//!
//! Where predecessor and successor can be combinations of:
//! - **I**: Input (device input)
//! - **O**: Output (device output)
//! - **R**: Read (memory loads)
//! - **W**: Write (memory stores)
//!
//! ## Common Fence Types
//!
//! | Fence          | Effect                                    |
//! |----------------|-------------------------------------------|
//! | FENCE          | Full fence (IORW, IORW)                   |
//! | FENCE R, R     | Load-Load ordering                        |
//! | FENCE R, W     | Load-Store ordering                       |
//! | FENCE W, R     | Store-Load ordering (also for atomics)    |
//! | FENCE W, W     | Store-Store ordering                      |
//! | FENCE RW, RW   | Acquire + Release semantics               |
//! | FENCE.TSO      | Total Store Ordering (special encoding)   |
//!
//! ## SFENCE.VMA
//!
//! The SFENCE.VMA instruction is used for TLB management:
//! - `SFENCE.VMA x0, x0` - Flush entire TLB
//! - `SFENCE.VMA rs1, x0` - Flush TLB entries for address in rs1
//! - `SFENCE.VMA x0, rs2` - Flush TLB entries for ASID in rs2
//! - `SFENCE.VMA rs1, rs2` - Flush TLB entry for (address, ASID)

use core::arch::asm;

// ============================================================================
// Full Memory Barriers
// ============================================================================

/// Full memory barrier (FENCE IORW, IORW)
///
/// Ensures all prior memory accesses (including device I/O) complete
/// before any subsequent memory accesses.
#[inline]
pub fn fence() {
    unsafe {
        asm!("fence iorw, iorw", options(nostack, preserves_flags));
    }
}

/// Full memory barrier (alias for fence())
#[inline]
pub fn mb() {
    fence();
}

/// Memory barrier for regular memory only (FENCE RW, RW)
///
/// Ensures all prior memory accesses complete before subsequent ones.
/// Does not order device I/O.
#[inline]
pub fn fence_rw_rw() {
    unsafe {
        asm!("fence rw, rw", options(nostack, preserves_flags));
    }
}

// ============================================================================
// Directional Barriers
// ============================================================================

/// Read memory barrier (FENCE R, R)
///
/// Ensures all prior loads complete before any subsequent loads.
#[inline]
pub fn rmb() {
    unsafe {
        asm!("fence r, r", options(nostack, preserves_flags));
    }
}

/// Write memory barrier (FENCE W, W)
///
/// Ensures all prior stores complete before any subsequent stores.
#[inline]
pub fn wmb() {
    unsafe {
        asm!("fence w, w", options(nostack, preserves_flags));
    }
}

/// Acquire barrier (FENCE R, RW)
///
/// Ensures loads before the barrier complete before any loads/stores after.
/// Used after acquiring a lock.
#[inline]
pub fn acquire() {
    unsafe {
        asm!("fence r, rw", options(nostack, preserves_flags));
    }
}

/// Release barrier (FENCE RW, W)
///
/// Ensures all loads/stores before the barrier complete before stores after.
/// Used before releasing a lock.
#[inline]
pub fn release() {
    unsafe {
        asm!("fence rw, w", options(nostack, preserves_flags));
    }
}

/// Load-Store barrier (FENCE R, W)
///
/// Ensures loads complete before subsequent stores.
#[inline]
pub fn fence_r_w() {
    unsafe {
        asm!("fence r, w", options(nostack, preserves_flags));
    }
}

/// Store-Load barrier (FENCE W, R)
///
/// Ensures stores complete before subsequent loads.
/// This is the most expensive barrier type on relaxed memory models.
#[inline]
pub fn fence_w_r() {
    unsafe {
        asm!("fence w, r", options(nostack, preserves_flags));
    }
}

// ============================================================================
// Total Store Ordering (TSO)
// ============================================================================

/// TSO fence (FENCE.TSO)
///
/// Provides Total Store Ordering semantics.
/// This is a special encoding that's more efficient than full fence
/// on TSO-capable implementations.
///
/// TSO ensures:
/// - Loads are not reordered with other loads
/// - Stores are not reordered with other stores
/// - Stores are not reordered with older loads
/// - (but loads CAN be reordered with older stores)
#[inline]
pub fn fence_tso() {
    unsafe {
        // FENCE.TSO is encoded as FENCE RW, RW with the TSO bit set
        // The hint bits fm=0x8 indicate TSO
        asm!("fence.tso", options(nostack, preserves_flags));
    }
}

// ============================================================================
// Device I/O Barriers
// ============================================================================

/// I/O read barrier (FENCE I, I)
///
/// Ensures prior device input operations complete before subsequent ones.
#[inline]
pub fn io_rmb() {
    unsafe {
        asm!("fence i, i", options(nostack, preserves_flags));
    }
}

/// I/O write barrier (FENCE O, O)
///
/// Ensures prior device output operations complete before subsequent ones.
#[inline]
pub fn io_wmb() {
    unsafe {
        asm!("fence o, o", options(nostack, preserves_flags));
    }
}

/// I/O memory barrier (FENCE IO, IO)
///
/// Full barrier for device I/O operations.
#[inline]
pub fn io_mb() {
    unsafe {
        asm!("fence io, io", options(nostack, preserves_flags));
    }
}

/// Barrier between device I/O and memory (FENCE IORW, IORW)
///
/// Ensures device I/O and memory operations are properly ordered.
#[inline]
pub fn device_mb() {
    fence();
}

// ============================================================================
// TLB Barriers (SFENCE.VMA)
// ============================================================================

/// Flush entire TLB
///
/// Invalidates all TLB entries for all address spaces.
#[inline]
pub fn sfence_vma_all() {
    unsafe {
        asm!("sfence.vma x0, x0", options(nostack, preserves_flags));
    }
}

/// Flush TLB entries for a specific virtual address
///
/// Invalidates TLB entries matching the given address in all ASIDs.
#[inline]
pub fn sfence_vma_addr(vaddr: usize) {
    unsafe {
        asm!("sfence.vma {}, x0", in(reg) vaddr, options(nostack, preserves_flags));
    }
}

/// Flush TLB entries for a specific ASID
///
/// Invalidates all TLB entries for the given ASID.
#[inline]
pub fn sfence_vma_asid(asid: u16) {
    unsafe {
        asm!("sfence.vma x0, {}", in(reg) asid as u64, options(nostack, preserves_flags));
    }
}

/// Flush TLB entry for a specific (address, ASID) pair
///
/// Most fine-grained TLB invalidation.
#[inline]
pub fn sfence_vma(vaddr: usize, asid: u16) {
    unsafe {
        asm!("sfence.vma {}, {}", in(reg) vaddr, in(reg) asid as u64, options(nostack, preserves_flags));
    }
}

// ============================================================================
// Hypervisor TLB Barriers (H extension)
// ============================================================================

/// Hypervisor fence for guest virtual addresses (HFENCE.GVMA)
///
/// Only available with H extension.
#[cfg(feature = "hypervisor")]
#[inline]
pub fn hfence_gvma_all() {
    unsafe {
        asm!("hfence.gvma x0, x0", options(nostack, preserves_flags));
    }
}

/// Hypervisor fence for VS-stage mappings (HFENCE.VVMA)
///
/// Only available with H extension.
#[cfg(feature = "hypervisor")]
#[inline]
pub fn hfence_vvma_all() {
    unsafe {
        asm!("hfence.vvma x0, x0", options(nostack, preserves_flags));
    }
}

// ============================================================================
// Fine-grained TLB Invalidation (Svinval Extension)
// ============================================================================

/// Begin TLB invalidation batch (Svinval extension)
///
/// Groups multiple invalidations together for better performance.
#[cfg(feature = "svinval")]
#[inline]
pub fn sinval_vma_begin() {
    unsafe {
        asm!("sfence.w.inval", options(nostack, preserves_flags));
    }
}

/// End TLB invalidation batch (Svinval extension)
#[cfg(feature = "svinval")]
#[inline]
pub fn sinval_vma_end() {
    unsafe {
        asm!("sfence.inval.ir", options(nostack, preserves_flags));
    }
}

/// Invalidate single TLB entry (Svinval extension)
///
/// Must be called between sinval_vma_begin() and sinval_vma_end().
#[cfg(feature = "svinval")]
#[inline]
pub fn sinval_vma(vaddr: usize, asid: u16) {
    unsafe {
        asm!("sinval.vma {}, {}", in(reg) vaddr, in(reg) asid as u64, options(nostack, preserves_flags));
    }
}

// ============================================================================
// Compiler Barriers
// ============================================================================

/// Compiler barrier (no hardware instructions emitted)
///
/// Prevents the compiler from reordering memory accesses across this point.
/// Does not emit any hardware instructions.
#[inline]
pub fn compiler_barrier() {
    unsafe {
        asm!("", options(nostack, preserves_flags));
    }
}

/// Compiler read barrier
#[inline]
pub fn compiler_rmb() {
    compiler_barrier();
}

/// Compiler write barrier
#[inline]
pub fn compiler_wmb() {
    compiler_barrier();
}

// ============================================================================
// Synchronization Helpers
// ============================================================================

/// Data synchronization barrier
///
/// Ensures all memory accesses are complete before continuing.
/// On RISC-V, this is equivalent to a full fence.
#[inline]
pub fn dsb() {
    fence();
}

/// Instruction synchronization barrier
///
/// Ensures instruction stream is synchronized.
/// On RISC-V, this combines fence and fence.i.
#[inline]
pub fn isb() {
    fence();
    super::cache::fence_i();
}

/// Synchronize after updating page tables
///
/// This should be called after modifying page table entries.
/// Ensures the TLB sees the new mappings.
#[inline]
pub fn sync_pagetable() {
    fence_rw_rw();  // Ensure page table writes are visible
    sfence_vma_all();  // Flush TLB
}

/// Synchronize after updating page tables for specific address
#[inline]
pub fn sync_pagetable_addr(vaddr: usize) {
    fence_rw_rw();
    sfence_vma_addr(vaddr);
}

/// Synchronize after updating page tables for specific ASID
#[inline]
pub fn sync_pagetable_asid(asid: u16) {
    fence_rw_rw();
    sfence_vma_asid(asid);
}

// ============================================================================
// Atomic Operation Barriers
// ============================================================================

/// Barrier before atomic operation (for acquire semantics)
#[inline]
pub fn pre_atomic_acquire() {
    // RISC-V atomics with .aq already provide acquire semantics
    // This is for explicit barrier usage
    acquire();
}

/// Barrier after atomic operation (for release semantics)
#[inline]
pub fn post_atomic_release() {
    // RISC-V atomics with .rl already provide release semantics
    release();
}

/// Full barrier for sequentially consistent atomics
#[inline]
pub fn seq_cst_barrier() {
    fence_rw_rw();
}

// ============================================================================
// SMP Barriers
// ============================================================================

/// SMP memory barrier
///
/// Ensures memory operations are ordered across all harts.
#[inline]
pub fn smp_mb() {
    fence_rw_rw();
}

/// SMP read barrier
#[inline]
pub fn smp_rmb() {
    rmb();
}

/// SMP write barrier
#[inline]
pub fn smp_wmb() {
    wmb();
}

/// SMP store-then-load barrier
///
/// The most expensive SMP barrier, ensures stores complete before loads.
#[inline]
pub fn smp_mb_before_load() {
    fence_w_r();
}

/// SMP load-then-store barrier
#[inline]
pub fn smp_mb_after_load() {
    fence_r_w();
}

// ============================================================================
// Wait For Interrupt
// ============================================================================

/// Wait for interrupt (WFI)
///
/// Puts the hart in a low-power state until an interrupt occurs.
///
/// # Safety
/// Interrupts should be properly configured before calling this.
#[inline]
pub fn wfi() {
    unsafe {
        asm!("wfi", options(nostack, preserves_flags));
    }
}

/// Wait for event
///
/// RISC-V doesn't have a dedicated WFE instruction.
/// This uses WFI as the closest equivalent.
#[inline]
pub fn wfe() {
    wfi();
}

// ============================================================================
// Pause Hint
// ============================================================================

/// Pause hint for spin loops
///
/// Signals to the CPU that we're in a spin loop.
/// This is the PAUSE instruction (part of Zihintpause extension).
/// Falls back to NOP on implementations without it.
#[inline]
pub fn pause() {
    unsafe {
        // PAUSE is encoded as FENCE with specific hint bits
        // If not supported, it executes as NOP
        asm!(".insn i 0x0F, 0, x0, x0, 0x010", options(nostack, preserves_flags));
    }
}

/// Spin loop hint
///
/// Call this in busy-wait loops to reduce power and improve performance.
#[inline]
pub fn spin_hint() {
    pause();
}

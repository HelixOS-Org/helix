//! # RISC-V Cache Operations
//!
//! This module provides cache control operations for RISC-V.
//!
//! ## Important Notes
//!
//! RISC-V has a relatively simple cache model:
//! - **FENCE.I**: Synchronizes instruction and data caches
//! - **FENCE**: Memory ordering (not cache control per se)
//! - **CMO Extensions**: Optional Cache Management Operations (Zicbom, Zicboz, Zicbop)
//!
//! Most RISC-V implementations have coherent caches, but FENCE.I is needed
//! after modifying code to ensure the instruction cache sees the changes.

use core::arch::asm;

// ============================================================================
// Cache Line Size
// ============================================================================

/// Typical cache line size (implementation-dependent)
/// Most RISC-V implementations use 64-byte cache lines
pub const CACHE_LINE_SIZE: usize = 64;

/// Minimum cache line size to assume for safety
pub const MIN_CACHE_LINE_SIZE: usize = 16;

/// Maximum cache line size to assume
pub const MAX_CACHE_LINE_SIZE: usize = 128;

// ============================================================================
// Instruction Cache Operations
// ============================================================================

/// Execute FENCE.I instruction
///
/// This instruction synchronizes the instruction and data streams.
/// It ensures that all preceding stores to instruction memory are visible
/// to subsequent instruction fetches.
///
/// Use this after:
/// - Writing code to memory (JIT, module loading)
/// - Self-modifying code
/// - Setting up trap handlers
#[inline]
pub fn fence_i() {
    unsafe {
        asm!("fence.i", options(nostack, preserves_flags));
    }
}

/// Invalidate entire instruction cache
///
/// On RISC-V, this is typically accomplished via FENCE.I
/// which provides a stronger guarantee than just invalidation.
#[inline]
pub fn icache_invalidate_all() {
    fence_i();
}

/// Invalidate instruction cache for address range
///
/// RISC-V standard doesn't provide fine-grained I-cache invalidation,
/// so this falls back to FENCE.I which invalidates everything.
///
/// The CMO extension (Zicbom) provides CBO.INVAL but it's optional.
#[inline]
pub fn icache_invalidate_range(_start: usize, _size: usize) {
    // Standard RISC-V has no range-based I-cache invalidation
    // FENCE.I is the only option in base ISA
    fence_i();
}

// ============================================================================
// Data Cache Operations (CMO Extension)
// ============================================================================

/// Cache operation type for CMO instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheOp {
    /// Clean (write back dirty data)
    Clean,
    /// Invalidate (discard cache line)
    Invalidate,
    /// Clean and invalidate
    Flush,
    /// Zero cache line
    Zero,
}

/// Cache block size for CMO operations (implementation-defined)
/// This should be probed from the system at runtime
static mut CBO_BLOCK_SIZE: usize = 64;

/// Set the cache block size for CMO operations
///
/// # Safety
/// Must be called once at init time before any cache operations
pub unsafe fn set_cbo_block_size(size: usize) {
    CBO_BLOCK_SIZE = size;
}

/// Get the cache block size for CMO operations
#[inline]
pub fn get_cbo_block_size() -> usize {
    unsafe { CBO_BLOCK_SIZE }
}

/// Clean data cache line (Zicbom extension)
///
/// Writes back dirty data to the next level of cache/memory.
///
/// # Safety
/// Address should be valid and accessible.
#[inline]
#[cfg(feature = "cmo")]
pub unsafe fn dcache_clean_line(addr: usize) {
    asm!(
        ".insn r 0x0F, 0x2, 0x0, x0, {}, x0",  // CBO.CLEAN
        in(reg) addr,
        options(nostack)
    );
}

/// Invalidate data cache line (Zicbom extension)
///
/// Discards the cache line without writing back.
///
/// # Safety
/// Address should be valid. Data in the cache line will be lost.
#[inline]
#[cfg(feature = "cmo")]
pub unsafe fn dcache_invalidate_line(addr: usize) {
    asm!(
        ".insn r 0x0F, 0x2, 0x0, x0, {}, x1",  // CBO.INVAL
        in(reg) addr,
        options(nostack)
    );
}

/// Flush (clean + invalidate) data cache line (Zicbom extension)
///
/// # Safety
/// Address should be valid.
#[inline]
#[cfg(feature = "cmo")]
pub unsafe fn dcache_flush_line(addr: usize) {
    asm!(
        ".insn r 0x0F, 0x2, 0x0, x0, {}, x2",  // CBO.FLUSH
        in(reg) addr,
        options(nostack)
    );
}

/// Zero cache line (Zicboz extension)
///
/// Writes zeros to memory at cache line granularity.
/// More efficient than storing zeros if the cache line is not in cache.
///
/// # Safety
/// Address should be valid and writable.
#[inline]
#[cfg(feature = "cmo")]
pub unsafe fn dcache_zero_line(addr: usize) {
    asm!(
        ".insn r 0x0F, 0x2, 0x4, x0, {}, x0",  // CBO.ZERO
        in(reg) addr,
        options(nostack)
    );
}

// ============================================================================
// Range-Based Cache Operations
// ============================================================================

/// Clean data cache range
///
/// Writes back all dirty cache lines in the given range.
#[inline]
pub fn dcache_clean_range(start: usize, size: usize) {
    #[cfg(feature = "cmo")]
    {
        let block_size = get_cbo_block_size();
        let aligned_start = start & !(block_size - 1);
        let end = start + size;

        let mut addr = aligned_start;
        while addr < end {
            unsafe { dcache_clean_line(addr); }
            addr += block_size;
        }
    }

    #[cfg(not(feature = "cmo"))]
    {
        // Without CMO, use FENCE to ensure ordering
        let _ = (start, size);
        super::barriers::fence_rw_rw();
    }
}

/// Invalidate data cache range
///
/// Discards all cache lines in the given range.
///
/// # Safety
/// Any dirty data in the range will be lost.
#[inline]
pub unsafe fn dcache_invalidate_range(start: usize, size: usize) {
    #[cfg(feature = "cmo")]
    {
        let block_size = get_cbo_block_size();
        let aligned_start = start & !(block_size - 1);
        let end = start + size;

        let mut addr = aligned_start;
        while addr < end {
            dcache_invalidate_line(addr);
            addr += block_size;
        }
    }

    #[cfg(not(feature = "cmo"))]
    {
        // Without CMO, just ensure ordering
        let _ = (start, size);
        super::barriers::fence_rw_rw();
    }
}

/// Flush (clean + invalidate) data cache range
#[inline]
pub fn dcache_flush_range(start: usize, size: usize) {
    #[cfg(feature = "cmo")]
    {
        let block_size = get_cbo_block_size();
        let aligned_start = start & !(block_size - 1);
        let end = start + size;

        let mut addr = aligned_start;
        while addr < end {
            unsafe { dcache_flush_line(addr); }
            addr += block_size;
        }
    }

    #[cfg(not(feature = "cmo"))]
    {
        // Without CMO, just ensure ordering
        let _ = (start, size);
        super::barriers::fence_rw_rw();
    }
}

/// Zero memory range using cache operations
///
/// More efficient than stores for large ranges.
#[inline]
pub fn dcache_zero_range(start: usize, size: usize) {
    #[cfg(feature = "cmo")]
    {
        let block_size = get_cbo_block_size();
        let aligned_start = start & !(block_size - 1);
        let end = start + size;

        // Handle unaligned start
        if aligned_start < start {
            // Zero partial first block with stores
            unsafe {
                let ptr = start as *mut u8;
                let count = (aligned_start + block_size).min(end) - start;
                core::ptr::write_bytes(ptr, 0, count);
            }
        }

        // Zero aligned blocks with CBO.ZERO
        let mut addr = if aligned_start < start {
            aligned_start + block_size
        } else {
            aligned_start
        };

        while addr + block_size <= end {
            unsafe { dcache_zero_line(addr); }
            addr += block_size;
        }

        // Handle unaligned end
        if addr < end {
            unsafe {
                let ptr = addr as *mut u8;
                core::ptr::write_bytes(ptr, 0, end - addr);
            }
        }
    }

    #[cfg(not(feature = "cmo"))]
    {
        // Without CMO, use regular stores
        unsafe {
            core::ptr::write_bytes(start as *mut u8, 0, size);
        }
    }
}

// ============================================================================
// DMA Cache Coherence Helpers
// ============================================================================

/// Prepare memory region for DMA read (device will read from memory)
///
/// Cleans the cache to ensure device sees latest data.
#[inline]
pub fn dma_prepare_for_read(start: usize, size: usize) {
    dcache_clean_range(start, size);
}

/// Prepare memory region for DMA write (device will write to memory)
///
/// Invalidates the cache so CPU will fetch fresh data after DMA completes.
///
/// # Safety
/// Any dirty data in the range will be lost.
#[inline]
pub unsafe fn dma_prepare_for_write(start: usize, size: usize) {
    dcache_invalidate_range(start, size);
}

/// Complete DMA write operation
///
/// Invalidates cache to ensure CPU sees new data from device.
///
/// # Safety
/// Any dirty data in the range will be lost.
#[inline]
pub unsafe fn dma_complete_write(start: usize, size: usize) {
    dcache_invalidate_range(start, size);
}

/// Prepare memory region for bidirectional DMA
///
/// Cleans and invalidates the cache range.
#[inline]
pub fn dma_prepare_bidirectional(start: usize, size: usize) {
    dcache_flush_range(start, size);
}

// ============================================================================
// Prefetch Operations (Zicbop Extension)
// ============================================================================

/// Prefetch intent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchHint {
    /// Prefetch for read
    Read,
    /// Prefetch for write
    Write,
    /// Prefetch for instruction fetch
    Instruction,
}

/// Prefetch cache line (Zicbop extension)
///
/// This is a hint instruction - it may be ignored by the implementation.
#[inline]
#[cfg(feature = "cmo")]
pub fn prefetch(addr: usize, hint: PrefetchHint) {
    unsafe {
        match hint {
            PrefetchHint::Read => {
                asm!(
                    ".insn i 0x13, 0x6, x0, {}, 0",  // PREFETCH.R
                    in(reg) addr,
                    options(nostack)
                );
            }
            PrefetchHint::Write => {
                asm!(
                    ".insn i 0x13, 0x6, x0, {}, 1",  // PREFETCH.W
                    in(reg) addr,
                    options(nostack)
                );
            }
            PrefetchHint::Instruction => {
                asm!(
                    ".insn i 0x13, 0x6, x0, {}, 2",  // PREFETCH.I
                    in(reg) addr,
                    options(nostack)
                );
            }
        }
    }
}

/// Prefetch with no extension - just a no-op
#[inline]
#[cfg(not(feature = "cmo"))]
pub fn prefetch(_addr: usize, _hint: PrefetchHint) {
    // No-op without CMO extension
}

// ============================================================================
// Cache Information
// ============================================================================

/// Cache level information
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheInfo {
    /// Cache size in bytes
    pub size: usize,
    /// Cache line size in bytes
    pub line_size: usize,
    /// Number of ways (associativity)
    pub ways: usize,
    /// Number of sets
    pub sets: usize,
}

/// Get cache information for a level
///
/// Returns None if cache info cannot be determined.
/// On RISC-V, this typically requires device tree or platform-specific info.
pub fn get_cache_info(_level: u8, _is_data: bool) -> Option<CacheInfo> {
    // RISC-V doesn't have standard cache discovery mechanism
    // This would need to be populated from device tree
    None
}

// ============================================================================
// Unified Cache Operations for Kernel Use
// ============================================================================

/// Synchronize caches after code modification
///
/// Call this after writing executable code to memory.
#[inline]
pub fn sync_code_cache(start: usize, size: usize) {
    // Clean D-cache to ensure code is in memory
    dcache_clean_range(start, size);
    // Then synchronize I-cache
    fence_i();
}

/// Ensure all pending memory operations are complete
#[inline]
pub fn sync_all() {
    super::barriers::fence_rw_rw();
}

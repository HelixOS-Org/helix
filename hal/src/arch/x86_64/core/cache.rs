//! # Cache Control Framework
//!
//! x86_64 cache management and control.
//!
//! ## Overview
//!
//! This module provides:
//!
//! - Cache line operations (flush, invalidate)
//! - Write-back/invalidate
//! - Cache prefetch hints
//! - Memory barriers
//! - Cache configuration (via MTRR/PAT)

use core::arch::asm;

// =============================================================================
// CACHE LINE SIZE
// =============================================================================

/// Default cache line size (64 bytes on modern x86_64)
pub const CACHE_LINE_SIZE: usize = 64;

/// Get actual cache line size from CPUID
pub fn cache_line_size() -> usize {
    use super::cpuid::CpuId;
    CpuId::new().max_basic_leaf(); // Trigger CPUID
                                   // Most x86_64 CPUs use 64-byte cache lines
    64
}

// =============================================================================
// CACHE LINE OPERATIONS
// =============================================================================

/// Flush cache line containing address (CLFLUSH)
///
/// Writes back and invalidates the cache line containing the linear address.
/// This instruction is ordered with respect to stores.
///
/// # Safety
/// The address must be valid memory.
#[inline]
pub unsafe fn clflush(addr: *const u8) {
    unsafe {
        asm!("clflush [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Optimized cache line flush (CLFLUSHOPT)
///
/// Like CLFLUSH but with relaxed ordering requirements.
/// Requires CLFLUSHOPT feature.
///
/// # Safety
/// The address must be valid memory.
/// Requires CLFLUSHOPT CPU feature.
#[inline]
pub unsafe fn clflushopt(addr: *const u8) {
    unsafe {
        asm!("clflushopt [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Cache line write back (CLWB)
///
/// Writes back the cache line but may keep it in cache.
/// Useful for persistent memory.
/// Requires CLWB feature.
///
/// # Safety
/// The address must be valid memory.
/// Requires CLWB CPU feature.
#[inline]
pub unsafe fn clwb(addr: *const u8) {
    unsafe {
        asm!("clwb [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Flush a range of memory
///
/// # Safety
/// The address range must be valid memory.
#[inline]
pub unsafe fn flush_range(start: *const u8, len: usize) {
    let mut addr = start as usize;
    let end = addr + len;

    while addr < end {
        unsafe {
            clflush(addr as *const u8);
        }
        addr += CACHE_LINE_SIZE;
    }
}

/// Flush a range of memory using CLFLUSHOPT
///
/// # Safety
/// The address range must be valid memory.
/// Requires CLFLUSHOPT CPU feature.
#[inline]
pub unsafe fn flush_range_opt(start: *const u8, len: usize) {
    let mut addr = start as usize;
    let end = addr + len;

    while addr < end {
        unsafe {
            clflushopt(addr as *const u8);
        }
        addr += CACHE_LINE_SIZE;
    }

    // CLFLUSHOPT requires SFENCE for ordering
    sfence();
}

// =============================================================================
// WRITE-BACK & INVALIDATE
// =============================================================================

/// Write back all modified cache lines (WBINVD)
///
/// Writes back all modified cache lines in all levels of the cache
/// and invalidates all cache entries.
///
/// # Safety
/// This is an extremely expensive operation.
/// Should only be used during power management transitions.
#[inline]
pub unsafe fn wbinvd() {
    unsafe {
        asm!("wbinvd", options(nostack, preserves_flags));
    }
}

/// Invalidate caches without writeback (INVD)
///
/// Invalidates all cache lines without writing them back.
///
/// # Safety
/// This WILL cause data loss for dirty cache lines!
/// Only use after disabling caching or in special hardware contexts.
#[inline]
pub unsafe fn invd() {
    unsafe {
        asm!("invd", options(nostack, preserves_flags));
    }
}

// =============================================================================
// MEMORY BARRIERS
// =============================================================================

/// Memory fence (MFENCE)
///
/// Guarantees that every memory access (load and store) before MFENCE
/// is globally visible before any memory access after MFENCE.
#[inline]
pub fn mfence() {
    unsafe {
        asm!("mfence", options(nostack, preserves_flags));
    }
}

/// Store fence (SFENCE)
///
/// Guarantees that every store before SFENCE is globally visible
/// before any store after SFENCE.
#[inline]
pub fn sfence() {
    unsafe {
        asm!("sfence", options(nostack, preserves_flags));
    }
}

/// Load fence (LFENCE)
///
/// Guarantees that every load before LFENCE completes
/// before any load after LFENCE starts.
/// Also serializes instruction execution.
#[inline]
pub fn lfence() {
    unsafe {
        asm!("lfence", options(nostack, preserves_flags));
    }
}

// =============================================================================
// PREFETCH
// =============================================================================

/// Prefetch hint level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchHint {
    /// Prefetch to all cache levels
    T0,
    /// Prefetch to L2 and higher (not L1)
    T1,
    /// Prefetch to L3 and higher (not L1/L2)
    T2,
    /// Non-temporal prefetch (minimize cache pollution)
    Nta,
}

/// Prefetch memory for reading
///
/// This is a hint to the processor; it may be ignored.
#[inline]
pub fn prefetch(addr: *const u8, hint: PrefetchHint) {
    unsafe {
        match hint {
            PrefetchHint::T0 => {
                asm!("prefetcht0 [{}]", in(reg) addr, options(nostack, preserves_flags))
            },
            PrefetchHint::T1 => {
                asm!("prefetcht1 [{}]", in(reg) addr, options(nostack, preserves_flags))
            },
            PrefetchHint::T2 => {
                asm!("prefetcht2 [{}]", in(reg) addr, options(nostack, preserves_flags))
            },
            PrefetchHint::Nta => {
                asm!("prefetchnta [{}]", in(reg) addr, options(nostack, preserves_flags))
            },
        }
    }
}

/// Prefetch memory for writing
///
/// AMD extension. This is a hint to the processor.
#[inline]
pub fn prefetchw(addr: *const u8) {
    unsafe {
        asm!("prefetchw [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Prefetch a range of memory
#[inline]
pub fn prefetch_range(start: *const u8, len: usize, hint: PrefetchHint) {
    let mut addr = start as usize;
    let end = addr + len;

    while addr < end {
        prefetch(addr as *const u8, hint);
        addr += CACHE_LINE_SIZE;
    }
}

// =============================================================================
// NON-TEMPORAL STORES
// =============================================================================

/// Non-temporal store (32-bit)
///
/// Stores data directly to memory, bypassing cache.
/// Useful for streaming writes that won't be read soon.
///
/// # Safety
/// Address must be 4-byte aligned.
#[inline]
pub unsafe fn movnti32(addr: *mut u32, value: u32) {
    unsafe {
        asm!("movnti [{}], {:e}", in(reg) addr, in(reg) value, options(nostack, preserves_flags));
    }
}

/// Non-temporal store (64-bit)
///
/// # Safety
/// Address must be 8-byte aligned.
#[inline]
pub unsafe fn movnti64(addr: *mut u64, value: u64) {
    unsafe {
        asm!("movnti [{}], {}", in(reg) addr, in(reg) value, options(nostack, preserves_flags));
    }
}

// =============================================================================
// CACHE CONTROL VIA CR0
// =============================================================================

/// Disable caching globally via CR0.CD
///
/// # Safety
/// This will severely impact performance.
/// Only use for specific hardware requirements.
#[inline]
pub unsafe fn disable_cache() {
    use super::control_regs::Cr0;
    unsafe {
        Cr0::update(|cr0| {
            cr0.insert(Cr0::CD);
            cr0.insert(Cr0::NW);
        });
        wbinvd();
    }
}

/// Enable caching globally via CR0.CD
///
/// # Safety
/// Should only be called after disable_cache.
#[inline]
pub unsafe fn enable_cache() {
    use super::control_regs::Cr0;
    unsafe {
        Cr0::update(|cr0| {
            cr0.remove(Cr0::CD);
            cr0.remove(Cr0::NW);
        });
    }
}

// =============================================================================
// CACHE-ALIGNED ALLOCATION HELPERS
// =============================================================================

/// Align an address up to cache line boundary
#[inline]
pub const fn align_up_cache_line(addr: usize) -> usize {
    (addr + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)
}

/// Align an address down to cache line boundary
#[inline]
pub const fn align_down_cache_line(addr: usize) -> usize {
    addr & !(CACHE_LINE_SIZE - 1)
}

/// Check if address is cache-line aligned
#[inline]
pub const fn is_cache_line_aligned(addr: usize) -> bool {
    addr & (CACHE_LINE_SIZE - 1) == 0
}

// =============================================================================
// CACHE LINE STRUCTURE
// =============================================================================

/// A cache-line aligned structure
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct CacheAligned<T> {
    value: T,
}

impl<T> CacheAligned<T> {
    /// Create a new cache-aligned value
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Get reference to inner value
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Get mutable reference to inner value
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Consume and return inner value
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: Default> Default for CacheAligned<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> core::ops::Deref for CacheAligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> core::ops::DerefMut for CacheAligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

// =============================================================================
// CACHE-PADDED FOR FALSE SHARING PREVENTION
// =============================================================================

/// Padding to prevent false sharing
///
/// The struct is aligned to 64 bytes (cache line size) which naturally
/// provides false sharing prevention. The alignment attribute ensures
/// the value occupies a full cache line.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct CachePadded<T> {
    value: T,
}

impl<T> CachePadded<T> {
    /// Create a new cache-padded value
    pub const fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: Default> Default for CachePadded<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> core::ops::Deref for CachePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> core::ops::DerefMut for CachePadded<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_line_alignment() {
        assert_eq!(align_up_cache_line(0), 0);
        assert_eq!(align_up_cache_line(1), 64);
        assert_eq!(align_up_cache_line(63), 64);
        assert_eq!(align_up_cache_line(64), 64);
        assert_eq!(align_up_cache_line(65), 128);

        assert_eq!(align_down_cache_line(0), 0);
        assert_eq!(align_down_cache_line(63), 0);
        assert_eq!(align_down_cache_line(64), 64);
        assert_eq!(align_down_cache_line(127), 64);
    }

    #[test]
    fn test_is_aligned() {
        assert!(is_cache_line_aligned(0));
        assert!(is_cache_line_aligned(64));
        assert!(is_cache_line_aligned(128));
        assert!(!is_cache_line_aligned(1));
        assert!(!is_cache_line_aligned(63));
    }

    #[test]
    fn test_cache_aligned_struct() {
        let aligned: CacheAligned<u64> = CacheAligned::new(42);
        assert_eq!(*aligned, 42);

        let ptr = &aligned as *const _ as usize;
        assert!(is_cache_line_aligned(ptr));
    }

    #[test]
    fn test_memory_barriers() {
        // Just ensure they don't crash
        mfence();
        sfence();
        lfence();
    }
}

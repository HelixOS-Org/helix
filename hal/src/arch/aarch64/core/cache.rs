//! # AArch64 Cache Maintenance Operations
//!
//! This module provides cache maintenance primitives for data and instruction
//! caches on AArch64.

use core::arch::asm;

// =============================================================================
// Cache Type Register
// =============================================================================

/// Cache level identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheInfo {
    /// Number of cache levels (1-7)
    pub levels: u8,
    /// L1 data cache line size in bytes
    pub l1d_line_size: usize,
    /// L1 instruction cache line size in bytes
    pub l1i_line_size: usize,
    /// Smallest cache line size in bytes
    pub min_line_size: usize,
}

impl CacheInfo {
    /// Read cache information from CTR_EL0
    pub fn read() -> Self {
        let ctr = read_ctr_el0();

        // IminLine: Log2 of the number of words in smallest icache line
        let imin_line = (ctr & 0xF) as usize;
        let l1i_line_size = 4 << imin_line;

        // DminLine: Log2 of the number of words in smallest dcache line
        let dmin_line = ((ctr >> 16) & 0xF) as usize;
        let l1d_line_size = 4 << dmin_line;

        let min_line_size = l1d_line_size.min(l1i_line_size);

        // Read CLIDR_EL1 for cache levels
        let clidr = read_clidr_el1();
        let mut levels = 0u8;
        for i in 0..7 {
            let ctype = (clidr >> (i * 3)) & 0x7;
            if ctype != 0 {
                levels = i as u8 + 1;
            }
        }

        Self {
            levels,
            l1d_line_size,
            l1i_line_size,
            min_line_size,
        }
    }
}

/// Cache level type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheLevelType {
    /// No cache
    None            = 0,
    /// Instruction cache only
    ICache          = 1,
    /// Data cache only
    DCache          = 2,
    /// Separate instruction and data caches
    SeparateIdCache = 3,
    /// Unified cache
    UnifiedCache    = 4,
}

impl From<u8> for CacheLevelType {
    fn from(value: u8) -> Self {
        match value & 0x7 {
            0 => Self::None,
            1 => Self::ICache,
            2 => Self::DCache,
            3 => Self::SeparateIdCache,
            4 => Self::UnifiedCache,
            _ => Self::None,
        }
    }
}

// =============================================================================
// Data Cache Operations
// =============================================================================

/// Clean data cache line by virtual address to Point of Coherency
#[inline]
pub fn dc_cvac(addr: usize) {
    unsafe {
        asm!("dc cvac, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Clean data cache line by virtual address to Point of Unification
#[inline]
pub fn dc_cvau(addr: usize) {
    unsafe {
        asm!("dc cvau, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Clean data cache line by virtual address to Point of Persistence
#[inline]
pub fn dc_cvap(addr: usize) {
    unsafe {
        asm!("dc cvap, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Clean data cache line by virtual address to Point of Deep Persistence
#[inline]
#[cfg(feature = "armv8.5")]
pub fn dc_cvadp(addr: usize) {
    unsafe {
        asm!("dc cvadp, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Invalidate data cache line by virtual address to Point of Coherency
#[inline]
pub fn dc_ivac(addr: usize) {
    unsafe {
        asm!("dc ivac, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Clean and invalidate data cache line by virtual address to Point of Coherency
#[inline]
pub fn dc_civac(addr: usize) {
    unsafe {
        asm!("dc civac, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Zero data cache line by virtual address
#[inline]
pub fn dc_zva(addr: usize) {
    unsafe {
        asm!("dc zva, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

// =============================================================================
// Instruction Cache Operations
// =============================================================================

/// Invalidate all instruction caches to Point of Unification
#[inline]
pub fn ic_iallu() {
    unsafe {
        asm!("ic iallu", options(nostack, preserves_flags));
    }
}

/// Invalidate all instruction caches in Inner Shareable domain to Point of Unification
#[inline]
pub fn ic_ialluis() {
    unsafe {
        asm!("ic ialluis", options(nostack, preserves_flags));
    }
}

/// Invalidate instruction cache line by virtual address to Point of Unification
#[inline]
pub fn ic_ivau(addr: usize) {
    unsafe {
        asm!("ic ivau, {}", in(reg) addr, options(nostack, preserves_flags));
    }
}

// =============================================================================
// Range Operations
// =============================================================================

/// Clean data cache range to Point of Coherency
pub fn dc_clean_range(start: usize, size: usize) {
    let cache_info = CacheInfo::read();
    let line_size = cache_info.l1d_line_size;
    let line_mask = !(line_size - 1);

    let aligned_start = start & line_mask;
    let end = start + size;

    let mut addr = aligned_start;
    while addr < end {
        dc_cvac(addr);
        addr += line_size;
    }
    dsb_ish();
}

/// Invalidate data cache range
pub fn dc_invalidate_range(start: usize, size: usize) {
    let cache_info = CacheInfo::read();
    let line_size = cache_info.l1d_line_size;
    let line_mask = !(line_size - 1);

    let aligned_start = start & line_mask;
    let end = start + size;

    let mut addr = aligned_start;
    while addr < end {
        dc_ivac(addr);
        addr += line_size;
    }
    dsb_ish();
}

/// Clean and invalidate data cache range
pub fn dc_clean_invalidate_range(start: usize, size: usize) {
    let cache_info = CacheInfo::read();
    let line_size = cache_info.l1d_line_size;
    let line_mask = !(line_size - 1);

    let aligned_start = start & line_mask;
    let end = start + size;

    let mut addr = aligned_start;
    while addr < end {
        dc_civac(addr);
        addr += line_size;
    }
    dsb_ish();
}

/// Invalidate instruction cache range
pub fn ic_invalidate_range(start: usize, size: usize) {
    let cache_info = CacheInfo::read();
    let line_size = cache_info.l1i_line_size;
    let line_mask = !(line_size - 1);

    let aligned_start = start & line_mask;
    let end = start + size;

    let mut addr = aligned_start;
    while addr < end {
        dc_cvau(addr); // Clean to PoU first
        addr += line_size;
    }
    dsb_ish();

    let mut addr = aligned_start;
    while addr < end {
        ic_ivau(addr);
        addr += line_size;
    }
    dsb_ish();
    isb();
}

// =============================================================================
// Full Cache Operations
// =============================================================================

/// Clean and invalidate all data caches
pub fn dc_clean_invalidate_all() {
    let clidr = read_clidr_el1();

    // Process each cache level
    for level in 0..7u64 {
        let ctype = (clidr >> (level * 3)) & 0x7;
        if ctype == 0 {
            break; // No more cache levels
        }

        // Only process data or unified caches
        if ctype >= 2 {
            dc_clean_invalidate_level(level as u32);
        }
    }

    dsb_sy();
    isb();
}

/// Clean and invalidate a specific cache level
fn dc_clean_invalidate_level(level: u32) {
    // Select cache level
    let csselr = (level << 1) as u64;
    write_csselr_el1(csselr);
    isb();

    // Read cache size ID register
    let ccsidr = read_ccsidr_el1();

    // Extract cache geometry
    let line_size = ((ccsidr & 0x7) + 4) as u32;
    let assoc = (((ccsidr >> 3) & 0x3FF) + 1) as u32;
    let num_sets = (((ccsidr >> 13) & 0x7FFF) + 1) as u32;

    let way_shift = clz(assoc - 1);
    let set_shift = line_size;

    // Iterate over all ways and sets
    for way in 0..assoc {
        for set in 0..num_sets {
            let val =
                ((way as u64) << way_shift) | ((set as u64) << set_shift) | ((level as u64) << 1);
            dc_cisw(val);
        }
    }
}

/// Clean and invalidate data cache by set/way
#[inline]
fn dc_cisw(value: u64) {
    unsafe {
        asm!("dc cisw, {}", in(reg) value, options(nostack, preserves_flags));
    }
}

/// Count leading zeros
#[inline]
fn clz(value: u32) -> u32 {
    value.leading_zeros()
}

// =============================================================================
// Barriers (imported for convenience)
// =============================================================================

/// Data Synchronization Barrier - Inner Shareable
#[inline]
fn dsb_ish() {
    unsafe {
        asm!("dsb ish", options(nostack, preserves_flags));
    }
}

/// Data Synchronization Barrier - Full System
#[inline]
fn dsb_sy() {
    unsafe {
        asm!("dsb sy", options(nostack, preserves_flags));
    }
}

/// Instruction Synchronization Barrier
#[inline]
fn isb() {
    unsafe {
        asm!("isb", options(nostack, preserves_flags));
    }
}

// =============================================================================
// System Register Access
// =============================================================================

/// Read Cache Type Register (EL0)
#[inline]
pub fn read_ctr_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, CTR_EL0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read Cache Level ID Register (EL1)
#[inline]
pub fn read_clidr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, CLIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read Cache Size ID Register (EL1)
#[inline]
pub fn read_ccsidr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, CCSIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write Cache Size Selection Register (EL1)
#[inline]
pub fn write_csselr_el1(value: u64) {
    unsafe {
        asm!("msr CSSELR_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Read Data Cache Zero ID Register (EL0)
#[inline]
pub fn read_dczid_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, DCZID_EL0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

// =============================================================================
// High-Level API
// =============================================================================

/// Ensure data written to a memory range is visible
pub fn flush_dcache(addr: usize, size: usize) {
    dc_clean_range(addr, size);
}

/// Invalidate data cache (use carefully - may lose data)
pub fn invalidate_dcache(addr: usize, size: usize) {
    dc_invalidate_range(addr, size);
}

/// Flush data cache and invalidate instruction cache for code
pub fn flush_icache(addr: usize, size: usize) {
    ic_invalidate_range(addr, size);
}

/// Prepare memory for DMA read (device will read from memory)
pub fn prepare_dma_read(addr: usize, size: usize) {
    dc_clean_range(addr, size);
}

/// Prepare memory for DMA write (device will write to memory)
pub fn prepare_dma_write(addr: usize, size: usize) {
    dc_invalidate_range(addr, size);
}

/// Complete DMA transfer
pub fn complete_dma_transfer(addr: usize, size: usize) {
    dc_invalidate_range(addr, size);
}

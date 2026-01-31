//! # RISC-V Address Space Identifier (ASID) Management
//!
//! This module provides ASID allocation and management for RISC-V.
//!
//! ## ASID Overview
//!
//! ASIDs allow the TLB to cache entries from multiple address spaces
//! simultaneously, avoiding TLB flushes on context switches.
//!
//! - RISC-V supports up to 16-bit ASIDs (65536 values)
//! - ASID 0 is typically reserved for the kernel
//! - Not all bits may be implemented (check by writing and reading back)

use core::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use super::satp;
use super::tlb;

// ============================================================================
// ASID Constants
// ============================================================================

/// Maximum possible ASID value (16-bit)
pub const MAX_ASID: u16 = 0xFFFF;

/// Reserved ASID for kernel
pub const KERNEL_ASID: u16 = 0;

/// Invalid ASID marker
pub const INVALID_ASID: u16 = u16::MAX;

// ============================================================================
// ASID Type
// ============================================================================

/// Address Space Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Asid(u16);

impl Asid {
    /// Create new ASID
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Get the raw value
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Kernel ASID
    pub const fn kernel() -> Self {
        Self(KERNEL_ASID)
    }

    /// Check if this is the kernel ASID
    pub const fn is_kernel(self) -> bool {
        self.0 == KERNEL_ASID
    }

    /// Create invalid ASID
    pub const fn invalid() -> Self {
        Self(INVALID_ASID)
    }

    /// Check if ASID is valid
    pub const fn is_valid(self) -> bool {
        self.0 != INVALID_ASID
    }
}

impl Default for Asid {
    fn default() -> Self {
        Self::invalid()
    }
}

impl From<u16> for Asid {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<Asid> for u16 {
    fn from(asid: Asid) -> u16 {
        asid.0
    }
}

// ============================================================================
// Global ASID State
// ============================================================================

/// Maximum implemented ASID bits (detected at runtime)
static MAX_ASID_BITS: AtomicU16 = AtomicU16::new(0);

/// Next ASID to allocate
static NEXT_ASID: AtomicU16 = AtomicU16::new(1);

/// Current ASID generation (for lazy TLB invalidation)
static ASID_GENERATION: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// ASID Detection
// ============================================================================

/// Detect the number of implemented ASID bits
///
/// This should be called once during initialization.
pub fn detect_asid_bits() -> u16 {
    // Save current SATP
    let original = satp::read_satp();

    // Try to set all ASID bits
    let test_satp = satp::Satp::from_bits(
        (original.bits() & !satp::Satp::ASID_MASK) | (MAX_ASID as u64) << satp::Satp::ASID_SHIFT
    );
    satp::write_satp(test_satp);

    // Read back to see what stuck
    let result = satp::read_satp();
    let max_asid = result.asid();

    // Restore original
    satp::write_satp(original);

    // Count bits
    let bits = if max_asid == 0 {
        0
    } else {
        16 - max_asid.leading_zeros() as u16
    };

    MAX_ASID_BITS.store(bits, Ordering::Relaxed);
    bits
}

/// Get the maximum ASID bits
pub fn get_max_asid_bits() -> u16 {
    MAX_ASID_BITS.load(Ordering::Relaxed)
}

/// Get the maximum valid ASID value
pub fn get_max_asid() -> u16 {
    let bits = get_max_asid_bits();
    if bits == 0 {
        0
    } else {
        (1 << bits) - 1
    }
}

/// Check if ASIDs are supported
pub fn asids_supported() -> bool {
    get_max_asid_bits() > 0
}

// ============================================================================
// Current ASID Access
// ============================================================================

/// Get the current ASID
pub fn get_current_asid() -> Asid {
    Asid(satp::get_current_asid())
}

/// Set the current ASID
pub fn set_current_asid(asid: Asid) {
    satp::set_current_asid(asid.value());
}

// ============================================================================
// ASID Allocator
// ============================================================================

/// Simple ASID allocator
///
/// Uses a simple counter with wraparound. When ASIDs wrap around,
/// a new generation is started and all TLBs are flushed.
pub struct AsidAllocator {
    /// Next ASID to allocate
    next: u16,
    /// Maximum valid ASID
    max: u16,
    /// Current generation
    generation: u64,
    /// Number of allocated ASIDs
    allocated: u32,
}

impl AsidAllocator {
    /// Create a new ASID allocator
    pub const fn new() -> Self {
        Self {
            next: 1, // Start at 1, reserve 0 for kernel
            max: 0,
            generation: 0,
            allocated: 0,
        }
    }

    /// Initialize the allocator
    pub fn init(&mut self) {
        self.max = get_max_asid();
        if self.max == 0 {
            // No ASID support - everything uses ASID 0
            self.max = 0;
        }
    }

    /// Allocate a new ASID
    pub fn allocate(&mut self) -> Option<AsidContext> {
        if self.max == 0 {
            // No ASID support
            return Some(AsidContext {
                asid: Asid::kernel(),
                generation: self.generation,
            });
        }

        let asid = self.next;
        self.next += 1;
        self.allocated += 1;

        if self.next > self.max {
            // Wrap around - start new generation
            self.next = 1;
            self.generation += 1;
            // Flush all TLBs since we're reusing ASIDs
            tlb::flush_tlb_all();
        }

        Some(AsidContext {
            asid: Asid::new(asid),
            generation: self.generation,
        })
    }

    /// Free an ASID
    ///
    /// Note: Simple allocator doesn't actually recycle ASIDs,
    /// they're just abandoned until generation rolls over.
    pub fn free(&mut self, _asid: Asid) {
        self.allocated = self.allocated.saturating_sub(1);
    }

    /// Get current generation
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get number of allocated ASIDs
    pub fn allocated_count(&self) -> u32 {
        self.allocated
    }

    /// Get number of available ASIDs
    pub fn available_count(&self) -> u32 {
        if self.max == 0 {
            0
        } else {
            (self.max as u32 + 1).saturating_sub(self.allocated)
        }
    }
}

impl Default for AsidAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ASID Context
// ============================================================================

/// ASID with generation for lazy invalidation
#[derive(Debug, Clone, Copy)]
pub struct AsidContext {
    /// The ASID value
    pub asid: Asid,
    /// Generation when this ASID was allocated
    pub generation: u64,
}

impl AsidContext {
    /// Create new ASID context
    pub const fn new(asid: Asid, generation: u64) -> Self {
        Self { asid, generation }
    }

    /// Check if this context is still valid
    pub fn is_valid(&self, current_generation: u64) -> bool {
        self.generation == current_generation
    }

    /// Get the ASID value
    pub fn value(&self) -> u16 {
        self.asid.value()
    }
}

// ============================================================================
// Global ASID Allocator
// ============================================================================

/// Global ASID allocator instance
static mut GLOBAL_ASID_ALLOCATOR: AsidAllocator = AsidAllocator::new();

/// Initialize the global ASID allocator
///
/// # Safety
/// Must be called once during init before any ASID allocation.
pub unsafe fn init_global_allocator() {
    detect_asid_bits();
    GLOBAL_ASID_ALLOCATOR.init();
}

/// Allocate an ASID from the global allocator
///
/// # Safety
/// Must ensure proper synchronization in SMP environment.
pub unsafe fn allocate_asid() -> Option<AsidContext> {
    GLOBAL_ASID_ALLOCATOR.allocate()
}

/// Free an ASID to the global allocator
///
/// # Safety
/// Must ensure the ASID is no longer in use.
pub unsafe fn free_asid(asid: Asid) {
    GLOBAL_ASID_ALLOCATOR.free(asid);
}

/// Get current global generation
pub fn current_generation() -> u64 {
    ASID_GENERATION.load(Ordering::Acquire)
}

// ============================================================================
// ASID-based Context Switching
// ============================================================================

/// Context switch using ASID
///
/// Switches to a new address space, potentially reusing TLB entries
/// if the ASID is still valid.
pub fn switch_context(root_table: usize, ctx: &mut AsidContext, current_gen: u64) {
    if ctx.is_valid(current_gen) {
        // ASID is still valid, just switch
        satp::switch_address_space(root_table, ctx.asid.value());
    } else {
        // ASID is stale, need a new one
        // In production, would allocate new ASID here
        // For now, just invalidate the old one
        ctx.generation = current_gen;
        satp::switch_address_space(root_table, ctx.asid.value());
        tlb::flush_tlb_asid(ctx.asid.value());
    }
}

// ============================================================================
// Per-CPU ASID Cache
// ============================================================================

/// Per-CPU ASID cache entry
#[derive(Debug, Clone, Copy, Default)]
pub struct AsidCacheEntry {
    /// The ASID in use
    pub asid: Asid,
    /// Generation when cached
    pub generation: u64,
    /// Whether entry is valid
    pub valid: bool,
}

impl AsidCacheEntry {
    /// Create new cache entry
    pub const fn new() -> Self {
        Self {
            asid: Asid::new(0),
            generation: 0,
            valid: false,
        }
    }

    /// Check if cache entry is usable
    pub fn is_usable(&self, current_gen: u64) -> bool {
        self.valid && self.generation == current_gen
    }

    /// Invalidate the entry
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Update the entry
    pub fn update(&mut self, asid: Asid, generation: u64) {
        self.asid = asid;
        self.generation = generation;
        self.valid = true;
    }
}

/// Per-CPU ASID cache for fast context switches
#[derive(Debug)]
pub struct PerCpuAsidCache {
    /// Cache entries indexed by some identifier (e.g., process ID)
    entries: [AsidCacheEntry; 16],
}

impl PerCpuAsidCache {
    /// Create new per-CPU cache
    pub const fn new() -> Self {
        Self {
            entries: [AsidCacheEntry::new(); 16],
        }
    }

    /// Look up ASID for a context
    pub fn lookup(&self, index: usize, current_gen: u64) -> Option<Asid> {
        let entry = self.entries.get(index % 16)?;
        if entry.is_usable(current_gen) {
            Some(entry.asid)
        } else {
            None
        }
    }

    /// Store ASID for a context
    pub fn store(&mut self, index: usize, asid: Asid, generation: u64) {
        if let Some(entry) = self.entries.get_mut(index % 16) {
            entry.update(asid, generation);
        }
    }

    /// Invalidate all entries
    pub fn invalidate_all(&mut self) {
        for entry in &mut self.entries {
            entry.invalidate();
        }
    }

    /// Invalidate entries for old generation
    pub fn invalidate_old(&mut self, current_gen: u64) {
        for entry in &mut self.entries {
            if entry.generation != current_gen {
                entry.invalidate();
            }
        }
    }
}

impl Default for PerCpuAsidCache {
    fn default() -> Self {
        Self::new()
    }
}

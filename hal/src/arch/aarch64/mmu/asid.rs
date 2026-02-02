//! # AArch64 ASID Management
//!
//! This module provides Address Space Identifier (ASID) management for
//! efficient TLB usage across multiple address spaces.

use core::sync::atomic::{AtomicU16, AtomicU64, Ordering};

// =============================================================================
// ASID Type
// =============================================================================

/// ASID (Address Space Identifier)
///
/// ASIDs allow multiple address spaces to coexist in the TLB without
/// requiring full TLB flushes on context switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Asid(u16);

impl Asid {
    /// ASID value 0 (reserved/global)
    pub const ZERO: Self = Self(0);

    /// Maximum ASID value for 8-bit ASID
    pub const MAX_8BIT: u16 = 255;

    /// Maximum ASID value for 16-bit ASID
    pub const MAX_16BIT: u16 = 65535;

    /// Create a new ASID
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Get the raw value
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Check if this is the zero/global ASID
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Get as u64 for TTBR encoding
    pub const fn as_ttbr(self) -> u64 {
        (self.0 as u64) << 48
    }
}

impl From<u16> for Asid {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<Asid> for u16 {
    fn from(asid: Asid) -> Self {
        asid.0
    }
}

// =============================================================================
// ASID Generation
// =============================================================================

/// ASID generation for tracking TLB invalidation needs
///
/// When the generation changes, all ASIDs from previous generations
/// are considered stale and need TLB invalidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsidGeneration {
    /// Generation counter
    pub generation: u64,
    /// ASID value
    pub asid: Asid,
}

impl AsidGeneration {
    /// Create a new ASID generation
    pub const fn new(generation: u64, asid: Asid) -> Self {
        Self { generation, asid }
    }

    /// Check if this generation matches current
    pub fn is_current(&self, current_gen: u64) -> bool {
        self.generation == current_gen
    }
}

// =============================================================================
// ASID Allocator
// =============================================================================

/// ASID allocator for managing ASID assignment
///
/// Uses a simple bump allocator with generation tracking for
/// handling ASID wraparound.
pub struct AsidAllocator {
    /// Next ASID to allocate
    next_asid: AtomicU16,
    /// Current generation
    generation: AtomicU64,
    /// Maximum ASID value
    max_asid: u16,
    /// Whether 16-bit ASIDs are supported
    asid16: bool,
}

impl AsidAllocator {
    /// Create a new allocator with 8-bit ASIDs
    pub const fn new_8bit() -> Self {
        Self {
            next_asid: AtomicU16::new(1), // Start at 1, 0 is reserved
            generation: AtomicU64::new(1),
            max_asid: Asid::MAX_8BIT,
            asid16: false,
        }
    }

    /// Create a new allocator with 16-bit ASIDs
    pub const fn new_16bit() -> Self {
        Self {
            next_asid: AtomicU16::new(1),
            generation: AtomicU64::new(1),
            max_asid: Asid::MAX_16BIT,
            asid16: true,
        }
    }

    /// Detect ASID size from CPU features and create allocator
    pub fn detect() -> Self {
        // Read ID_AA64MMFR0_EL1 ASIDBits field
        let mmfr0 = read_id_aa64mmfr0_el1();
        let asid_bits = (mmfr0 >> 4) & 0xF;

        if asid_bits >= 2 {
            Self::new_16bit()
        } else {
            Self::new_8bit()
        }
    }

    /// Get maximum ASID value
    pub fn max_asid(&self) -> u16 {
        self.max_asid
    }

    /// Check if 16-bit ASIDs are supported
    pub fn supports_16bit(&self) -> bool {
        self.asid16
    }

    /// Get current generation
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Allocate a new ASID with generation
    pub fn allocate(&self) -> AsidGeneration {
        loop {
            let current = self.next_asid.load(Ordering::Acquire);
            let gen = self.generation.load(Ordering::Acquire);

            if current > self.max_asid {
                // ASID space exhausted, need new generation
                self.rollover();
                continue;
            }

            // Try to increment
            if self
                .next_asid
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return AsidGeneration::new(gen, Asid::new(current));
            }
            // CAS failed, retry
        }
    }

    /// Handle ASID rollover
    fn rollover(&self) {
        // Increment generation
        self.generation.fetch_add(1, Ordering::AcqRel);

        // Reset ASID counter
        self.next_asid.store(1, Ordering::Release);

        // Full TLB flush needed - all old ASIDs are now stale
        super::tlb::tlb_flush_all();
    }

    /// Check if an ASID generation is still valid
    pub fn is_valid(&self, asid_gen: AsidGeneration) -> bool {
        asid_gen.generation == self.generation.load(Ordering::Acquire)
    }

    /// Refresh an ASID if its generation is stale
    ///
    /// Returns the new ASID generation if refresh was needed
    pub fn refresh_if_needed(&self, asid_gen: AsidGeneration) -> AsidGeneration {
        if self.is_valid(asid_gen) {
            asid_gen
        } else {
            self.allocate()
        }
    }
}

impl Default for AsidAllocator {
    fn default() -> Self {
        Self::detect()
    }
}

// =============================================================================
// ASID Context
// =============================================================================

/// ASID context for a process/address space
#[derive(Debug)]
pub struct AsidContext {
    /// Current ASID generation
    asid_gen: AsidGeneration,
}

impl AsidContext {
    /// Create a new ASID context
    pub fn new(allocator: &AsidAllocator) -> Self {
        Self {
            asid_gen: allocator.allocate(),
        }
    }

    /// Get the current ASID
    pub fn asid(&self) -> Asid {
        self.asid_gen.asid
    }

    /// Get the ASID generation
    pub fn generation(&self) -> AsidGeneration {
        self.asid_gen
    }

    /// Check if ASID is still valid and refresh if needed
    pub fn ensure_valid(&mut self, allocator: &AsidAllocator) {
        self.asid_gen = allocator.refresh_if_needed(self.asid_gen);
    }
}

// =============================================================================
// TTBR Encoding
// =============================================================================

/// Encode ASID into TTBR value
pub const fn encode_ttbr_asid(base_addr: u64, asid: Asid) -> u64 {
    (base_addr & 0x0000_FFFF_FFFF_F000) | ((asid.value() as u64) << 48)
}

/// Decode ASID from TTBR value
pub const fn decode_ttbr_asid(ttbr: u64) -> Asid {
    Asid::new((ttbr >> 48) as u16)
}

/// Decode base address from TTBR value
pub const fn decode_ttbr_addr(ttbr: u64) -> u64 {
    ttbr & 0x0000_FFFF_FFFF_F000
}

// =============================================================================
// System Register Access
// =============================================================================

use core::arch::asm;

/// Read ID_AA64MMFR0_EL1
#[inline]
fn read_id_aa64mmfr0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64MMFR0_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read TTBR0_EL1
#[inline]
pub fn read_ttbr0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, TTBR0_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TTBR0_EL1
#[inline]
pub fn write_ttbr0_el1(value: u64) {
    unsafe {
        asm!("msr TTBR0_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Read TTBR1_EL1
#[inline]
pub fn read_ttbr1_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, TTBR1_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TTBR1_EL1
#[inline]
pub fn write_ttbr1_el1(value: u64) {
    unsafe {
        asm!("msr TTBR1_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Switch to a new address space (TTBR0 + ASID)
#[inline]
pub fn switch_ttbr0(page_table_addr: u64, asid: Asid) {
    let ttbr = encode_ttbr_asid(page_table_addr, asid);
    unsafe {
        asm!(
            "msr TTBR0_EL1, {ttbr}",
            "isb",
            ttbr = in(reg) ttbr,
            options(nomem, nostack)
        );
    }
}

/// Switch address space with proper TLB handling
pub fn switch_address_space(
    page_table_addr: u64,
    asid_context: &mut AsidContext,
    allocator: &AsidAllocator,
) {
    // Ensure ASID is still valid
    asid_context.ensure_valid(allocator);

    // Switch TTBR0 with new/refreshed ASID
    switch_ttbr0(page_table_addr, asid_context.asid());
}

// =============================================================================
// Per-CPU ASID State
// =============================================================================

/// Per-CPU ASID state
#[derive(Debug)]
pub struct PerCpuAsid {
    /// Current ASID on this CPU
    current_asid: Asid,
    /// Current generation on this CPU
    current_gen: u64,
}

impl PerCpuAsid {
    /// Create new per-CPU state
    pub const fn new() -> Self {
        Self {
            current_asid: Asid::ZERO,
            current_gen: 0,
        }
    }

    /// Check if context switch needs TLB flush
    pub fn needs_flush(&self, new_gen: AsidGeneration) -> bool {
        self.current_gen != new_gen.generation || self.current_asid != new_gen.asid
    }

    /// Update state after context switch
    pub fn update(&mut self, new_gen: AsidGeneration) {
        self.current_asid = new_gen.asid;
        self.current_gen = new_gen.generation;
    }
}

impl Default for PerCpuAsid {
    fn default() -> Self {
        Self::new()
    }
}

//! # TLB Management
//!
//! This module provides TLB (Translation Lookaside Buffer) management
//! operations including INVLPG, INVPCID, and PCID support.
//!
//! ## TLB Flushing Operations
//!
//! - `invlpg`: Invalidate a single page
//! - `invpcid`: Invalidate with PCID (more granular control)
//! - CR3 write: Flush entire TLB (or not, with PCID)
//!
//! ## PCID (Process Context Identifier)
//!
//! PCID allows the TLB to maintain entries for multiple address spaces
//! simultaneously, reducing TLB misses on context switches.

use core::fmt;
use super::addresses::VirtualAddress;

// =============================================================================
// PCID (Process Context Identifier)
// =============================================================================

/// Process Context Identifier
///
/// PCIDs are 12-bit identifiers (0-4095) that tag TLB entries.
/// PCID 0 is typically reserved for kernel use.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Pcid(u16);

impl Pcid {
    /// Maximum PCID value (12-bit)
    pub const MAX: u16 = 0xFFF;
    
    /// Kernel PCID (reserved)
    pub const KERNEL: Pcid = Pcid(0);
    
    /// Create a new PCID
    ///
    /// # Panics
    ///
    /// Panics if the value is > 4095.
    #[inline]
    pub const fn new(value: u16) -> Self {
        assert!(value <= Self::MAX);
        Self(value)
    }
    
    /// Create a new PCID, truncating to valid range
    #[inline]
    pub const fn new_truncate(value: u16) -> Self {
        Self(value & Self::MAX)
    }
    
    /// Get the PCID value
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self.0
    }
    
    /// Check if this is the kernel PCID
    #[inline]
    pub const fn is_kernel(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for Pcid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pcid({})", self.0)
    }
}

impl fmt::Display for Pcid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for Pcid {
    fn from(value: u16) -> Self {
        Self::new_truncate(value)
    }
}

impl From<Pcid> for u16 {
    fn from(pcid: Pcid) -> Self {
        pcid.0
    }
}

// =============================================================================
// INVPCID Types
// =============================================================================

/// INVPCID operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum InvpcidType {
    /// Invalidate single address for specific PCID
    IndividualAddress = 0,
    
    /// Invalidate all entries for specific PCID
    SingleContext = 1,
    
    /// Invalidate all entries for all PCIDs including globals
    AllContextsIncludingGlobals = 2,
    
    /// Invalidate all entries for all PCIDs except globals
    AllContextsExceptGlobals = 3,
}

/// INVPCID descriptor
#[derive(Clone, Copy)]
#[repr(C)]
pub struct InvpcidDescriptor {
    /// PCID
    pcid: u64,
    /// Virtual address
    address: u64,
}

impl InvpcidDescriptor {
    /// Create a new descriptor
    #[inline]
    pub const fn new(pcid: Pcid, address: VirtualAddress) -> Self {
        Self {
            pcid: pcid.0 as u64,
            address: address.as_u64(),
        }
    }
    
    /// Create a descriptor for single context invalidation
    #[inline]
    pub const fn single_context(pcid: Pcid) -> Self {
        Self {
            pcid: pcid.0 as u64,
            address: 0,
        }
    }
    
    /// Create an empty descriptor for global operations
    #[inline]
    pub const fn global() -> Self {
        Self {
            pcid: 0,
            address: 0,
        }
    }
}

// =============================================================================
// TLB Flush Operations
// =============================================================================

/// Flush a single page from the TLB
///
/// # Safety
///
/// This is always safe to call but may affect performance.
#[inline]
pub fn flush_tlb(addr: VirtualAddress) {
    unsafe {
        core::arch::asm!(
            "invlpg [{}]",
            in(reg) addr.as_u64(),
            options(nostack, preserves_flags),
        );
    }
}

/// Flush the entire TLB by reloading CR3
///
/// This invalidates all non-global TLB entries.
///
/// # Safety
///
/// This is safe but expensive.
#[inline]
pub fn flush_tlb_all() {
    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {0}, cr3",
            "mov cr3, {0}",
            out(reg) cr3,
            options(nostack),
        );
    }
    let _ = cr3; // Silence unused warning
}

/// Flush TLB entries for a range of pages
///
/// # Arguments
///
/// * `start` - Starting virtual address
/// * `count` - Number of pages to flush
/// * `page_size` - Size of each page
#[inline]
pub fn flush_tlb_range(start: VirtualAddress, count: usize, page_size: usize) {
    let mut addr = start.as_u64();
    for _ in 0..count {
        unsafe {
            core::arch::asm!(
                "invlpg [{}]",
                in(reg) addr,
                options(nostack, preserves_flags),
            );
        }
        addr += page_size as u64;
    }
}

/// Flush TLB using INVPCID instruction
///
/// # Safety
///
/// INVPCID must be supported by the CPU.
#[inline]
pub unsafe fn flush_tlb_pcid(inv_type: InvpcidType, descriptor: &InvpcidDescriptor) {
    core::arch::asm!(
        "invpcid {0}, [{1}]",
        in(reg) inv_type as u64,
        in(reg) descriptor,
        options(nostack, preserves_flags),
    );
}

/// Flush a single address for a specific PCID
///
/// # Safety
///
/// INVPCID must be supported by the CPU.
#[inline]
pub unsafe fn flush_tlb_pcid_address(pcid: Pcid, addr: VirtualAddress) {
    let desc = InvpcidDescriptor::new(pcid, addr);
    flush_tlb_pcid(InvpcidType::IndividualAddress, &desc);
}

/// Flush all entries for a specific PCID
///
/// # Safety
///
/// INVPCID must be supported by the CPU.
#[inline]
pub unsafe fn flush_tlb_pcid_context(pcid: Pcid) {
    let desc = InvpcidDescriptor::single_context(pcid);
    flush_tlb_pcid(InvpcidType::SingleContext, &desc);
}

/// Flush all TLB entries for all PCIDs including globals
///
/// # Safety
///
/// INVPCID must be supported by the CPU.
#[inline]
pub unsafe fn flush_tlb_all_pcid_including_globals() {
    let desc = InvpcidDescriptor::global();
    flush_tlb_pcid(InvpcidType::AllContextsIncludingGlobals, &desc);
}

/// Flush all TLB entries for all PCIDs except globals
///
/// # Safety
///
/// INVPCID must be supported by the CPU.
#[inline]
pub unsafe fn flush_tlb_all_pcid_except_globals() {
    let desc = InvpcidDescriptor::global();
    flush_tlb_pcid(InvpcidType::AllContextsExceptGlobals, &desc);
}

// =============================================================================
// PCID Support Detection
// =============================================================================

/// Check if PCID is supported
#[inline]
pub fn is_pcid_supported() -> bool {
    // CPUID.01H:ECX.PCID[bit 17]
    let ecx: u32;
    unsafe {
        core::arch::asm!(
            "mov eax, 1",
            "cpuid",
            out("ecx") ecx,
            out("eax") _,
            out("ebx") _,
            out("edx") _,
            options(nostack, preserves_flags),
        );
    }
    ecx & (1 << 17) != 0
}

/// Check if INVPCID is supported
#[inline]
pub fn is_invpcid_supported() -> bool {
    // CPUID.07H:EBX.INVPCID[bit 10]
    let ebx: u32;
    unsafe {
        core::arch::asm!(
            "mov eax, 7",
            "xor ecx, ecx",
            "cpuid",
            out("ebx") ebx,
            out("eax") _,
            out("ecx") _,
            out("edx") _,
            options(nostack, preserves_flags),
        );
    }
    ebx & (1 << 10) != 0
}

/// Check if PCID is enabled in CR4
#[inline]
pub fn is_pcid_enabled() -> bool {
    let cr4: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
            options(nomem, nostack, preserves_flags),
        );
    }
    cr4 & (1 << 17) != 0 // CR4.PCIDE
}

/// Enable PCID in CR4
///
/// # Safety
///
/// PCID must be supported and proper precautions must be taken
/// for existing TLB entries.
#[inline]
pub unsafe fn enable_pcid() {
    let mut cr4: u64;
    core::arch::asm!(
        "mov {}, cr4",
        out(reg) cr4,
        options(nomem, nostack, preserves_flags),
    );
    cr4 |= 1 << 17; // CR4.PCIDE
    core::arch::asm!(
        "mov cr4, {}",
        in(reg) cr4,
        options(nostack, preserves_flags),
    );
}

// =============================================================================
// Global Pages Support
// =============================================================================

/// Check if global pages are supported
#[inline]
pub fn is_pge_supported() -> bool {
    // CPUID.01H:EDX.PGE[bit 13]
    let edx: u32;
    unsafe {
        core::arch::asm!(
            "mov eax, 1",
            "cpuid",
            out("edx") edx,
            out("eax") _,
            out("ebx") _,
            out("ecx") _,
            options(nostack, preserves_flags),
        );
    }
    edx & (1 << 13) != 0
}

/// Check if global pages are enabled
#[inline]
pub fn is_pge_enabled() -> bool {
    let cr4: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
            options(nomem, nostack, preserves_flags),
        );
    }
    cr4 & (1 << 7) != 0 // CR4.PGE
}

/// Enable global pages
///
/// # Safety
///
/// PGE must be supported.
#[inline]
pub unsafe fn enable_pge() {
    let mut cr4: u64;
    core::arch::asm!(
        "mov {}, cr4",
        out(reg) cr4,
        options(nomem, nostack, preserves_flags),
    );
    cr4 |= 1 << 7; // CR4.PGE
    core::arch::asm!(
        "mov cr4, {}",
        in(reg) cr4,
        options(nostack, preserves_flags),
    );
}

/// Flush global TLB entries by toggling CR4.PGE
///
/// # Safety
///
/// PGE must be enabled.
#[inline]
pub unsafe fn flush_global_tlb() {
    let mut cr4: u64;
    
    // Read CR4
    core::arch::asm!(
        "mov {}, cr4",
        out(reg) cr4,
        options(nomem, nostack, preserves_flags),
    );
    
    // Clear PGE
    core::arch::asm!(
        "mov cr4, {}",
        in(reg) cr4 & !(1 << 7),
        options(nostack, preserves_flags),
    );
    
    // Set PGE again
    core::arch::asm!(
        "mov cr4, {}",
        in(reg) cr4,
        options(nostack, preserves_flags),
    );
}

// =============================================================================
// TLB Statistics (for debugging/profiling)
// =============================================================================

/// TLB flush counter (for performance monitoring)
#[cfg(feature = "tlb_stats")]
pub mod stats {
    use core::sync::atomic::{AtomicU64, Ordering};
    
    static INVLPG_COUNT: AtomicU64 = AtomicU64::new(0);
    static FULL_FLUSH_COUNT: AtomicU64 = AtomicU64::new(0);
    static INVPCID_COUNT: AtomicU64 = AtomicU64::new(0);
    
    pub fn record_invlpg() {
        INVLPG_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_full_flush() {
        FULL_FLUSH_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn record_invpcid() {
        INVPCID_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_stats() -> (u64, u64, u64) {
        (
            INVLPG_COUNT.load(Ordering::Relaxed),
            FULL_FLUSH_COUNT.load(Ordering::Relaxed),
            INVPCID_COUNT.load(Ordering::Relaxed),
        )
    }
    
    pub fn reset_stats() {
        INVLPG_COUNT.store(0, Ordering::Relaxed);
        FULL_FLUSH_COUNT.store(0, Ordering::Relaxed);
        INVPCID_COUNT.store(0, Ordering::Relaxed);
    }
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    use core::mem::size_of;
    
    // INVPCID descriptor must be 16 bytes
    assert!(size_of::<InvpcidDescriptor>() == 16);
    
    // PCID must fit in 12 bits
    assert!(Pcid::MAX == 0xFFF);
};

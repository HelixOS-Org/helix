//! # AArch64 TLB Operations
//!
//! This module provides TLB (Translation Lookaside Buffer) management
//! operations for AArch64.

use core::arch::asm;
use super::asid::Asid;

// =============================================================================
// TLB Invalidation Operations
// =============================================================================

/// Invalidate entire TLB (all ASIDs)
#[inline]
pub fn tlb_flush_all() {
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate entire TLB (current CPU only)
#[inline]
pub fn tlb_flush_all_local() {
    unsafe {
        asm!(
            "dsb nshst",
            "tlbi vmalle1",
            "dsb nsh",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entries for specific ASID
#[inline]
pub fn tlb_flush_asid(asid: Asid) {
    let asid_val = (asid.value() as u64) << 48;
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi aside1is, {asid}",
            "dsb ish",
            "isb",
            asid = in(reg) asid_val,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entries for specific ASID (current CPU only)
#[inline]
pub fn tlb_flush_asid_local(asid: Asid) {
    let asid_val = (asid.value() as u64) << 48;
    unsafe {
        asm!(
            "dsb nshst",
            "tlbi aside1, {asid}",
            "dsb nsh",
            "isb",
            asid = in(reg) asid_val,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entry for specific page address
#[inline]
pub fn tlb_flush_page(va: u64) {
    // VA must be shifted right by 12 bits
    let va_shifted = va >> 12;
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vaae1is, {va}",
            "dsb ish",
            "isb",
            va = in(reg) va_shifted,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entry for specific page address (current CPU only)
#[inline]
pub fn tlb_flush_page_local(va: u64) {
    let va_shifted = va >> 12;
    unsafe {
        asm!(
            "dsb nshst",
            "tlbi vaae1, {va}",
            "dsb nsh",
            "isb",
            va = in(reg) va_shifted,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entry for specific page and ASID
#[inline]
pub fn tlb_flush_page_asid(va: u64, asid: Asid) {
    // Combine ASID and VA
    let va_shifted = va >> 12;
    let operand = ((asid.value() as u64) << 48) | va_shifted;
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vae1is, {op}",
            "dsb ish",
            "isb",
            op = in(reg) operand,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entry for specific page and ASID (current CPU only)
#[inline]
pub fn tlb_flush_page_asid_local(va: u64, asid: Asid) {
    let va_shifted = va >> 12;
    let operand = ((asid.value() as u64) << 48) | va_shifted;
    unsafe {
        asm!(
            "dsb nshst",
            "tlbi vae1, {op}",
            "dsb nsh",
            "isb",
            op = in(reg) operand,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entries for a range of pages
pub fn tlb_flush_range(start_va: u64, end_va: u64) {
    let page_size = super::PAGE_SIZE as u64;
    let mut va = start_va & !(page_size - 1);

    unsafe {
        asm!("dsb ishst", options(nostack, preserves_flags));
    }

    while va < end_va {
        let va_shifted = va >> 12;
        unsafe {
            asm!(
                "tlbi vaae1is, {va}",
                va = in(reg) va_shifted,
                options(nostack, preserves_flags)
            );
        }
        va += page_size;
    }

    unsafe {
        asm!(
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entries for a range of pages with ASID
pub fn tlb_flush_range_asid(start_va: u64, end_va: u64, asid: Asid) {
    let page_size = super::PAGE_SIZE as u64;
    let mut va = start_va & !(page_size - 1);
    let asid_shifted = (asid.value() as u64) << 48;

    unsafe {
        asm!("dsb ishst", options(nostack, preserves_flags));
    }

    while va < end_va {
        let va_shifted = va >> 12;
        let operand = asid_shifted | va_shifted;
        unsafe {
            asm!(
                "tlbi vae1is, {op}",
                op = in(reg) operand,
                options(nostack, preserves_flags)
            );
        }
        va += page_size;
    }

    unsafe {
        asm!(
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

// =============================================================================
// TLB Range Invalidation (ARMv8.4-TLBI)
// =============================================================================

/// TLB range invalidation (if supported by CPU)
#[cfg(feature = "tlbi_range")]
pub mod range {
    use super::*;

    /// Range scale (determines range size)
    #[derive(Debug, Clone, Copy)]
    #[repr(u8)]
    pub enum RangeScale {
        /// 1 entry
        Scale1 = 0,
        /// 2 entries
        Scale2 = 1,
        /// 4 entries
        Scale4 = 2,
        /// 8 entries
        Scale8 = 3,
    }

    /// Invalidate TLB range (ARMv8.4+)
    #[inline]
    pub fn tlb_flush_range_v84(base_va: u64, num_pages: u64, asid: Asid) {
        let operand = encode_range_operand(base_va, num_pages, asid);
        unsafe {
            asm!(
                "dsb ishst",
                "tlbi rvae1is, {op}",
                "dsb ish",
                "isb",
                op = in(reg) operand,
                options(nostack, preserves_flags)
            );
        }
    }

    /// Encode range operand for TLBI range instructions
    fn encode_range_operand(base_va: u64, num_pages: u64, asid: Asid) -> u64 {
        // BaseADDR [36:0] = VA[48:12]
        // Scale [38:37]
        // Num [43:39]
        // ASID [63:48]
        let base = (base_va >> 12) & 0x1FF_FFFF_FFFF;
        let (scale, num) = compute_scale_num(num_pages);

        base | ((scale as u64) << 37) | ((num as u64) << 39) | ((asid.value() as u64) << 48)
    }

    /// Compute optimal scale and num for given number of pages
    fn compute_scale_num(num_pages: u64) -> (u8, u8) {
        // Simplified: just use scale 1 for now
        let num = (num_pages - 1).min(31) as u8;
        (0, num)
    }
}

// =============================================================================
// EL2 TLB Operations
// =============================================================================

/// Invalidate all EL2 TLB entries
#[inline]
pub fn tlb_flush_all_el2() {
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi alle2is",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate stage 2 TLB entries for VMID
#[inline]
pub fn tlb_flush_vmid(vmid: u16) {
    let vmid_val = (vmid as u64) << 48;
    unsafe {
        asm!(
            "dsb ishst",
            "tlbi vmalls12e1is, {vmid}",
            "dsb ish",
            "isb",
            vmid = in(reg) vmid_val,
            options(nostack, preserves_flags)
        );
    }
}

// =============================================================================
// Barrier Helpers
// =============================================================================

/// Ensure all prior TLB operations are complete
#[inline]
pub fn tlb_sync() {
    unsafe {
        asm!(
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Local TLB sync (current CPU only)
#[inline]
pub fn tlb_sync_local() {
    unsafe {
        asm!(
            "dsb nsh",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

// =============================================================================
// TLB Statistics (for debugging)
// =============================================================================

/// TLB operation statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct TlbStats {
    /// Number of full TLB flushes
    pub full_flushes: u64,
    /// Number of page flushes
    pub page_flushes: u64,
    /// Number of ASID flushes
    pub asid_flushes: u64,
    /// Number of range flushes
    pub range_flushes: u64,
}

impl TlbStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            full_flushes: 0,
            page_flushes: 0,
            asid_flushes: 0,
            range_flushes: 0,
        }
    }
}

// =============================================================================
// High-Level API
// =============================================================================

/// TLB manager for coordinating invalidations
pub struct TlbManager {
    stats: TlbStats,
}

impl TlbManager {
    /// Create a new TLB manager
    pub const fn new() -> Self {
        Self {
            stats: TlbStats::new(),
        }
    }

    /// Flush entire TLB
    pub fn flush_all(&mut self) {
        tlb_flush_all();
        self.stats.full_flushes += 1;
    }

    /// Flush TLB for specific ASID
    pub fn flush_asid(&mut self, asid: Asid) {
        tlb_flush_asid(asid);
        self.stats.asid_flushes += 1;
    }

    /// Flush TLB for specific page
    pub fn flush_page(&mut self, va: u64) {
        tlb_flush_page(va);
        self.stats.page_flushes += 1;
    }

    /// Flush TLB for page with ASID
    pub fn flush_page_with_asid(&mut self, va: u64, asid: Asid) {
        tlb_flush_page_asid(va, asid);
        self.stats.page_flushes += 1;
    }

    /// Flush TLB for range
    pub fn flush_range(&mut self, start: u64, end: u64) {
        tlb_flush_range(start, end);
        self.stats.range_flushes += 1;
    }

    /// Get statistics
    pub fn stats(&self) -> &TlbStats {
        &self.stats
    }
}

impl Default for TlbManager {
    fn default() -> Self {
        Self::new()
    }
}

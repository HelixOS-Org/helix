//! # RISC-V TLB Operations
//!
//! This module provides TLB (Translation Lookaside Buffer) management
//! using SFENCE.VMA and related instructions.
//!
//! ## SFENCE.VMA Variants
//!
//! ```text
//! SFENCE.VMA x0, x0     - Flush entire TLB
//! SFENCE.VMA rs1, x0    - Flush TLB entries for address in rs1 (all ASIDs)
//! SFENCE.VMA x0, rs2    - Flush all entries for ASID in rs2
//! SFENCE.VMA rs1, rs2   - Flush entry for specific (address, ASID)
//! ```
//!
//! ## Svinval Extension
//!
//! The Svinval extension provides batched TLB invalidation:
//! - `SFENCE.W.INVAL`: Wait for writes to complete before invalidations
//! - `SINVAL.VMA`: Queue a TLB invalidation
//! - `SFENCE.INVAL.IR`: Complete queued invalidations

use super::super::core::barriers;

// ============================================================================
// Basic TLB Flush Operations
// ============================================================================

/// Flush entire TLB
///
/// Invalidates all TLB entries for all address spaces.
/// This is the most expensive but safest option.
#[inline]
pub fn flush_tlb_all() {
    barriers::sfence_vma_all();
}

/// Alias for flush_tlb_all
#[inline]
pub fn flush_tlb() {
    flush_tlb_all();
}

/// Flush TLB entries for a specific virtual address
///
/// Invalidates TLB entries matching the given address in all ASIDs.
/// Use this after unmapping or changing permissions on a page.
#[inline]
pub fn flush_tlb_addr(vaddr: usize) {
    barriers::sfence_vma_addr(vaddr);
}

/// Flush TLB entries for a specific ASID
///
/// Invalidates all TLB entries for the given ASID.
/// Use this when switching away from an address space.
#[inline]
pub fn flush_tlb_asid(asid: u16) {
    barriers::sfence_vma_asid(asid);
}

/// Flush TLB entry for a specific (address, ASID) pair
///
/// Most fine-grained TLB invalidation.
/// Use this for targeted invalidation in a specific address space.
#[inline]
pub fn flush_tlb_addr_asid(vaddr: usize, asid: u16) {
    barriers::sfence_vma(vaddr, asid);
}

// ============================================================================
// Range-Based TLB Flush
// ============================================================================

/// Flush TLB entries for an address range
///
/// Flushes all TLB entries for pages in the given range.
pub fn flush_tlb_range(start: usize, size: usize) {
    const PAGE_SIZE: usize = 4096;

    // For small ranges, invalidate individual pages
    // For large ranges, just flush everything
    let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

    if pages <= 32 {
        let mut addr = start & !(PAGE_SIZE - 1);
        let end = start + size;
        while addr < end {
            flush_tlb_addr(addr);
            addr += PAGE_SIZE;
        }
    } else {
        // Too many pages, just flush everything
        flush_tlb_all();
    }
}

/// Flush TLB entries for an address range with specific ASID
pub fn flush_tlb_range_asid(start: usize, size: usize, asid: u16) {
    const PAGE_SIZE: usize = 4096;

    let pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

    if pages <= 32 {
        let mut addr = start & !(PAGE_SIZE - 1);
        let end = start + size;
        while addr < end {
            flush_tlb_addr_asid(addr, asid);
            addr += PAGE_SIZE;
        }
    } else {
        // Too many pages, flush entire ASID
        flush_tlb_asid(asid);
    }
}

// ============================================================================
// Batched TLB Invalidation (Svinval Extension)
// ============================================================================

/// TLB invalidation batch
///
/// Collects multiple TLB invalidations and executes them efficiently
/// using the Svinval extension if available.
pub struct TlbFlushBatch {
    /// Addresses to flush
    addrs: [usize; Self::MAX_ENTRIES],
    /// ASIDs for each address
    asids: [u16; Self::MAX_ENTRIES],
    /// Number of entries
    count: usize,
    /// ASID for all entries (if uniform)
    uniform_asid: Option<u16>,
    /// Whether to use Svinval extension
    use_svinval: bool,
}

impl TlbFlushBatch {
    /// Maximum entries before falling back to full flush
    const MAX_ENTRIES: usize = 64;

    /// Create a new batch
    pub const fn new() -> Self {
        Self {
            addrs: [0; Self::MAX_ENTRIES],
            asids: [0; Self::MAX_ENTRIES],
            count: 0,
            uniform_asid: None,
            use_svinval: false,
        }
    }

    /// Create a batch for a specific ASID
    pub const fn for_asid(asid: u16) -> Self {
        Self {
            addrs: [0; Self::MAX_ENTRIES],
            asids: [0; Self::MAX_ENTRIES],
            count: 0,
            uniform_asid: Some(asid),
            use_svinval: false,
        }
    }

    /// Add an address to flush
    pub fn add(&mut self, vaddr: usize) {
        if self.count < Self::MAX_ENTRIES {
            self.addrs[self.count] = vaddr;
            self.asids[self.count] = self.uniform_asid.unwrap_or(0);
            self.count += 1;
        }
    }

    /// Add an address with specific ASID
    pub fn add_with_asid(&mut self, vaddr: usize, asid: u16) {
        if self.count < Self::MAX_ENTRIES {
            self.addrs[self.count] = vaddr;
            self.asids[self.count] = asid;
            self.count += 1;
        }
    }

    /// Check if the batch is full
    pub fn is_full(&self) -> bool {
        self.count >= Self::MAX_ENTRIES
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clear the batch
    pub fn clear(&mut self) {
        self.count = 0;
    }

    /// Execute the batch
    pub fn flush(&self) {
        if self.count == 0 {
            return;
        }

        if self.count >= Self::MAX_ENTRIES / 2 {
            // Too many entries, just do a full flush
            if let Some(asid) = self.uniform_asid {
                flush_tlb_asid(asid);
            } else {
                flush_tlb_all();
            }
            return;
        }

        #[cfg(feature = "svinval")]
        if self.use_svinval {
            self.flush_with_svinval();
            return;
        }

        // Standard SFENCE.VMA for each entry
        for i in 0..self.count {
            if let Some(asid) = self.uniform_asid {
                flush_tlb_addr_asid(self.addrs[i], asid);
            } else {
                flush_tlb_addr_asid(self.addrs[i], self.asids[i]);
            }
        }
    }

    /// Flush using Svinval extension
    #[cfg(feature = "svinval")]
    fn flush_with_svinval(&self) {
        // Begin batch
        barriers::sinval_vma_begin();

        // Queue all invalidations
        for i in 0..self.count {
            let asid = self.uniform_asid.unwrap_or(self.asids[i]);
            barriers::sinval_vma(self.addrs[i], asid);
        }

        // Complete batch
        barriers::sinval_vma_end();
    }
}

impl Default for TlbFlushBatch {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TLB Shootdown (Multi-Hart)
// ============================================================================

/// TLB shootdown request for SMP systems
#[derive(Debug, Clone, Copy)]
pub struct TlbShootdownRequest {
    /// Type of shootdown
    pub kind: TlbShootdownKind,
    /// ASID (if applicable)
    pub asid: u16,
    /// Start address (if range)
    pub start: usize,
    /// Size (if range)
    pub size: usize,
}

/// Types of TLB shootdown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbShootdownKind {
    /// Flush all TLB entries
    All,
    /// Flush entries for an ASID
    Asid,
    /// Flush entries for an address
    Address,
    /// Flush entries for an address range
    Range,
    /// Flush entries for an address range in an ASID
    RangeAsid,
}

impl TlbShootdownRequest {
    /// Create request to flush all TLBs
    pub const fn all() -> Self {
        Self {
            kind: TlbShootdownKind::All,
            asid: 0,
            start: 0,
            size: 0,
        }
    }

    /// Create request to flush an ASID
    pub const fn asid(asid: u16) -> Self {
        Self {
            kind: TlbShootdownKind::Asid,
            asid,
            start: 0,
            size: 0,
        }
    }

    /// Create request to flush an address
    pub const fn address(addr: usize) -> Self {
        Self {
            kind: TlbShootdownKind::Address,
            asid: 0,
            start: addr,
            size: 0,
        }
    }

    /// Create request to flush an address range
    pub const fn range(start: usize, size: usize) -> Self {
        Self {
            kind: TlbShootdownKind::Range,
            asid: 0,
            start,
            size,
        }
    }

    /// Create request to flush an address range in an ASID
    pub const fn range_asid(start: usize, size: usize, asid: u16) -> Self {
        Self {
            kind: TlbShootdownKind::RangeAsid,
            asid,
            start,
            size,
        }
    }

    /// Execute this request locally
    pub fn execute_local(&self) {
        match self.kind {
            TlbShootdownKind::All => flush_tlb_all(),
            TlbShootdownKind::Asid => flush_tlb_asid(self.asid),
            TlbShootdownKind::Address => flush_tlb_addr(self.start),
            TlbShootdownKind::Range => flush_tlb_range(self.start, self.size),
            TlbShootdownKind::RangeAsid => {
                flush_tlb_range_asid(self.start, self.size, self.asid)
            }
        }
    }
}

// ============================================================================
// Lazy TLB Flushing
// ============================================================================

/// Pending TLB flush state for lazy flushing
#[derive(Debug, Clone)]
pub struct PendingTlbFlush {
    /// Whether a flush is pending
    pub pending: bool,
    /// Type of pending flush
    pub kind: TlbShootdownKind,
    /// ASID for pending flush
    pub asid: u16,
}

impl PendingTlbFlush {
    /// Create new pending flush state
    pub const fn new() -> Self {
        Self {
            pending: false,
            kind: TlbShootdownKind::All,
            asid: 0,
        }
    }

    /// Mark a flush as pending
    pub fn mark_pending(&mut self, kind: TlbShootdownKind, asid: u16) {
        if self.pending {
            // Merge with existing pending flush
            match (self.kind, kind) {
                // If either is All, result is All
                (TlbShootdownKind::All, _) | (_, TlbShootdownKind::All) => {
                    self.kind = TlbShootdownKind::All;
                    self.asid = 0;
                }
                // Same type and ASID
                (a, b) if a == b && self.asid == asid => {}
                // Different - escalate to full flush
                _ => {
                    self.kind = TlbShootdownKind::All;
                    self.asid = 0;
                }
            }
        } else {
            self.pending = true;
            self.kind = kind;
            self.asid = asid;
        }
    }

    /// Execute pending flush if any
    pub fn execute_pending(&mut self) {
        if self.pending {
            match self.kind {
                TlbShootdownKind::All => flush_tlb_all(),
                TlbShootdownKind::Asid => flush_tlb_asid(self.asid),
                _ => flush_tlb_all(),
            }
            self.pending = false;
        }
    }

    /// Check if flush is pending
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// Clear pending flush without executing
    pub fn clear(&mut self) {
        self.pending = false;
    }
}

impl Default for PendingTlbFlush {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TLB Flush Coalescing
// ============================================================================

/// Coalesce multiple TLB flush requests
pub struct TlbFlushCoalescer {
    /// Minimum number of pages before switching to ASID flush
    page_threshold: usize,
    /// Current pending pages for each ASID
    pending_pages: usize,
    /// Current ASID
    current_asid: u16,
}

impl TlbFlushCoalescer {
    /// Create new coalescer with threshold
    pub const fn new(page_threshold: usize) -> Self {
        Self {
            page_threshold,
            pending_pages: 0,
            current_asid: 0,
        }
    }

    /// Record a page flush
    pub fn record_page(&mut self, asid: u16) {
        if self.current_asid != asid {
            self.flush_if_needed();
            self.current_asid = asid;
            self.pending_pages = 0;
        }
        self.pending_pages += 1;
    }

    /// Check if we should flush and do so if needed
    fn flush_if_needed(&mut self) {
        if self.pending_pages >= self.page_threshold {
            flush_tlb_asid(self.current_asid);
        }
        self.pending_pages = 0;
    }

    /// Finish and perform any remaining flushes
    pub fn finish(&mut self) {
        if self.pending_pages >= self.page_threshold {
            flush_tlb_asid(self.current_asid);
        }
        self.pending_pages = 0;
    }
}

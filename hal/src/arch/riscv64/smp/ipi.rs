//! # Inter-Processor Interrupts (IPI)
//!
//! IPI support for RISC-V multi-hart systems.
//!
//! On RISC-V, IPIs are sent using:
//! - SBI spi extension (preferred for S-mode)
//! - Direct CLINT access (if available from S-mode)

use core::sync::atomic::{AtomicU64, Ordering};

use super::{MAX_HARTS, HartMask, handle_cross_call};
use super::hartid::get_hart_id;
use super::percpu::PerCpu;

// ============================================================================
// IPI Types
// ============================================================================

/// Types of IPIs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IpiType {
    /// Generic wakeup
    Wakeup = 0,
    /// Reschedule request
    Reschedule = 1,
    /// Function call
    FunctionCall = 2,
    /// TLB shootdown
    TlbShootdown = 3,
    /// Stop the hart
    Stop = 4,
    /// Custom IPI (user-defined)
    Custom(u8) = 5,
}

impl IpiType {
    /// Get the bit for this IPI type
    pub const fn bit(self) -> u64 {
        match self {
            Self::Wakeup => 1 << 0,
            Self::Reschedule => 1 << 1,
            Self::FunctionCall => 1 << 2,
            Self::TlbShootdown => 1 << 3,
            Self::Stop => 1 << 4,
            Self::Custom(n) => 1 << (8 + n as u64),
        }
    }
}

// ============================================================================
// SBI IPI Interface
// ============================================================================

/// SBI extension IDs
mod sbi {
    /// IPI extension ID
    pub const IPI_EID: usize = 0x735049;

    /// RFENCE extension ID (for TLB shootdowns)
    pub const RFENCE_EID: usize = 0x52464E43;

    /// RFENCE function IDs
    pub mod rfence {
        pub const REMOTE_FENCE_I: usize = 0;
        pub const REMOTE_SFENCE_VMA: usize = 1;
        pub const REMOTE_SFENCE_VMA_ASID: usize = 2;
        pub const REMOTE_HFENCE_GVMA_VMID: usize = 3;
        pub const REMOTE_HFENCE_GVMA: usize = 4;
        pub const REMOTE_HFENCE_VVMA_ASID: usize = 5;
        pub const REMOTE_HFENCE_VVMA: usize = 6;
    }
}

/// SBI return structure
#[derive(Debug, Clone, Copy)]
struct SbiRet {
    error: i64,
    value: i64,
}

impl SbiRet {
    fn is_success(&self) -> bool {
        self.error == 0
    }
}

/// Make an SBI call
#[inline]
fn sbi_call_2(eid: usize, fid: usize, arg0: usize, arg1: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

/// Make an SBI call with 4 arguments
#[inline]
fn sbi_call_4(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a3") arg3,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

// ============================================================================
// IPI Pending Flags
// ============================================================================

/// IPI pending flags per hart
static IPI_PENDING: [AtomicU64; MAX_HARTS] = {
    const ZERO: AtomicU64 = AtomicU64::new(0);
    [ZERO; MAX_HARTS]
};

/// Set an IPI as pending for a hart
fn set_ipi_pending(hart_id: usize, ipi_type: IpiType) {
    if hart_id < MAX_HARTS {
        IPI_PENDING[hart_id].fetch_or(ipi_type.bit(), Ordering::Release);
    }
}

/// Clear an IPI pending flag
fn clear_ipi_pending(hart_id: usize, ipi_type: IpiType) {
    if hart_id < MAX_HARTS {
        IPI_PENDING[hart_id].fetch_and(!ipi_type.bit(), Ordering::Release);
    }
}

/// Get and clear all pending IPIs
fn get_and_clear_pending(hart_id: usize) -> u64 {
    if hart_id < MAX_HARTS {
        IPI_PENDING[hart_id].swap(0, Ordering::AcqRel)
    } else {
        0
    }
}

/// Check if any IPI is pending
pub fn is_ipi_pending(hart_id: usize) -> bool {
    if hart_id < MAX_HARTS {
        IPI_PENDING[hart_id].load(Ordering::Acquire) != 0
    } else {
        false
    }
}

// ============================================================================
// IPI Sending
// ============================================================================

/// Send an IPI to a specific hart
pub fn send_ipi(target_hart: usize, ipi_type: IpiType) {
    // Set pending flag
    set_ipi_pending(target_hart, ipi_type);

    // Send via SBI
    let mask = HartMask::single(target_hart);
    send_ipi_raw(mask.low, mask.base);
}

/// Send an IPI to multiple harts
pub fn send_ipi_multi(mask: HartMask, ipi_type: IpiType) {
    // Set pending flags
    for hart in 0..128 {
        if mask.contains(hart) {
            set_ipi_pending(hart, ipi_type);
        }
    }

    // Send via SBI
    send_ipi_raw(mask.low, mask.base);
    if mask.high != 0 {
        send_ipi_raw(mask.high, 64);
    }
}

/// Broadcast an IPI to all other harts
pub fn broadcast_ipi(ipi_type: IpiType) {
    let current = get_hart_id();
    let mask = HartMask::all_except(current);
    send_ipi_multi(mask, ipi_type);
}

/// Send raw IPI via SBI
fn send_ipi_raw(hart_mask: u64, hart_mask_base: usize) {
    let _ = sbi_call_2(
        sbi::IPI_EID,
        0, // FID = 0 for send_ipi
        hart_mask as usize,
        hart_mask_base,
    );
}

// ============================================================================
// IPI Handling
// ============================================================================

/// Handle incoming IPIs
///
/// Called from the software interrupt handler.
/// Returns true if rescheduling is needed.
pub fn handle_ipi(hart_id: usize) -> bool {
    let pending = get_and_clear_pending(hart_id);
    let mut need_resched = false;

    if pending & IpiType::Wakeup.bit() != 0 {
        // Just a wakeup, no action needed
    }

    if pending & IpiType::Reschedule.bit() != 0 {
        need_resched = true;
    }

    if pending & IpiType::FunctionCall.bit() != 0 {
        handle_cross_call(hart_id);
    }

    if pending & IpiType::TlbShootdown.bit() != 0 {
        handle_tlb_shootdown(hart_id);
    }

    if pending & IpiType::Stop.bit() != 0 {
        super::startup::stop_current_hart();
    }

    // Handle custom IPIs
    for i in 0..8 {
        if pending & (1 << (8 + i)) != 0 {
            handle_custom_ipi(hart_id, i as u8);
        }
    }

    need_resched
}

/// Custom IPI handler type
pub type CustomIpiHandler = fn(hart_id: usize, custom_id: u8);

/// Custom IPI handlers
static mut CUSTOM_IPI_HANDLERS: [Option<CustomIpiHandler>; 8] = [None; 8];

/// Register a custom IPI handler
///
/// # Safety
/// Handler must be thread-safe.
pub unsafe fn register_custom_ipi_handler(id: u8, handler: CustomIpiHandler) -> bool {
    if id >= 8 {
        return false;
    }
    CUSTOM_IPI_HANDLERS[id as usize] = Some(handler);
    true
}

/// Handle a custom IPI
fn handle_custom_ipi(hart_id: usize, custom_id: u8) {
    if custom_id < 8 {
        unsafe {
            if let Some(handler) = CUSTOM_IPI_HANDLERS[custom_id as usize] {
                handler(hart_id, custom_id);
            }
        }
    }
}

// ============================================================================
// TLB Shootdown
// ============================================================================

/// TLB shootdown request
#[derive(Debug, Clone, Copy)]
pub struct TlbShootdownRequest {
    /// Start address (0 for all)
    pub start_addr: usize,
    /// Size in bytes (0 for all)
    pub size: usize,
    /// ASID (u16::MAX for all ASIDs)
    pub asid: u16,
}

impl TlbShootdownRequest {
    /// Shootdown all entries
    pub const fn all() -> Self {
        Self {
            start_addr: 0,
            size: 0,
            asid: u16::MAX,
        }
    }

    /// Shootdown a specific range
    pub const fn range(start: usize, size: usize) -> Self {
        Self {
            start_addr: start,
            size,
            asid: u16::MAX,
        }
    }

    /// Shootdown a specific ASID
    pub const fn asid_only(asid: u16) -> Self {
        Self {
            start_addr: 0,
            size: 0,
            asid,
        }
    }

    /// Shootdown a range within an ASID
    pub const fn range_asid(start: usize, size: usize, asid: u16) -> Self {
        Self {
            start_addr: start,
            size,
            asid,
        }
    }
}

/// Pending TLB shootdown requests per hart
static mut PENDING_TLB_SHOOTDOWNS: [Option<TlbShootdownRequest>; MAX_HARTS] = [None; MAX_HARTS];

/// Request a TLB shootdown on remote harts
pub fn request_tlb_shootdown(mask: HartMask, request: TlbShootdownRequest) {
    let current = get_hart_id();

    // Use SBI RFENCE for remote TLB invalidation
    if request.size == 0 && request.asid == u16::MAX {
        // Full shootdown via RFENCE
        remote_sfence_vma(mask, 0, usize::MAX);
    } else if request.asid != u16::MAX {
        // ASID-specific shootdown
        remote_sfence_vma_asid(mask, request.start_addr, request.size, request.asid);
    } else {
        // Range shootdown
        remote_sfence_vma(mask, request.start_addr, request.size);
    }
}

/// Request TLB shootdown on all other harts
pub fn broadcast_tlb_shootdown(request: TlbShootdownRequest) {
    let current = get_hart_id();
    let mask = HartMask::all_except(current);
    request_tlb_shootdown(mask, request);
}

/// Handle a TLB shootdown request
fn handle_tlb_shootdown(hart_id: usize) {
    unsafe {
        if let Some(req) = PENDING_TLB_SHOOTDOWNS[hart_id].take() {
            if req.size == 0 && req.asid == u16::MAX {
                // Full flush
                core::arch::asm!("sfence.vma", options(nostack));
            } else if req.asid != u16::MAX {
                // ASID-specific flush
                let asid = req.asid as usize;
                if req.size == 0 {
                    core::arch::asm!(
                        "sfence.vma zero, {asid}",
                        asid = in(reg) asid,
                        options(nostack)
                    );
                } else {
                    // Range with ASID
                    let end = req.start_addr.saturating_add(req.size);
                    let mut addr = req.start_addr;
                    while addr < end {
                        core::arch::asm!(
                            "sfence.vma {addr}, {asid}",
                            addr = in(reg) addr,
                            asid = in(reg) asid,
                            options(nostack)
                        );
                        addr += 4096;
                    }
                }
            } else {
                // Range flush (all ASIDs)
                let end = req.start_addr.saturating_add(req.size);
                let mut addr = req.start_addr;
                while addr < end {
                    core::arch::asm!(
                        "sfence.vma {addr}, zero",
                        addr = in(reg) addr,
                        options(nostack)
                    );
                    addr += 4096;
                }
            }
        }
    }
}

// ============================================================================
// SBI RFENCE Interface
// ============================================================================

/// Remote SFENCE.VMA via SBI
pub fn remote_sfence_vma(mask: HartMask, start_addr: usize, size: usize) {
    let _ = sbi_call_4(
        sbi::RFENCE_EID,
        sbi::rfence::REMOTE_SFENCE_VMA,
        mask.low as usize,
        mask.base,
        start_addr,
        size,
    );
}

/// Remote SFENCE.VMA with ASID via SBI
pub fn remote_sfence_vma_asid(mask: HartMask, start_addr: usize, size: usize, asid: u16) {
    // SBI rfence_sfence_vma_asid takes 5 arguments
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") sbi::RFENCE_EID,
            in("a6") sbi::rfence::REMOTE_SFENCE_VMA_ASID,
            in("a0") mask.low as usize,
            in("a1") mask.base,
            in("a2") start_addr,
            in("a3") size,
            in("a4") asid as usize,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }
}

/// Remote FENCE.I via SBI
pub fn remote_fence_i(mask: HartMask) {
    let _ = sbi_call_2(
        sbi::RFENCE_EID,
        sbi::rfence::REMOTE_FENCE_I,
        mask.low as usize,
        mask.base,
    );
}

// ============================================================================
// IPI Statistics
// ============================================================================

/// IPI statistics
#[derive(Debug, Clone, Default)]
pub struct IpiStats {
    /// IPIs sent
    pub sent: u64,
    /// IPIs received
    pub received: u64,
    /// Reschedule IPIs
    pub reschedule: u64,
    /// Function call IPIs
    pub function_call: u64,
    /// TLB shootdown IPIs
    pub tlb_shootdown: u64,
}

/// Per-hart IPI statistics
static mut IPI_STATS: [IpiStats; MAX_HARTS] = {
    const INIT: IpiStats = IpiStats {
        sent: 0,
        received: 0,
        reschedule: 0,
        function_call: 0,
        tlb_shootdown: 0,
    };
    [INIT; MAX_HARTS]
};

/// Get IPI statistics for a hart
pub fn get_ipi_stats(hart_id: usize) -> Option<IpiStats> {
    if hart_id < MAX_HARTS {
        Some(unsafe { IPI_STATS[hart_id].clone() })
    } else {
        None
    }
}

/// Update sent counter
pub(crate) fn record_ipi_sent(hart_id: usize) {
    if hart_id < MAX_HARTS {
        unsafe { IPI_STATS[hart_id].sent += 1 };
    }
}

/// Update received counter
pub(crate) fn record_ipi_received(hart_id: usize) {
    if hart_id < MAX_HARTS {
        unsafe { IPI_STATS[hart_id].received += 1 };
    }
}

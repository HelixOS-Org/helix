//! # AArch64 Early Boot Implementation
//!
//! Complete early boot sequence for ARM64 architecture.
//! Supports exception levels, MMU setup, GIC, and generic timer.

pub mod cpu;
pub mod gic;
pub mod mmu;
pub mod psci;
pub mod serial;
pub mod timer;

use crate::core::BootContext;
use crate::error::{BootError, BootResult};

// =============================================================================
// SYSTEM REGISTERS
// =============================================================================

/// Current Exception Level
pub const CURRENT_EL: u32 = 0b11_000_0100_0010_010;
/// Saved Program Status Register (EL1)
pub const SPSR_EL1: u32 = 0b11_000_0100_0000_000;
/// Exception Link Register (EL1)
pub const ELR_EL1: u32 = 0b11_000_0100_0000_001;
/// System Control Register (EL1)
pub const SCTLR_EL1: u32 = 0b11_000_0001_0000_000;
/// Translation Control Register (EL1)
pub const TCR_EL1: u32 = 0b11_000_0010_0000_010;
/// Translation Table Base Register 0 (EL1)
pub const TTBR0_EL1: u32 = 0b11_000_0010_0000_000;
/// Translation Table Base Register 1 (EL1)
pub const TTBR1_EL1: u32 = 0b11_000_0010_0000_001;
/// Memory Attribute Indirection Register (EL1)
pub const MAIR_EL1: u32 = 0b11_000_1010_0010_000;
/// Vector Base Address Register (EL1)
pub const VBAR_EL1: u32 = 0b11_000_1100_0000_000;

/// Multiprocessor Affinity Register
pub const MPIDR_EL1: u32 = 0b11_000_0000_0000_101;
/// Main ID Register
pub const MIDR_EL1: u32 = 0b11_000_0000_0000_000;
/// Processor Feature Register 0
pub const ID_AA64PFR0_EL1: u32 = 0b11_000_0000_0100_000;
/// Memory Model Feature Register 0
pub const ID_AA64MMFR0_EL1: u32 = 0b11_000_0000_0111_000;
/// Instruction Set Attribute Register 0
pub const ID_AA64ISAR0_EL1: u32 = 0b11_000_0000_0110_000;

/// Counter-timer Physical Count
pub const CNTPCT_EL0: u32 = 0b11_011_1110_0000_001;
/// Counter-timer Frequency
pub const CNTFRQ_EL0: u32 = 0b11_011_1110_0000_000;
/// Counter-timer Physical Timer Control
pub const CNTP_CTL_EL0: u32 = 0b11_011_1110_0010_001;
/// Counter-timer Physical Timer Compare Value
pub const CNTP_CVAL_EL0: u32 = 0b11_011_1110_0010_010;
/// Counter-timer Physical Timer Value
pub const CNTP_TVAL_EL0: u32 = 0b11_011_1110_0010_000;

// =============================================================================
// SCTLR FLAGS
// =============================================================================

/// MMU enable
pub const SCTLR_M: u64 = 1 << 0;
/// Alignment check enable
pub const SCTLR_A: u64 = 1 << 1;
/// Data cache enable
pub const SCTLR_C: u64 = 1 << 2;
/// Stack alignment check enable
pub const SCTLR_SA: u64 = 1 << 3;
/// Stack alignment check enable for EL0
pub const SCTLR_SA0: u64 = 1 << 4;
/// Instruction cache enable
pub const SCTLR_I: u64 = 1 << 12;
/// Write permission implies XN
pub const SCTLR_WXN: u64 = 1 << 19;
/// Exception entry is context synchronizing
pub const SCTLR_EOS: u64 = 1 << 11;
/// No trap on DC ZVA
pub const SCTLR_DZE: u64 = 1 << 14;
/// User Cache Type register access
pub const SCTLR_UCT: u64 = 1 << 15;
/// No trap on CNTP
pub const SCTLR_NTWE: u64 = 1 << 18;

// =============================================================================
// TCR FLAGS (Translation Control Register)
// =============================================================================

/// Size offset of memory region addressed by TTBR0
pub const TCR_T0SZ_SHIFT: u64 = 0;
/// Size offset of memory region addressed by TTBR1
pub const TCR_T1SZ_SHIFT: u64 = 16;
/// Inner cacheability for TTBR0
pub const TCR_IRGN0_SHIFT: u64 = 8;
/// Outer cacheability for TTBR0
pub const TCR_ORGN0_SHIFT: u64 = 10;
/// Shareability for TTBR0
pub const TCR_SH0_SHIFT: u64 = 12;
/// Granule size for TTBR0
pub const TCR_TG0_SHIFT: u64 = 14;
/// Granule size for TTBR1
pub const TCR_TG1_SHIFT: u64 = 30;
/// Physical address size
pub const TCR_IPS_SHIFT: u64 = 32;
/// ASID size
pub const TCR_AS: u64 = 1 << 36;
/// Top byte ignored for TTBR0
pub const TCR_TBI0: u64 = 1 << 37;
/// Top byte ignored for TTBR1
pub const TCR_TBI1: u64 = 1 << 38;

/// 4KB granule
pub const TCR_TG0_4KB: u64 = 0 << 14;
/// 16KB granule
pub const TCR_TG0_16KB: u64 = 2 << 14;
/// 64KB granule
pub const TCR_TG0_64KB: u64 = 1 << 14;

/// 4KB granule for TTBR1
pub const TCR_TG1_4KB: u64 = 2 << 30;
/// 16KB granule for TTBR1
pub const TCR_TG1_16KB: u64 = 1 << 30;
/// 64KB granule for TTBR1
pub const TCR_TG1_64KB: u64 = 3 << 30;

/// Non-shareable
pub const TCR_SH_NON: u64 = 0;
/// Outer shareable
pub const TCR_SH_OUTER: u64 = 2;
/// Inner shareable
pub const TCR_SH_INNER: u64 = 3;

/// Write-back, read-allocate, write-allocate
pub const TCR_RGN_WB_WA: u64 = 1;
/// Write-through
pub const TCR_RGN_WT: u64 = 2;
/// Write-back, read-allocate
pub const TCR_RGN_WB_RA: u64 = 3;

// =============================================================================
// MAIR DEFINITIONS
// =============================================================================

/// Device-nGnRnE memory
pub const MAIR_DEVICE_nGnRnE: u64 = 0x00;
/// Device-nGnRE memory
pub const MAIR_DEVICE_nGnRE: u64 = 0x04;
/// Device-GRE memory
pub const MAIR_DEVICE_GRE: u64 = 0x0C;
/// Normal non-cacheable
pub const MAIR_NORMAL_NC: u64 = 0x44;
/// Normal write-through
pub const MAIR_NORMAL_WT: u64 = 0xBB;
/// Normal write-back
pub const MAIR_NORMAL_WB: u64 = 0xFF;

/// MAIR index for device memory
pub const MAIR_IDX_DEVICE: u64 = 0;
/// MAIR index for normal non-cacheable
pub const MAIR_IDX_NORMAL_NC: u64 = 1;
/// MAIR index for normal cacheable
pub const MAIR_IDX_NORMAL: u64 = 2;

// =============================================================================
// EXCEPTION LEVELS
// =============================================================================

/// Exception level 0 (user)
pub const EL0: u64 = 0;
/// Exception level 1 (kernel)
pub const EL1: u64 = 1;
/// Exception level 2 (hypervisor)
pub const EL2: u64 = 2;
/// Exception level 3 (secure monitor)
pub const EL3: u64 = 3;

// =============================================================================
// PAGE SIZES
// =============================================================================

/// 4KB page size
pub const PAGE_SIZE_4K: u64 = 0x1000;
/// 16KB page size
pub const PAGE_SIZE_16K: u64 = 0x4000;
/// 64KB page size
pub const PAGE_SIZE_64K: u64 = 0x10000;
/// 2MB block (for 4KB granule)
pub const BLOCK_SIZE_2M: u64 = 0x200000;
/// 1GB block (for 4KB granule)
pub const BLOCK_SIZE_1G: u64 = 0x40000000;

// =============================================================================
// SYSTEM REGISTER ACCESS
// =============================================================================

/// Read a system register
#[macro_export]
macro_rules! read_sysreg {
    ($reg:expr) => {{
        let value: u64;
        unsafe {
            core::arch::asm!(
                concat!("mrs {}, ", stringify!($reg)),
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }};
}

/// Write a system register
#[macro_export]
macro_rules! write_sysreg {
    ($reg:expr, $value:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("msr ", stringify!($reg), ", {}"),
                in(reg) $value as u64,
                options(nomem, nostack, preserves_flags)
            );
        }
    }};
}

/// Read CurrentEL register
pub fn read_current_el() -> u64 {
    let el: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CurrentEL",
            out(reg) el,
            options(nomem, nostack, preserves_flags)
        );
    }
    (el >> 2) & 3
}

/// Read MPIDR register
pub fn read_mpidr() -> u64 {
    let mpidr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, MPIDR_EL1",
            out(reg) mpidr,
            options(nomem, nostack, preserves_flags)
        );
    }
    mpidr
}

/// Read MIDR register
pub fn read_midr() -> u64 {
    let midr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, MIDR_EL1",
            out(reg) midr,
            options(nomem, nostack, preserves_flags)
        );
    }
    midr
}

/// Get CPU affinity from MPIDR
pub fn get_cpu_affinity() -> u64 {
    let mpidr = read_mpidr();
    // Extract Aff0, Aff1, Aff2, Aff3
    (mpidr & 0xFF)
        | ((mpidr >> 8) & 0xFF) << 8
        | ((mpidr >> 16) & 0xFF) << 16
        | ((mpidr >> 32) & 0xFF) << 24
}

/// Check if this is the primary CPU
pub fn is_primary_cpu() -> bool {
    get_cpu_affinity() == 0
}

// =============================================================================
// CACHE AND BARRIER OPERATIONS
// =============================================================================

/// Data synchronization barrier
#[inline(always)]
pub fn dsb() {
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }
}

/// Data synchronization barrier (inner shareable)
#[inline(always)]
pub fn dsb_ish() {
    unsafe {
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
    }
}

/// Instruction synchronization barrier
#[inline(always)]
pub fn isb() {
    unsafe {
        core::arch::asm!("isb", options(nostack, preserves_flags));
    }
}

/// Data memory barrier
#[inline(always)]
pub fn dmb() {
    unsafe {
        core::arch::asm!("dmb sy", options(nostack, preserves_flags));
    }
}

/// Wait for event
#[inline(always)]
pub fn wfe() {
    unsafe {
        core::arch::asm!("wfe", options(nomem, nostack, preserves_flags));
    }
}

/// Wait for interrupt
#[inline(always)]
pub fn wfi() {
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

/// Send event
#[inline(always)]
pub fn sev() {
    unsafe {
        core::arch::asm!("sev", options(nomem, nostack, preserves_flags));
    }
}

/// No operation
#[inline(always)]
pub fn nop() {
    unsafe {
        core::arch::asm!("nop", options(nomem, nostack, preserves_flags));
    }
}

/// Invalidate instruction cache
pub fn invalidate_icache() {
    unsafe {
        core::arch::asm!(
            "ic iallu",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB
pub fn invalidate_tlb() {
    unsafe {
        core::arch::asm!(
            "tlbi vmalle1",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate data cache by virtual address
///
/// # Safety
///
/// The caller must ensure the virtual address is valid.
pub unsafe fn invalidate_dcache_va(va: u64) {
    core::arch::asm!(
        "dc ivac, {}",
        in(reg) va,
        options(nostack, preserves_flags)
    );
}

/// Clean data cache by virtual address
///
/// # Safety
///
/// The caller must ensure the virtual address is valid.
pub unsafe fn clean_dcache_va(va: u64) {
    core::arch::asm!(
        "dc cvac, {}",
        in(reg) va,
        options(nostack, preserves_flags)
    );
}

/// Clean and invalidate data cache by virtual address
///
/// # Safety
///
/// The caller must ensure the virtual address is valid.
pub unsafe fn clean_invalidate_dcache_va(va: u64) {
    core::arch::asm!(
        "dc civac, {}",
        in(reg) va,
        options(nostack, preserves_flags)
    );
}

// =============================================================================
// INTERRUPT CONTROL
// =============================================================================

/// Disable interrupts
pub fn disable_interrupts() {
    unsafe {
        core::arch::asm!(
            "msr daifset, #0xf",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Enable interrupts
pub fn enable_interrupts() {
    unsafe {
        core::arch::asm!(
            "msr daifclr, #0xf",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Disable IRQs only
pub fn disable_irq() {
    unsafe {
        core::arch::asm!("msr daifset, #2", options(nomem, nostack, preserves_flags));
    }
}

/// Enable IRQs only
pub fn enable_irq() {
    unsafe {
        core::arch::asm!("msr daifclr, #2", options(nomem, nostack, preserves_flags));
    }
}

/// Read DAIF register
pub fn read_daif() -> u64 {
    let daif: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, DAIF",
            out(reg) daif,
            options(nomem, nostack, preserves_flags)
        );
    }
    daif
}

/// Write DAIF register
pub fn write_daif(daif: u64) {
    unsafe {
        core::arch::asm!(
            "msr DAIF, {}",
            in(reg) daif,
            options(nomem, nostack, preserves_flags)
        );
    }
}

// =============================================================================
// EARLY BOOT FUNCTIONS
// =============================================================================

/// Pre-initialization (very early)
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn pre_init(ctx: &mut BootContext) -> BootResult<()> {
    // Disable interrupts
    disable_interrupts();

    // Check exception level
    let el = read_current_el();
    ctx.arch_data.arm.current_el = el as u8;

    // Store CPU info
    ctx.arch_data.arm.mpidr = read_mpidr();
    ctx.arch_data.arm.midr = read_midr();

    Ok(())
}

/// Initialize serial console
///
/// # Safety
///
/// The caller must ensure serial port I/O is safe and the port is not in use.
pub unsafe fn init_serial(ctx: &mut BootContext) -> BootResult<()> {
    serial::init_uart(ctx)
}

/// Detect CPU features
///
/// # Safety
///
/// The caller must ensure the firmware is accessible.
pub unsafe fn detect_cpu_features(ctx: &mut BootContext) -> BootResult<()> {
    cpu::detect_features(ctx)
}

/// CPU initialization
///
/// # Safety
///
/// The caller must ensure this is called once per CPU during initialization.
pub unsafe fn cpu_init(ctx: &mut BootContext) -> BootResult<()> {
    cpu::init(ctx)
}

/// Set up page tables
///
/// # Safety
///
/// The caller must ensure the page table pointer is valid and properly aligned.
pub unsafe fn setup_page_tables(ctx: &mut BootContext) -> BootResult<()> {
    mmu::setup_page_tables(ctx)
}

/// Initialize interrupts (GIC)
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_interrupts(ctx: &mut BootContext) -> BootResult<()> {
    gic::init(ctx)
}

/// Initialize timers
///
/// # Safety
///
/// The caller must ensure timer hardware is accessible.
pub unsafe fn init_timers(ctx: &mut BootContext) -> BootResult<()> {
    timer::init(ctx)
}

/// Initialize SMP
///
/// # Safety
///
/// The caller must ensure SMP initialization is done after BSP is fully initialized.
pub unsafe fn init_smp(ctx: &mut BootContext) -> BootResult<()> {
    psci::init_smp(ctx)
}

/// Apply KASLR
///
/// # Safety
///
/// The caller must ensure page tables support the randomization offset.
pub unsafe fn apply_kaslr(ctx: &mut BootContext, offset: u64) -> BootResult<()> {
    mmu::apply_kaslr(ctx, offset)
}

/// Prepare for handoff to kernel
///
/// # Safety
///
/// The caller must ensure all safety invariants are upheld.
pub unsafe fn prepare_handoff(ctx: &mut BootContext) -> BootResult<()> {
    // Final barrier to ensure all writes are visible
    dsb();
    isb();

    Ok(())
}

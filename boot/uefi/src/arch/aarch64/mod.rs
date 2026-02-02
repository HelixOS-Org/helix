//! AArch64 Architecture Support
//!
//! ARM64 architecture-specific code for UEFI boot.

pub mod cpu;
pub mod gic;
pub mod mmu;

// Re-exports
pub use cpu::*;
pub use gic::*;
pub use mmu::*;

use crate::arch::{ArchOperations, CpuFeatures, InitState};
use crate::error::Result;
use crate::raw::types::VirtualAddress;

// =============================================================================
// SYSTEM REGISTERS
// =============================================================================

/// Read system register
#[macro_export]
macro_rules! read_sysreg {
    ($reg:tt) => {{
        let value: u64;
        unsafe {
            core::arch::asm!(
                concat!("mrs {}, ", $reg),
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }};
}

/// Write system register
#[macro_export]
macro_rules! write_sysreg {
    ($reg:tt, $value:expr) => {{
        let value: u64 = $value;
        unsafe {
            core::arch::asm!(
                concat!("msr ", $reg, ", {}"),
                in(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
    }};
}

// =============================================================================
// CURRENT EL (Exception Level)
// =============================================================================

/// Get current exception level (0-3)
pub fn current_el() -> u8 {
    let el: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CurrentEL",
            out(reg) el,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((el >> 2) & 0x3) as u8
}

/// Exception Level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionLevel {
    /// EL0: User mode
    EL0 = 0,
    /// EL1: Kernel mode
    EL1 = 1,
    /// EL2: Hypervisor mode
    EL2 = 2,
    /// EL3: Secure monitor mode
    EL3 = 3,
}

impl ExceptionLevel {
    /// Get current exception level
    pub fn current() -> Self {
        match current_el() {
            0 => Self::EL0,
            1 => Self::EL1,
            2 => Self::EL2,
            3 => Self::EL3,
            _ => Self::EL0,
        }
    }
}

// =============================================================================
// INTERRUPT MANAGEMENT
// =============================================================================

/// Disable IRQs
#[inline(always)]
pub fn disable_irq() {
    unsafe {
        core::arch::asm!("msr DAIFSet, #2", options(nomem, nostack, preserves_flags));
    }
}

/// Enable IRQs
#[inline(always)]
pub fn enable_irq() {
    unsafe {
        core::arch::asm!("msr DAIFClr, #2", options(nomem, nostack, preserves_flags));
    }
}

/// Disable FIQs
#[inline(always)]
pub fn disable_fiq() {
    unsafe {
        core::arch::asm!("msr DAIFSet, #1", options(nomem, nostack, preserves_flags));
    }
}

/// Enable FIQs
#[inline(always)]
pub fn enable_fiq() {
    unsafe {
        core::arch::asm!("msr DAIFClr, #1", options(nomem, nostack, preserves_flags));
    }
}

/// Disable all interrupts (IRQ + FIQ)
#[inline(always)]
fn disable_interrupts_impl() {
    unsafe {
        core::arch::asm!("msr DAIFSet, #3", options(nomem, nostack, preserves_flags));
    }
}

/// Enable all interrupts (IRQ + FIQ)
#[inline(always)]
fn enable_interrupts_impl() {
    unsafe {
        core::arch::asm!("msr DAIFClr, #3", options(nomem, nostack, preserves_flags));
    }
}

/// Get DAIF flags
pub fn get_daif() -> u64 {
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

/// Check if IRQs are disabled
pub fn irqs_disabled() -> bool {
    (get_daif() & (1 << 7)) != 0
}

/// Execute closure with interrupts disabled
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let daif = get_daif();
    disable_interrupts_impl();
    let result = f();
    if (daif & (1 << 7)) == 0 {
        enable_irq();
    }
    if (daif & (1 << 6)) == 0 {
        enable_fiq();
    }
    result
}

// =============================================================================
// BARRIERS AND SYNCHRONIZATION
// =============================================================================

/// Data Memory Barrier (all types)
#[inline(always)]
pub fn dmb() {
    unsafe {
        core::arch::asm!("dmb sy", options(nostack, preserves_flags));
    }
}

/// Data Memory Barrier (inner shareable)
#[inline(always)]
pub fn dmb_ish() {
    unsafe {
        core::arch::asm!("dmb ish", options(nostack, preserves_flags));
    }
}

/// Data Memory Barrier (non-shareable)
#[inline(always)]
pub fn dmb_nsh() {
    unsafe {
        core::arch::asm!("dmb nsh", options(nostack, preserves_flags));
    }
}

/// Data Synchronization Barrier
#[inline(always)]
pub fn dsb() {
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }
}

/// Data Synchronization Barrier (inner shareable)
#[inline(always)]
pub fn dsb_ish() {
    unsafe {
        core::arch::asm!("dsb ish", options(nostack, preserves_flags));
    }
}

/// Instruction Synchronization Barrier
#[inline(always)]
pub fn isb() {
    unsafe {
        core::arch::asm!("isb", options(nostack, preserves_flags));
    }
}

// =============================================================================
// TLB MANAGEMENT
// =============================================================================

/// Invalidate all TLB entries (EL1)
#[inline(always)]
pub fn tlbi_all() {
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vmalle1is",
            "dsb ish",
            "isb",
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entry by virtual address (EL1)
#[inline(always)]
pub fn tlbi_va(va: u64) {
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi vale1is, {}",
            "dsb ish",
            "isb",
            in(reg) va >> 12,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate TLB entry by ASID
#[inline(always)]
pub fn tlbi_asid(asid: u16) {
    unsafe {
        core::arch::asm!(
            "dsb ishst",
            "tlbi aside1is, {}",
            "dsb ish",
            "isb",
            in(reg) (asid as u64) << 48,
            options(nostack, preserves_flags)
        );
    }
}

// =============================================================================
// CACHE MANAGEMENT
// =============================================================================

/// Data cache clean by virtual address to PoC
#[inline(always)]
pub fn dc_cvac(va: u64) {
    unsafe {
        core::arch::asm!(
            "dc cvac, {}",
            in(reg) va,
            options(nostack, preserves_flags)
        );
    }
}

/// Data cache clean and invalidate by virtual address to PoC
#[inline(always)]
pub fn dc_civac(va: u64) {
    unsafe {
        core::arch::asm!(
            "dc civac, {}",
            in(reg) va,
            options(nostack, preserves_flags)
        );
    }
}

/// Data cache invalidate by virtual address to PoC
#[inline(always)]
pub fn dc_ivac(va: u64) {
    unsafe {
        core::arch::asm!(
            "dc ivac, {}",
            in(reg) va,
            options(nostack, preserves_flags)
        );
    }
}

/// Data cache zero by virtual address
#[inline(always)]
pub fn dc_zva(va: u64) {
    unsafe {
        core::arch::asm!(
            "dc zva, {}",
            in(reg) va,
            options(nostack, preserves_flags)
        );
    }
}

/// Instruction cache invalidate all to PoU
#[inline(always)]
pub fn ic_iallu() {
    unsafe {
        core::arch::asm!("ic iallu", options(nostack, preserves_flags));
    }
}

/// Instruction cache invalidate by virtual address to PoU
#[inline(always)]
pub fn ic_ivau(va: u64) {
    unsafe {
        core::arch::asm!(
            "ic ivau, {}",
            in(reg) va,
            options(nostack, preserves_flags)
        );
    }
}

/// Clean and invalidate data cache range
pub fn clean_invalidate_dcache_range(start: u64, size: u64) {
    let line_size = dcache_line_size();
    let end = start + size;
    let mut addr = start & !(line_size as u64 - 1);

    while addr < end {
        dc_civac(addr);
        addr += line_size as u64;
    }
    dsb();
}

/// Get data cache line size
pub fn dcache_line_size() -> usize {
    let ctr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CTR_EL0",
            out(reg) ctr,
            options(nomem, nostack, preserves_flags)
        );
    }
    4 << ((ctr >> 16) & 0xF)
}

/// Get instruction cache line size
pub fn icache_line_size() -> usize {
    let ctr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CTR_EL0",
            out(reg) ctr,
            options(nomem, nostack, preserves_flags)
        );
    }
    4 << (ctr & 0xF)
}

// =============================================================================
// TIMING
// =============================================================================

/// Read system counter (CNTVCT_EL0)
#[inline(always)]
pub fn read_counter() -> u64 {
    let cnt: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTVCT_EL0",
            out(reg) cnt,
            options(nomem, nostack, preserves_flags)
        );
    }
    cnt
}

/// Read counter frequency (CNTFRQ_EL0)
pub fn counter_frequency() -> u64 {
    let freq: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTFRQ_EL0",
            out(reg) freq,
            options(nomem, nostack, preserves_flags)
        );
    }
    freq
}

/// Delay for specified microseconds
pub fn delay_us(us: u64) {
    let freq = counter_frequency();
    let start = read_counter();
    let ticks = (freq * us) / 1_000_000;

    while read_counter() - start < ticks {
        core::hint::spin_loop();
    }
}

/// Delay for specified milliseconds
pub fn delay_ms(ms: u64) {
    delay_us(ms * 1000);
}

// =============================================================================
// CPU CONTROL
// =============================================================================

/// Wait For Interrupt (low power idle)
#[inline(always)]
pub fn wfi() {
    unsafe {
        core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

/// Wait For Event
#[inline(always)]
pub fn wfe() {
    unsafe {
        core::arch::asm!("wfe", options(nomem, nostack, preserves_flags));
    }
}

/// Send Event (wake up WFE)
#[inline(always)]
pub fn sev() {
    unsafe {
        core::arch::asm!("sev", options(nomem, nostack, preserves_flags));
    }
}

/// Send Event Local (wake up local WFE)
#[inline(always)]
pub fn sevl() {
    unsafe {
        core::arch::asm!("sevl", options(nomem, nostack, preserves_flags));
    }
}

/// Yield to scheduler
#[inline(always)]
pub fn yield_cpu() {
    unsafe {
        core::arch::asm!("yield", options(nomem, nostack, preserves_flags));
    }
}

/// No-operation
#[inline(always)]
pub fn nop() {
    unsafe {
        core::arch::asm!("nop", options(nomem, nostack, preserves_flags));
    }
}

/// Breakpoint
#[inline(always)]
pub fn brk() {
    unsafe {
        core::arch::asm!("brk #0", options(nomem, nostack));
    }
}

/// Halt CPU
pub fn halt() -> ! {
    loop {
        wfi();
    }
}

// =============================================================================
// MULTIPROCESSOR AFFINITY
// =============================================================================

/// Read MPIDR_EL1 (Multiprocessor Affinity Register)
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

/// MPIDR fields
pub mod mpidr {
    /// Affinity level 0 (usually core ID)
    pub fn aff0(mpidr: u64) -> u8 {
        (mpidr & 0xFF) as u8
    }

    /// Affinity level 1 (usually cluster ID)
    pub fn aff1(mpidr: u64) -> u8 {
        ((mpidr >> 8) & 0xFF) as u8
    }

    /// Affinity level 2
    pub fn aff2(mpidr: u64) -> u8 {
        ((mpidr >> 16) & 0xFF) as u8
    }

    /// Affinity level 3
    pub fn aff3(mpidr: u64) -> u8 {
        ((mpidr >> 32) & 0xFF) as u8
    }

    /// Uniprocessor system
    pub fn is_up(mpidr: u64) -> bool {
        (mpidr & (1 << 30)) != 0
    }

    /// Multithreading type
    pub fn mt(mpidr: u64) -> bool {
        (mpidr & (1 << 24)) != 0
    }
}

/// Get CPU ID (linear from affinity)
pub fn cpu_id() -> u32 {
    let mpidr = read_mpidr();
    let aff0 = mpidr::aff0(mpidr) as u32;
    let aff1 = mpidr::aff1(mpidr) as u32;
    let aff2 = mpidr::aff2(mpidr) as u32;
    // Simple linear mapping (may need adjustment for specific SoCs)
    aff0 | (aff1 << 8) | (aff2 << 16)
}

// =============================================================================
// ARCH OPERATIONS IMPLEMENTATION
// =============================================================================

/// AArch64 architecture operations
pub struct Aarch64Ops;

impl ArchOperations for Aarch64Ops {
    fn init() -> Result<()> {
        Ok(())
    }

    fn current_cpu_id() -> u32 {
        cpu_id()
    }

    fn halt() -> ! {
        halt()
    }

    fn enable_interrupts() {
        enable_interrupts_impl();
    }

    fn disable_interrupts() {
        disable_interrupts_impl();
    }

    fn interrupts_enabled() -> bool {
        !irqs_disabled()
    }

    fn read_timestamp() -> u64 {
        read_counter()
    }

    fn invalidate_tlb_entry(addr: VirtualAddress) {
        tlbi_va(addr.0);
    }

    fn invalidate_tlb_all() {
        tlbi_all();
    }

    fn memory_barrier() {
        dmb();
    }

    fn read_barrier() {
        dmb();
    }

    fn write_barrier() {
        dmb();
    }
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Early initialization for AArch64
pub fn early_init() {
    // Ensure we're at EL1 or EL2
    let el = current_el();

    // Disable interrupts during initialization
    disable_interrupts_impl();

    // Synchronize
    isb();
}

/// Initialize CPU features
pub fn init_cpu() {
    // Enable floating point and SIMD
    enable_fp_simd();

    // Synchronize
    isb();
}

/// Enable FP/SIMD (NEON)
fn enable_fp_simd() {
    // Read CPACR_EL1
    let cpacr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CPACR_EL1",
            out(reg) cpacr,
            options(nomem, nostack, preserves_flags)
        );
    }

    // Enable FP/SIMD (bits 20-21 = 0b11)
    let new_cpacr = cpacr | (0b11 << 20);

    unsafe {
        core::arch::asm!(
            "msr CPACR_EL1, {}",
            "isb",
            in(reg) new_cpacr,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Initialize memory management
pub fn init_memory() {
    // Configure MAIR (Memory Attribute Indirection Register)
    // Configured in mmu module
}

/// Initialize interrupt handling
pub fn init_interrupts() {
    // Configure exception vectors
    // GIC initialization in gic module
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_level() {
        let el = ExceptionLevel::current();
        // In user mode tests, we're at EL0
        // In kernel, we'd be at EL1
        assert!(matches!(el, ExceptionLevel::EL0 | ExceptionLevel::EL1));
    }

    #[test]
    fn test_cpu_id() {
        let id = cpu_id();
        // Just verify it doesn't panic
        let _ = id;
    }
}

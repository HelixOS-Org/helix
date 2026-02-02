//! # AArch64 General Purpose Registers
//!
//! This module provides access to AArch64 general purpose registers
//! and the PSTATE (processor state) flags.
//!
//! ## Register Set
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  AArch64 Register Set                        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  X0-X7   │ Arguments / Results                              │
//! │  X8      │ Indirect result location                         │
//! │  X9-X15  │ Temporary registers                              │
//! │  X16-X17 │ Intra-procedure-call scratch (IP0/IP1)           │
//! │  X18     │ Platform register (reserved)                     │
//! │  X19-X28 │ Callee-saved registers                           │
//! │  X29     │ Frame pointer (FP)                                │
//! │  X30     │ Link register (LR)                                │
//! │  SP      │ Stack pointer (SP_EL0 or SP_ELx)                  │
//! │  PC      │ Program counter                                   │
//! │  PSTATE  │ Processor state (NZCV, DAIF, etc.)               │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use core::arch::asm;

// =============================================================================
// PSTATE Flags
// =============================================================================

bitflags::bitflags! {
    /// PSTATE (Processor State) flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PState: u64 {
        /// Negative flag
        const N = 1 << 31;
        /// Zero flag
        const Z = 1 << 30;
        /// Carry flag
        const C = 1 << 29;
        /// Overflow flag
        const V = 1 << 28;

        /// Debug exception mask
        const D = 1 << 9;
        /// SError interrupt mask
        const A = 1 << 8;
        /// IRQ interrupt mask
        const I = 1 << 7;
        /// FIQ interrupt mask
        const F = 1 << 6;

        /// Execution state (0 = AArch64)
        const N_RW = 1 << 4;

        /// Exception level mask
        const EL_MASK = 0b11 << 2;
        /// Stack pointer select
        const SP = 1 << 0;

        /// All interrupt masks (DAIF)
        const DAIF = Self::D.bits() | Self::A.bits() | Self::I.bits() | Self::F.bits();
    }
}

impl PState {
    /// Get current exception level (0-3)
    #[inline]
    pub fn exception_level(&self) -> u8 {
        ((self.bits() >> 2) & 0b11) as u8
    }

    /// Check if using SP_ELx (vs SP_EL0)
    #[inline]
    pub fn using_sp_elx(&self) -> bool {
        self.contains(Self::SP)
    }

    /// Check if interrupts are masked
    #[inline]
    pub fn interrupts_masked(&self) -> bool {
        self.contains(Self::I)
    }
}

// =============================================================================
// Register Access
// =============================================================================

/// General purpose registers structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Registers {
    /// X0-X30 general purpose registers
    pub x: [u64; 31],
    /// Stack pointer
    pub sp: u64,
    /// Program counter (ELR_EL1 on exception)
    pub pc: u64,
    /// Processor state (SPSR_EL1 on exception)
    pub pstate: u64,
}

impl Registers {
    /// Create a new empty register set
    pub const fn new() -> Self {
        Self {
            x: [0; 31],
            sp: 0,
            pc: 0,
            pstate: 0,
        }
    }

    /// Create registers for a new user thread
    pub fn new_user_thread(entry: u64, stack: u64, arg: u64) -> Self {
        let mut regs = Self::new();
        regs.pc = entry;
        regs.sp = stack;
        regs.x[0] = arg;
        // EL0, interrupts enabled, AArch64
        regs.pstate = 0;
        regs
    }

    /// Create registers for a new kernel thread
    pub fn new_kernel_thread(entry: u64, stack: u64, arg: u64) -> Self {
        let mut regs = Self::new();
        regs.pc = entry;
        regs.sp = stack;
        regs.x[0] = arg;
        // EL1, SP_EL1, interrupts enabled, AArch64
        regs.pstate = 0b0101; // EL1h
        regs
    }

    /// Get frame pointer (X29)
    #[inline]
    pub fn fp(&self) -> u64 {
        self.x[29]
    }

    /// Get link register (X30)
    #[inline]
    pub fn lr(&self) -> u64 {
        self.x[30]
    }

    /// Set frame pointer (X29)
    #[inline]
    pub fn set_fp(&mut self, value: u64) {
        self.x[29] = value;
    }

    /// Set link register (X30)
    #[inline]
    pub fn set_lr(&mut self, value: u64) {
        self.x[30] = value;
    }

    /// Get PSTATE as flags
    pub fn pstate_flags(&self) -> PState {
        PState::from_bits_truncate(self.pstate)
    }
}

// =============================================================================
// Register Read Functions
// =============================================================================

/// Read the stack pointer
#[inline]
pub fn read_sp() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, sp", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read the frame pointer (X29)
#[inline]
pub fn read_fp() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, x29", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read the link register (X30)
#[inline]
pub fn read_lr() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, x30", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read DAIF (interrupt mask flags)
#[inline]
pub fn read_daif() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, DAIF", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write DAIF (interrupt mask flags)
#[inline]
pub fn write_daif(value: u64) {
    unsafe {
        asm!("msr DAIF, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Read NZCV (condition flags)
#[inline]
pub fn read_nzcv() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, NZCV", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read current exception level
#[inline]
pub fn read_current_el() -> u8 {
    let value: u64;
    unsafe {
        asm!("mrs {}, CurrentEL", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    ((value >> 2) & 0b11) as u8
}

/// Read SPSel (stack pointer selection)
#[inline]
pub fn read_spsel() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, SPSel", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write SPSel (stack pointer selection)
#[inline]
pub fn write_spsel(value: u64) {
    unsafe {
        asm!("msr SPSel, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

// =============================================================================
// Interrupt Control
// =============================================================================

/// Disable all interrupts (IRQ and FIQ)
#[inline]
pub fn disable_interrupts() {
    unsafe {
        asm!(
            "msr DAIFSet, #0xf",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Enable all interrupts (IRQ and FIQ)
#[inline]
pub fn enable_interrupts() {
    unsafe {
        asm!(
            "msr DAIFClr, #0xf",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Disable IRQ only
#[inline]
pub fn disable_irq() {
    unsafe {
        asm!(
            "msr DAIFSet, #0x2",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Enable IRQ only
#[inline]
pub fn enable_irq() {
    unsafe {
        asm!(
            "msr DAIFClr, #0x2",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Disable FIQ only
#[inline]
pub fn disable_fiq() {
    unsafe {
        asm!(
            "msr DAIFSet, #0x1",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Enable FIQ only
#[inline]
pub fn enable_fiq() {
    unsafe {
        asm!(
            "msr DAIFClr, #0x1",
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Check if interrupts are enabled
#[inline]
pub fn interrupts_enabled() -> bool {
    (read_daif() & (1 << 7)) == 0
}

/// Execute with interrupts disabled, then restore
#[inline]
pub fn with_interrupts_disabled<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let daif = read_daif();
    disable_interrupts();
    let result = f();
    write_daif(daif);
    result
}

// =============================================================================
// CPU Control
// =============================================================================

/// Wait for interrupt (low-power wait)
#[inline]
pub fn wfi() {
    unsafe {
        asm!("wfi", options(nomem, nostack, preserves_flags));
    }
}

/// Wait for event
#[inline]
pub fn wfe() {
    unsafe {
        asm!("wfe", options(nomem, nostack, preserves_flags));
    }
}

/// Send event (wake other CPUs from WFE)
#[inline]
pub fn sev() {
    unsafe {
        asm!("sev", options(nomem, nostack, preserves_flags));
    }
}

/// Send event local
#[inline]
pub fn sevl() {
    unsafe {
        asm!("sevl", options(nomem, nostack, preserves_flags));
    }
}

/// Yield to other threads
#[inline]
pub fn yield_cpu() {
    unsafe {
        asm!("yield", options(nomem, nostack, preserves_flags));
    }
}

/// No operation
#[inline]
pub fn nop() {
    unsafe {
        asm!("nop", options(nomem, nostack, preserves_flags));
    }
}

/// Halt the CPU (infinite WFI loop)
#[inline]
pub fn halt() -> ! {
    loop {
        wfi();
    }
}

/// Breakpoint
#[inline]
pub fn breakpoint() {
    unsafe {
        asm!("brk #0", options(nomem, nostack));
    }
}

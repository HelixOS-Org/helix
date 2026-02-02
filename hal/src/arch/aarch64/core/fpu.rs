//! # AArch64 Floating-Point and SIMD State
//!
//! This module handles NEON (Advanced SIMD) and optionally SVE
//! floating-point state management for context switching.

use core::arch::asm;

// =============================================================================
// FPU State Structure
// =============================================================================

/// NEON/FP register state (Advanced SIMD)
///
/// Contains all 32 128-bit V registers plus control/status registers.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FpuState {
    /// V0-V31 (128-bit SIMD registers)
    pub v: [u128; 32],
    /// Floating-point Control Register
    pub fpcr: u32,
    /// Floating-point Status Register
    pub fpsr: u32,
}

impl Default for FpuState {
    fn default() -> Self {
        Self::new()
    }
}

impl FpuState {
    /// Create a new zeroed FPU state
    pub const fn new() -> Self {
        Self {
            v: [0u128; 32],
            fpcr: 0,
            fpsr: 0,
        }
    }

    /// Create a new FPU state with default FPCR settings
    pub fn with_defaults() -> Self {
        let mut state = Self::new();
        // Default FPCR: Round to Nearest, no exception traps
        state.fpcr = 0;
        state
    }

    /// Save current FPU state
    pub fn save(&mut self) {
        save_fpu_state(self);
    }

    /// Restore FPU state
    pub fn restore(&self) {
        restore_fpu_state(self);
    }
}

impl core::fmt::Debug for FpuState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FpuState")
            .field("fpcr", &format_args!("{:#010x}", self.fpcr))
            .field("fpsr", &format_args!("{:#010x}", self.fpsr))
            .finish()
    }
}

// =============================================================================
// FPCR Flags
// =============================================================================

bitflags::bitflags! {
    /// Floating-point Control Register flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Fpcr: u32 {
        /// Alternate Half-Precision
        const AHP = 1 << 26;
        /// Default NaN mode
        const DN = 1 << 25;
        /// Flush-to-zero mode
        const FZ = 1 << 24;

        // Rounding Mode (bits 23:22)
        /// Round to Nearest (default)
        const RN = 0b00 << 22;
        /// Round towards Plus Infinity
        const RP = 0b01 << 22;
        /// Round towards Minus Infinity
        const RM = 0b10 << 22;
        /// Round towards Zero
        const RZ = 0b11 << 22;

        /// Flush-to-zero mode for FP16
        const FZ16 = 1 << 19;

        // Exception Trap Enables
        /// Invalid Operation exception trap enable
        const IDE = 1 << 15;
        /// Divide by Zero exception trap enable
        const IXE = 1 << 12;
        /// Underflow exception trap enable
        const UFE = 1 << 11;
        /// Overflow exception trap enable
        const OFE = 1 << 10;
        /// Division by Zero exception trap enable
        const DZE = 1 << 9;
        /// Input Denormal exception trap enable
        const IOE = 1 << 8;

        /// NEP - controls SIMD scalar behavior
        const NEP = 1 << 2;
        /// AH - alternate handling
        const AH = 1 << 1;
        /// FIZ - flush inputs to zero
        const FIZ = 1 << 0;
    }
}

// =============================================================================
// FPSR Flags
// =============================================================================

bitflags::bitflags! {
    /// Floating-point Status Register flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Fpsr: u32 {
        /// Negative condition flag
        const N = 1 << 31;
        /// Zero condition flag
        const Z = 1 << 30;
        /// Carry condition flag
        const C = 1 << 29;
        /// Overflow condition flag
        const V = 1 << 28;

        /// Cumulative saturation bit (SIMD)
        const QC = 1 << 27;

        // Cumulative Exception Flags
        /// Input Denormal cumulative
        const IDC = 1 << 7;
        /// Inexact cumulative
        const IXC = 1 << 4;
        /// Underflow cumulative
        const UFC = 1 << 3;
        /// Overflow cumulative
        const OFC = 1 << 2;
        /// Division by Zero cumulative
        const DZC = 1 << 1;
        /// Invalid Operation cumulative
        const IOC = 1 << 0;
    }
}

// =============================================================================
// FPU State Save/Restore
// =============================================================================

/// Save FPU state to memory
pub fn save_fpu_state(state: &mut FpuState) {
    unsafe {
        let ptr = state.v.as_mut_ptr() as *mut u8;

        // Save V0-V31 using STP pairs
        asm!(
            "stp q0, q1, [{ptr}, #0]",
            "stp q2, q3, [{ptr}, #32]",
            "stp q4, q5, [{ptr}, #64]",
            "stp q6, q7, [{ptr}, #96]",
            "stp q8, q9, [{ptr}, #128]",
            "stp q10, q11, [{ptr}, #160]",
            "stp q12, q13, [{ptr}, #192]",
            "stp q14, q15, [{ptr}, #224]",
            "stp q16, q17, [{ptr}, #256]",
            "stp q18, q19, [{ptr}, #288]",
            "stp q20, q21, [{ptr}, #320]",
            "stp q22, q23, [{ptr}, #352]",
            "stp q24, q25, [{ptr}, #384]",
            "stp q26, q27, [{ptr}, #416]",
            "stp q28, q29, [{ptr}, #448]",
            "stp q30, q31, [{ptr}, #480]",
            ptr = in(reg) ptr,
            options(nostack, preserves_flags)
        );

        // Save FPCR and FPSR
        let fpcr: u64;
        let fpsr: u64;
        asm!(
            "mrs {fpcr}, FPCR",
            "mrs {fpsr}, FPSR",
            fpcr = out(reg) fpcr,
            fpsr = out(reg) fpsr,
            options(nomem, nostack, preserves_flags)
        );
        state.fpcr = fpcr as u32;
        state.fpsr = fpsr as u32;
    }
}

/// Restore FPU state from memory
pub fn restore_fpu_state(state: &FpuState) {
    unsafe {
        // Restore FPCR and FPSR first
        let fpcr = state.fpcr as u64;
        let fpsr = state.fpsr as u64;
        asm!(
            "msr FPCR, {fpcr}",
            "msr FPSR, {fpsr}",
            fpcr = in(reg) fpcr,
            fpsr = in(reg) fpsr,
            options(nomem, nostack, preserves_flags)
        );

        let ptr = state.v.as_ptr() as *const u8;

        // Restore V0-V31 using LDP pairs
        asm!(
            "ldp q0, q1, [{ptr}, #0]",
            "ldp q2, q3, [{ptr}, #32]",
            "ldp q4, q5, [{ptr}, #64]",
            "ldp q6, q7, [{ptr}, #96]",
            "ldp q8, q9, [{ptr}, #128]",
            "ldp q10, q11, [{ptr}, #160]",
            "ldp q12, q13, [{ptr}, #192]",
            "ldp q14, q15, [{ptr}, #224]",
            "ldp q16, q17, [{ptr}, #256]",
            "ldp q18, q19, [{ptr}, #288]",
            "ldp q20, q21, [{ptr}, #320]",
            "ldp q22, q23, [{ptr}, #352]",
            "ldp q24, q25, [{ptr}, #384]",
            "ldp q26, q27, [{ptr}, #416]",
            "ldp q28, q29, [{ptr}, #448]",
            "ldp q30, q31, [{ptr}, #480]",
            ptr = in(reg) ptr,
            options(nostack, preserves_flags)
        );
    }
}

// =============================================================================
// FPU Control
// =============================================================================

/// Read FPCR
#[inline]
pub fn read_fpcr() -> Fpcr {
    let value: u64;
    unsafe {
        asm!("mrs {}, FPCR", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Fpcr::from_bits_truncate(value as u32)
}

/// Write FPCR
#[inline]
pub fn write_fpcr(fpcr: Fpcr) {
    let value = fpcr.bits() as u64;
    unsafe {
        asm!("msr FPCR, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Read FPSR
#[inline]
pub fn read_fpsr() -> Fpsr {
    let value: u64;
    unsafe {
        asm!("mrs {}, FPSR", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Fpsr::from_bits_truncate(value as u32)
}

/// Write FPSR
#[inline]
pub fn write_fpsr(fpsr: Fpsr) {
    let value = fpsr.bits() as u64;
    unsafe {
        asm!("msr FPSR, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Clear all FPSR exception flags
#[inline]
pub fn clear_fp_exceptions() {
    let fpsr = read_fpsr();
    let cleared = fpsr & !(Fpsr::IOC | Fpsr::DZC | Fpsr::OFC | Fpsr::UFC | Fpsr::IXC | Fpsr::IDC);
    write_fpsr(cleared);
}

// =============================================================================
// FPU Enable/Disable
// =============================================================================

/// Enable FPU/SIMD access for EL1
///
/// This sets CPACR_EL1.FPEN to allow FP/SIMD at EL0 and EL1.
#[inline]
pub fn enable_fpu() {
    unsafe {
        asm!(
            "mrs {tmp}, CPACR_EL1",
            "orr {tmp}, {tmp}, #(0x3 << 20)",
            "msr CPACR_EL1, {tmp}",
            "isb",
            tmp = out(reg) _,
            options(nomem, nostack)
        );
    }
}

/// Disable FPU/SIMD access
#[inline]
pub fn disable_fpu() {
    unsafe {
        asm!(
            "mrs {tmp}, CPACR_EL1",
            "bic {tmp}, {tmp}, #(0x3 << 20)",
            "msr CPACR_EL1, {tmp}",
            "isb",
            tmp = out(reg) _,
            options(nomem, nostack)
        );
    }
}

/// Check if FPU is enabled
#[inline]
pub fn is_fpu_enabled() -> bool {
    let cpacr: u64;
    unsafe {
        asm!("mrs {}, CPACR_EL1", out(reg) cpacr, options(nomem, nostack, preserves_flags));
    }
    ((cpacr >> 20) & 0x3) == 0x3
}

// =============================================================================
// Lazy FPU Context Switching
// =============================================================================

/// FPU trap enable - triggers exception on FPU use
///
/// Used for lazy FPU context switching. When enabled, using any FP/SIMD
/// instruction will trap to the kernel, which can then save/restore state.
#[inline]
pub fn trap_fpu() {
    unsafe {
        asm!(
            "mrs {tmp}, CPACR_EL1",
            "bic {tmp}, {tmp}, #(0x3 << 20)",
            "orr {tmp}, {tmp}, #(0x1 << 20)",  // Trap EL0, allow EL1
            "msr CPACR_EL1, {tmp}",
            "isb",
            tmp = out(reg) _,
            options(nomem, nostack)
        );
    }
}

/// Disable FPU trap
#[inline]
pub fn untrap_fpu() {
    enable_fpu();
}

// =============================================================================
// Rounding Mode
// =============================================================================

/// Rounding mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RoundingMode {
    /// Round to Nearest, ties to Even
    RoundNearest = 0b00,
    /// Round towards Plus Infinity
    RoundUp      = 0b01,
    /// Round towards Minus Infinity
    RoundDown    = 0b10,
    /// Round towards Zero
    RoundToZero  = 0b11,
}

/// Get current rounding mode
#[inline]
pub fn get_rounding_mode() -> RoundingMode {
    let fpcr = read_fpcr();
    match (fpcr.bits() >> 22) & 0x3 {
        0b00 => RoundingMode::RoundNearest,
        0b01 => RoundingMode::RoundUp,
        0b10 => RoundingMode::RoundDown,
        0b11 => RoundingMode::RoundToZero,
        _ => RoundingMode::RoundNearest,
    }
}

/// Set rounding mode
#[inline]
pub fn set_rounding_mode(mode: RoundingMode) {
    let mut fpcr = read_fpcr();
    // Clear rounding mode bits
    fpcr = Fpcr::from_bits_truncate(fpcr.bits() & !(0x3 << 22));
    // Set new rounding mode
    fpcr = Fpcr::from_bits_truncate(fpcr.bits() | ((mode as u32) << 22));
    write_fpcr(fpcr);
}

// =============================================================================
// SVE Support (ARMv8.2+)
// =============================================================================

#[cfg(feature = "sve")]
mod sve {
    use core::arch::asm;

    /// Get current SVE vector length in bytes
    #[inline]
    pub fn get_sve_vl() -> usize {
        let vl: u64;
        unsafe {
            asm!("rdvl {}, #1", out(reg) vl, options(nomem, nostack, preserves_flags));
        }
        vl as usize
    }

    /// Enable SVE for EL1
    #[inline]
    pub fn enable_sve() {
        unsafe {
            asm!(
                "mrs {tmp}, CPACR_EL1",
                "orr {tmp}, {tmp}, #(0x3 << 16)",
                "msr CPACR_EL1, {tmp}",
                "isb",
                tmp = out(reg) _,
                options(nomem, nostack)
            );
        }
    }
}

#[cfg(feature = "sve")]
pub use sve::*;

// =============================================================================
// Utility Functions
// =============================================================================

/// Initialize FPU to default state
pub fn init_fpu() {
    enable_fpu();

    // Set default FPCR: Round to Nearest, no flush-to-zero
    write_fpcr(Fpcr::empty());

    // Clear all exception flags
    write_fpsr(Fpsr::empty());
}

/// Check for and clear any pending FP exceptions
pub fn check_fp_exceptions() -> Option<Fpsr> {
    let fpsr = read_fpsr();
    let exceptions = fpsr & (Fpsr::IOC | Fpsr::DZC | Fpsr::OFC | Fpsr::UFC | Fpsr::IXC | Fpsr::IDC);

    if !exceptions.is_empty() {
        clear_fp_exceptions();
        Some(exceptions)
    } else {
        None
    }
}

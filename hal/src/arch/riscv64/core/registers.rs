//! # RISC-V General Purpose Registers
//!
//! This module provides definitions and utilities for RISC-V general purpose
//! registers (x0-x31) and their ABI names.
//!
//! ## Register Convention (RV64I)
//!
//! | Register | ABI Name | Description                    | Saver  |
//! |----------|----------|--------------------------------|--------|
//! | x0       | zero     | Hardwired zero                 | -      |
//! | x1       | ra       | Return address                 | Caller |
//! | x2       | sp       | Stack pointer                  | Callee |
//! | x3       | gp       | Global pointer                 | -      |
//! | x4       | tp       | Thread pointer                 | -      |
//! | x5-x7    | t0-t2    | Temporaries                    | Caller |
//! | x8       | s0/fp    | Saved register / Frame pointer | Callee |
//! | x9       | s1       | Saved register                 | Callee |
//! | x10-x11  | a0-a1    | Function args / Return values  | Caller |
//! | x12-x17  | a2-a7    | Function arguments             | Caller |
//! | x18-x27  | s2-s11   | Saved registers                | Callee |
//! | x28-x31  | t3-t6    | Temporaries                    | Caller |

use core::arch::asm;

// ============================================================================
// Register Indices
// ============================================================================

/// Register index type
pub type RegIndex = u8;

/// Zero register (always 0)
pub const REG_ZERO: RegIndex = 0;
/// Return address
pub const REG_RA: RegIndex = 1;
/// Stack pointer
pub const REG_SP: RegIndex = 2;
/// Global pointer
pub const REG_GP: RegIndex = 3;
/// Thread pointer
pub const REG_TP: RegIndex = 4;
/// Temporary register 0
pub const REG_T0: RegIndex = 5;
/// Temporary register 1
pub const REG_T1: RegIndex = 6;
/// Temporary register 2
pub const REG_T2: RegIndex = 7;
/// Saved register 0 / Frame pointer
pub const REG_S0: RegIndex = 8;
pub const REG_FP: RegIndex = 8;
/// Saved register 1
pub const REG_S1: RegIndex = 9;
/// Argument 0 / Return value 0
pub const REG_A0: RegIndex = 10;
/// Argument 1 / Return value 1
pub const REG_A1: RegIndex = 11;
/// Argument 2
pub const REG_A2: RegIndex = 12;
/// Argument 3
pub const REG_A3: RegIndex = 13;
/// Argument 4
pub const REG_A4: RegIndex = 14;
/// Argument 5
pub const REG_A5: RegIndex = 15;
/// Argument 6
pub const REG_A6: RegIndex = 16;
/// Argument 7
pub const REG_A7: RegIndex = 17;
/// Saved register 2
pub const REG_S2: RegIndex = 18;
/// Saved register 3
pub const REG_S3: RegIndex = 19;
/// Saved register 4
pub const REG_S4: RegIndex = 20;
/// Saved register 5
pub const REG_S5: RegIndex = 21;
/// Saved register 6
pub const REG_S6: RegIndex = 22;
/// Saved register 7
pub const REG_S7: RegIndex = 23;
/// Saved register 8
pub const REG_S8: RegIndex = 24;
/// Saved register 9
pub const REG_S9: RegIndex = 25;
/// Saved register 10
pub const REG_S10: RegIndex = 26;
/// Saved register 11
pub const REG_S11: RegIndex = 27;
/// Temporary register 3
pub const REG_T3: RegIndex = 28;
/// Temporary register 4
pub const REG_T4: RegIndex = 29;
/// Temporary register 5
pub const REG_T5: RegIndex = 30;
/// Temporary register 6
pub const REG_T6: RegIndex = 31;

// ============================================================================
// Register Names
// ============================================================================

/// Get ABI name for a register
pub const fn reg_abi_name(index: RegIndex) -> &'static str {
    match index {
        0 => "zero",
        1 => "ra",
        2 => "sp",
        3 => "gp",
        4 => "tp",
        5 => "t0",
        6 => "t1",
        7 => "t2",
        8 => "s0",
        9 => "s1",
        10 => "a0",
        11 => "a1",
        12 => "a2",
        13 => "a3",
        14 => "a4",
        15 => "a5",
        16 => "a6",
        17 => "a7",
        18 => "s2",
        19 => "s3",
        20 => "s4",
        21 => "s5",
        22 => "s6",
        23 => "s7",
        24 => "s8",
        25 => "s9",
        26 => "s10",
        27 => "s11",
        28 => "t3",
        29 => "t4",
        30 => "t5",
        31 => "t6",
        _ => "unknown",
    }
}

// ============================================================================
// General Registers Structure
// ============================================================================

/// All general purpose registers (for context save/restore)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GeneralRegisters {
    /// x0 (zero) - always 0, included for indexing convenience
    pub zero: u64,
    /// x1 (ra) - return address
    pub ra: u64,
    /// x2 (sp) - stack pointer
    pub sp: u64,
    /// x3 (gp) - global pointer
    pub gp: u64,
    /// x4 (tp) - thread pointer
    pub tp: u64,
    /// x5 (t0) - temporary
    pub t0: u64,
    /// x6 (t1) - temporary
    pub t1: u64,
    /// x7 (t2) - temporary
    pub t2: u64,
    /// x8 (s0/fp) - saved register / frame pointer
    pub s0: u64,
    /// x9 (s1) - saved register
    pub s1: u64,
    /// x10 (a0) - argument / return value
    pub a0: u64,
    /// x11 (a1) - argument / return value
    pub a1: u64,
    /// x12 (a2) - argument
    pub a2: u64,
    /// x13 (a3) - argument
    pub a3: u64,
    /// x14 (a4) - argument
    pub a4: u64,
    /// x15 (a5) - argument
    pub a5: u64,
    /// x16 (a6) - argument
    pub a6: u64,
    /// x17 (a7) - argument
    pub a7: u64,
    /// x18 (s2) - saved register
    pub s2: u64,
    /// x19 (s3) - saved register
    pub s3: u64,
    /// x20 (s4) - saved register
    pub s4: u64,
    /// x21 (s5) - saved register
    pub s5: u64,
    /// x22 (s6) - saved register
    pub s6: u64,
    /// x23 (s7) - saved register
    pub s7: u64,
    /// x24 (s8) - saved register
    pub s8: u64,
    /// x25 (s9) - saved register
    pub s9: u64,
    /// x26 (s10) - saved register
    pub s10: u64,
    /// x27 (s11) - saved register
    pub s11: u64,
    /// x28 (t3) - temporary
    pub t3: u64,
    /// x29 (t4) - temporary
    pub t4: u64,
    /// x30 (t5) - temporary
    pub t5: u64,
    /// x31 (t6) - temporary
    pub t6: u64,
}

impl GeneralRegisters {
    /// Create new zeroed register set
    pub const fn new() -> Self {
        Self {
            zero: 0, ra: 0, sp: 0, gp: 0, tp: 0,
            t0: 0, t1: 0, t2: 0,
            s0: 0, s1: 0,
            a0: 0, a1: 0, a2: 0, a3: 0, a4: 0, a5: 0, a6: 0, a7: 0,
            s2: 0, s3: 0, s4: 0, s5: 0, s6: 0, s7: 0, s8: 0, s9: 0, s10: 0, s11: 0,
            t3: 0, t4: 0, t5: 0, t6: 0,
        }
    }

    /// Get register by index
    pub fn get(&self, index: RegIndex) -> u64 {
        match index {
            0 => 0, // zero is always 0
            1 => self.ra,
            2 => self.sp,
            3 => self.gp,
            4 => self.tp,
            5 => self.t0,
            6 => self.t1,
            7 => self.t2,
            8 => self.s0,
            9 => self.s1,
            10 => self.a0,
            11 => self.a1,
            12 => self.a2,
            13 => self.a3,
            14 => self.a4,
            15 => self.a5,
            16 => self.a6,
            17 => self.a7,
            18 => self.s2,
            19 => self.s3,
            20 => self.s4,
            21 => self.s5,
            22 => self.s6,
            23 => self.s7,
            24 => self.s8,
            25 => self.s9,
            26 => self.s10,
            27 => self.s11,
            28 => self.t3,
            29 => self.t4,
            30 => self.t5,
            31 => self.t6,
            _ => 0,
        }
    }

    /// Set register by index (x0 writes are ignored)
    pub fn set(&mut self, index: RegIndex, value: u64) {
        match index {
            0 => {} // x0 is hardwired to 0
            1 => self.ra = value,
            2 => self.sp = value,
            3 => self.gp = value,
            4 => self.tp = value,
            5 => self.t0 = value,
            6 => self.t1 = value,
            7 => self.t2 = value,
            8 => self.s0 = value,
            9 => self.s1 = value,
            10 => self.a0 = value,
            11 => self.a1 = value,
            12 => self.a2 = value,
            13 => self.a3 = value,
            14 => self.a4 = value,
            15 => self.a5 = value,
            16 => self.a6 = value,
            17 => self.a7 = value,
            18 => self.s2 = value,
            19 => self.s3 = value,
            20 => self.s4 = value,
            21 => self.s5 = value,
            22 => self.s6 = value,
            23 => self.s7 = value,
            24 => self.s8 = value,
            25 => self.s9 = value,
            26 => self.s10 = value,
            27 => self.s11 = value,
            28 => self.t3 = value,
            29 => self.t4 = value,
            30 => self.t5 = value,
            31 => self.t6 = value,
            _ => {}
        }
    }

    /// Get frame pointer (alias for s0)
    pub fn fp(&self) -> u64 {
        self.s0
    }

    /// Set frame pointer (alias for s0)
    pub fn set_fp(&mut self, value: u64) {
        self.s0 = value;
    }
}

// ============================================================================
// Register Access Functions
// ============================================================================

/// Read stack pointer
#[inline]
pub fn read_sp() -> u64 {
    let value: u64;
    unsafe {
        asm!("mv {}, sp", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write stack pointer
///
/// # Safety
/// Changing SP can corrupt the stack if not done carefully.
#[inline]
pub unsafe fn write_sp(value: u64) {
    asm!("mv sp, {}", in(reg) value, options(nomem, nostack));
}

/// Read global pointer
#[inline]
pub fn read_gp() -> u64 {
    let value: u64;
    unsafe {
        asm!("mv {}, gp", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read thread pointer
#[inline]
pub fn read_tp() -> u64 {
    let value: u64;
    unsafe {
        asm!("mv {}, tp", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write thread pointer
///
/// # Safety
/// TP is used for TLS and per-hart data. Changing it affects all TLS accesses.
#[inline]
pub unsafe fn write_tp(value: u64) {
    asm!("mv tp, {}", in(reg) value, options(nomem, nostack));
}

/// Read return address
#[inline]
pub fn read_ra() -> u64 {
    let value: u64;
    unsafe {
        asm!("mv {}, ra", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

// ============================================================================
// Floating Point Registers (if F/D extension present)
// ============================================================================

/// Floating point registers (F extension: 32 x 32-bit, D extension: 32 x 64-bit)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FloatRegisters {
    /// f0-f31 (as 64-bit for D extension compatibility)
    pub f: [u64; 32],
    /// fcsr - Floating-point Control and Status Register
    pub fcsr: u32,
}

impl FloatRegisters {
    /// Create new zeroed FP register set
    pub const fn new() -> Self {
        Self {
            f: [0; 32],
            fcsr: 0,
        }
    }
}

impl Default for FloatRegisters {
    fn default() -> Self {
        Self::new()
    }
}

/// Read fcsr (floating-point control and status register)
#[inline]
pub fn read_fcsr() -> u32 {
    let value: u32;
    unsafe {
        asm!("frcsr {}", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write fcsr
#[inline]
pub fn write_fcsr(value: u32) {
    unsafe {
        asm!("fscsr {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read frm (rounding mode)
#[inline]
pub fn read_frm() -> u32 {
    let value: u32;
    unsafe {
        asm!("frrm {}", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write frm (rounding mode)
#[inline]
pub fn write_frm(value: u32) {
    unsafe {
        asm!("fsrm {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read fflags (exception flags)
#[inline]
pub fn read_fflags() -> u32 {
    let value: u32;
    unsafe {
        asm!("frflags {}", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write fflags (exception flags)
#[inline]
pub fn write_fflags(value: u32) {
    unsafe {
        asm!("fsflags {}", in(reg) value, options(nomem, nostack));
    }
}

// ============================================================================
// Rounding Modes
// ============================================================================

/// Floating-point rounding modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RoundingMode {
    /// Round to Nearest, ties to Even
    RoundNearestEven = 0b000,
    /// Round towards Zero
    RoundTowardsZero = 0b001,
    /// Round Down (towards -∞)
    RoundDown = 0b010,
    /// Round Up (towards +∞)
    RoundUp = 0b011,
    /// Round to Nearest, ties to Max Magnitude
    RoundNearestMaxMag = 0b100,
    /// Dynamic rounding mode (use frm)
    Dynamic = 0b111,
}

// ============================================================================
// Exception Flags
// ============================================================================

/// Floating-point exception flags
pub mod fflags {
    /// Inexact
    pub const NX: u32 = 1 << 0;
    /// Underflow
    pub const UF: u32 = 1 << 1;
    /// Overflow
    pub const OF: u32 = 1 << 2;
    /// Divide by Zero
    pub const DZ: u32 = 1 << 3;
    /// Invalid Operation
    pub const NV: u32 = 1 << 4;
}

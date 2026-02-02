//! # AArch64 Exception Vectors
//!
//! This module provides the exception vector table for AArch64.
//!
//! The vector table has 16 entries organized as:
//! - 4 entries for exceptions from current EL with SP_EL0
//! - 4 entries for exceptions from current EL with SP_ELx
//! - 4 entries for exceptions from lower EL using AArch64
//! - 4 entries for exceptions from lower EL using AArch32
//!
//! Each entry type: Synchronous, IRQ, FIQ, SError

use core::arch::asm;

use super::handlers::ExceptionHandler;

// =============================================================================
// Vector Table Constants
// =============================================================================

/// Size of each vector entry (128 bytes = 32 instructions)
pub const VECTOR_ENTRY_SIZE: usize = 0x80;

/// Total vector table size (16 entries × 128 bytes)
pub const VECTOR_TABLE_SIZE: usize = 16 * VECTOR_ENTRY_SIZE;

/// Vector table alignment requirement (2KB)
pub const VECTOR_TABLE_ALIGN: usize = 0x800;

// =============================================================================
// Vector Offsets
// =============================================================================

/// Exception vector offsets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum VectorOffset {
    // From current EL, SP_EL0
    /// Synchronous exception from current EL, SP_EL0
    CurrentElSp0Sync     = 0x000,
    /// IRQ from current EL, SP_EL0
    CurrentElSp0Irq      = 0x080,
    /// FIQ from current EL, SP_EL0
    CurrentElSp0Fiq      = 0x100,
    /// SError from current EL, SP_EL0
    CurrentElSp0Serror   = 0x180,

    // From current EL, SP_ELx
    /// Synchronous exception from current EL, SP_ELx
    CurrentElSpxSync     = 0x200,
    /// IRQ from current EL, SP_ELx
    CurrentElSpxIrq      = 0x280,
    /// FIQ from current EL, SP_ELx
    CurrentElSpxFiq      = 0x300,
    /// SError from current EL, SP_ELx
    CurrentElSpxSerror   = 0x380,

    // From lower EL, AArch64
    /// Synchronous exception from lower EL, AArch64
    LowerElAarch64Sync   = 0x400,
    /// IRQ from lower EL, AArch64
    LowerElAarch64Irq    = 0x480,
    /// FIQ from lower EL, AArch64
    LowerElAarch64Fiq    = 0x500,
    /// SError from lower EL, AArch64
    LowerElAarch64Serror = 0x580,

    // From lower EL, AArch32
    /// Synchronous exception from lower EL, AArch32
    LowerElAarch32Sync   = 0x600,
    /// IRQ from lower EL, AArch32
    LowerElAarch32Irq    = 0x680,
    /// FIQ from lower EL, AArch32
    LowerElAarch32Fiq    = 0x700,
    /// SError from lower EL, AArch32
    LowerElAarch32Serror = 0x780,
}

impl VectorOffset {
    /// Get the vector offset from raw value
    pub fn from_offset(offset: u16) -> Option<Self> {
        match offset {
            0x000 => Some(Self::CurrentElSp0Sync),
            0x080 => Some(Self::CurrentElSp0Irq),
            0x100 => Some(Self::CurrentElSp0Fiq),
            0x180 => Some(Self::CurrentElSp0Serror),
            0x200 => Some(Self::CurrentElSpxSync),
            0x280 => Some(Self::CurrentElSpxIrq),
            0x300 => Some(Self::CurrentElSpxFiq),
            0x380 => Some(Self::CurrentElSpxSerror),
            0x400 => Some(Self::LowerElAarch64Sync),
            0x480 => Some(Self::LowerElAarch64Irq),
            0x500 => Some(Self::LowerElAarch64Fiq),
            0x580 => Some(Self::LowerElAarch64Serror),
            0x600 => Some(Self::LowerElAarch32Sync),
            0x680 => Some(Self::LowerElAarch32Irq),
            0x700 => Some(Self::LowerElAarch32Fiq),
            0x780 => Some(Self::LowerElAarch32Serror),
            _ => None,
        }
    }

    /// Check if exception is synchronous
    pub fn is_sync(&self) -> bool {
        matches!(
            self,
            Self::CurrentElSp0Sync
                | Self::CurrentElSpxSync
                | Self::LowerElAarch64Sync
                | Self::LowerElAarch32Sync
        )
    }

    /// Check if exception is IRQ
    pub fn is_irq(&self) -> bool {
        matches!(
            self,
            Self::CurrentElSp0Irq
                | Self::CurrentElSpxIrq
                | Self::LowerElAarch64Irq
                | Self::LowerElAarch32Irq
        )
    }

    /// Check if exception is FIQ
    pub fn is_fiq(&self) -> bool {
        matches!(
            self,
            Self::CurrentElSp0Fiq
                | Self::CurrentElSpxFiq
                | Self::LowerElAarch64Fiq
                | Self::LowerElAarch32Fiq
        )
    }

    /// Check if exception is SError
    pub fn is_serror(&self) -> bool {
        matches!(
            self,
            Self::CurrentElSp0Serror
                | Self::CurrentElSpxSerror
                | Self::LowerElAarch64Serror
                | Self::LowerElAarch32Serror
        )
    }

    /// Check if exception is from lower EL
    pub fn is_from_lower_el(&self) -> bool {
        (*self as u16) >= 0x400
    }

    /// Check if exception uses SP_EL0
    pub fn uses_sp_el0(&self) -> bool {
        (*self as u16) < 0x200
    }
}

// =============================================================================
// Exception Vectors Structure
// =============================================================================

/// Exception handler function type
pub type VectorHandler = extern "C" fn();

/// Exception vectors configuration
#[derive(Clone, Copy)]
pub struct ExceptionVectors {
    /// Base address of vector table
    base: u64,
    /// Handler table
    handlers: [Option<ExceptionHandler>; 16],
}

impl ExceptionVectors {
    /// Create new exception vectors at given base address
    pub const fn new(base: u64) -> Self {
        Self {
            base,
            handlers: [None; 16],
        }
    }

    /// Get the base address
    pub const fn base(&self) -> u64 {
        self.base
    }

    /// Set handler for a specific vector
    pub fn set_handler(&mut self, offset: VectorOffset, handler: ExceptionHandler) {
        let index = (offset as u16 / VECTOR_ENTRY_SIZE as u16) as usize;
        if index < 16 {
            self.handlers[index] = Some(handler);
        }
    }

    /// Get handler for a specific vector
    pub fn get_handler(&self, offset: VectorOffset) -> Option<ExceptionHandler> {
        let index = (offset as u16 / VECTOR_ENTRY_SIZE as u16) as usize;
        if index < 16 {
            self.handlers[index]
        } else {
            None
        }
    }

    /// Install these vectors to VBAR_EL1
    pub fn install(&self) {
        write_vbar_el1(self.base);
    }
}

// =============================================================================
// VBAR Register Access
// =============================================================================

/// Read VBAR_EL1 (Vector Base Address Register)
#[inline]
pub fn read_vbar_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, VBAR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write VBAR_EL1
#[inline]
pub fn write_vbar_el1(addr: u64) {
    unsafe {
        asm!(
            "msr VBAR_EL1, {}",
            "isb",
            in(reg) addr,
            options(nomem, nostack)
        );
    }
}

/// Read VBAR_EL2
#[inline]
pub fn read_vbar_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, VBAR_EL2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write VBAR_EL2
#[inline]
pub fn write_vbar_el2(addr: u64) {
    unsafe {
        asm!(
            "msr VBAR_EL2, {}",
            "isb",
            in(reg) addr,
            options(nomem, nostack)
        );
    }
}

/// Read VBAR_EL3
#[inline]
pub fn read_vbar_el3() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, VBAR_EL3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write VBAR_EL3
#[inline]
pub fn write_vbar_el3(addr: u64) {
    unsafe {
        asm!(
            "msr VBAR_EL3, {}",
            "isb",
            in(reg) addr,
            options(nomem, nostack)
        );
    }
}

// =============================================================================
// High-Level API
// =============================================================================

/// Install exception vectors at the given address
///
/// # Safety
/// The address must point to a valid vector table that is:
/// - At least 2048 bytes (0x800)
/// - Aligned to 2048 bytes (0x800)
/// - Contains valid exception handler code
pub unsafe fn install_vectors(addr: u64) {
    // Verify alignment
    debug_assert!(
        addr & (VECTOR_TABLE_ALIGN as u64 - 1) == 0,
        "Vector table must be 2KB aligned"
    );

    write_vbar_el1(addr);
}

/// Get current vector table base address
pub fn current_vectors() -> u64 {
    read_vbar_el1()
}

// =============================================================================
// Vector Table Assembly Template
// =============================================================================

/// Generate vector table assembly
///
/// This is a template showing the expected structure of the vector table.
/// The actual implementation should be in a separate .S file.
#[cfg(doc)]
const VECTOR_TABLE_TEMPLATE: &str = r#"
.section .text.vectors
.global exception_vectors
.balign 0x800

exception_vectors:
    // Current EL with SP_EL0
    .balign 0x80
    b sync_current_el_sp0
    .balign 0x80
    b irq_current_el_sp0
    .balign 0x80
    b fiq_current_el_sp0
    .balign 0x80
    b serror_current_el_sp0

    // Current EL with SP_ELx
    .balign 0x80
    b sync_current_el_spx
    .balign 0x80
    b irq_current_el_spx
    .balign 0x80
    b fiq_current_el_spx
    .balign 0x80
    b serror_current_el_spx

    // Lower EL using AArch64
    .balign 0x80
    b sync_lower_el_aarch64
    .balign 0x80
    b irq_lower_el_aarch64
    .balign 0x80
    b fiq_lower_el_aarch64
    .balign 0x80
    b serror_lower_el_aarch64

    // Lower EL using AArch32
    .balign 0x80
    b sync_lower_el_aarch32
    .balign 0x80
    b irq_lower_el_aarch32
    .balign 0x80
    b fiq_lower_el_aarch32
    .balign 0x80
    b serror_lower_el_aarch32
"#;

// =============================================================================
// Exception Entry/Exit Macros (Rust representation)
// =============================================================================

/// Save all general-purpose registers to stack (trap frame)
///
/// Must be called at exception entry before any other code.
/// Creates a TrapFrame on the stack.
#[macro_export]
macro_rules! exception_entry {
    () => {
        // This would be assembly in practice:
        // sub sp, sp, #(36 * 8)    // Allocate TrapFrame
        // stp x0, x1, [sp, #0]
        // stp x2, x3, [sp, #16]
        // ... (save all registers)
        // mrs x0, ESR_EL1
        // mrs x1, FAR_EL1
        // stp x0, x1, [sp, #(34 * 8)]  // Save ESR, FAR
    };
}

/// Restore all general-purpose registers from stack
///
/// Must be called at exception exit to restore state and ERET.
#[macro_export]
macro_rules! exception_exit {
    () => {
        // This would be assembly in practice:
        // ldp x0, x1, [sp, #0]
        // ldp x2, x3, [sp, #16]
        // ... (restore all registers)
        // add sp, sp, #(36 * 8)    // Deallocate TrapFrame
        // eret
    };
}

// =============================================================================
// Vector Table Builder
// =============================================================================

/// Builder for creating exception vector tables
pub struct VectorTableBuilder {
    sync_handler: Option<extern "C" fn(*mut super::context::TrapFrame)>,
    irq_handler: Option<extern "C" fn(*mut super::context::TrapFrame)>,
    fiq_handler: Option<extern "C" fn(*mut super::context::TrapFrame)>,
    serror_handler: Option<extern "C" fn(*mut super::context::TrapFrame)>,
}

impl VectorTableBuilder {
    /// Create a new builder
    pub const fn new() -> Self {
        Self {
            sync_handler: None,
            irq_handler: None,
            fiq_handler: None,
            serror_handler: None,
        }
    }

    /// Set synchronous exception handler
    pub fn sync_handler(mut self, handler: extern "C" fn(*mut super::context::TrapFrame)) -> Self {
        self.sync_handler = Some(handler);
        self
    }

    /// Set IRQ handler
    pub fn irq_handler(mut self, handler: extern "C" fn(*mut super::context::TrapFrame)) -> Self {
        self.irq_handler = Some(handler);
        self
    }

    /// Set FIQ handler
    pub fn fiq_handler(mut self, handler: extern "C" fn(*mut super::context::TrapFrame)) -> Self {
        self.fiq_handler = Some(handler);
        self
    }

    /// Set SError handler
    pub fn serror_handler(
        mut self,
        handler: extern "C" fn(*mut super::context::TrapFrame),
    ) -> Self {
        self.serror_handler = Some(handler);
        self
    }
}

impl Default for VectorTableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

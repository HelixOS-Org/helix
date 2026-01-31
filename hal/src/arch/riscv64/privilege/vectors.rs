//! # RISC-V Trap Vector Management
//!
//! This module manages the trap vector table for RISC-V.
//!
//! ## Vector Modes
//!
//! RISC-V supports two trap vector modes controlled by stvec:
//!
//! - **Direct Mode** (mode=0): All traps jump to the same address (BASE)
//! - **Vectored Mode** (mode=1): Async interrupts jump to BASE + 4*cause
//!
//! ## Vector Table Layout (Vectored Mode)
//!
//! ```text
//! BASE + 0x00: Exception handler (all exceptions)
//! BASE + 0x04: Reserved
//! BASE + 0x08: Supervisor software interrupt
//! BASE + 0x0C: Reserved
//! BASE + 0x10: Reserved
//! BASE + 0x14: Supervisor timer interrupt
//! BASE + 0x18: Reserved
//! BASE + 0x1C: Reserved
//! BASE + 0x20: Supervisor external interrupt
//! ...
//! ```

use core::arch::asm;
use super::super::core::csr::{self, tvec};

// ============================================================================
// Trap Vector Configuration
// ============================================================================

/// Trap vector mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrapVectorMode {
    /// Direct mode - all traps go to BASE
    Direct = 0,
    /// Vectored mode - interrupts go to BASE + 4*cause
    Vectored = 1,
}

impl TrapVectorMode {
    pub const fn from_u8(value: u8) -> Self {
        match value & 0b11 {
            0 => Self::Direct,
            1 => Self::Vectored,
            _ => Self::Direct,
        }
    }
}

/// Trap vector configuration
#[derive(Debug, Clone, Copy)]
pub struct TrapVector {
    /// Base address of trap handler
    pub base: usize,
    /// Vector mode
    pub mode: TrapVectorMode,
}

impl TrapVector {
    /// Create a new trap vector in direct mode
    pub const fn direct(handler: usize) -> Self {
        Self {
            base: handler & !0b11,
            mode: TrapVectorMode::Direct,
        }
    }

    /// Create a new trap vector in vectored mode
    pub const fn vectored(table: usize) -> Self {
        Self {
            base: table & !0b11,
            mode: TrapVectorMode::Vectored,
        }
    }

    /// Convert to stvec register value
    pub const fn to_stvec(&self) -> u64 {
        (self.base as u64 & tvec::BASE_MASK) | (self.mode as u64)
    }

    /// Parse from stvec register value
    pub const fn from_stvec(stvec: u64) -> Self {
        Self {
            base: (stvec & tvec::BASE_MASK) as usize,
            mode: TrapVectorMode::from_u8((stvec & tvec::MODE_MASK) as u8),
        }
    }
}

// ============================================================================
// Vector Table
// ============================================================================

/// Maximum number of interrupt vectors
pub const MAX_VECTORS: usize = 16;

/// Vector table entry type
pub type VectorHandler = extern "C" fn();

/// Aligned vector table for vectored mode
///
/// Must be aligned to 4 bytes (or larger for implementations that require it).
#[repr(C, align(256))]
pub struct VectorTable {
    /// Vector entries - each is a jump to the actual handler
    pub entries: [u64; MAX_VECTORS],
}

impl VectorTable {
    /// Create an empty vector table
    pub const fn new() -> Self {
        Self {
            entries: [0; MAX_VECTORS],
        }
    }

    /// Get the base address of the table
    pub fn base(&self) -> usize {
        self as *const Self as usize
    }

    /// Set a vector entry
    pub fn set_vector(&mut self, index: usize, handler: usize) {
        if index < MAX_VECTORS {
            self.entries[index] = handler as u64;
        }
    }
}

impl Default for VectorTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Global Vector Table
// ============================================================================

/// Global vector table for supervisor mode
static mut VECTOR_TABLE: VectorTable = VectorTable::new();

/// Get reference to the global vector table
pub fn get_vector_table() -> &'static VectorTable {
    unsafe { &VECTOR_TABLE }
}

/// Get mutable reference to the global vector table
///
/// # Safety
/// Must ensure exclusive access (e.g., during init or with interrupts disabled)
pub unsafe fn get_vector_table_mut() -> &'static mut VectorTable {
    &mut VECTOR_TABLE
}

// ============================================================================
// Trap Vector Setup
// ============================================================================

/// Set the trap vector (stvec)
pub fn set_trap_vector(vector: TrapVector) {
    csr::write_stvec(vector.to_stvec());
}

/// Get the current trap vector
pub fn get_trap_vector() -> TrapVector {
    TrapVector::from_stvec(csr::read_stvec())
}

/// Set trap handler in direct mode
pub fn set_trap_handler(handler: usize) {
    let vector = TrapVector::direct(handler);
    set_trap_vector(vector);
}

/// Set trap handler in vectored mode
pub fn set_vectored_trap_handler(table: usize) {
    let vector = TrapVector::vectored(table);
    set_trap_vector(vector);
}

/// Initialize trap handling
pub fn init_trap_vectors(handler: extern "C" fn()) {
    // Use direct mode with a single handler
    set_trap_handler(handler as usize);
}

/// Initialize vectored trap handling
pub fn init_vectored_traps() {
    let table = get_vector_table();
    set_vectored_trap_handler(table.base());
}

// ============================================================================
// Interrupt Vector Indices
// ============================================================================

/// Interrupt vector indices for vectored mode
pub mod vector_index {
    /// Supervisor software interrupt
    pub const SSI: usize = 1;
    /// Supervisor timer interrupt
    pub const STI: usize = 5;
    /// Supervisor external interrupt
    pub const SEI: usize = 9;
}

// ============================================================================
// Exception Vector (for vectored mode, exceptions still go to base)
// ============================================================================

/// Register exception handler (called for all exceptions in vectored mode)
pub fn register_exception_handler(handler: VectorHandler) {
    unsafe {
        get_vector_table_mut().set_vector(0, handler as usize);
    }
}

/// Register supervisor software interrupt handler
pub fn register_ssi_handler(handler: VectorHandler) {
    unsafe {
        get_vector_table_mut().set_vector(vector_index::SSI, handler as usize);
    }
}

/// Register supervisor timer interrupt handler
pub fn register_sti_handler(handler: VectorHandler) {
    unsafe {
        get_vector_table_mut().set_vector(vector_index::STI, handler as usize);
    }
}

/// Register supervisor external interrupt handler
pub fn register_sei_handler(handler: VectorHandler) {
    unsafe {
        get_vector_table_mut().set_vector(vector_index::SEI, handler as usize);
    }
}

// ============================================================================
// Default Handlers
// ============================================================================

/// Default trap handler (direct mode)
///
/// This is the main entry point for all traps in direct mode.
#[naked]
pub unsafe extern "C" fn default_trap_handler() {
    asm!(
        // Save context on stack
        "addi sp, sp, -256",

        // Save general purpose registers
        "sd ra, 8(sp)",
        "sd gp, 24(sp)",
        "sd tp, 32(sp)",
        "sd t0, 40(sp)",
        "sd t1, 48(sp)",
        "sd t2, 56(sp)",
        "sd s0, 64(sp)",
        "sd s1, 72(sp)",
        "sd a0, 80(sp)",
        "sd a1, 88(sp)",
        "sd a2, 96(sp)",
        "sd a3, 104(sp)",
        "sd a4, 112(sp)",
        "sd a5, 120(sp)",
        "sd a6, 128(sp)",
        "sd a7, 136(sp)",
        "sd s2, 144(sp)",
        "sd s3, 152(sp)",
        "sd s4, 160(sp)",
        "sd s5, 168(sp)",
        "sd s6, 176(sp)",
        "sd s7, 184(sp)",
        "sd s8, 192(sp)",
        "sd s9, 200(sp)",
        "sd s10, 208(sp)",
        "sd s11, 216(sp)",
        "sd t3, 224(sp)",
        "sd t4, 232(sp)",
        "sd t5, 240(sp)",
        "sd t6, 248(sp)",

        // Save CSRs
        "csrr t0, sepc",
        "csrr t1, sstatus",
        "csrr t2, scause",
        "csrr t3, stval",

        // Call Rust handler with frame pointer
        "mv a0, sp",
        "call {handler}",

        // Restore CSRs
        "ld t0, 256(sp)",  // sepc
        "ld t1, 264(sp)",  // sstatus
        "csrw sepc, t0",
        "csrw sstatus, t1",

        // Restore general purpose registers
        "ld ra, 8(sp)",
        "ld gp, 24(sp)",
        "ld tp, 32(sp)",
        "ld t0, 40(sp)",
        "ld t1, 48(sp)",
        "ld t2, 56(sp)",
        "ld s0, 64(sp)",
        "ld s1, 72(sp)",
        "ld a0, 80(sp)",
        "ld a1, 88(sp)",
        "ld a2, 96(sp)",
        "ld a3, 104(sp)",
        "ld a4, 112(sp)",
        "ld a5, 120(sp)",
        "ld a6, 128(sp)",
        "ld a7, 136(sp)",
        "ld s2, 144(sp)",
        "ld s3, 152(sp)",
        "ld s4, 160(sp)",
        "ld s5, 168(sp)",
        "ld s6, 176(sp)",
        "ld s7, 184(sp)",
        "ld s8, 192(sp)",
        "ld s9, 200(sp)",
        "ld s10, 208(sp)",
        "ld s11, 216(sp)",
        "ld t3, 224(sp)",
        "ld t4, 232(sp)",
        "ld t5, 240(sp)",
        "ld t6, 248(sp)",

        // Restore stack pointer and return
        "addi sp, sp, 256",
        "sret",

        handler = sym trap_handler_rust,
        options(noreturn)
    );
}

/// Rust trap handler called from assembly
extern "C" fn trap_handler_rust(frame: &mut super::traps::TrapFrame) {
    let _ = super::traps::handle_trap(frame);
}

// ============================================================================
// Trap Vector Assembly Stubs
// ============================================================================

/// Generate a vector stub for vectored mode
macro_rules! vector_stub {
    ($name:ident, $cause:expr) => {
        #[naked]
        #[allow(dead_code)]
        unsafe extern "C" fn $name() {
            asm!(
                // Push cause and jump to common handler
                "li a0, {cause}",
                "j {common}",
                cause = const $cause,
                common = sym vector_common,
                options(noreturn)
            );
        }
    };
}

/// Common vector handler
#[naked]
unsafe extern "C" fn vector_common() {
    asm!(
        // Save minimal context and call handler
        // a0 already contains the cause
        "j {handler}",
        handler = sym default_trap_handler,
        options(noreturn)
    );
}

// Generate stubs for each vector
vector_stub!(vec_ssi, 1);  // Supervisor software interrupt
vector_stub!(vec_sti, 5);  // Supervisor timer interrupt
vector_stub!(vec_sei, 9);  // Supervisor external interrupt

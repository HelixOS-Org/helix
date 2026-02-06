//! # x86_64 Interrupt Descriptor Table (IDT) Framework
//!
//! This module provides an industrial-grade IDT implementation for x86_64
//! systems, with comprehensive interrupt handling, IST support, and proper
//! vector allocation for SMP environments.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                       Interrupt Flow                                 │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                      │
//! │   Hardware/Software ──> CPU ──> IDT Lookup ──> Gate Handler         │
//! │                          │                           │               │
//! │                          └── IST Stack Switch ◄──────┘               │
//! │                                     │                                │
//! │                          ┌──────────┴──────────┐                    │
//! │                          ▼                     ▼                    │
//! │                    Exception Handler    Interrupt Handler           │
//! │                          │                     │                    │
//! │                    Error Code?           EOI to APIC               │
//! │                          │                     │                    │
//! │                          ▼                     ▼                    │
//! │                    Kernel Panic          User Handler               │
//! │                    or Recovery                                      │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Vector Allocation
//!
//! - `0x00-0x1F`: CPU Exceptions (reserved by Intel)
//! - `0x20-0x2F`: Legacy PIC IRQs (remapped from 0x00-0x0F)
//! - `0x30-0x3F`: System Calls and IPI
//! - `0x40-0xEF`: Device interrupts (APIC/MSI)
//! - `0xF0-0xFE`: Reserved for future use
//! - `0xFF`: Spurious interrupt
//!
//! ## IST (Interrupt Stack Table) Assignment
//!
//! - IST1: Double Fault (#DF) - dedicated 16KB stack
//! - IST2: NMI - dedicated 8KB stack
//! - IST3: Machine Check (#MC) - dedicated 8KB stack
//! - IST4: Debug (#DB) - dedicated 8KB stack
//! - IST5-7: Reserved for kernel use
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use hal::arch::x86_64::interrupts;
//!
//! // Initialize IDT for BSP
//! unsafe { interrupts::init(); }
//!
//! // Register a custom interrupt handler
//! interrupts::register_handler(
//!     interrupts::Vector::User(0x50),
//!     my_handler,
//!     interrupts::HandlerType::Interrupt,
//! );
//!
//! // Enable interrupts
//! interrupts::enable();
//! ```

#![allow(dead_code)]

mod entries;
mod frame;
mod handlers;
mod idt;
mod vectors;

use core::sync::atomic::{AtomicBool, Ordering};

pub use entries::{Dpl, GateOptions, GateType, IdtEntry};
pub use frame::InterruptStackFrame;
pub use handlers::{ExceptionHandlerFn, HandlerFn};
pub use idt::{load_idt, Idt, IdtDescriptor};
pub use vectors::{ExceptionVector, IrqVector, SystemVector, Vector};

use super::segmentation;

// =============================================================================
// Constants
// =============================================================================

/// Number of IDT entries (256 vectors)
pub const IDT_ENTRIES: usize = 256;

/// Maximum number of CPUs supported
pub const MAX_CPUS: usize = 256;

/// Start of exception vector range (0x00)
pub const EXCEPTION_START: u8 = 0x00;
/// End of exception vector range (0x1F)
pub const EXCEPTION_END: u8 = 0x1F;

/// Start of legacy PIC IRQ vector range (0x20)
pub const PIC_IRQ_START: u8 = 0x20;
/// End of legacy PIC IRQ vector range (0x2F)
pub const PIC_IRQ_END: u8 = 0x2F;

/// Start of system vector range for IPIs and syscalls (0x30)
pub const SYSTEM_VECTOR_START: u8 = 0x30;
/// End of system vector range (0x3F)
pub const SYSTEM_VECTOR_END: u8 = 0x3F;

/// Start of APIC/device interrupt vector range (0x40)
pub const DEVICE_VECTOR_START: u8 = 0x40;
/// End of APIC/device interrupt vector range (0xEF)
pub const DEVICE_VECTOR_END: u8 = 0xEF;

/// Spurious interrupt vector
pub const SPURIOUS_VECTOR: u8 = 0xFF;

/// APIC Timer interrupt vector (commonly used for scheduling)
pub const APIC_TIMER_VECTOR: u8 = 0x40;

/// APIC Error interrupt vector for handling APIC errors
pub const APIC_ERROR_VECTOR: u8 = 0xFE;

/// IPI vector for triggering reschedule on another CPU
pub const IPI_RESCHEDULE_VECTOR: u8 = 0x30;
/// IPI vector for TLB shootdown across CPUs
pub const IPI_TLB_SHOOTDOWN_VECTOR: u8 = 0x31;
/// IPI vector for halting a CPU
pub const IPI_HALT_VECTOR: u8 = 0x32;
/// IPI vector for remote function calls between CPUs
pub const IPI_CALL_FUNCTION_VECTOR: u8 = 0x33;

/// System call vector used with INT instruction as SYSCALL/SYSENTER fallback
pub const SYSCALL_VECTOR: u8 = 0x80;

// =============================================================================
// Initialization State
// =============================================================================

/// Tracks whether the IDT has been initialized (prevents double initialization)
static IDT_INITIALIZED: AtomicBool = AtomicBool::new(false);

// =============================================================================
// Public Interface
// =============================================================================

/// Initialize the IDT for the Bootstrap Processor (BSP)
///
/// This function sets up the master IDT with all exception handlers,
/// default interrupt handlers, and loads it into the CPU.
///
/// # Safety
///
/// This function must be called exactly once during early boot,
/// before enabling interrupts.
///
/// # Panics
///
/// Panics if called more than once.
#[inline]
pub unsafe fn init() {
    if IDT_INITIALIZED.swap(true, Ordering::SeqCst) {
        panic!("IDT already initialized");
    }

    // Initialize the static IDT
    unsafe { idt::init_idt() };

    // Load the IDT
    unsafe { idt::load_idt() };

    log::info!("IDT: Initialized with {} entries", IDT_ENTRIES);
}

/// Initialize the IDT for an Application Processor (AP)
///
/// This loads the already-initialized IDT into the AP's IDTR.
/// The IDT is shared across all CPUs.
///
/// # Safety
///
/// Must be called after `init()` has completed on the BSP.
#[inline]
pub unsafe fn init_for_ap() {
    debug_assert!(IDT_INITIALIZED.load(Ordering::Acquire));
    unsafe { idt::load_idt() };
}

/// Enable interrupts on the current CPU
///
/// # Safety
///
/// Caller must ensure interrupt handlers are properly set up.
#[inline]
pub unsafe fn enable() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Disable interrupts on the current CPU
///
/// Returns the previous interrupt state (true if interrupts were enabled).
///
/// # Safety
///
/// This is always safe to call but enabling interrupts again requires care.
#[inline]
pub fn disable() -> bool {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {0}",
            "cli",
            out(reg) flags,
            options(nomem, preserves_flags),
        );
    }
    flags & (1 << 9) != 0 // IF flag
}

/// Check if interrupts are enabled on the current CPU
#[inline]
pub fn are_enabled() -> bool {
    let flags: u64;
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {0}",
            out(reg) flags,
            options(nomem, nostack, preserves_flags),
        );
    }
    flags & (1 << 9) != 0
}

/// Execute a closure with interrupts disabled
///
/// Restores the previous interrupt state after the closure returns.
#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_enabled = disable();
    let result = f();
    if was_enabled {
        unsafe {
            enable();
        }
    }
    result
}

/// Halt the CPU until the next interrupt
///
/// # Safety
///
/// Interrupts must be enabled for this to ever return.
#[inline]
pub unsafe fn halt() {
    unsafe {
        core::arch::asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Halt the CPU forever (infinite loop with hlt)
///
/// This is useful for fatal errors.
#[inline]
pub fn halt_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack));
        }
    }
}

/// Send End-Of-Interrupt signal
///
/// This is a placeholder - actual implementation depends on APIC configuration.
///
/// # Safety
///
/// - Must be called only after properly handling an interrupt.
/// - The vector must match the interrupt being acknowledged.
#[inline]
pub unsafe fn end_of_interrupt(_vector: u8) {
    // TODO: Send EOI to Local APIC
    // For now, this is handled by the APIC module
}

/// Register a custom interrupt handler
///
/// # Arguments
///
/// * `vector` - The interrupt vector number
/// * `handler` - Function pointer to the handler
/// * `gate_type` - Whether this is an interrupt or trap gate
///
/// # Safety
///
/// The handler must be a valid function that properly handles the interrupt.
pub unsafe fn register_handler(vector: u8, handler: usize, gate_type: GateType) {
    unsafe {
        idt::set_handler(vector, handler, gate_type);
    }
}

/// Register an exception handler with IST
///
/// # Safety
///
/// The handler must be a valid exception handler.
pub unsafe fn register_exception_handler(vector: u8, handler: usize, ist: u8) {
    unsafe {
        idt::set_exception_handler(vector, handler, ist);
    }
}

// =============================================================================
// Interrupt State Guard
// =============================================================================

/// RAII guard for interrupt state
///
/// Disables interrupts when created, restores previous state when dropped.
pub struct InterruptGuard {
    /// Whether interrupts were enabled before this guard was created
    was_enabled: bool,
}

impl InterruptGuard {
    /// Create a new interrupt guard, disabling interrupts
    pub fn new() -> Self {
        Self {
            was_enabled: disable(),
        }
    }
}

impl Default for InterruptGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if self.was_enabled {
            unsafe {
                enable();
            }
        }
    }
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    // Verify vector ranges don't overlap incorrectly
    assert!(EXCEPTION_END < PIC_IRQ_START);
    assert!(PIC_IRQ_END < SYSTEM_VECTOR_START);
    assert!(SYSTEM_VECTOR_END < DEVICE_VECTOR_START);
};

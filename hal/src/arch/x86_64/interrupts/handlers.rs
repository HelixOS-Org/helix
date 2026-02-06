//! # Interrupt Handlers
//!
//! This module provides the default exception handlers and the
//! infrastructure for registering custom interrupt handlers.
//!
//! ## Handler Types
//!
//! - **Exception Handlers**: Handle CPU exceptions (faults, traps, aborts)
//! - **Interrupt Handlers**: Handle hardware/software interrupts
//! - **IPI Handlers**: Handle inter-processor interrupts
//!
//! ## Handler Registration
//!
//! Custom handlers can be registered per-vector using the handler table.

use core::sync::atomic::{AtomicPtr, Ordering};

use super::frame::{ExceptionStackFrame, InterruptStackFrame, PageFaultErrorCode};
use super::vectors::ExceptionVector;

// =============================================================================
// Handler Function Types
// =============================================================================

/// Simple interrupt handler (no return value)
pub type HandlerFn = extern "x86-interrupt" fn(InterruptStackFrame);

/// Exception handler without error code
pub type ExceptionHandlerFn = extern "x86-interrupt" fn(InterruptStackFrame);

/// Exception handler with error code
pub type ExceptionHandlerWithErrorFn = extern "x86-interrupt" fn(ExceptionStackFrame);

/// Diverging exception handler (for aborts like double fault)
pub type DivergingHandlerFn = extern "x86-interrupt" fn(InterruptStackFrame) -> !;

/// Diverging exception handler with error code
pub type DivergingHandlerWithErrorFn = extern "x86-interrupt" fn(ExceptionStackFrame) -> !;

/// Generic handler function pointer (for handler table)
pub type RawHandlerFn = unsafe extern "C" fn();

// =============================================================================
// Handler Table
// =============================================================================

/// Number of interrupt vectors
const NUM_VECTORS: usize = 256;

/// Handler table for all interrupt vectors
///
/// Each entry is an atomic pointer to a handler function.
/// NULL means use the default handler.
static HANDLER_TABLE: [AtomicPtr<()>; NUM_VECTORS] = {
    #[allow(clippy::declare_interior_mutable_const)]
    const NULL_PTR: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());
    [NULL_PTR; NUM_VECTORS]
};

/// Register a handler for a specific vector
///
/// # Safety
///
/// The handler must be a valid function pointer that correctly
/// handles the interrupt.
pub unsafe fn register_handler(vector: u8, handler: usize) {
    HANDLER_TABLE[vector as usize].store(handler as *mut (), Ordering::Release);
}

/// Get the registered handler for a vector
pub fn get_handler(vector: u8) -> Option<usize> {
    let ptr = HANDLER_TABLE[vector as usize].load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(ptr as usize)
    }
}

/// Clear a handler registration
pub fn clear_handler(vector: u8) {
    HANDLER_TABLE[vector as usize].store(core::ptr::null_mut(), Ordering::Release);
}

// =============================================================================
// Default Exception Handlers
// =============================================================================

/// Division Error Handler (#DE)
pub extern "x86-interrupt" fn divide_error_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: Divide Error (#DE)");
    log::error!("{:?}", frame);
    panic!("Divide by zero at {:#x}", frame.rip);
}

/// Debug Exception Handler (#DB)
pub extern "x86-interrupt" fn debug_handler(frame: InterruptStackFrame) {
    log::warn!("EXCEPTION: Debug (#DB)");
    log::debug!("{:?}", frame);
    // Debug exceptions are typically handled by a debugger
    // For now, just log and continue
}

/// Non-Maskable Interrupt Handler
pub extern "x86-interrupt" fn nmi_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: Non-Maskable Interrupt (NMI)");
    log::error!("{:?}", frame);
    // NMI could be a hardware failure, parity error, etc.
    // In a production system, this should trigger diagnostics
}

/// Breakpoint Handler (#BP)
pub extern "x86-interrupt" fn breakpoint_handler(frame: InterruptStackFrame) {
    log::info!("EXCEPTION: Breakpoint (#BP) at {:#x}", frame.rip);
    log::debug!("{:?}", frame);
    // Breakpoints are typically used for debugging
}

/// Overflow Handler (#OF)
pub extern "x86-interrupt" fn overflow_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: Overflow (#OF)");
    log::error!("{:?}", frame);
    panic!("Overflow at {:#x}", frame.rip);
}

/// Bound Range Exceeded Handler (#BR)
pub extern "x86-interrupt" fn bound_range_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: Bound Range Exceeded (#BR)");
    log::error!("{:?}", frame);
    panic!("Bound range exceeded at {:#x}", frame.rip);
}

/// Invalid Opcode Handler (#UD)
pub extern "x86-interrupt" fn invalid_opcode_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: Invalid Opcode (#UD)");
    log::error!("{:?}", frame);
    panic!("Invalid opcode at {:#x}", frame.rip);
}

/// Device Not Available Handler (#NM)
pub extern "x86-interrupt" fn device_not_available_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: Device Not Available (#NM)");
    log::error!("{:?}", frame);
    // This typically means FPU/SSE instructions used without proper setup
    // Could implement lazy FPU switching here
    panic!("FPU/SSE not available at {:#x}", frame.rip);
}

/// Double Fault Handler (#DF)
///
/// This is a diverging handler because a double fault cannot be recovered from.
pub extern "x86-interrupt" fn double_fault_handler(frame: ExceptionStackFrame) -> ! {
    log::error!("========================================");
    log::error!("FATAL: Double Fault (#DF)");
    log::error!("========================================");
    log::error!("Error Code: {:#x}", frame.error_code);
    log::error!("RIP: {:#x}", frame.rip);
    log::error!("CS:  {:#x}", frame.cs);
    log::error!("RSP: {:#x}", frame.rsp);
    log::error!("SS:  {:#x}", frame.ss);
    log::error!("RFLAGS: {:#x}", frame.rflags);
    log::error!("========================================");

    // Double fault is unrecoverable
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

/// Invalid TSS Handler (#TS)
pub extern "x86-interrupt" fn invalid_tss_handler(frame: ExceptionStackFrame) {
    log::error!("EXCEPTION: Invalid TSS (#TS)");
    log::error!("Error Code: {:#x}", frame.error_code);
    log::error!("{:?}", frame.as_interrupt_frame());
    panic!("Invalid TSS at {:#x}", frame.rip);
}

/// Segment Not Present Handler (#NP)
pub extern "x86-interrupt" fn segment_not_present_handler(frame: ExceptionStackFrame) {
    log::error!("EXCEPTION: Segment Not Present (#NP)");
    log::error!("Selector: {:#x}", frame.error_code);
    log::error!("{:?}", frame.as_interrupt_frame());
    panic!("Segment not present at {:#x}", frame.rip);
}

/// Stack Segment Fault Handler (#SS)
pub extern "x86-interrupt" fn stack_segment_handler(frame: ExceptionStackFrame) {
    log::error!("EXCEPTION: Stack Segment Fault (#SS)");
    log::error!("Error Code: {:#x}", frame.error_code);
    log::error!("{:?}", frame.as_interrupt_frame());
    panic!("Stack segment fault at {:#x}", frame.rip);
}

/// General Protection Fault Handler (#GP)
pub extern "x86-interrupt" fn general_protection_handler(frame: ExceptionStackFrame) {
    log::error!("EXCEPTION: General Protection Fault (#GP)");
    log::error!("Error Code: {:#x}", frame.error_code);
    log::error!("{:?}", frame.as_interrupt_frame());

    if frame.error_code != 0 {
        let external = frame.error_code & 1 != 0;
        let table = (frame.error_code >> 1) & 0x3;
        let index = (frame.error_code >> 3) & 0x1FFF;

        let table_name = match table {
            0 => "GDT",
            1 | 3 => "IDT",
            2 => "LDT",
            _ => "???",
        };

        log::error!(
            "  Selector: {} index {}, external={}",
            table_name,
            index,
            external
        );
    }

    panic!("General protection fault at {:#x}", frame.rip);
}

/// Page Fault Handler (#PF)
pub extern "x86-interrupt" fn page_fault_handler(frame: ExceptionStackFrame) {
    // Get the faulting address from CR2
    let faulting_address: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) faulting_address, options(nomem, nostack));
    }

    let error = PageFaultErrorCode::from_bits_truncate(frame.error_code);

    log::error!("EXCEPTION: Page Fault (#PF)");
    log::error!("  Faulting Address: {:#018x}", faulting_address);
    log::error!("  Error: {}", error);
    log::error!("{:?}", frame.as_interrupt_frame());

    // In a real kernel, we would handle the page fault here
    // (lazy allocation, copy-on-write, swap in, etc.)

    panic!(
        "Page fault at {:#x} accessing {:#x}",
        frame.rip, faulting_address
    );
}

/// x87 FPU Error Handler (#MF)
pub extern "x86-interrupt" fn x87_fpu_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: x87 FPU Error (#MF)");
    log::error!("{:?}", frame);
    panic!("x87 FPU error at {:#x}", frame.rip);
}

/// Alignment Check Handler (#AC)
pub extern "x86-interrupt" fn alignment_check_handler(frame: ExceptionStackFrame) {
    log::error!("EXCEPTION: Alignment Check (#AC)");
    log::error!("Error Code: {:#x}", frame.error_code);
    log::error!("{:?}", frame.as_interrupt_frame());
    panic!("Alignment check at {:#x}", frame.rip);
}

/// Machine Check Handler (#MC)
///
/// This is a diverging handler because machine check typically indicates
/// hardware failure.
pub extern "x86-interrupt" fn machine_check_handler(frame: InterruptStackFrame) -> ! {
    log::error!("========================================");
    log::error!("FATAL: Machine Check Exception (#MC)");
    log::error!("========================================");
    log::error!("{:?}", frame);
    log::error!("========================================");

    // Machine check is typically unrecoverable
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

/// SIMD Floating-Point Handler (#XM)
pub extern "x86-interrupt" fn simd_floating_point_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: SIMD Floating-Point (#XM)");
    log::error!("{:?}", frame);
    panic!("SIMD floating-point exception at {:#x}", frame.rip);
}

/// Virtualization Exception Handler (#VE)
pub extern "x86-interrupt" fn virtualization_handler(frame: InterruptStackFrame) {
    log::error!("EXCEPTION: Virtualization Exception (#VE)");
    log::error!("{:?}", frame);
    panic!("Virtualization exception at {:#x}", frame.rip);
}

/// Control Protection Exception Handler (#CP)
pub extern "x86-interrupt" fn control_protection_handler(frame: ExceptionStackFrame) {
    log::error!("EXCEPTION: Control Protection (#CP)");
    log::error!("Error Code: {:#x}", frame.error_code);
    log::error!("{:?}", frame.as_interrupt_frame());
    panic!("Control protection exception at {:#x}", frame.rip);
}

/// Security Exception Handler (#SX)
pub extern "x86-interrupt" fn security_handler(frame: ExceptionStackFrame) {
    log::error!("EXCEPTION: Security Exception (#SX)");
    log::error!("Error Code: {:#x}", frame.error_code);
    log::error!("{:?}", frame.as_interrupt_frame());
    panic!("Security exception at {:#x}", frame.rip);
}

// =============================================================================
// Default Interrupt Handler
// =============================================================================

/// Default handler for unexpected interrupts
pub extern "x86-interrupt" fn default_interrupt_handler(frame: InterruptStackFrame) {
    log::warn!("Unexpected interrupt at {:#x}", frame.rip);
    log::debug!("{:?}", frame);
    // Just return - this was an unexpected but non-fatal interrupt
}

/// Spurious interrupt handler
pub extern "x86-interrupt" fn spurious_interrupt_handler(_frame: InterruptStackFrame) {
    // Spurious interrupts should be ignored
    // Don't send EOI for spurious interrupts
    log::trace!("Spurious interrupt");
}

// =============================================================================
// IPI Handlers
// =============================================================================

/// IPI: Reschedule request
pub extern "x86-interrupt" fn ipi_reschedule_handler(_frame: InterruptStackFrame) {
    // TODO: Trigger scheduler on this CPU
    log::trace!("IPI: Reschedule request");
    // Send EOI (will be done by APIC module)
}

/// IPI: TLB shootdown
pub extern "x86-interrupt" fn ipi_tlb_shootdown_handler(_frame: InterruptStackFrame) {
    // TODO: Invalidate TLB entries as requested
    log::trace!("IPI: TLB shootdown");
    // Send EOI
}

/// IPI: Halt processor
pub extern "x86-interrupt" fn ipi_halt_handler(_frame: InterruptStackFrame) -> ! {
    log::info!("IPI: Halt received, stopping CPU");
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

// =============================================================================
// Handler Stubs (for assembly wrappers)
// =============================================================================

/// Get the address of a default exception handler
pub fn get_default_exception_handler(vector: ExceptionVector) -> usize {
    match vector {
        ExceptionVector::DivideError => divide_error_handler as usize,
        ExceptionVector::Debug => debug_handler as usize,
        ExceptionVector::NonMaskableInterrupt => nmi_handler as usize,
        ExceptionVector::Breakpoint => breakpoint_handler as usize,
        ExceptionVector::Overflow => overflow_handler as usize,
        ExceptionVector::BoundRangeExceeded => bound_range_handler as usize,
        ExceptionVector::InvalidOpcode => invalid_opcode_handler as usize,
        ExceptionVector::DeviceNotAvailable => device_not_available_handler as usize,
        ExceptionVector::DoubleFault => double_fault_handler as usize,
        ExceptionVector::InvalidTss => invalid_tss_handler as usize,
        ExceptionVector::SegmentNotPresent => segment_not_present_handler as usize,
        ExceptionVector::StackSegmentFault => stack_segment_handler as usize,
        ExceptionVector::GeneralProtection => general_protection_handler as usize,
        ExceptionVector::PageFault => page_fault_handler as usize,
        ExceptionVector::X87FloatingPoint => x87_fpu_handler as usize,
        ExceptionVector::AlignmentCheck => alignment_check_handler as usize,
        ExceptionVector::MachineCheck => machine_check_handler as usize,
        ExceptionVector::SimdFloatingPoint => simd_floating_point_handler as usize,
        ExceptionVector::VirtualizationException => virtualization_handler as usize,
        ExceptionVector::ControlProtection => control_protection_handler as usize,
        ExceptionVector::SecurityException => security_handler as usize,
        _ => default_interrupt_handler as usize,
    }
}

/// Check if an exception vector pushes an error code
pub const fn exception_has_error_code(vector: u8) -> bool {
    matches!(
        vector,
        0x08 | 0x0A | 0x0B | 0x0C | 0x0D | 0x0E | 0x11 | 0x15 | 0x1D | 0x1E
    )
}

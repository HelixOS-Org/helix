//! # Interrupt Handlers
//!
//! Common interrupt handlers.

use core::sync::atomic::{AtomicU64, Ordering};

use helix_hal::interrupts::InterruptVector;

/// Timer interrupt handler statistics
pub static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

/// Handle timer interrupt
pub fn timer_handler(_vector: InterruptVector) {
    let ticks = TIMER_TICKS.fetch_add(1, Ordering::Relaxed);

    // Every 100 ticks, check for rescheduling
    if ticks % 100 == 0 {
        // TODO: Trigger scheduler
    }
}

/// Handle spurious interrupt
pub fn spurious_handler(vector: InterruptVector) {
    log::trace!("Spurious interrupt: {}", vector);
}

/// Handle page fault
pub fn page_fault_handler(_vector: InterruptVector) {
    // TODO: Get fault information from HAL
    // TODO: Dispatch to memory subsystem
    log::error!("Page fault occurred");
}

/// Handle general protection fault
pub fn gpf_handler(_vector: InterruptVector) {
    log::error!("General protection fault");
    // TODO: Kill the faulting process or panic if in kernel mode
}

/// Handle double fault
pub fn double_fault_handler(_vector: InterruptVector) -> ! {
    log::error!("DOUBLE FAULT - System halted");
    loop {
        core::hint::spin_loop();
    }
}

/// Handle breakpoint
pub fn breakpoint_handler(_vector: InterruptVector) {
    log::debug!("Breakpoint hit");
    // TODO: Notify debugger
}

/// Handle invalid opcode
pub fn invalid_opcode_handler(_vector: InterruptVector) {
    log::error!("Invalid opcode");
    // TODO: Kill the faulting process
}

/// Handle device not available (FPU)
pub fn device_not_available_handler(_vector: InterruptVector) {
    // TODO: Lazy FPU context switching
    log::trace!("Device not available - loading FPU context");
}

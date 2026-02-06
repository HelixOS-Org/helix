//! # IRQ Handlers
//!
//! Hardware interrupt handlers for timer, keyboard, etc.

use core::arch::{asm, naked_asm};

use super::pic::{self, Irq};
use super::{pit, task};

/// Interrupt frame pushed by CPU on interrupt
#[repr(C)]
pub struct InterruptFrame {
    /// Instruction pointer at time of interrupt
    pub rip: u64,
    /// Code segment selector
    pub cs: u64,
    /// CPU flags register
    pub rflags: u64,
    /// Stack pointer at time of interrupt
    pub rsp: u64,
    /// Stack segment selector
    pub ss: u64,
}

/// Saved registers during interrupt
#[repr(C)]
pub struct SavedRegs {
    /// Accumulator register
    pub rax: u64,
    /// Base register (callee-saved)
    pub rbx: u64,
    /// Counter register
    pub rcx: u64,
    /// Data register
    pub rdx: u64,
    /// Source index register
    pub rsi: u64,
    /// Destination index register
    pub rdi: u64,
    /// Base pointer register (callee-saved)
    pub rbp: u64,
    /// Extended register 8
    pub r8: u64,
    /// Extended register 9
    pub r9: u64,
    /// Extended register 10
    pub r10: u64,
    /// Extended register 11
    pub r11: u64,
    /// Extended register 12 (callee-saved)
    pub r12: u64,
    /// Extended register 13 (callee-saved)
    pub r13: u64,
    /// Extended register 14 (callee-saved)
    pub r14: u64,
    /// Extended register 15 (callee-saved)
    pub r15: u64,
}

/// Timer tick counter for display
static mut TIMER_TICKS: u64 = 0;

/// Timer interrupt handler (IRQ 0 = vector 0x20)
#[no_mangle]
pub extern "C" fn timer_handler_inner() {
    // Update PIT tick counter
    let ticks = pit::tick();

    unsafe {
        TIMER_TICKS = ticks;
    }

    // Check if we should preempt
    let should_switch = task::scheduler().tick();

    // Send EOI before potentially switching (important!)
    pic::end_of_interrupt(Irq::Timer);

    // Perform context switch if needed
    if should_switch {
        if let Some((old_ctx, new_ctx)) = task::scheduler().schedule() {
            unsafe {
                super::context::context_switch(old_ctx, new_ctx);
            }
        }
    }
}

/// Timer interrupt entry point (naked function to save all registers)
///
/// # Safety
///
/// - Must only be called by hardware interrupt dispatch.
/// - The interrupt stack frame must be properly set up by the CPU.
#[naked]
pub unsafe extern "C" fn timer_handler() {
    unsafe {
        naked_asm!(
            // Save all registers
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push rbp",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            // Call the Rust handler
            "call {handler}",

            // Restore all registers
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rbp",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",

            // Return from interrupt
            "iretq",

            handler = sym timer_handler_inner,
        );
    }
}

/// Keyboard interrupt handler (IRQ 1 = vector 0x21)
#[no_mangle]
pub extern "C" fn keyboard_handler_inner() {
    // Read scancode from keyboard controller
    let scancode: u8;
    unsafe {
        asm!(
            "in al, 0x60",
            out("al") scancode,
            options(nomem, nostack, preserves_flags)
        );
    }

    // Only log key presses (not releases)
    if scancode < 0x80 {
        log::debug!("Keyboard: scancode 0x{:02X}", scancode);
    }

    pic::end_of_interrupt(Irq::Keyboard);
}

/// Keyboard interrupt entry point
///
/// # Safety
///
/// - Must only be called by hardware interrupt dispatch.
/// - The interrupt stack frame must be properly set up by the CPU.
#[naked]
pub unsafe extern "C" fn keyboard_handler() {
    unsafe {
        naked_asm!(
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push rbp",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            "call {handler}",

            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rbp",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",

            "iretq",

            handler = sym keyboard_handler_inner,
        );
    }
}

/// Generic IRQ handler for unhandled IRQs
#[no_mangle]
pub extern "C" fn spurious_handler_inner(irq: u8) {
    log::warn!("Spurious IRQ: {}", irq);

    // Determine which PIC to EOI
    if irq >= 8 {
        pic::end_of_interrupt(Irq::RtcClock); // Any slave IRQ
    } else {
        pic::end_of_interrupt(Irq::Timer); // Any master IRQ
    }
}

/// Get current timer ticks
pub fn get_ticks() -> u64 {
    unsafe { TIMER_TICKS }
}

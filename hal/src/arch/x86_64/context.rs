//! # Context Switch
//!
//! Low-level assembly routines for switching between tasks.
//!
//! The context switch saves the current task's registers and restores
//! the next task's registers, effectively switching execution.

use core::arch::naked_asm;

use super::task::CpuContext;

/// Perform a context switch from one task to another
///
/// # Safety
///
/// - `old` must point to valid memory for saving the current context
/// - `new` must point to a valid context to restore
/// - This function must be called with interrupts disabled
#[naked]
pub unsafe extern "C" fn context_switch(_old: *mut CpuContext, _new: *const CpuContext) {
    // This function saves the current CPU state to `old` and restores from `new`
    unsafe {
        naked_asm!(
            // Save callee-saved registers to old context
            "mov [rdi + 0x00], r15", // old->r15
            "mov [rdi + 0x08], r14", // old->r14
            "mov [rdi + 0x10], r13", // old->r13
            "mov [rdi + 0x18], r12", // old->r12
            "mov [rdi + 0x20], rbx", // old->rbx
            "mov [rdi + 0x28], rbp", // old->rbp
            // Save return address (will be the instruction after call)
            "lea rax, [rip + 2f]",   // Address of label 2:
            "mov [rdi + 0x30], rax", // old->rip
            // Save stack pointer
            "mov [rdi + 0x38], rsp", // old->rsp
            // Save flags
            "pushfq",
            "pop rax",
            "mov [rdi + 0x40], rax", // old->rflags
            // === Now restore from new context ===

            // Restore callee-saved registers from new context
            "mov r15, [rsi + 0x00]", // new->r15
            "mov r14, [rsi + 0x08]", // new->r14
            "mov r13, [rsi + 0x10]", // new->r13
            "mov r12, [rsi + 0x18]", // new->r12
            "mov rbx, [rsi + 0x20]", // new->rbx
            "mov rbp, [rsi + 0x28]", // new->rbp
            // Restore stack pointer
            "mov rsp, [rsi + 0x38]", // new->rsp
            // Restore flags
            "mov rax, [rsi + 0x40]", // new->rflags
            "push rax",
            "popfq",
            // Jump to new context's instruction pointer
            "mov rax, [rsi + 0x30]", // new->rip
            "jmp rax",
            // Return label (old context will return here when switched back)
            "2:",
            "ret",
        );
    }
}

/// Initial context switch for a newly created task
///
/// This sets up the stack and jumps to the task's entry point.
/// Used when a task runs for the first time.
///
/// # Safety
///
/// - `ctx` must point to a valid context
/// - The stack in the context must be properly set up
#[naked]
pub unsafe extern "C" fn context_switch_initial(_ctx: *const CpuContext) {
    unsafe {
        naked_asm!(
            // Restore callee-saved registers
            "mov r15, [rdi + 0x00]",
            "mov r14, [rdi + 0x08]",
            "mov r13, [rdi + 0x10]",
            "mov r12, [rdi + 0x18]",
            "mov rbx, [rdi + 0x20]",
            "mov rbp, [rdi + 0x28]",
            // Set up stack
            "mov rsp, [rdi + 0x38]",
            // Restore flags
            "mov rax, [rdi + 0x40]",
            "push rax",
            "popfq",
            // Enable interrupts and jump to entry point
            "sti",
            "mov rax, [rdi + 0x30]",
            "jmp rax",
        );
    }
}

/// Switch to userspace using iretq
///
/// This performs a privilege level switch from ring 0 to ring 3.
///
/// # Safety
///
/// - The context must have valid user segments (cs=0x1B, ss=0x23)
/// - User stack must be properly mapped and accessible
/// - Entry point must be valid user code
#[naked]
pub unsafe extern "C" fn switch_to_userspace(
    _entry: u64,      // rdi - user entry point
    _user_stack: u64, // rsi - user stack pointer
) {
    unsafe {
        naked_asm!(
            // Set up iretq frame on stack:
            // [rsp + 32]: ss
            // [rsp + 24]: rsp (user)
            // [rsp + 16]: rflags
            // [rsp + 8]:  cs
            // [rsp + 0]:  rip (entry)

            // User data segment (0x18 | 3 = 0x1B) - updated for new GDT layout
            "push 0x1B",
            // User stack pointer
            "push rsi",
            // RFLAGS with interrupts enabled
            "pushfq",
            "pop rax",
            "or rax, 0x200", // Set IF (interrupt flag)
            "push rax",
            // User code segment (0x20 | 3 = 0x23) - updated for new GDT layout
            "push 0x23",
            // User instruction pointer (entry point)
            "push rdi",
            // Clear registers for security
            "xor rax, rax",
            "xor rbx, rbx",
            "xor rcx, rcx",
            "xor rdx, rdx",
            "xor rsi, rsi",
            "xor rdi, rdi",
            "xor rbp, rbp",
            "xor r8, r8",
            "xor r9, r9",
            "xor r10, r10",
            "xor r11, r11",
            "xor r12, r12",
            "xor r13, r13",
            "xor r14, r14",
            "xor r15, r15",
            // Switch to user mode
            "iretq",
        );
    }
}

/// Return from interrupt/exception handler
///
/// Used by interrupt handlers to return to the interrupted code.
#[macro_export]
macro_rules! iret {
    () => {
        unsafe {
            core::arch::asm!(
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
                "add rsp, 16", // Skip error code and vector
                "iretq",
                options(noreturn)
            );
        }
    };
}

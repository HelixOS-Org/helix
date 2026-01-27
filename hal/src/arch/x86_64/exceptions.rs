//! # CPU Exception Handlers
//!
//! This module provides handlers for all x86_64 CPU exceptions.
//! Each handler logs the exception and either recovers or panics.
//!
//! ## Exception Categories
//! - **Faults**: Can be corrected, execution resumes at faulting instruction
//! - **Traps**: Reported after instruction execution
//! - **Aborts**: Severe errors, no reliable recovery

use core::arch::asm;

/// Interrupt Stack Frame pushed by CPU on exception
#[derive(Debug, Clone)]
#[repr(C)]
pub struct InterruptStackFrame {
    /// Instruction pointer
    pub instruction_pointer: u64,
    /// Code segment
    pub code_segment: u64,
    /// CPU flags
    pub cpu_flags: u64,
    /// Stack pointer
    pub stack_pointer: u64,
    /// Stack segment
    pub stack_segment: u64,
}

impl InterruptStackFrame {
    /// Print the stack frame to serial
    fn log(&self, name: &str) {
        log::error!("=== {} ===", name);
        log::error!("  RIP: {:#018x}", self.instruction_pointer);
        log::error!("  CS:  {:#06x}", self.code_segment);
        log::error!("  FLAGS: {:#018x}", self.cpu_flags);
        log::error!("  RSP: {:#018x}", self.stack_pointer);
        log::error!("  SS:  {:#06x}", self.stack_segment);
    }
}

/// Page fault error code bits
pub mod page_fault_error {
    /// The fault was caused by a present page
    pub const PRESENT: u64 = 1 << 0;
    /// The fault was caused by a write
    pub const WRITE: u64 = 1 << 1;
    /// The fault was caused in user mode
    pub const USER: u64 = 1 << 2;
    /// A reserved bit was set in the page table
    pub const RESERVED_WRITE: u64 = 1 << 3;
    /// The fault was caused by an instruction fetch
    pub const INSTRUCTION_FETCH: u64 = 1 << 4;
    /// Protection key violation
    pub const PROTECTION_KEY: u64 = 1 << 5;
    /// Shadow stack access fault
    pub const SHADOW_STACK: u64 = 1 << 6;
    /// SGX violation
    pub const SGX: u64 = 1 << 15;
}

// =============================================================================
// Inner Handlers (called from naked wrappers)
// =============================================================================

extern "C" fn division_error_inner(frame: &InterruptStackFrame) {
    frame.log("Division Error (#DE)");
    panic!("Division by zero at {:#x}", frame.instruction_pointer);
}

extern "C" fn debug_inner(frame: &InterruptStackFrame) {
    frame.log("Debug Exception (#DB)");
    // Debug exceptions are usually recoverable
}

extern "C" fn nmi_inner(frame: &InterruptStackFrame) {
    frame.log("Non-Maskable Interrupt (NMI)");
    // NMI could be hardware failure, memory error, etc.
    panic!("NMI received");
}

extern "C" fn breakpoint_inner(frame: &InterruptStackFrame) {
    frame.log("Breakpoint (#BP)");
    // Breakpoints are traps, execution continues after INT3
}

extern "C" fn overflow_inner(frame: &InterruptStackFrame) {
    frame.log("Overflow (#OF)");
    panic!("Overflow at {:#x}", frame.instruction_pointer);
}

extern "C" fn bound_range_inner(frame: &InterruptStackFrame) {
    frame.log("Bound Range Exceeded (#BR)");
    panic!("Bound range exceeded at {:#x}", frame.instruction_pointer);
}

extern "C" fn invalid_opcode_inner(frame: &InterruptStackFrame) {
    frame.log("Invalid Opcode (#UD)");
    panic!("Invalid opcode at {:#x}", frame.instruction_pointer);
}

extern "C" fn device_not_available_inner(frame: &InterruptStackFrame) {
    frame.log("Device Not Available (#NM)");
    // This could be FPU lazy switching - for now, panic
    panic!("FPU not available at {:#x}", frame.instruction_pointer);
}

extern "C" fn double_fault_inner(frame: &InterruptStackFrame, error_code: u64) {
    // Direct serial output for debugging
    unsafe {
        let msg = b"\n!!! DOUBLE FAULT (#DF) !!!\n";
        for &c in msg {
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c);
        }
        
        let msg = b"RIP: 0x";
        for &c in msg { core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c); }
        let hex = b"0123456789abcdef";
        for i in (0..16).rev() {
            let nibble = ((frame.instruction_pointer >> (i * 4)) & 0xF) as usize;
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") hex[nibble]);
        }
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n');
        
        let msg = b"Error Code: 0x";
        for &c in msg { core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c); }
        for i in (0..4).rev() {
            let nibble = ((error_code >> (i * 4)) & 0xF) as usize;
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") hex[nibble]);
        }
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n');
    }
    
    frame.log("Double Fault (#DF)");
    log::error!("  Error Code: {:#x}", error_code);
    panic!("Double fault (unrecoverable)");
}

extern "C" fn invalid_tss_inner(frame: &InterruptStackFrame, error_code: u64) {
    frame.log("Invalid TSS (#TS)");
    log::error!("  Error Code: {:#x}", error_code);
    panic!("Invalid TSS at {:#x}", frame.instruction_pointer);
}

extern "C" fn segment_not_present_inner(frame: &InterruptStackFrame, error_code: u64) {
    frame.log("Segment Not Present (#NP)");
    log::error!("  Segment Selector: {:#x}", error_code);
    panic!("Segment not present at {:#x}", frame.instruction_pointer);
}

extern "C" fn stack_segment_inner(frame: &InterruptStackFrame, error_code: u64) {
    frame.log("Stack-Segment Fault (#SS)");
    log::error!("  Error Code: {:#x}", error_code);
    panic!("Stack segment fault at {:#x}", frame.instruction_pointer);
}

extern "C" fn general_protection_inner(frame: &InterruptStackFrame, error_code: u64) {
    // Direct serial output for debugging
    unsafe {
        let msg = b"\n!!! GENERAL PROTECTION FAULT (#GP) !!!\n";
        for &c in msg {
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c);
        }
        
        let msg = b"RIP: 0x";
        for &c in msg { core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c); }
        let hex = b"0123456789abcdef";
        for i in (0..16).rev() {
            let nibble = ((frame.instruction_pointer >> (i * 4)) & 0xF) as usize;
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") hex[nibble]);
        }
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n');
        
        let msg = b"CS: 0x";
        for &c in msg { core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c); }
        for i in (0..4).rev() {
            let nibble = ((frame.code_segment >> (i * 4)) & 0xF) as usize;
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") hex[nibble]);
        }
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n');
        
        let msg = b"Error Code: 0x";
        for &c in msg { core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c); }
        for i in (0..4).rev() {
            let nibble = ((error_code >> (i * 4)) & 0xF) as usize;
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") hex[nibble]);
        }
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n');
    }
    
    frame.log("General Protection Fault (#GP)");
    log::error!("  Error Code: {:#x}", error_code);
    if error_code != 0 {
        log::error!("  Segment Index: {}", error_code >> 3);
        log::error!("  Table: {}", match (error_code >> 1) & 0x3 {
            0 => "GDT",
            1 | 3 => "IDT",
            2 => "LDT",
            _ => "Unknown",
        });
    }
    panic!("General protection fault at {:#x}", frame.instruction_pointer);
}

extern "C" fn page_fault_inner(frame: &InterruptStackFrame, error_code: u64) {
    // Direct serial output for debugging
    let msg = b"\n### PAGE FAULT ###\n";
    for &c in msg {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
    }
    
    // Get the faulting address from CR2
    let cr2: u64;
    unsafe {
        asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
    }
    
    // Print CR2 in hex
    let msg2 = b"CR2=";
    for &c in msg2 {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
    }
    
    // Print address as hex
    for i in (0..16).rev() {
        let nibble = ((cr2 >> (i * 4)) & 0xF) as u8;
        let ch = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
        unsafe {
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") ch, options(nomem, nostack));
        }
    }
    
    let msg3 = b" ERR=";
    for &c in msg3 {
        unsafe {
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") c, options(nomem, nostack));
        }
    }
    
    // Print error code
    for i in (0..4).rev() {
        let nibble = ((error_code >> (i * 4)) & 0xF) as u8;
        let ch = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
        unsafe {
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") ch, options(nomem, nostack));
        }
    }
    
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b'\n', options(nomem, nostack));
    }
    
    frame.log("Page Fault (#PF)");
    log::error!("  Faulting Address: {:#018x}", cr2);
    log::error!("  Error Code: {:#x}", error_code);
    log::error!("  Caused by:");
    
    if error_code & page_fault_error::PRESENT != 0 {
        log::error!("    - Protection violation");
    } else {
        log::error!("    - Page not present");
    }
    
    if error_code & page_fault_error::WRITE != 0 {
        log::error!("    - Write access");
    } else {
        log::error!("    - Read access");
    }
    
    if error_code & page_fault_error::USER != 0 {
        log::error!("    - User mode");
    } else {
        log::error!("    - Kernel mode");
    }
    
    if error_code & page_fault_error::INSTRUCTION_FETCH != 0 {
        log::error!("    - Instruction fetch");
    }
    
    panic!("Page fault at {:#x} accessing {:#x}", frame.instruction_pointer, cr2);
}

extern "C" fn x87_floating_point_inner(frame: &InterruptStackFrame) {
    frame.log("x87 Floating-Point Exception (#MF)");
    panic!("FPU error at {:#x}", frame.instruction_pointer);
}

extern "C" fn alignment_check_inner(frame: &InterruptStackFrame, error_code: u64) {
    frame.log("Alignment Check (#AC)");
    log::error!("  Error Code: {:#x}", error_code);
    panic!("Alignment check at {:#x}", frame.instruction_pointer);
}

extern "C" fn machine_check_inner(frame: &InterruptStackFrame) {
    frame.log("Machine Check (#MC)");
    panic!("Machine check exception (hardware failure)");
}

extern "C" fn simd_floating_point_inner(frame: &InterruptStackFrame) {
    frame.log("SIMD Floating-Point Exception (#XM)");
    panic!("SIMD error at {:#x}", frame.instruction_pointer);
}

// =============================================================================
// Handler Wrappers using naked_asm! with sym operands
// =============================================================================

macro_rules! exception_handler {
    ($name:ident, $handler:expr) => {
        #[naked]
        pub extern "C" fn $name() {
            unsafe {
                core::arch::naked_asm!(
                    // Push all general-purpose registers
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
                    
                    // First argument: pointer to InterruptStackFrame
                    // (after our pushes, it's at rsp + 15*8 = rsp + 120)
                    "mov rdi, rsp",
                    "add rdi, 120",
                    
                    // Call the handler
                    "call {handler}",
                    
                    // Restore registers
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
                    handler = sym $handler,
                );
            }
        }
    };
}

/// Exception handler with error code
macro_rules! exception_handler_with_error {
    ($name:ident, $handler:expr) => {
        #[naked]
        pub extern "C" fn $name() {
            unsafe {
                core::arch::naked_asm!(
                    // Error code is already on stack
                    // Push all general-purpose registers
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
                    
                    // First argument: pointer to InterruptStackFrame
                    // (after our pushes + error code: rsp + 15*8 + 8 = rsp + 128)
                    "mov rdi, rsp",
                    "add rdi, 128",
                    
                    // Second argument: error code (at rsp + 120)
                    "mov rsi, [rsp + 120]",
                    
                    // Call the handler
                    "call {handler}",
                    
                    // Restore registers
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
                    
                    // Remove error code from stack
                    "add rsp, 8",
                    
                    // Return from interrupt
                    "iretq",
                    handler = sym $handler,
                );
            }
        }
    };
}

// =============================================================================
// Generate Handler Wrappers
// =============================================================================

exception_handler!(division_error_handler, division_error_inner);
exception_handler!(debug_handler, debug_inner);
exception_handler!(nmi_handler, nmi_inner);
exception_handler!(breakpoint_handler, breakpoint_inner);
exception_handler!(overflow_handler, overflow_inner);
exception_handler!(bound_range_handler, bound_range_inner);
exception_handler!(invalid_opcode_handler, invalid_opcode_inner);
exception_handler!(device_not_available_handler, device_not_available_inner);
exception_handler_with_error!(double_fault_handler, double_fault_inner);
exception_handler_with_error!(invalid_tss_handler, invalid_tss_inner);
exception_handler_with_error!(segment_not_present_handler, segment_not_present_inner);
exception_handler_with_error!(stack_segment_handler, stack_segment_inner);
exception_handler_with_error!(general_protection_handler, general_protection_inner);
exception_handler_with_error!(page_fault_handler, page_fault_inner);
exception_handler!(x87_floating_point_handler, x87_floating_point_inner);
exception_handler_with_error!(alignment_check_handler, alignment_check_inner);
exception_handler!(machine_check_handler, machine_check_inner);
exception_handler!(simd_floating_point_handler, simd_floating_point_inner);

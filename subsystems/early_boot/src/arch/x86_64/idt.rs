//! # x86_64 IDT and Exception Handling
//!
//! Interrupt Descriptor Table setup and exception handlers for early boot.

use core::mem::size_of;

use super::gdt::DescriptorTablePointer;
use crate::core::BootContext;
use crate::error::{BootError, BootResult};

// =============================================================================
// GATE TYPES
// =============================================================================

/// Interrupt gate type (clears IF)
pub const GATE_INTERRUPT: u8 = 0x0E;
/// Trap gate type (preserves IF)
pub const GATE_TRAP: u8 = 0x0F;

// =============================================================================
// EXCEPTION NUMBERS
// =============================================================================

/// Division by zero
pub const DIVIDE_ERROR: u8 = 0;
/// Debug exception
pub const DEBUG: u8 = 1;
/// Non-maskable interrupt
pub const NMI: u8 = 2;
/// Breakpoint
pub const BREAKPOINT: u8 = 3;
/// Overflow
pub const OVERFLOW: u8 = 4;
/// Bound range exceeded
pub const BOUND_RANGE: u8 = 5;
/// Invalid opcode
pub const INVALID_OPCODE: u8 = 6;
/// Device not available
pub const DEVICE_NOT_AVAILABLE: u8 = 7;
/// Double fault
pub const DOUBLE_FAULT: u8 = 8;
/// Coprocessor segment overrun (legacy)
pub const COPROCESSOR_SEGMENT: u8 = 9;
/// Invalid TSS
pub const INVALID_TSS: u8 = 10;
/// Segment not present
pub const SEGMENT_NOT_PRESENT: u8 = 11;
/// Stack segment fault
pub const STACK_SEGMENT: u8 = 12;
/// General protection fault
pub const GENERAL_PROTECTION: u8 = 13;
/// Page fault
pub const PAGE_FAULT: u8 = 14;
/// x87 floating-point exception
pub const X87_FPU: u8 = 16;
/// Alignment check
pub const ALIGNMENT_CHECK: u8 = 17;
/// Machine check
pub const MACHINE_CHECK: u8 = 18;
/// SIMD floating-point exception
pub const SIMD_FP: u8 = 19;
/// Virtualization exception
pub const VIRTUALIZATION: u8 = 20;
/// Control protection exception
pub const CONTROL_PROTECTION: u8 = 21;
/// Hypervisor injection exception
pub const HYPERVISOR_INJECTION: u8 = 28;
/// VMM communication exception
pub const VMM_COMMUNICATION: u8 = 29;
/// Security exception
pub const SECURITY_EXCEPTION: u8 = 30;

// =============================================================================
// IDT ENTRY
// =============================================================================

/// 64-bit IDT gate descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    /// Offset bits 0-15
    offset_low: u16,
    /// Code segment selector
    selector: u16,
    /// IST index (bits 0-2) and reserved
    ist: u8,
    /// Type and attributes
    type_attr: u8,
    /// Offset bits 16-31
    offset_mid: u16,
    /// Offset bits 32-63
    offset_high: u32,
    /// Reserved
    reserved: u32,
}

impl IdtEntry {
    /// Create a null IDT entry
    pub const fn null() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create an interrupt gate
    pub const fn interrupt_gate(handler: u64, selector: u16, dpl: u8, ist: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: ist & 0x7,
            type_attr: GATE_INTERRUPT | ((dpl & 0x3) << 5) | 0x80, // Present bit
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Create a trap gate
    pub const fn trap_gate(handler: u64, selector: u16, dpl: u8, ist: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: ist & 0x7,
            type_attr: GATE_TRAP | ((dpl & 0x3) << 5) | 0x80, // Present bit
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Set handler address
    pub fn set_handler(&mut self, handler: u64) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
    }

    /// Set IST index (1-7, or 0 for none)
    pub fn set_ist(&mut self, ist: u8) {
        self.ist = ist & 0x7;
    }

    /// Mark as present
    pub fn set_present(&mut self, present: bool) {
        if present {
            self.type_attr |= 0x80;
        } else {
            self.type_attr &= !0x80;
        }
    }
}

// =============================================================================
// INTERRUPT DESCRIPTOR TABLE
// =============================================================================

/// Full IDT (256 entries)
#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    /// Create a new empty IDT
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::null(); 256],
        }
    }

    /// Set an entry
    pub fn set_entry(&mut self, index: u8, entry: IdtEntry) {
        self.entries[index as usize] = entry;
    }

    /// Get a mutable reference to an entry
    pub fn entry_mut(&mut self, index: u8) -> &mut IdtEntry {
        &mut self.entries[index as usize]
    }

    /// Get the IDT pointer
    pub fn pointer(&self) -> DescriptorTablePointer {
        DescriptorTablePointer {
            limit: (size_of::<Self>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }
}

// =============================================================================
// EXCEPTION FRAME
// =============================================================================

/// Exception stack frame (pushed by CPU)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ExceptionFrame {
    /// Instruction pointer
    pub rip: u64,
    /// Code segment
    pub cs: u64,
    /// CPU flags
    pub rflags: u64,
    /// Stack pointer
    pub rsp: u64,
    /// Stack segment
    pub ss: u64,
}

/// Extended exception frame with error code
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ExceptionFrameWithError {
    /// Error code (pushed by some exceptions)
    pub error_code: u64,
    /// Exception frame
    pub frame: ExceptionFrame,
}

/// Full register state for exception handler
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SavedRegisters {
    // General purpose registers
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    // Vector number
    pub vector: u64,
    // Error code (may be dummy)
    pub error_code: u64,
    // Exception frame
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

// =============================================================================
// STATIC IDT
// =============================================================================

static mut IDT: Idt = Idt::new();

// =============================================================================
// EXCEPTION HANDLER STUBS (Assembly)
// =============================================================================

/// Generate exception handler stub macro
macro_rules! exception_handler {
    ($name:ident, $vector:expr) => {
        #[naked]
        unsafe extern "C" fn $name() {
            core::arch::naked_asm!(
                "push 0",         // Dummy error code
                "push {}",        // Vector number
                "jmp {}",
                const $vector,
                sym exception_common,
            )
        }
    };
}

/// Generate exception handler with error code
macro_rules! exception_handler_with_error {
    ($name:ident, $vector:expr) => {
        #[naked]
        unsafe extern "C" fn $name() {
            core::arch::naked_asm!(
                "push {}",        // Vector number (error code already pushed by CPU)
                "jmp {}",
                const $vector,
                sym exception_common,
            )
        }
    };
}

/// Common exception handler (saves all registers)
#[naked]
unsafe extern "C" fn exception_common() {
    core::arch::naked_asm!(
        // Save all general purpose registers
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

        // Call Rust handler with pointer to saved state
        "mov rdi, rsp",
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

        // Skip vector and error code
        "add rsp, 16",

        // Return from interrupt
        "iretq",
        handler = sym exception_dispatch,
    )
}

// Generate handlers for all exceptions
exception_handler!(exc_divide_error, 0);
exception_handler!(exc_debug, 1);
exception_handler!(exc_nmi, 2);
exception_handler!(exc_breakpoint, 3);
exception_handler!(exc_overflow, 4);
exception_handler!(exc_bound_range, 5);
exception_handler!(exc_invalid_opcode, 6);
exception_handler!(exc_device_not_available, 7);
exception_handler_with_error!(exc_double_fault, 8);
exception_handler!(exc_coprocessor_segment, 9);
exception_handler_with_error!(exc_invalid_tss, 10);
exception_handler_with_error!(exc_segment_not_present, 11);
exception_handler_with_error!(exc_stack_segment, 12);
exception_handler_with_error!(exc_general_protection, 13);
exception_handler_with_error!(exc_page_fault, 14);
exception_handler!(exc_reserved_15, 15);
exception_handler!(exc_x87_fpu, 16);
exception_handler_with_error!(exc_alignment_check, 17);
exception_handler!(exc_machine_check, 18);
exception_handler!(exc_simd_fp, 19);
exception_handler!(exc_virtualization, 20);
exception_handler_with_error!(exc_control_protection, 21);
exception_handler!(exc_reserved_22, 22);
exception_handler!(exc_reserved_23, 23);
exception_handler!(exc_reserved_24, 24);
exception_handler!(exc_reserved_25, 25);
exception_handler!(exc_reserved_26, 26);
exception_handler!(exc_reserved_27, 27);
exception_handler!(exc_hypervisor_injection, 28);
exception_handler_with_error!(exc_vmm_communication, 29);
exception_handler_with_error!(exc_security_exception, 30);
exception_handler!(exc_reserved_31, 31);

// =============================================================================
// EXCEPTION DISPATCH
// =============================================================================

/// Main exception dispatcher
extern "C" fn exception_dispatch(regs: *mut SavedRegisters) {
    let regs = unsafe { &*regs };

    match regs.vector as u8 {
        DIVIDE_ERROR => handle_divide_error(regs),
        DEBUG => handle_debug(regs),
        NMI => handle_nmi(regs),
        BREAKPOINT => handle_breakpoint(regs),
        OVERFLOW => handle_overflow(regs),
        BOUND_RANGE => handle_bound_range(regs),
        INVALID_OPCODE => handle_invalid_opcode(regs),
        DEVICE_NOT_AVAILABLE => handle_device_not_available(regs),
        DOUBLE_FAULT => handle_double_fault(regs),
        INVALID_TSS => handle_invalid_tss(regs),
        SEGMENT_NOT_PRESENT => handle_segment_not_present(regs),
        STACK_SEGMENT => handle_stack_segment(regs),
        GENERAL_PROTECTION => handle_general_protection(regs),
        PAGE_FAULT => handle_page_fault(regs),
        X87_FPU => handle_x87_fpu(regs),
        ALIGNMENT_CHECK => handle_alignment_check(regs),
        MACHINE_CHECK => handle_machine_check(regs),
        SIMD_FP => handle_simd_fp(regs),
        VIRTUALIZATION => handle_virtualization(regs),
        CONTROL_PROTECTION => handle_control_protection(regs),
        HYPERVISOR_INJECTION => handle_hypervisor_injection(regs),
        VMM_COMMUNICATION => handle_vmm_communication(regs),
        SECURITY_EXCEPTION => handle_security_exception(regs),
        _ => handle_unknown(regs),
    }
}

// =============================================================================
// EXCEPTION HANDLERS
// =============================================================================

fn handle_divide_error(regs: &SavedRegisters) {
    panic_exception("Division by zero", regs);
}

fn handle_debug(regs: &SavedRegisters) {
    // Debug exception - could be single step, breakpoint, etc.
    // For early boot, just log and continue
    early_print!("Debug exception at RIP: {:#x}\n", regs.rip);
}

fn handle_nmi(regs: &SavedRegisters) {
    // Non-maskable interrupt - could be hardware failure
    panic_exception("Non-maskable interrupt", regs);
}

fn handle_breakpoint(regs: &SavedRegisters) {
    // INT3 breakpoint
    early_print!("Breakpoint at RIP: {:#x}\n", regs.rip);
}

fn handle_overflow(regs: &SavedRegisters) {
    panic_exception("Overflow", regs);
}

fn handle_bound_range(regs: &SavedRegisters) {
    panic_exception("Bound range exceeded", regs);
}

fn handle_invalid_opcode(regs: &SavedRegisters) {
    panic_exception("Invalid opcode", regs);
}

fn handle_device_not_available(regs: &SavedRegisters) {
    panic_exception("Device not available (FPU)", regs);
}

fn handle_double_fault(regs: &SavedRegisters) {
    // Double fault is fatal and non-recoverable
    panic_exception("DOUBLE FAULT", regs);
}

fn handle_invalid_tss(regs: &SavedRegisters) {
    panic_exception("Invalid TSS", regs);
}

fn handle_segment_not_present(regs: &SavedRegisters) {
    panic_exception("Segment not present", regs);
}

fn handle_stack_segment(regs: &SavedRegisters) {
    panic_exception("Stack segment fault", regs);
}

fn handle_general_protection(regs: &SavedRegisters) {
    let selector = regs.error_code;
    early_print!("General Protection Fault!\n");
    early_print!("  Selector: {:#x}\n", selector);
    panic_exception("General protection fault", regs);
}

fn handle_page_fault(regs: &SavedRegisters) {
    // Get faulting address from CR2
    let cr2: u64;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nostack, preserves_flags));
    }

    let error = regs.error_code;
    early_print!("Page Fault!\n");
    early_print!("  Faulting address: {:#x}\n", cr2);
    early_print!("  Error code: {:#x}\n", error);
    early_print!("    Present: {}\n", error & 1 != 0);
    early_print!("    Write: {}\n", error & 2 != 0);
    early_print!("    User: {}\n", error & 4 != 0);
    early_print!("    Reserved: {}\n", error & 8 != 0);
    early_print!("    Fetch: {}\n", error & 16 != 0);

    panic_exception("Page fault", regs);
}

fn handle_x87_fpu(regs: &SavedRegisters) {
    panic_exception("x87 FPU exception", regs);
}

fn handle_alignment_check(regs: &SavedRegisters) {
    panic_exception("Alignment check", regs);
}

fn handle_machine_check(regs: &SavedRegisters) {
    panic_exception("Machine check exception", regs);
}

fn handle_simd_fp(regs: &SavedRegisters) {
    panic_exception("SIMD floating-point exception", regs);
}

fn handle_virtualization(regs: &SavedRegisters) {
    panic_exception("Virtualization exception", regs);
}

fn handle_control_protection(regs: &SavedRegisters) {
    panic_exception("Control protection exception", regs);
}

fn handle_hypervisor_injection(regs: &SavedRegisters) {
    panic_exception("Hypervisor injection exception", regs);
}

fn handle_vmm_communication(regs: &SavedRegisters) {
    panic_exception("VMM communication exception", regs);
}

fn handle_security_exception(regs: &SavedRegisters) {
    panic_exception("Security exception", regs);
}

fn handle_unknown(regs: &SavedRegisters) {
    early_print!("Unknown exception: {}\n", regs.vector);
    panic_exception("Unknown exception", regs);
}

/// Print exception info and halt
fn panic_exception(name: &str, regs: &SavedRegisters) -> ! {
    early_print!("\n=== KERNEL PANIC: {} ===\n", name);
    early_print!("Vector: {} (0x{:x})\n", regs.vector, regs.vector);
    early_print!("Error code: {:#x}\n", regs.error_code);
    early_print!("\nRegisters:\n");
    early_print!("  RAX: {:#018x}  RBX: {:#018x}\n", regs.rax, regs.rbx);
    early_print!("  RCX: {:#018x}  RDX: {:#018x}\n", regs.rcx, regs.rdx);
    early_print!("  RSI: {:#018x}  RDI: {:#018x}\n", regs.rsi, regs.rdi);
    early_print!("  RBP: {:#018x}  RSP: {:#018x}\n", regs.rbp, regs.rsp);
    early_print!("  R8:  {:#018x}  R9:  {:#018x}\n", regs.r8, regs.r9);
    early_print!("  R10: {:#018x}  R11: {:#018x}\n", regs.r10, regs.r11);
    early_print!("  R12: {:#018x}  R13: {:#018x}\n", regs.r12, regs.r13);
    early_print!("  R14: {:#018x}  R15: {:#018x}\n", regs.r14, regs.r15);
    early_print!("\n");
    early_print!("  RIP: {:#018x}  CS:  {:#06x}\n", regs.rip, regs.cs);
    early_print!("  RFLAGS: {:#018x}\n", regs.rflags);
    early_print!("  SS:  {:#06x}\n", regs.ss);

    // Halt the CPU
    loop {
        unsafe {
            core::arch::asm!("cli; hlt", options(nostack, nomem));
        }
    }
}

// =============================================================================
// EARLY PRINT MACRO
// =============================================================================

/// Simple early print for exception handling
macro_rules! early_print {
    ($($arg:tt)*) => {
        // Use serial port for output
        let _ = write_serial_str(&format_args!($($arg)*).to_string());
    };
}

use early_print;

/// Write string to serial port (COM1)
fn write_serial_str(_s: &str) {
    // TODO: Implement proper serial output
    // For now, this is a placeholder
}

// =============================================================================
// IDT INITIALIZATION
// =============================================================================

/// Initialize the IDT with exception handlers
///
/// # Safety
///
/// The caller must ensure this is called exactly once during system initialization.
pub unsafe fn init_idt(ctx: &mut BootContext) -> BootResult<()> {
    let kernel_cs = super::KERNEL_CS;

    // Set up exception handlers (vectors 0-31)
    let handlers: [(u8, unsafe extern "C" fn()); 32] = [
        (0, exc_divide_error),
        (1, exc_debug),
        (2, exc_nmi),
        (3, exc_breakpoint),
        (4, exc_overflow),
        (5, exc_bound_range),
        (6, exc_invalid_opcode),
        (7, exc_device_not_available),
        (8, exc_double_fault),
        (9, exc_coprocessor_segment),
        (10, exc_invalid_tss),
        (11, exc_segment_not_present),
        (12, exc_stack_segment),
        (13, exc_general_protection),
        (14, exc_page_fault),
        (15, exc_reserved_15),
        (16, exc_x87_fpu),
        (17, exc_alignment_check),
        (18, exc_machine_check),
        (19, exc_simd_fp),
        (20, exc_virtualization),
        (21, exc_control_protection),
        (22, exc_reserved_22),
        (23, exc_reserved_23),
        (24, exc_reserved_24),
        (25, exc_reserved_25),
        (26, exc_reserved_26),
        (27, exc_reserved_27),
        (28, exc_hypervisor_injection),
        (29, exc_vmm_communication),
        (30, exc_security_exception),
        (31, exc_reserved_31),
    ];

    for (vector, handler) in handlers {
        let ist = match vector {
            DOUBLE_FAULT => 1,  // Use IST1 for double fault
            NMI => 2,           // Use IST2 for NMI
            MACHINE_CHECK => 3, // Use IST3 for machine check
            _ => 0,             // No IST for other exceptions
        };

        let entry = IdtEntry::interrupt_gate(
            handler as u64,
            kernel_cs,
            0, // DPL 0 (kernel only)
            ist,
        );
        IDT.set_entry(vector, entry);
    }

    // Load IDT
    let idt_ptr = IDT.pointer();
    core::arch::asm!(
        "lidt [{}]",
        in(reg) &idt_ptr,
        options(nostack)
    );

    // Store in context
    ctx.arch_data.x86.idt_base = &raw const IDT as u64;
    ctx.arch_data.x86.idt_limit = idt_ptr.limit;

    Ok(())
}

// =============================================================================
// IRQ HANDLERS
// =============================================================================

/// Generic IRQ handler stub
#[naked]
unsafe extern "C" fn irq_common() {
    core::arch::naked_asm!(
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

        // Call IRQ handler with vector number
        "mov rdi, rsp",
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

        // Skip vector and dummy error code
        "add rsp, 16",

        "iretq",
        handler = sym irq_dispatch,
    )
}

/// Generate IRQ handler stub
macro_rules! irq_handler {
    ($name:ident, $vector:expr) => {
        #[naked]
        pub unsafe extern "C" fn $name() {
            core::arch::naked_asm!(
                "push 0",         // Dummy error code
                "push {}",        // Vector number
                "jmp {}",
                const $vector,
                sym irq_common,
            )
        }
    };
}

// Generate IRQ stubs for vectors 32-47 (legacy PIC) and 48-255 (APIC)
irq_handler!(irq_32, 32);
irq_handler!(irq_33, 33);
irq_handler!(irq_34, 34);
irq_handler!(irq_35, 35);
irq_handler!(irq_36, 36);
irq_handler!(irq_37, 37);
irq_handler!(irq_38, 38);
irq_handler!(irq_39, 39);
irq_handler!(irq_40, 40);
irq_handler!(irq_41, 41);
irq_handler!(irq_42, 42);
irq_handler!(irq_43, 43);
irq_handler!(irq_44, 44);
irq_handler!(irq_45, 45);
irq_handler!(irq_46, 46);
irq_handler!(irq_47, 47);

// APIC timer and IPI vectors
irq_handler!(irq_timer, 0xFE);
irq_handler!(irq_spurious, 0xFF);

/// IRQ dispatch handler
extern "C" fn irq_dispatch(regs: *mut SavedRegisters) {
    let regs = unsafe { &*regs };
    let vector = regs.vector as u8;

    match vector {
        32..=47 => {
            // Legacy PIC IRQ
            handle_legacy_irq(vector - 32);
        },
        0xFE => {
            // APIC timer
            handle_apic_timer();
        },
        0xFF => {
            // Spurious interrupt
        },
        _ => {
            // Other APIC interrupt
            handle_apic_irq(vector);
        },
    }

    // Send EOI to APIC if needed
    if vector >= 32 {
        unsafe {
            send_apic_eoi();
        }
    }
}

fn handle_legacy_irq(irq: u8) {
    // Handle legacy PIC IRQ
    // For early boot, most IRQs are ignored
    let _ = irq;
}

fn handle_apic_timer() {
    // Handle APIC timer tick
    // Increment tick counter, etc.
}

fn handle_apic_irq(vector: u8) {
    // Handle other APIC interrupts
    let _ = vector;
}

/// Send End-of-Interrupt to local APIC
unsafe fn send_apic_eoi() {
    let apic_base = super::LAPIC_BASE;
    let eoi_reg = (apic_base + 0xB0) as *mut u32;
    core::ptr::write_volatile(eoi_reg, 0);
}

/// Set up IRQ entries in IDT
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_irq_handlers(ctx: &mut BootContext) -> BootResult<()> {
    let kernel_cs = super::KERNEL_CS;

    // Legacy PIC IRQs (32-47)
    let legacy_handlers: [unsafe extern "C" fn(); 16] = [
        irq_32, irq_33, irq_34, irq_35, irq_36, irq_37, irq_38, irq_39, irq_40, irq_41, irq_42,
        irq_43, irq_44, irq_45, irq_46, irq_47,
    ];

    for (i, handler) in legacy_handlers.iter().enumerate() {
        let entry = IdtEntry::interrupt_gate(*handler as u64, kernel_cs, 0, 0);
        IDT.set_entry((32 + i) as u8, entry);
    }

    // APIC timer
    let timer_entry = IdtEntry::interrupt_gate(irq_timer as u64, kernel_cs, 0, 0);
    IDT.set_entry(0xFE, timer_entry);

    // Spurious interrupt
    let spurious_entry = IdtEntry::interrupt_gate(irq_spurious as u64, kernel_cs, 0, 0);
    IDT.set_entry(0xFF, spurious_entry);

    // Store in context
    ctx.interrupt_state.vectors_configured = 48; // 32 exceptions + 16 IRQs

    Ok(())
}

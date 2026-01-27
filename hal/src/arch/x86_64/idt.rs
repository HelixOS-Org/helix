//! # Interrupt Descriptor Table (IDT)
//!
//! The IDT maps interrupt vectors to handler functions.
//! In x86_64 long mode, each entry is 16 bytes.
//!
//! ## Vector Layout
//! ```text
//! 0x00-0x1F: CPU Exceptions (reserved by Intel)
//!   0x00: Division Error
//!   0x01: Debug
//!   0x02: NMI
//!   0x03: Breakpoint
//!   0x04: Overflow
//!   0x05: Bound Range Exceeded
//!   0x06: Invalid Opcode
//!   0x07: Device Not Available
//!   0x08: Double Fault (IST1)
//!   0x09: Coprocessor Segment Overrun (legacy)
//!   0x0A: Invalid TSS
//!   0x0B: Segment Not Present
//!   0x0C: Stack-Segment Fault
//!   0x0D: General Protection Fault
//!   0x0E: Page Fault (IST2)
//!   0x0F: Reserved
//!   0x10: x87 Floating-Point Exception
//!   0x11: Alignment Check
//!   0x12: Machine Check
//!   0x13: SIMD Floating-Point Exception
//!   0x14: Virtualization Exception
//!   0x15-0x1F: Reserved
//!
//! 0x20-0x2F: IRQs (remapped from legacy PIC)
//! 0x30-0xFF: Software interrupts, syscalls, etc.
//! ```

use core::arch::asm;
use core::mem::size_of;
use super::gdt::KERNEL_CODE_SELECTOR;

/// Total number of IDT entries
pub const IDT_ENTRIES: usize = 256;

/// IDT Gate Types
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum GateType {
    /// Interrupt Gate (clears IF)
    Interrupt = 0xE,
    /// Trap Gate (doesn't clear IF)
    Trap = 0xF,
}

/// IDT Entry Options
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct IdtEntryOptions(u16);

impl IdtEntryOptions {
    /// Create new options for a kernel interrupt gate
    pub const fn new(gate_type: GateType, ist: u8) -> Self {
        let mut value = 0u16;
        // Bits 0-2: IST index (0 = no IST)
        value |= (ist & 0x7) as u16;
        // Bits 3-7: Reserved (0)
        // Bits 8-11: Gate type
        value |= (gate_type as u16) << 8;
        // Bit 12: 0 (must be 0)
        // Bits 13-14: DPL (0 for kernel)
        // Bit 15: Present
        value |= 1 << 15;
        Self(value)
    }

    /// Create options with user-accessible gate (DPL 3)
    pub const fn new_user(gate_type: GateType, ist: u8) -> Self {
        let mut opts = Self::new(gate_type, ist);
        opts.0 |= 3 << 13; // DPL = 3
        opts
    }

    /// Kernel interrupt gate (most common)
    pub const fn interrupt() -> Self {
        Self::new(GateType::Interrupt, 0)
    }

    /// Kernel interrupt gate with IST
    pub const fn interrupt_with_ist(ist: u8) -> Self {
        Self::new(GateType::Interrupt, ist)
    }

    /// Kernel trap gate
    pub const fn trap() -> Self {
        Self::new(GateType::Trap, 0)
    }
}

/// IDT Entry (16 bytes in 64-bit mode)
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    /// Lower 16 bits of handler address
    offset_low: u16,
    /// Code segment selector
    selector: u16,
    /// Options (IST, type, DPL, present)
    options: IdtEntryOptions,
    /// Middle 16 bits of handler address
    offset_middle: u16,
    /// Upper 32 bits of handler address
    offset_high: u32,
    /// Reserved (must be 0)
    reserved: u32,
}

impl IdtEntry {
    /// Create a missing (not present) entry
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            options: IdtEntryOptions(0),
            offset_middle: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create an entry from a handler address
    pub const fn new(handler: u64, options: IdtEntryOptions) -> Self {
        Self {
            offset_low: handler as u16,
            selector: KERNEL_CODE_SELECTOR,
            options,
            offset_middle: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }

    /// Set the handler for this entry
    pub fn set_handler(&mut self, handler: u64, options: IdtEntryOptions) {
        self.offset_low = handler as u16;
        self.selector = KERNEL_CODE_SELECTOR;
        self.options = options;
        self.offset_middle = (handler >> 16) as u16;
        self.offset_high = (handler >> 32) as u32;
        self.reserved = 0;
    }
}

/// IDT Pointer for LIDT instruction
#[repr(C, packed)]
pub struct IdtPointer {
    /// Size of IDT - 1
    pub limit: u16,
    /// Base address of IDT
    pub base: u64,
}

/// The Interrupt Descriptor Table
#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; IDT_ENTRIES],
}

impl Idt {
    /// Create a new IDT with all entries missing
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::missing(); IDT_ENTRIES],
        }
    }

    /// Set a handler for a specific vector
    pub fn set_handler(&mut self, vector: u8, handler: u64, options: IdtEntryOptions) {
        self.entries[vector as usize].set_handler(handler, options);
    }

    /// Get a pointer for the LIDT instruction
    pub fn pointer(&self) -> IdtPointer {
        IdtPointer {
            limit: (size_of::<Self>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }

    /// Load this IDT
    /// 
    /// # Safety
    /// The IDT must remain valid and at the same memory location.
    pub unsafe fn load(&self) {
        let ptr = self.pointer();
        unsafe {
            asm!(
                "lidt [{}]",
                in(reg) &ptr,
                options(readonly, nostack, preserves_flags)
            );
        }
    }
}

// =============================================================================
// Global IDT
// =============================================================================

/// Static IDT
static mut IDT: Idt = Idt::new();

/// Exception vector numbers
pub mod vectors {
    pub const DIVISION_ERROR: u8 = 0;
    pub const DEBUG: u8 = 1;
    pub const NMI: u8 = 2;
    pub const BREAKPOINT: u8 = 3;
    pub const OVERFLOW: u8 = 4;
    pub const BOUND_RANGE: u8 = 5;
    pub const INVALID_OPCODE: u8 = 6;
    pub const DEVICE_NOT_AVAILABLE: u8 = 7;
    pub const DOUBLE_FAULT: u8 = 8;
    pub const INVALID_TSS: u8 = 10;
    pub const SEGMENT_NOT_PRESENT: u8 = 11;
    pub const STACK_SEGMENT: u8 = 12;
    pub const GENERAL_PROTECTION: u8 = 13;
    pub const PAGE_FAULT: u8 = 14;
    pub const X87_FLOATING_POINT: u8 = 16;
    pub const ALIGNMENT_CHECK: u8 = 17;
    pub const MACHINE_CHECK: u8 = 18;
    pub const SIMD_FLOATING_POINT: u8 = 19;
    pub const VIRTUALIZATION: u8 = 20;
    
    /// First IRQ vector (after remapping PIC)
    pub const IRQ_BASE: u8 = 32;
    
    /// Timer IRQ
    pub const TIMER: u8 = IRQ_BASE + 0;
    /// Keyboard IRQ
    pub const KEYBOARD: u8 = IRQ_BASE + 1;
    /// Syscall vector
    pub const SYSCALL: u8 = 0x80;
}

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the IDT with default exception handlers
/// 
/// # Safety
/// Must be called only once during early boot.
pub unsafe fn init() {
    use super::exceptions;
    
    unsafe {
        // CPU Exceptions
        IDT.set_handler(vectors::DIVISION_ERROR, 
            exceptions::division_error_handler as u64, 
            IdtEntryOptions::interrupt());
    
        IDT.set_handler(vectors::DEBUG,
            exceptions::debug_handler as u64,
            IdtEntryOptions::trap());
        
        IDT.set_handler(vectors::NMI,
            exceptions::nmi_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::BREAKPOINT,
            exceptions::breakpoint_handler as u64,
            IdtEntryOptions::trap());
        
        IDT.set_handler(vectors::OVERFLOW,
            exceptions::overflow_handler as u64,
            IdtEntryOptions::trap());
        
        IDT.set_handler(vectors::BOUND_RANGE,
            exceptions::bound_range_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::INVALID_OPCODE,
            exceptions::invalid_opcode_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::DEVICE_NOT_AVAILABLE,
            exceptions::device_not_available_handler as u64,
            IdtEntryOptions::interrupt());
        
        // Double fault uses IST1 for safety
        IDT.set_handler(vectors::DOUBLE_FAULT,
            exceptions::double_fault_handler as u64,
            IdtEntryOptions::interrupt_with_ist(1));
        
        IDT.set_handler(vectors::INVALID_TSS,
            exceptions::invalid_tss_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::SEGMENT_NOT_PRESENT,
            exceptions::segment_not_present_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::STACK_SEGMENT,
            exceptions::stack_segment_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::GENERAL_PROTECTION,
            exceptions::general_protection_handler as u64,
            IdtEntryOptions::interrupt());
        
        // Page fault uses IST2 for safety
        IDT.set_handler(vectors::PAGE_FAULT,
            exceptions::page_fault_handler as u64,
            IdtEntryOptions::interrupt_with_ist(2));
        
        IDT.set_handler(vectors::X87_FLOATING_POINT,
            exceptions::x87_floating_point_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::ALIGNMENT_CHECK,
            exceptions::alignment_check_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::MACHINE_CHECK,
            exceptions::machine_check_handler as u64,
            IdtEntryOptions::interrupt());
        
        IDT.set_handler(vectors::SIMD_FLOATING_POINT,
            exceptions::simd_floating_point_handler as u64,
            IdtEntryOptions::interrupt());
        
        // Load the IDT
        IDT.load();
        
        log::debug!("IDT initialized at {:p}", &raw const IDT);
    }
}

/// Register a handler for an interrupt vector
/// 
/// # Safety
/// The handler must be a valid function pointer that follows the x86_64 interrupt
/// calling convention.
pub unsafe fn set_handler(vector: u8, handler: u64, options: IdtEntryOptions) {
    unsafe { IDT.set_handler(vector, handler, options); }
}

/// Reload the IDT (after modifying handlers)
/// 
/// # Safety
/// The IDT must still be valid.
pub unsafe fn reload() {
    unsafe { IDT.load(); }
}

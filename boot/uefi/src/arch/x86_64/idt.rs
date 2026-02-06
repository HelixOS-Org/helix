//! `x86_64` Interrupt Descriptor Table (IDT)
//!
//! Complete IDT implementation for 64-bit mode.

use core::mem::size_of;

use super::gdt::selectors;

// =============================================================================
// IDT CONSTANTS
// =============================================================================

/// Number of IDT entries (256 vectors)
pub const IDT_ENTRIES: usize = 256;

/// Exception vector numbers
pub mod vectors {
    pub const DIVIDE_ERROR: u8 = 0;
    pub const DEBUG: u8 = 1;
    pub const NMI: u8 = 2;
    pub const BREAKPOINT: u8 = 3;
    pub const OVERFLOW: u8 = 4;
    pub const BOUND_RANGE: u8 = 5;
    pub const INVALID_OPCODE: u8 = 6;
    pub const DEVICE_NOT_AVAILABLE: u8 = 7;
    pub const DOUBLE_FAULT: u8 = 8;
    pub const COPROCESSOR_SEGMENT: u8 = 9;
    pub const INVALID_TSS: u8 = 10;
    pub const SEGMENT_NOT_PRESENT: u8 = 11;
    pub const STACK_SEGMENT: u8 = 12;
    pub const GENERAL_PROTECTION: u8 = 13;
    pub const PAGE_FAULT: u8 = 14;
    pub const X87_FPU: u8 = 16;
    pub const ALIGNMENT_CHECK: u8 = 17;
    pub const MACHINE_CHECK: u8 = 18;
    pub const SIMD_FPU: u8 = 19;
    pub const VIRTUALIZATION: u8 = 20;
    pub const CONTROL_PROTECTION: u8 = 21;
    pub const HYPERVISOR_INJECTION: u8 = 28;
    pub const VMM_COMMUNICATION: u8 = 29;
    pub const SECURITY_EXCEPTION: u8 = 30;

    /// First user-defined interrupt
    pub const IRQ_BASE: u8 = 32;
    /// Legacy PIC IRQs (remapped)
    pub const PIC_IRQ0: u8 = 32;
    pub const PIC_IRQ7: u8 = 39;
    pub const PIC_IRQ8: u8 = 40;
    pub const PIC_IRQ15: u8 = 47;
    /// APIC timer
    pub const APIC_TIMER: u8 = 48;
    /// APIC spurious
    pub const APIC_SPURIOUS: u8 = 0xFF;
    /// System call (via int instruction)
    pub const SYSCALL: u8 = 0x80;
}

/// Exception names
pub const EXCEPTION_NAMES: [&str; 32] = [
    "Divide Error",
    "Debug",
    "Non-Maskable Interrupt",
    "Breakpoint",
    "Overflow",
    "Bound Range Exceeded",
    "Invalid Opcode",
    "Device Not Available",
    "Double Fault",
    "Coprocessor Segment Overrun",
    "Invalid TSS",
    "Segment Not Present",
    "Stack-Segment Fault",
    "General Protection Fault",
    "Page Fault",
    "Reserved",
    "x87 FPU Error",
    "Alignment Check",
    "Machine Check",
    "SIMD Floating-Point",
    "Virtualization Exception",
    "Control Protection Exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Hypervisor Injection",
    "VMM Communication",
    "Security Exception",
    "Reserved",
];

/// Exceptions with error codes
pub const fn has_error_code(vector: u8) -> bool {
    matches!(vector, 8 | 10 | 11 | 12 | 13 | 14 | 17 | 21 | 29 | 30)
}

// =============================================================================
// GATE TYPES
// =============================================================================

/// Gate type values
pub mod gate_type {
    /// Interrupt gate (clears IF)
    pub const INTERRUPT: u8 = 0xE;
    /// Trap gate (preserves IF)
    pub const TRAP: u8 = 0xF;
}

/// Gate attributes
pub mod gate_attr {
    /// Gate present
    pub const PRESENT: u8 = 1 << 7;
    /// DPL 0 (kernel)
    pub const DPL_KERNEL: u8 = 0 << 5;
    /// DPL 3 (user)
    pub const DPL_USER: u8 = 3 << 5;

    /// Kernel interrupt gate
    pub const KERNEL_INTERRUPT: u8 = PRESENT | DPL_KERNEL | super::gate_type::INTERRUPT;
    /// Kernel trap gate
    pub const KERNEL_TRAP: u8 = PRESENT | DPL_KERNEL | super::gate_type::TRAP;
    /// User interrupt gate (callable from ring 3)
    pub const USER_INTERRUPT: u8 = PRESENT | DPL_USER | super::gate_type::INTERRUPT;
    /// User trap gate (callable from ring 3)
    pub const USER_TRAP: u8 = PRESENT | DPL_USER | super::gate_type::TRAP;
}

// =============================================================================
// IDT ENTRY (GATE DESCRIPTOR)
// =============================================================================

/// IDT Entry (Gate Descriptor) - 16 bytes in 64-bit mode
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct IdtEntry {
    /// Offset bits 0-15
    offset_low: u16,
    /// Segment selector
    selector: u16,
    /// IST index (bits 0-2) and reserved
    ist: u8,
    /// Type and attributes
    type_attr: u8,
    /// Offset bits 16-31
    offset_middle: u16,
    /// Offset bits 32-63
    offset_high: u32,
    /// Reserved (must be zero)
    reserved: u32,
}

impl IdtEntry {
    /// Create empty (not present) entry
    pub const fn missing() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_middle: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create new IDT entry
    pub fn new(handler: u64, selector: u16, ist: u8, type_attr: u8) -> Self {
        Self {
            offset_low: u16::try_from(handler & 0xFFFF).unwrap_or(0),
            selector,
            ist: ist & 0x7, // Only 3 bits for IST
            type_attr,
            offset_middle: u16::try_from((handler >> 16) & 0xFFFF).unwrap_or(0),
            offset_high: u32::try_from(handler >> 32).unwrap_or(0),
            reserved: 0,
        }
    }

    /// Create kernel interrupt gate
    pub fn interrupt(handler: u64) -> Self {
        Self::new(
            handler,
            selectors::KERNEL_CODE,
            0,
            gate_attr::KERNEL_INTERRUPT,
        )
    }

    /// Create kernel interrupt gate with IST
    pub fn interrupt_ist(handler: u64, ist: u8) -> Self {
        Self::new(
            handler,
            selectors::KERNEL_CODE,
            ist,
            gate_attr::KERNEL_INTERRUPT,
        )
    }

    /// Create kernel trap gate
    pub fn trap(handler: u64) -> Self {
        Self::new(handler, selectors::KERNEL_CODE, 0, gate_attr::KERNEL_TRAP)
    }

    /// Create user-callable interrupt gate
    pub fn user_interrupt(handler: u64) -> Self {
        Self::new(
            handler,
            selectors::KERNEL_CODE,
            0,
            gate_attr::USER_INTERRUPT,
        )
    }

    /// Get handler offset
    pub const fn offset(&self) -> u64 {
        (self.offset_low as u64)
            | ((self.offset_middle as u64) << 16)
            | ((self.offset_high as u64) << 32)
    }

    /// Set handler offset
    pub fn set_offset(&mut self, offset: u64) {
        self.offset_low = u16::try_from(offset & 0xFFFF).unwrap_or(0);
        self.offset_middle = u16::try_from((offset >> 16) & 0xFFFF).unwrap_or(0);
        self.offset_high = u32::try_from(offset >> 32).unwrap_or(0);
    }

    /// Get selector
    pub const fn selector(&self) -> u16 {
        self.selector
    }

    /// Get IST index
    pub const fn ist(&self) -> u8 {
        self.ist & 0x7
    }

    /// Check if present
    pub const fn is_present(&self) -> bool {
        (self.type_attr & gate_attr::PRESENT) != 0
    }

    /// Get DPL
    pub const fn dpl(&self) -> u8 {
        (self.type_attr >> 5) & 3
    }

    /// Get gate type
    pub const fn gate_type(&self) -> u8 {
        self.type_attr & 0xF
    }
}

impl Default for IdtEntry {
    fn default() -> Self {
        Self::missing()
    }
}

// =============================================================================
// INTERRUPT STACK FRAME
// =============================================================================

/// Interrupt stack frame pushed by CPU
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptStackFrame {
    /// Instruction pointer
    pub rip: u64,
    /// Code segment
    pub cs: u64,
    /// RFLAGS
    pub rflags: u64,
    /// Stack pointer
    pub rsp: u64,
    /// Stack segment
    pub ss: u64,
}

impl InterruptStackFrame {
    /// Check if interrupted from user mode
    pub const fn from_user(&self) -> bool {
        (self.cs & 3) == 3
    }

    /// Check if interrupted from kernel mode
    pub const fn from_kernel(&self) -> bool {
        self.cs.trailing_zeros() >= 2
    }
}

/// Full interrupt context (including saved registers)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InterruptContext {
    // Saved by interrupt stub
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
    /// Vector number
    pub vector: u64,
    /// Error code (or 0)
    pub error_code: u64,
    /// Interrupt stack frame
    pub frame: InterruptStackFrame,
}

impl InterruptContext {
    /// Get exception name
    pub const fn exception_name(&self) -> &'static str {
        if self.vector < 32 {
            EXCEPTION_NAMES[self.vector as usize]
        } else {
            "Interrupt"
        }
    }

    /// Check if has error code
    pub const fn has_error_code(&self) -> bool {
        has_error_code((self.vector & 0xFF) as u8)
    }
}

// =============================================================================
// PAGE FAULT ERROR CODE
// =============================================================================

/// Page fault error code bits
pub mod page_fault {
    /// Fault was a protection violation (else, page not present)
    pub const PROTECTION: u64 = 1 << 0;
    /// Write access (else, read)
    pub const WRITE: u64 = 1 << 1;
    /// User mode access (else, kernel)
    pub const USER: u64 = 1 << 2;
    /// Reserved bit violation
    pub const RESERVED: u64 = 1 << 3;
    /// Instruction fetch (else, data access)
    pub const INSTRUCTION: u64 = 1 << 4;
    /// Protection key violation
    pub const PROTECTION_KEY: u64 = 1 << 5;
    /// Shadow stack access
    pub const SHADOW_STACK: u64 = 1 << 6;
    /// SGX-specific
    pub const SGX: u64 = 1 << 15;
}

/// Parse page fault error code
pub struct PageFaultError(pub u64);

impl PageFaultError {
    pub const fn present(&self) -> bool {
        (self.0 & page_fault::PROTECTION) != 0
    }
    pub const fn write(&self) -> bool {
        (self.0 & page_fault::WRITE) != 0
    }
    pub const fn user(&self) -> bool {
        (self.0 & page_fault::USER) != 0
    }
    pub const fn reserved(&self) -> bool {
        (self.0 & page_fault::RESERVED) != 0
    }
    pub const fn instruction(&self) -> bool {
        (self.0 & page_fault::INSTRUCTION) != 0
    }

    pub fn describe(&self) -> &'static str {
        match (self.present(), self.write(), self.user()) {
            (false, false, false) => "Kernel read from non-present page",
            (false, false, true) => "User read from non-present page",
            (false, true, false) => "Kernel write to non-present page",
            (false, true, true) => "User write to non-present page",
            (true, false, false) => "Kernel read protection violation",
            (true, false, true) => "User read protection violation",
            (true, true, false) => "Kernel write protection violation",
            (true, true, true) => "User write protection violation",
        }
    }
}

// =============================================================================
// IDT STRUCTURE
// =============================================================================

/// Interrupt Descriptor Table
#[repr(C, align(16))]
pub struct Idt {
    /// IDT entries
    entries: [IdtEntry; IDT_ENTRIES],
}

impl Idt {
    /// Create new empty IDT
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::missing(); IDT_ENTRIES],
        }
    }

    /// Set entry
    pub fn set(&mut self, vector: u8, entry: IdtEntry) {
        self.entries[vector as usize] = entry;
    }

    /// Get entry
    pub const fn get(&self, vector: u8) -> &IdtEntry {
        &self.entries[vector as usize]
    }

    /// Get entry (mutable)
    pub fn get_mut(&mut self, vector: u8) -> &mut IdtEntry {
        &mut self.entries[vector as usize]
    }

    /// Set interrupt handler
    pub fn set_handler(&mut self, vector: u8, handler: u64) {
        self.entries[vector as usize] = IdtEntry::interrupt(handler);
    }

    /// Set interrupt handler with IST
    pub fn set_handler_ist(&mut self, vector: u8, handler: u64, ist: u8) {
        self.entries[vector as usize] = IdtEntry::interrupt_ist(handler, ist);
    }

    /// Set trap handler
    pub fn set_trap(&mut self, vector: u8, handler: u64) {
        self.entries[vector as usize] = IdtEntry::trap(handler);
    }

    /// Get IDT pointer
    pub fn pointer(&self) -> IdtPointer {
        IdtPointer {
            limit: u16::try_from(size_of::<Self>() - 1).unwrap_or(u16::MAX),
            base: self as *const _ as u64,
        }
    }

    /// Load this IDT
    ///
    /// # Safety
    /// The caller must ensure this IDT remains valid while loaded.
    pub unsafe fn load(&self) {
        let ptr = self.pointer();
        load_idt(&ptr);
    }
}

impl Default for Idt {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// IDT POINTER
// =============================================================================

/// IDT pointer for LIDT instruction
#[repr(C, packed)]
pub struct IdtPointer {
    /// Size of IDT - 1
    pub limit: u16,
    /// Base address of IDT
    pub base: u64,
}

/// Load IDT
///
/// # Safety
/// The caller must ensure the pointer is valid and points to a well-formed IDT.
#[inline]
pub unsafe fn load_idt(idt: &IdtPointer) {
    core::arch::asm!(
        "lidt [{}]",
        in(reg) idt,
        options(nostack, preserves_flags),
    );
}

/// Store IDT
#[inline]
pub fn store_idt() -> IdtPointer {
    let mut idt = IdtPointer { limit: 0, base: 0 };
    unsafe {
        core::arch::asm!(
            "sidt [{}]",
            in(reg) &mut idt,
            options(nostack, preserves_flags),
        );
    }
    idt
}

// =============================================================================
// INTERRUPT HANDLER TYPE
// =============================================================================

/// Interrupt handler function type (without error code)
pub type InterruptHandler = extern "x86-interrupt" fn(InterruptStackFrame);

/// Interrupt handler function type (with error code)
pub type InterruptHandlerWithError = extern "x86-interrupt" fn(InterruptStackFrame, u64);

/// Generic interrupt handler (for raw handlers)
pub type RawInterruptHandler = extern "C" fn(*mut InterruptContext);

// =============================================================================
// STATIC IDT
// =============================================================================

use core::cell::UnsafeCell;

/// Wrapper for CPU-referenced static data.
#[repr(transparent)]
struct CpuStatic<T>(UnsafeCell<T>);

impl<T> CpuStatic<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

// SAFETY: IDT only mutated during single-CPU init.
unsafe impl<T: Sync> Sync for CpuStatic<T> {}

/// Static IDT instance
static STATIC_IDT: CpuStatic<Idt> = CpuStatic::new(Idt::new());

/// Initialize static IDT
///
/// # Safety
/// Must be called exactly once during early boot on a single CPU.
pub unsafe fn init_static() {
    let idt = unsafe { STATIC_IDT.as_mut() };
    idt.load();
}

/// Set handler in static IDT
///
/// # Safety
/// Must only be called after `init_static` and with a valid handler address.
pub unsafe fn set_handler(vector: u8, handler: u64) {
    let idt = unsafe { STATIC_IDT.as_mut() };
    idt.set_handler(vector, handler);
}

/// Set handler with IST in static IDT
///
/// # Safety
/// Must only be called after `init_static` and with valid handler/IST values.
pub unsafe fn set_handler_ist(vector: u8, handler: u64, ist: u8) {
    let table = unsafe { STATIC_IDT.as_mut() };
    table.set_handler_ist(vector, handler, ist);
}

/// Get static IDT
///
/// # Safety
/// Returns a mutable reference to a global static; callers must ensure no aliasing.
pub unsafe fn get_static_idt() -> &'static mut Idt {
    unsafe { STATIC_IDT.as_mut() }
}

// =============================================================================
// INTERRUPT STUB GENERATION
// =============================================================================

/// Macro to generate interrupt stub
#[macro_export]
macro_rules! interrupt_stub {
    ($name:ident, $vector:expr) => {
        #[naked]
        pub unsafe extern "C" fn $name() {
            core::arch::asm!(
                // Push dummy error code if needed
                concat!("push 0"), // or "nop" for vectors with error code
                concat!("push ", stringify!($vector)),
                "jmp interrupt_common",
                options(noreturn)
            );
        }
    };
    ($name:ident, $vector:expr,error_code) => {
        #[naked]
        pub unsafe extern "C" fn $name() {
            core::arch::asm!(
                // Error code already pushed by CPU
                concat!("push ", stringify!($vector)),
                "jmp interrupt_common",
                options(noreturn)
            );
        }
    };
}

// Common interrupt handler stub (assembly)
// This saves all registers and calls the Rust handler
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
.global interrupt_common
interrupt_common:
    // Save all general purpose registers
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    // Call Rust handler with pointer to context
    mov rdi, rsp
    call interrupt_handler_rust

    // Restore registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    // Remove error code and vector
    add rsp, 16

    // Return from interrupt
    iretq
"#
);

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_idt_entry_size() {
        assert_eq!(size_of::<IdtEntry>(), 16);
    }

    #[test]
    fn test_idt_size() {
        assert_eq!(size_of::<Idt>(), 256 * 16);
    }

    #[test]
    fn test_missing_entry() {
        let entry = IdtEntry::missing();
        assert!(!entry.is_present());
        assert_eq!(entry.offset(), 0);
    }

    #[test]
    fn test_interrupt_entry() {
        let entry = IdtEntry::interrupt(0xDEADBEEF12345678);
        assert!(entry.is_present());
        assert_eq!(entry.offset(), 0xDEADBEEF12345678);
        assert_eq!(entry.dpl(), 0);
        assert_eq!(entry.gate_type(), gate_type::INTERRUPT);
    }

    #[test]
    fn test_trap_entry() {
        let entry = IdtEntry::trap(0x1234);
        assert!(entry.is_present());
        assert_eq!(entry.gate_type(), gate_type::TRAP);
    }

    #[test]
    fn test_ist_entry() {
        let entry = IdtEntry::interrupt_ist(0x1000, 1);
        assert_eq!(entry.ist(), 1);
    }

    #[test]
    fn test_has_error_code() {
        assert!(has_error_code(vectors::DOUBLE_FAULT));
        assert!(has_error_code(vectors::PAGE_FAULT));
        assert!(has_error_code(vectors::GENERAL_PROTECTION));
        assert!(!has_error_code(vectors::DIVIDE_ERROR));
        assert!(!has_error_code(vectors::BREAKPOINT));
    }

    #[test]
    fn test_page_fault_error() {
        let err = PageFaultError(page_fault::WRITE | page_fault::USER);
        assert!(err.write());
        assert!(err.user());
        assert!(!err.present());
    }
}

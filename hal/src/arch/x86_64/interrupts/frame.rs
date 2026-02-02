//! # Interrupt Stack Frame
//!
//! This module defines the stack frame layout pushed by the CPU
//! when an interrupt or exception occurs in 64-bit mode.
//!
//! ## Stack Layout on Interrupt (no privilege change)
//!
//! ```text
//!  (Higher addresses)
//!  ├──────────────────────┤
//!  │      SS (padded)     │ ← Old stack segment (always pushed in 64-bit)
//!  ├──────────────────────┤
//!  │        RSP           │ ← Old stack pointer
//!  ├──────────────────────┤
//!  │      RFLAGS          │ ← Flags register
//!  ├──────────────────────┤
//!  │      CS (padded)     │ ← Old code segment
//!  ├──────────────────────┤
//!  │        RIP           │ ← Return address
//!  ├──────────────────────┤
//!  │    Error Code        │ ← Only for some exceptions (optional)
//!  ├──────────────────────┤ ← RSP after interrupt entry
//!  (Lower addresses)
//! ```

use core::fmt;

use bitflags::bitflags;

// =============================================================================
// Interrupt Stack Frame
// =============================================================================

/// Interrupt Stack Frame
///
/// This structure represents the stack frame pushed by the CPU
/// when handling an interrupt or exception in 64-bit long mode.
///
/// All fields are 64-bit aligned as required by the x86-64 ABI.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct InterruptStackFrame {
    /// Instruction pointer at the time of interrupt
    pub rip: u64,

    /// Code segment selector (padded to 64 bits)
    pub cs: u64,

    /// CPU flags register
    pub rflags: u64,

    /// Stack pointer at the time of interrupt
    pub rsp: u64,

    /// Stack segment selector (padded to 64 bits)
    pub ss: u64,
}

impl InterruptStackFrame {
    /// Check if the interrupt came from user mode
    #[inline]
    pub const fn is_user_mode(&self) -> bool {
        // RPL (bits 0-1) of CS indicates privilege level
        (self.cs & 0x3) == 3
    }

    /// Check if the interrupt came from kernel mode
    #[inline]
    pub const fn is_kernel_mode(&self) -> bool {
        (self.cs & 0x3) == 0
    }

    /// Get the privilege level at interrupt time
    #[inline]
    pub const fn privilege_level(&self) -> u8 {
        (self.cs & 0x3) as u8
    }

    /// Get the code segment selector
    #[inline]
    pub const fn code_segment(&self) -> u16 {
        self.cs as u16
    }

    /// Get the stack segment selector
    #[inline]
    pub const fn stack_segment(&self) -> u16 {
        self.ss as u16
    }

    /// Get the RFLAGS as typed flags
    #[inline]
    pub const fn flags(&self) -> RFlags {
        RFlags::from_bits_truncate(self.rflags)
    }

    /// Check if interrupts were enabled before this interrupt
    #[inline]
    pub const fn interrupts_enabled(&self) -> bool {
        self.rflags & RFlags::INTERRUPT_FLAG.bits() != 0
    }
}

impl fmt::Debug for InterruptStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptStackFrame")
            .field("rip", &format_args!("{:#018x}", self.rip))
            .field("cs", &format_args!("{:#06x}", self.cs))
            .field("rflags", &format_args!("{:#018x}", self.rflags))
            .field("rsp", &format_args!("{:#018x}", self.rsp))
            .field("ss", &format_args!("{:#06x}", self.ss))
            .field("cpl", &self.privilege_level())
            .finish()
    }
}

impl fmt::Display for InterruptStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Interrupt Stack Frame:")?;
        writeln!(f, "  RIP:    {:#018x}", self.rip)?;
        writeln!(
            f,
            "  CS:     {:#06x} (CPL={})",
            self.cs,
            self.privilege_level()
        )?;
        writeln!(f, "  RFLAGS: {:#018x}", self.rflags)?;
        writeln!(f, "  RSP:    {:#018x}", self.rsp)?;
        writeln!(f, "  SS:     {:#06x}", self.ss)?;
        write!(f, "  Flags:  {:?}", self.flags())
    }
}

// =============================================================================
// Exception Stack Frame (with error code)
// =============================================================================

/// Exception Stack Frame with Error Code
///
/// Some exceptions push an error code onto the stack before the
/// standard interrupt stack frame.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ExceptionStackFrame {
    /// Exception-specific error code
    pub error_code: u64,

    /// Instruction pointer at the time of exception
    pub rip: u64,

    /// Code segment selector (padded to 64 bits)
    pub cs: u64,

    /// CPU flags register
    pub rflags: u64,

    /// Stack pointer at the time of exception
    pub rsp: u64,

    /// Stack segment selector (padded to 64 bits)
    pub ss: u64,
}

impl ExceptionStackFrame {
    /// Get the interrupt stack frame portion (without error code)
    #[inline]
    pub const fn as_interrupt_frame(&self) -> InterruptStackFrame {
        InterruptStackFrame {
            rip: self.rip,
            cs: self.cs,
            rflags: self.rflags,
            rsp: self.rsp,
            ss: self.ss,
        }
    }

    /// Check if the exception came from user mode
    #[inline]
    pub const fn is_user_mode(&self) -> bool {
        (self.cs & 0x3) == 3
    }

    /// Check if the exception came from kernel mode
    #[inline]
    pub const fn is_kernel_mode(&self) -> bool {
        (self.cs & 0x3) == 0
    }

    /// Get the privilege level at exception time
    #[inline]
    pub const fn privilege_level(&self) -> u8 {
        (self.cs & 0x3) as u8
    }
}

impl fmt::Debug for ExceptionStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExceptionStackFrame")
            .field("error_code", &format_args!("{:#018x}", self.error_code))
            .field("rip", &format_args!("{:#018x}", self.rip))
            .field("cs", &format_args!("{:#06x}", self.cs))
            .field("rflags", &format_args!("{:#018x}", self.rflags))
            .field("rsp", &format_args!("{:#018x}", self.rsp))
            .field("ss", &format_args!("{:#06x}", self.ss))
            .finish()
    }
}

impl fmt::Display for ExceptionStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Exception Stack Frame:")?;
        writeln!(f, "  Error:  {:#018x}", self.error_code)?;
        writeln!(f, "  RIP:    {:#018x}", self.rip)?;
        writeln!(
            f,
            "  CS:     {:#06x} (CPL={})",
            self.cs,
            self.privilege_level()
        )?;
        writeln!(f, "  RFLAGS: {:#018x}", self.rflags)?;
        writeln!(f, "  RSP:    {:#018x}", self.rsp)?;
        write!(f, "  SS:     {:#06x}", self.ss)
    }
}

// =============================================================================
// Page Fault Error Code
// =============================================================================

bitflags! {
    /// Page Fault Error Code
    ///
    /// The error code pushed on #PF contains information about what
    /// caused the page fault.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFaultErrorCode: u64 {
        /// The fault was caused by a protection violation (1) vs not-present (0)
        const PROTECTION_VIOLATION = 1 << 0;

        /// The fault was caused by a write (1) vs read (0)
        const WRITE = 1 << 1;

        /// The fault occurred in user mode (1) vs supervisor mode (0)
        const USER_MODE = 1 << 2;

        /// The fault was caused by reserved bit violation
        const RESERVED_WRITE = 1 << 3;

        /// The fault was caused by an instruction fetch
        const INSTRUCTION_FETCH = 1 << 4;

        /// The fault was caused by protection-key violation
        const PROTECTION_KEY = 1 << 5;

        /// The fault was caused by shadow stack access
        const SHADOW_STACK = 1 << 6;

        /// The fault was caused by SGX violation
        const SGX = 1 << 15;
    }
}

impl PageFaultErrorCode {
    /// Check if this was a present page (protection violation)
    #[inline]
    pub const fn is_present(&self) -> bool {
        self.contains(Self::PROTECTION_VIOLATION)
    }

    /// Check if this was a write access
    #[inline]
    pub const fn is_write(&self) -> bool {
        self.contains(Self::WRITE)
    }

    /// Check if this was a read access
    #[inline]
    pub const fn is_read(&self) -> bool {
        !self.is_write()
    }

    /// Check if fault occurred in user mode
    #[inline]
    pub const fn is_user(&self) -> bool {
        self.contains(Self::USER_MODE)
    }

    /// Check if fault was caused by instruction fetch
    #[inline]
    pub const fn is_instruction_fetch(&self) -> bool {
        self.contains(Self::INSTRUCTION_FETCH)
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match (self.is_present(), self.is_write(), self.is_user()) {
            (false, false, false) => "Kernel read non-present page",
            (false, false, true) => "User read non-present page",
            (false, true, false) => "Kernel write non-present page",
            (false, true, true) => "User write non-present page",
            (true, false, false) => "Kernel read protection violation",
            (true, false, true) => "User read protection violation",
            (true, true, false) => "Kernel write protection violation",
            (true, true, true) => "User write protection violation",
        }
    }
}

impl fmt::Display for PageFaultErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PageFault: {} ({:#x})", self.description(), self.bits())
    }
}

// =============================================================================
// General Protection Error Code
// =============================================================================

/// General Protection Fault Error Code
///
/// When #GP is caused by a segment-related error, the error code
/// contains a selector index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct GpErrorCode(pub u64);

impl GpErrorCode {
    /// Check if this refers to an external event (interrupt)
    #[inline]
    pub const fn is_external(&self) -> bool {
        self.0 & 1 != 0
    }

    /// Get the descriptor table type
    #[inline]
    pub const fn table_type(&self) -> DescriptorTable {
        match (self.0 >> 1) & 0x3 {
            0 => DescriptorTable::Gdt,
            1 => DescriptorTable::Idt,
            2 => DescriptorTable::Ldt,
            _ => DescriptorTable::Idt, // 3 also means IDT
        }
    }

    /// Get the selector index
    #[inline]
    pub const fn selector_index(&self) -> u16 {
        ((self.0 >> 3) & 0x1FFF) as u16
    }

    /// Check if error code is zero (null selector or other cause)
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for GpErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "GP: null/unknown")
        } else {
            write!(
                f,
                "GP: {:?} index {} (ext={})",
                self.table_type(),
                self.selector_index(),
                self.is_external()
            )
        }
    }
}

/// Descriptor Table Type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorTable {
    /// Global Descriptor Table
    Gdt,
    /// Interrupt Descriptor Table
    Idt,
    /// Local Descriptor Table
    Ldt,
}

// =============================================================================
// RFLAGS
// =============================================================================

bitflags! {
    /// x86_64 RFLAGS Register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RFlags: u64 {
        /// Carry Flag
        const CARRY_FLAG = 1 << 0;
        /// Parity Flag
        const PARITY_FLAG = 1 << 2;
        /// Auxiliary Carry Flag
        const AUXILIARY_CARRY_FLAG = 1 << 4;
        /// Zero Flag
        const ZERO_FLAG = 1 << 6;
        /// Sign Flag
        const SIGN_FLAG = 1 << 7;
        /// Trap Flag (single step)
        const TRAP_FLAG = 1 << 8;
        /// Interrupt Enable Flag
        const INTERRUPT_FLAG = 1 << 9;
        /// Direction Flag
        const DIRECTION_FLAG = 1 << 10;
        /// Overflow Flag
        const OVERFLOW_FLAG = 1 << 11;
        /// I/O Privilege Level (2 bits)
        const IOPL_LOW = 1 << 12;
        const IOPL_HIGH = 1 << 13;
        /// Nested Task
        const NESTED_TASK = 1 << 14;
        /// Resume Flag
        const RESUME_FLAG = 1 << 16;
        /// Virtual 8086 Mode
        const VIRTUAL_8086_MODE = 1 << 17;
        /// Alignment Check / Access Control
        const ALIGNMENT_CHECK = 1 << 18;
        /// Virtual Interrupt Flag
        const VIRTUAL_INTERRUPT_FLAG = 1 << 19;
        /// Virtual Interrupt Pending
        const VIRTUAL_INTERRUPT_PENDING = 1 << 20;
        /// ID Flag (CPUID available)
        const ID_FLAG = 1 << 21;
    }
}

impl RFlags {
    /// Get IOPL (I/O Privilege Level)
    #[inline]
    pub const fn iopl(&self) -> u8 {
        ((self.bits() >> 12) & 0x3) as u8
    }
}

// =============================================================================
// Saved Register State
// =============================================================================

/// Complete CPU Register State
///
/// This structure holds all general-purpose registers, useful for
/// context switching and full state preservation.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct RegisterState {
    // General purpose registers (in reverse order for PUSHAQ compatibility)
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
}

impl RegisterState {
    /// Create empty register state
    pub const fn new() -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rbp: 0,
            rdi: 0,
            rsi: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
        }
    }
}

impl fmt::Display for RegisterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Register State:")?;
        writeln!(f, "  RAX: {:#018x}  RBX: {:#018x}", self.rax, self.rbx)?;
        writeln!(f, "  RCX: {:#018x}  RDX: {:#018x}", self.rcx, self.rdx)?;
        writeln!(f, "  RSI: {:#018x}  RDI: {:#018x}", self.rsi, self.rdi)?;
        writeln!(f, "  RBP: {:#018x}  R8:  {:#018x}", self.rbp, self.r8)?;
        writeln!(f, "  R9:  {:#018x}  R10: {:#018x}", self.r9, self.r10)?;
        writeln!(f, "  R11: {:#018x}  R12: {:#018x}", self.r11, self.r12)?;
        writeln!(f, "  R13: {:#018x}  R14: {:#018x}", self.r13, self.r14)?;
        write!(f, "  R15: {:#018x}", self.r15)
    }
}

// =============================================================================
// Full Interrupt Context
// =============================================================================

/// Full Interrupt Context
///
/// Combines saved registers with the interrupt stack frame.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptContext {
    /// Saved general-purpose registers
    pub regs: RegisterState,

    /// Vector number (pushed by handler stub)
    pub vector: u64,

    /// Error code (or dummy for exceptions without error code)
    pub error_code: u64,

    /// CPU-pushed interrupt stack frame
    pub frame: InterruptStackFrame,
}

impl InterruptContext {
    /// Check if this interrupt came from user mode
    #[inline]
    pub const fn is_user_mode(&self) -> bool {
        self.frame.is_user_mode()
    }
}

impl fmt::Display for InterruptContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Interrupt Context (Vector {:#04x}):", self.vector)?;
        writeln!(f)?;
        writeln!(f, "{}", self.regs)?;
        writeln!(f)?;
        writeln!(f, "  Error Code: {:#018x}", self.error_code)?;
        writeln!(f)?;
        write!(f, "{}", self.frame)
    }
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    use core::mem::size_of;

    // Stack frame sizes
    assert!(size_of::<InterruptStackFrame>() == 40); // 5 x 8 bytes
    assert!(size_of::<ExceptionStackFrame>() == 48); // 6 x 8 bytes
    assert!(size_of::<RegisterState>() == 120); // 15 x 8 bytes
};

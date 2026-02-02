//! # Interrupt Controller Abstraction
//!
//! This module defines traits for interrupt handling across architectures.

use core::fmt::Debug;

use crate::HalResult;

/// Interrupt vector number
pub type InterruptVector = u8;

/// Interrupt priority level
pub type InterruptPriority = u8;

/// Interrupt handler function type
pub type InterruptHandler = fn(vector: InterruptVector, context: &mut dyn InterruptContext);

/// Interrupt controller abstraction
pub trait InterruptController: Send + Sync {
    /// Initialize the interrupt controller
    fn init(&mut self) -> HalResult<()>;

    /// Enable the interrupt controller
    fn enable(&mut self);

    /// Disable the interrupt controller
    fn disable(&mut self);

    /// Get the number of interrupt lines
    fn interrupt_count(&self) -> usize;

    /// Enable a specific interrupt
    fn enable_interrupt(&mut self, vector: InterruptVector) -> HalResult<()>;

    /// Disable a specific interrupt
    fn disable_interrupt(&mut self, vector: InterruptVector) -> HalResult<()>;

    /// Check if an interrupt is enabled
    fn is_interrupt_enabled(&self, vector: InterruptVector) -> bool;

    /// Set the priority for an interrupt
    fn set_priority(
        &mut self,
        vector: InterruptVector,
        priority: InterruptPriority,
    ) -> HalResult<()>;

    /// Get the priority for an interrupt
    fn get_priority(&self, vector: InterruptVector) -> HalResult<InterruptPriority>;

    /// Set the priority threshold (interrupts below this are masked)
    fn set_priority_threshold(&mut self, threshold: InterruptPriority);

    /// Acknowledge an interrupt (signal end of handling)
    fn acknowledge(&mut self, vector: InterruptVector);

    /// Send an Inter-Processor Interrupt
    fn send_ipi(&mut self, target: IpiTarget, vector: InterruptVector) -> HalResult<()>;

    /// Get the currently pending interrupt (if any)
    fn pending_interrupt(&self) -> Option<InterruptVector>;

    /// Check if an interrupt is pending
    fn is_interrupt_pending(&self, vector: InterruptVector) -> bool;

    /// Clear a pending interrupt
    fn clear_pending(&mut self, vector: InterruptVector);

    /// Configure interrupt trigger mode
    fn set_trigger_mode(&mut self, vector: InterruptVector, mode: TriggerMode) -> HalResult<()>;
}

/// Target for Inter-Processor Interrupts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpiTarget {
    /// Send to self
    Current,
    /// Send to a specific CPU
    Cpu(usize),
    /// Send to all CPUs
    All,
    /// Send to all CPUs except self
    AllExceptSelf,
}

/// Interrupt trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    /// Edge-triggered
    Edge,
    /// Level-triggered (high)
    LevelHigh,
    /// Level-triggered (low)
    LevelLow,
}

/// Interrupt context trait
///
/// Provides access to the processor state at the time of the interrupt.
pub trait InterruptContext: Send {
    /// Get the instruction pointer at the time of the interrupt
    fn instruction_pointer(&self) -> u64;

    /// Get the stack pointer at the time of the interrupt
    fn stack_pointer(&self) -> u64;

    /// Get the interrupt vector number
    fn vector(&self) -> InterruptVector;

    /// Get the error code (for exceptions)
    fn error_code(&self) -> Option<u64>;

    /// Check if the interrupt occurred in user mode
    fn from_user_mode(&self) -> bool;

    /// Set the return instruction pointer
    fn set_instruction_pointer(&mut self, ip: u64);

    /// Get a general-purpose register by index
    fn register(&self, index: usize) -> u64;

    /// Set a general-purpose register by index
    fn set_register(&mut self, index: usize, value: u64);
}

/// Exception types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exception {
    /// Division by zero
    DivisionByZero,
    /// Debug exception
    Debug,
    /// Non-maskable interrupt
    NonMaskable,
    /// Breakpoint
    Breakpoint,
    /// Overflow
    Overflow,
    /// Bound range exceeded
    BoundRangeExceeded,
    /// Invalid opcode
    InvalidOpcode,
    /// Device not available
    DeviceNotAvailable,
    /// Double fault
    DoubleFault,
    /// Invalid TSS
    InvalidTss,
    /// Segment not present
    SegmentNotPresent,
    /// Stack segment fault
    StackSegmentFault,
    /// General protection fault
    GeneralProtectionFault,
    /// Page fault
    PageFault,
    /// Floating point exception
    FloatingPoint,
    /// Alignment check
    AlignmentCheck,
    /// Machine check
    MachineCheck,
    /// SIMD exception
    Simd,
    /// Virtualization exception
    Virtualization,
    /// Security exception
    Security,
    /// Unknown exception
    Unknown(u8),
}

/// Page fault error information
#[derive(Debug, Clone, Copy)]
pub struct PageFaultInfo {
    /// The faulting address
    pub address: u64,
    /// Was this a read or write?
    pub was_write: bool,
    /// Was this an instruction fetch?
    pub was_instruction_fetch: bool,
    /// Was the page present?
    pub page_present: bool,
    /// Was this from user mode?
    pub from_user_mode: bool,
    /// Was this a reserved bit violation?
    pub reserved_bit_violation: bool,
}

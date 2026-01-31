//! # RISC-V Trap Handling
//!
//! This module provides the trap handling infrastructure for RISC-V.
//!
//! ## Trap Types
//!
//! RISC-V traps are divided into two categories:
//! - **Exceptions**: Synchronous events caused by instruction execution
//! - **Interrupts**: Asynchronous events from external sources
//!
//! ## Trap Delegation
//!
//! M-mode can delegate traps to S-mode via medeleg/mideleg registers.
//! OpenSBI typically delegates most traps to S-mode.
//!
//! ## Trap Frame
//!
//! On trap entry, we save all registers to enable proper context switching.

use core::arch::asm;
use super::super::core::csr::{self, exception, irq_cause, CAUSE_INTERRUPT_BIT, TrapCause};
use super::super::core::registers::GeneralRegisters;
use super::modes::PrivilegeMode;

// ============================================================================
// Trap Frame Structure
// ============================================================================

/// Complete trap frame saved on kernel stack
///
/// This structure contains all state needed to resume execution
/// after handling a trap.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TrapFrame {
    /// General purpose registers (x0-x31)
    pub regs: GeneralRegisters,
    /// Supervisor exception program counter
    pub sepc: u64,
    /// Supervisor status register
    pub sstatus: u64,
    /// Trap cause
    pub scause: u64,
    /// Trap value (faulting address, instruction, etc.)
    pub stval: u64,
    /// Original kernel stack pointer (for nested traps)
    pub kernel_sp: u64,
    /// Hart ID
    pub hart_id: u64,
}

impl TrapFrame {
    /// Create an empty trap frame
    pub const fn new() -> Self {
        Self {
            regs: GeneralRegisters::new(),
            sepc: 0,
            sstatus: 0,
            scause: 0,
            stval: 0,
            kernel_sp: 0,
            hart_id: 0,
        }
    }

    /// Get the faulting/returning instruction pointer
    pub fn pc(&self) -> u64 {
        self.sepc
    }

    /// Set the return instruction pointer
    pub fn set_pc(&mut self, pc: u64) {
        self.sepc = pc;
    }

    /// Advance PC past the current instruction
    ///
    /// Used after handling exceptions like ECALL where we want
    /// to resume at the next instruction.
    pub fn advance_pc(&mut self) {
        // RISC-V instructions are either 2 bytes (compressed) or 4 bytes
        // Check the low bits of the instruction at sepc to determine size
        // For now, assume 4-byte instruction (conservative)
        self.sepc += 4;
    }

    /// Advance PC for compressed instruction
    pub fn advance_pc_compressed(&mut self) {
        self.sepc += 2;
    }

    /// Get the syscall number (from a7)
    pub fn syscall_number(&self) -> u64 {
        self.regs.a7
    }

    /// Get syscall arguments
    pub fn syscall_args(&self) -> [u64; 6] {
        [
            self.regs.a0,
            self.regs.a1,
            self.regs.a2,
            self.regs.a3,
            self.regs.a4,
            self.regs.a5,
        ]
    }

    /// Set syscall return value
    pub fn set_syscall_return(&mut self, value: u64) {
        self.regs.a0 = value;
    }

    /// Set syscall error code
    pub fn set_syscall_error(&mut self, error: u64) {
        self.regs.a0 = error;
    }

    /// Get previous privilege mode
    pub fn previous_mode(&self) -> PrivilegeMode {
        use super::super::core::csr::status;
        if self.sstatus & status::SPP != 0 {
            PrivilegeMode::Supervisor
        } else {
            PrivilegeMode::User
        }
    }

    /// Check if trap came from user mode
    pub fn is_user(&self) -> bool {
        self.previous_mode() == PrivilegeMode::User
    }

    /// Check if trap came from kernel mode
    pub fn is_kernel(&self) -> bool {
        self.previous_mode() == PrivilegeMode::Supervisor
    }

    /// Parse the trap cause
    pub fn cause(&self) -> TrapCause {
        TrapCause::from_scause(self.scause)
    }

    /// Check if this is an interrupt (vs exception)
    pub fn is_interrupt(&self) -> bool {
        self.scause & CAUSE_INTERRUPT_BIT != 0
    }

    /// Get the raw cause code
    pub fn cause_code(&self) -> u64 {
        self.scause & !CAUSE_INTERRUPT_BIT
    }

    /// Get the faulting address (for page faults)
    pub fn fault_address(&self) -> u64 {
        self.stval
    }

    /// Get the faulting instruction (for illegal instruction)
    pub fn fault_instruction(&self) -> u64 {
        self.stval
    }
}

impl Default for TrapFrame {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Trap Context
// ============================================================================

/// Additional trap context beyond the frame
#[derive(Debug, Clone)]
pub struct TrapContext {
    /// The trap frame
    pub frame: TrapFrame,
    /// Was this a nested trap (trap in kernel)?
    pub nested: bool,
    /// Nesting depth
    pub depth: u32,
}

impl TrapContext {
    /// Create new trap context
    pub fn new(frame: TrapFrame, nested: bool, depth: u32) -> Self {
        Self { frame, nested, depth }
    }
}

// ============================================================================
// Trap Handlers
// ============================================================================

/// Trap handler result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrapResult {
    /// Trap handled, resume execution
    Handled,
    /// Trap handled, reschedule
    Reschedule,
    /// Trap requires signal delivery to user process
    Signal(i32),
    /// Fatal trap, panic
    Fatal,
}

/// Main trap handler function
///
/// This is called from the assembly trap entry point after
/// saving the trap frame.
pub fn handle_trap(frame: &mut TrapFrame) -> TrapResult {
    let cause = frame.cause();

    if cause.is_interrupt {
        handle_interrupt(frame, cause.code)
    } else {
        handle_exception(frame, cause.code)
    }
}

/// Handle interrupt
fn handle_interrupt(frame: &mut TrapFrame, code: u64) -> TrapResult {
    match code {
        irq_cause::SUPERVISOR_SOFTWARE => {
            handle_software_interrupt(frame)
        }
        irq_cause::SUPERVISOR_TIMER => {
            handle_timer_interrupt(frame)
        }
        irq_cause::SUPERVISOR_EXTERNAL => {
            handle_external_interrupt(frame)
        }
        _ => {
            // Unknown interrupt
            TrapResult::Fatal
        }
    }
}

/// Handle exception
fn handle_exception(frame: &mut TrapFrame, code: u64) -> TrapResult {
    match code {
        exception::INSTRUCTION_MISALIGNED => {
            handle_misaligned_instruction(frame)
        }
        exception::INSTRUCTION_ACCESS_FAULT => {
            handle_instruction_access_fault(frame)
        }
        exception::ILLEGAL_INSTRUCTION => {
            handle_illegal_instruction(frame)
        }
        exception::BREAKPOINT => {
            handle_breakpoint(frame)
        }
        exception::LOAD_MISALIGNED => {
            handle_misaligned_load(frame)
        }
        exception::LOAD_ACCESS_FAULT => {
            handle_load_access_fault(frame)
        }
        exception::STORE_MISALIGNED => {
            handle_misaligned_store(frame)
        }
        exception::STORE_ACCESS_FAULT => {
            handle_store_access_fault(frame)
        }
        exception::ECALL_FROM_U => {
            handle_syscall(frame)
        }
        exception::ECALL_FROM_S => {
            handle_supervisor_call(frame)
        }
        exception::INSTRUCTION_PAGE_FAULT => {
            handle_instruction_page_fault(frame)
        }
        exception::LOAD_PAGE_FAULT => {
            handle_load_page_fault(frame)
        }
        exception::STORE_PAGE_FAULT => {
            handle_store_page_fault(frame)
        }
        _ => {
            // Unknown exception
            TrapResult::Fatal
        }
    }
}

// ============================================================================
// Interrupt Handlers
// ============================================================================

/// Handle supervisor software interrupt (IPI)
fn handle_software_interrupt(_frame: &mut TrapFrame) -> TrapResult {
    // Clear the software interrupt pending bit
    csr::clear_sip(csr::interrupt::SSIP);

    // TODO: Handle IPI (inter-processor interrupt)
    // This is typically used for:
    // - TLB shootdown
    // - Reschedule requests
    // - Function call requests

    TrapResult::Handled
}

/// Handle supervisor timer interrupt
fn handle_timer_interrupt(_frame: &mut TrapFrame) -> TrapResult {
    // The timer interrupt is typically handled by:
    // 1. Clearing the pending timer (via SBI or stimecmp)
    // 2. Calling the scheduler tick function
    // 3. Possibly preempting the current task

    // Clear timer interrupt via SBI (will be replaced by direct stimecmp access)
    // For now, we clear SIE to stop the interrupt until we can handle it
    csr::disable_sie(csr::interrupt::STIP);

    TrapResult::Reschedule
}

/// Handle supervisor external interrupt
fn handle_external_interrupt(_frame: &mut TrapFrame) -> TrapResult {
    // External interrupts come from the PLIC
    // We need to:
    // 1. Claim the interrupt from PLIC
    // 2. Dispatch to the appropriate handler
    // 3. Complete the interrupt

    // TODO: Integrate with PLIC driver

    TrapResult::Handled
}

// ============================================================================
// Exception Handlers
// ============================================================================

/// Handle misaligned instruction fetch
fn handle_misaligned_instruction(frame: &mut TrapFrame) -> TrapResult {
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGBUS)
    } else {
        TrapResult::Fatal
    }
}

/// Handle instruction access fault
fn handle_instruction_access_fault(frame: &mut TrapFrame) -> TrapResult {
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGSEGV)
    } else {
        TrapResult::Fatal
    }
}

/// Handle illegal instruction
fn handle_illegal_instruction(frame: &mut TrapFrame) -> TrapResult {
    // Check if this might be an FPU instruction with FPU disabled
    // If so, enable FPU and retry

    // For user mode, send SIGILL
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGILL)
    } else {
        // Kernel illegal instruction is fatal
        TrapResult::Fatal
    }
}

/// Handle breakpoint (EBREAK)
fn handle_breakpoint(frame: &mut TrapFrame) -> TrapResult {
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGTRAP)
    } else {
        // Kernel breakpoint - invoke debugger
        // For now, just continue past the breakpoint
        frame.advance_pc_compressed(); // EBREAK is a compressed instruction
        TrapResult::Handled
    }
}

/// Handle misaligned load
fn handle_misaligned_load(frame: &mut TrapFrame) -> TrapResult {
    // Some RISC-V implementations don't support misaligned access
    // We could emulate it here, but for now treat as error
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGBUS)
    } else {
        TrapResult::Fatal
    }
}

/// Handle load access fault
fn handle_load_access_fault(frame: &mut TrapFrame) -> TrapResult {
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGSEGV)
    } else {
        TrapResult::Fatal
    }
}

/// Handle misaligned store
fn handle_misaligned_store(frame: &mut TrapFrame) -> TrapResult {
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGBUS)
    } else {
        TrapResult::Fatal
    }
}

/// Handle store access fault
fn handle_store_access_fault(frame: &mut TrapFrame) -> TrapResult {
    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGSEGV)
    } else {
        TrapResult::Fatal
    }
}

/// Handle syscall from user mode
fn handle_syscall(frame: &mut TrapFrame) -> TrapResult {
    // Advance PC past ECALL instruction before handling
    frame.advance_pc();

    // TODO: Dispatch to syscall handler
    // The syscall number is in a7, arguments in a0-a5
    // Return value goes in a0

    TrapResult::Handled
}

/// Handle ECALL from supervisor mode
fn handle_supervisor_call(_frame: &mut TrapFrame) -> TrapResult {
    // S-mode ECALL typically shouldn't happen in normal operation
    // It's used to call into M-mode (SBI)
    TrapResult::Fatal
}

/// Handle instruction page fault
fn handle_instruction_page_fault(frame: &mut TrapFrame) -> TrapResult {
    let addr = frame.fault_address();

    // TODO: Invoke page fault handler
    // - Check if address is valid for the process
    // - Load the page from backing store
    // - Update page tables
    // - Retry the instruction

    let _ = addr; // Suppress unused warning

    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGSEGV)
    } else {
        TrapResult::Fatal
    }
}

/// Handle load page fault
fn handle_load_page_fault(frame: &mut TrapFrame) -> TrapResult {
    let addr = frame.fault_address();

    // TODO: Invoke page fault handler for read fault
    let _ = addr;

    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGSEGV)
    } else {
        TrapResult::Fatal
    }
}

/// Handle store page fault
fn handle_store_page_fault(frame: &mut TrapFrame) -> TrapResult {
    let addr = frame.fault_address();

    // TODO: Invoke page fault handler for write fault
    // This includes copy-on-write handling
    let _ = addr;

    if frame.is_user() {
        TrapResult::Signal(libc_constants::SIGSEGV)
    } else {
        TrapResult::Fatal
    }
}

// ============================================================================
// Signal Constants (libc compatible)
// ============================================================================

mod libc_constants {
    pub const SIGILL: i32 = 4;
    pub const SIGTRAP: i32 = 5;
    pub const SIGBUS: i32 = 7;
    pub const SIGSEGV: i32 = 11;
}

// ============================================================================
// Trap Frame Assembly Helpers
// ============================================================================

/// Size of trap frame in bytes
pub const TRAP_FRAME_SIZE: usize = core::mem::size_of::<TrapFrame>();

/// Offset of each register in the trap frame
pub mod trap_frame_offsets {
    use super::*;

    /// Offset of general registers
    pub const REGS: usize = 0;
    /// Offset of sepc
    pub const SEPC: usize = core::mem::size_of::<GeneralRegisters>();
    /// Offset of sstatus
    pub const SSTATUS: usize = SEPC + 8;
    /// Offset of scause
    pub const SCAUSE: usize = SSTATUS + 8;
    /// Offset of stval
    pub const STVAL: usize = SCAUSE + 8;
    /// Offset of kernel_sp
    pub const KERNEL_SP: usize = STVAL + 8;
    /// Offset of hart_id
    pub const HART_ID: usize = KERNEL_SP + 8;
}

// ============================================================================
// Low-Level Trap Entry/Exit
// ============================================================================

/// Save trap frame to stack
///
/// This is typically done in assembly, but we provide a reference implementation.
#[inline(never)]
#[allow(dead_code)]
unsafe fn save_trap_frame(frame: *mut TrapFrame) {
    asm!(
        // Save all general purpose registers
        "sd x1, 8({0})",    // ra
        "sd x2, 16({0})",   // sp
        "sd x3, 24({0})",   // gp
        "sd x4, 32({0})",   // tp
        "sd x5, 40({0})",   // t0
        "sd x6, 48({0})",   // t1
        "sd x7, 56({0})",   // t2
        "sd x8, 64({0})",   // s0/fp
        "sd x9, 72({0})",   // s1
        "sd x10, 80({0})",  // a0
        "sd x11, 88({0})",  // a1
        "sd x12, 96({0})",  // a2
        "sd x13, 104({0})", // a3
        "sd x14, 112({0})", // a4
        "sd x15, 120({0})", // a5
        "sd x16, 128({0})", // a6
        "sd x17, 136({0})", // a7
        "sd x18, 144({0})", // s2
        "sd x19, 152({0})", // s3
        "sd x20, 160({0})", // s4
        "sd x21, 168({0})", // s5
        "sd x22, 176({0})", // s6
        "sd x23, 184({0})", // s7
        "sd x24, 192({0})", // s8
        "sd x25, 200({0})", // s9
        "sd x26, 208({0})", // s10
        "sd x27, 216({0})", // s11
        "sd x28, 224({0})", // t3
        "sd x29, 232({0})", // t4
        "sd x30, 240({0})", // t5
        "sd x31, 248({0})", // t6
        in(reg) frame,
        options(nostack)
    );
}

/// Restore trap frame from stack
#[inline(never)]
#[allow(dead_code)]
unsafe fn restore_trap_frame(frame: *const TrapFrame) {
    asm!(
        // Restore all general purpose registers
        "ld x1, 8({0})",    // ra
        // Skip x2 (sp) - will be restored separately
        "ld x3, 24({0})",   // gp
        // Skip x4 (tp) - thread pointer should stay
        "ld x5, 40({0})",   // t0
        "ld x6, 48({0})",   // t1
        "ld x7, 56({0})",   // t2
        "ld x8, 64({0})",   // s0/fp
        "ld x9, 72({0})",   // s1
        "ld x10, 80({0})",  // a0
        "ld x11, 88({0})",  // a1
        "ld x12, 96({0})",  // a2
        "ld x13, 104({0})", // a3
        "ld x14, 112({0})", // a4
        "ld x15, 120({0})", // a5
        "ld x16, 128({0})", // a6
        "ld x17, 136({0})", // a7
        "ld x18, 144({0})", // s2
        "ld x19, 152({0})", // s3
        "ld x20, 160({0})", // s4
        "ld x21, 168({0})", // s5
        "ld x22, 176({0})", // s6
        "ld x23, 184({0})", // s7
        "ld x24, 192({0})", // s8
        "ld x25, 200({0})", // s9
        "ld x26, 208({0})", // s10
        "ld x27, 216({0})", // s11
        "ld x28, 224({0})", // t3
        "ld x29, 232({0})", // t4
        "ld x30, 240({0})", // t5
        "ld x31, 248({0})", // t6
        in(reg) frame,
        options(nostack)
    );
}

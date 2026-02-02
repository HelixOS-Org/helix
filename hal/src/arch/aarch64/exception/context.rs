//! # AArch64 Exception Context
//!
//! This module defines the exception context (trap frame) saved when
//! entering an exception handler.

use crate::arch::aarch64::core::fpu::FpuState;
use crate::arch::aarch64::core::registers::Registers;

// =============================================================================
// Trap Frame
// =============================================================================

/// Complete CPU state saved on exception entry
///
/// This structure is pushed onto the stack by the exception vector
/// entry code and contains all state needed to resume execution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    // General purpose registers (x0-x30)
    /// x0 - First argument / return value
    pub x0: u64,
    /// x1 - Second argument / return value
    pub x1: u64,
    /// x2 - Third argument
    pub x2: u64,
    /// x3 - Fourth argument
    pub x3: u64,
    /// x4 - Fifth argument
    pub x4: u64,
    /// x5 - Sixth argument
    pub x5: u64,
    /// x6 - Seventh argument
    pub x6: u64,
    /// x7 - Eighth argument
    pub x7: u64,
    /// x8 - Indirect result location (XR)
    pub x8: u64,
    /// x9-x15 - Caller-saved temporaries
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    /// x16 - Intra-procedure call scratch (IP0)
    pub x16: u64,
    /// x17 - Intra-procedure call scratch (IP1)
    pub x17: u64,
    /// x18 - Platform register
    pub x18: u64,
    /// x19-x28 - Callee-saved registers
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    /// x29 - Frame pointer (FP)
    pub x29: u64,
    /// x30 - Link register (LR)
    pub x30: u64,

    // System state
    /// Stack pointer at exception
    pub sp: u64,
    /// Exception Link Register (return address)
    pub elr: u64,
    /// Saved Program Status Register
    pub spsr: u64,
    /// Exception Syndrome Register
    pub esr: u64,
    /// Fault Address Register
    pub far: u64,
}

impl TrapFrame {
    /// Create a new zeroed trap frame
    pub const fn new() -> Self {
        Self {
            x0: 0,
            x1: 0,
            x2: 0,
            x3: 0,
            x4: 0,
            x5: 0,
            x6: 0,
            x7: 0,
            x8: 0,
            x9: 0,
            x10: 0,
            x11: 0,
            x12: 0,
            x13: 0,
            x14: 0,
            x15: 0,
            x16: 0,
            x17: 0,
            x18: 0,
            x19: 0,
            x20: 0,
            x21: 0,
            x22: 0,
            x23: 0,
            x24: 0,
            x25: 0,
            x26: 0,
            x27: 0,
            x28: 0,
            x29: 0,
            x30: 0,
            sp: 0,
            elr: 0,
            spsr: 0,
            esr: 0,
            far: 0,
        }
    }

    /// Create a trap frame for starting a new thread at EL0
    pub fn for_user_thread(entry: u64, stack: u64) -> Self {
        let mut frame = Self::new();
        frame.elr = entry;
        frame.sp = stack;
        frame.spsr = 0; // EL0t with interrupts enabled
        frame
    }

    /// Create a trap frame for starting a kernel thread at EL1
    pub fn for_kernel_thread(entry: u64, stack: u64) -> Self {
        let mut frame = Self::new();
        frame.elr = entry;
        frame.sp = stack;
        frame.spsr = 0b0101; // EL1h (SP_EL1)
        frame
    }

    /// Get syscall number (x8)
    pub fn syscall_number(&self) -> usize {
        self.x8 as usize
    }

    /// Get syscall arguments (x0-x5)
    pub fn syscall_args(&self) -> [usize; 6] {
        [
            self.x0 as usize,
            self.x1 as usize,
            self.x2 as usize,
            self.x3 as usize,
            self.x4 as usize,
            self.x5 as usize,
        ]
    }

    /// Set syscall return value (x0)
    pub fn set_syscall_return(&mut self, value: usize) {
        self.x0 = value as u64;
    }

    /// Set error return value (x0 = -1, x1 = error code)
    pub fn set_error_return(&mut self, error: usize) {
        self.x0 = (-1i64) as u64;
        self.x1 = error as u64;
    }

    /// Get instruction pointer
    pub fn instruction_pointer(&self) -> u64 {
        self.elr
    }

    /// Get stack pointer
    pub fn stack_pointer(&self) -> u64 {
        self.sp
    }

    /// Get frame pointer
    pub fn frame_pointer(&self) -> u64 {
        self.x29
    }

    /// Get link register
    pub fn link_register(&self) -> u64 {
        self.x30
    }

    /// Get fault address
    pub fn fault_address(&self) -> u64 {
        self.far
    }

    /// Get exception class from ESR
    pub fn exception_class(&self) -> u8 {
        ((self.esr >> 26) & 0x3F) as u8
    }

    /// Get instruction length from ESR (true = 32-bit, false = 16-bit)
    pub fn instruction_is_32bit(&self) -> bool {
        (self.esr & (1 << 25)) != 0
    }

    /// Get ISS (Instruction Specific Syndrome) from ESR
    pub fn instruction_syndrome(&self) -> u32 {
        (self.esr & 0x1FFFFFF) as u32
    }

    /// Check if exception was from EL0
    pub fn from_el0(&self) -> bool {
        (self.spsr & 0xF) == 0
    }

    /// Check if exception was from EL1
    pub fn from_el1(&self) -> bool {
        let mode = self.spsr & 0xF;
        mode == 0b0100 || mode == 0b0101
    }

    /// Advance PC past the faulting instruction
    pub fn advance_pc(&mut self) {
        if self.instruction_is_32bit() {
            self.elr += 4;
        } else {
            self.elr += 2;
        }
    }
}

impl Default for TrapFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "TrapFrame {{")?;
        writeln!(
            f,
            "  x0-x7:   {:016x} {:016x} {:016x} {:016x}",
            self.x0, self.x1, self.x2, self.x3
        )?;
        writeln!(
            f,
            "           {:016x} {:016x} {:016x} {:016x}",
            self.x4, self.x5, self.x6, self.x7
        )?;
        writeln!(
            f,
            "  x8-x15:  {:016x} {:016x} {:016x} {:016x}",
            self.x8, self.x9, self.x10, self.x11
        )?;
        writeln!(
            f,
            "           {:016x} {:016x} {:016x} {:016x}",
            self.x12, self.x13, self.x14, self.x15
        )?;
        writeln!(
            f,
            "  x16-x23: {:016x} {:016x} {:016x} {:016x}",
            self.x16, self.x17, self.x18, self.x19
        )?;
        writeln!(
            f,
            "           {:016x} {:016x} {:016x} {:016x}",
            self.x20, self.x21, self.x22, self.x23
        )?;
        writeln!(
            f,
            "  x24-x30: {:016x} {:016x} {:016x} {:016x}",
            self.x24, self.x25, self.x26, self.x27
        )?;
        writeln!(
            f,
            "           {:016x} {:016x} {:016x}",
            self.x28, self.x29, self.x30
        )?;
        writeln!(f, "  sp:      {:016x}", self.sp)?;
        writeln!(f, "  elr:     {:016x}", self.elr)?;
        writeln!(f, "  spsr:    {:016x}", self.spsr)?;
        writeln!(f, "  esr:     {:016x}", self.esr)?;
        writeln!(f, "  far:     {:016x}", self.far)?;
        write!(f, "}}")
    }
}

// =============================================================================
// Exception Context
// =============================================================================

/// Full exception context including FPU state
#[repr(C)]
pub struct ExceptionContext {
    /// CPU trap frame
    pub trap_frame: TrapFrame,
    /// FPU/SIMD state (optional, for context switch)
    pub fpu_state: Option<FpuState>,
    /// Exception vector offset (for debugging)
    pub vector_offset: u16,
    /// Exception from lower EL
    pub from_lower_el: bool,
    /// Exception from AArch64 (vs AArch32)
    pub from_aarch64: bool,
}

impl ExceptionContext {
    /// Create a new exception context
    pub const fn new() -> Self {
        Self {
            trap_frame: TrapFrame::new(),
            fpu_state: None,
            vector_offset: 0,
            from_lower_el: false,
            from_aarch64: true,
        }
    }

    /// Create from trap frame
    pub fn from_trap_frame(frame: TrapFrame, vector_offset: u16) -> Self {
        let from_lower_el = frame.from_el0();
        Self {
            trap_frame: frame,
            fpu_state: None,
            vector_offset,
            from_lower_el,
            from_aarch64: true,
        }
    }

    /// Convert to basic registers struct
    pub fn to_registers(&self) -> Registers {
        let mut regs = Registers::new();
        regs.x[0] = self.trap_frame.x0;
        regs.x[1] = self.trap_frame.x1;
        regs.x[2] = self.trap_frame.x2;
        regs.x[3] = self.trap_frame.x3;
        regs.x[4] = self.trap_frame.x4;
        regs.x[5] = self.trap_frame.x5;
        regs.x[6] = self.trap_frame.x6;
        regs.x[7] = self.trap_frame.x7;
        regs.x[8] = self.trap_frame.x8;
        regs.x[9] = self.trap_frame.x9;
        regs.x[10] = self.trap_frame.x10;
        regs.x[11] = self.trap_frame.x11;
        regs.x[12] = self.trap_frame.x12;
        regs.x[13] = self.trap_frame.x13;
        regs.x[14] = self.trap_frame.x14;
        regs.x[15] = self.trap_frame.x15;
        regs.x[16] = self.trap_frame.x16;
        regs.x[17] = self.trap_frame.x17;
        regs.x[18] = self.trap_frame.x18;
        regs.x[19] = self.trap_frame.x19;
        regs.x[20] = self.trap_frame.x20;
        regs.x[21] = self.trap_frame.x21;
        regs.x[22] = self.trap_frame.x22;
        regs.x[23] = self.trap_frame.x23;
        regs.x[24] = self.trap_frame.x24;
        regs.x[25] = self.trap_frame.x25;
        regs.x[26] = self.trap_frame.x26;
        regs.x[27] = self.trap_frame.x27;
        regs.x[28] = self.trap_frame.x28;
        regs.x[29] = self.trap_frame.x29;
        regs.x[30] = self.trap_frame.x30;
        regs.sp = self.trap_frame.sp;
        regs.pc = self.trap_frame.elr;
        regs
    }
}

impl Default for ExceptionContext {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Context Switch Frame
// =============================================================================

/// Minimal frame for context switching between threads
///
/// Only contains callee-saved registers that must be preserved
/// across function calls.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ContextSwitchFrame {
    /// x19-x28 (callee-saved)
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    /// Frame pointer (x29)
    pub fp: u64,
    /// Link register (x30)
    pub lr: u64,
    /// Stack pointer
    pub sp: u64,
}

impl ContextSwitchFrame {
    /// Create a new context switch frame
    pub const fn new() -> Self {
        Self {
            x19: 0,
            x20: 0,
            x21: 0,
            x22: 0,
            x23: 0,
            x24: 0,
            x25: 0,
            x26: 0,
            x27: 0,
            x28: 0,
            fp: 0,
            lr: 0,
            sp: 0,
        }
    }

    /// Create a frame for a new thread
    pub fn for_new_thread(entry: u64, stack: u64) -> Self {
        let mut frame = Self::new();
        frame.lr = entry;
        frame.sp = stack;
        frame
    }
}

impl Default for ContextSwitchFrame {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Size Assertions
// =============================================================================

// Ensure TrapFrame is correctly sized
const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * 8); // 36 u64 fields

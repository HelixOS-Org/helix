//! # AArch64 Exception Handlers
//!
//! This module provides exception classification and handling infrastructure
//! for AArch64 exceptions.

use super::context::TrapFrame;
use super::vectors::VectorOffset;

// =============================================================================
// Exception Class (ESR_EL1.EC)
// =============================================================================

/// Exception class from ESR_EL1
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionClass {
    /// Unknown reason
    Unknown = 0x00,
    /// Trapped WFI or WFE
    TrappedWfx = 0x01,
    /// Trapped MCR/MRC (AArch32)
    TrappedMcrMrc = 0x03,
    /// Trapped MCRR/MRRC (AArch32)
    TrappedMcrrMrrc = 0x04,
    /// Trapped MCR/MRC (AArch32) - coproc 14
    TrappedMcrMrcCp14 = 0x05,
    /// Trapped LDC/STC
    TrappedLdcStc = 0x06,
    /// SVE/SIMD/FP access
    SveSimdFp = 0x07,
    /// Trapped LD64B/ST64B*
    TrappedLd64bSt64b = 0x0a,
    /// Trapped MRRC (AArch32) - coproc 14
    TrappedMrrcCp14 = 0x0c,
    /// Branch Target Exception
    Bti = 0x0d,
    /// Illegal Execution State
    IllegalState = 0x0e,
    /// SVC (AArch32)
    SvcAarch32 = 0x11,
    /// HVC (AArch32)
    HvcAarch32 = 0x12,
    /// SMC (AArch32)
    SmcAarch32 = 0x13,
    /// SVC (AArch64)
    SvcAarch64 = 0x15,
    /// HVC (AArch64)
    HvcAarch64 = 0x16,
    /// SMC (AArch64)
    SmcAarch64 = 0x17,
    /// Trapped MSR/MRS/System instruction
    TrappedMsrMrsSys = 0x18,
    /// SVE access
    Sve = 0x19,
    /// Trapped ERET/ERETAA/ERETAB
    TrappedEret = 0x1a,
    /// Pointer Authentication failure
    Pac = 0x1c,
    /// Instruction Abort from lower EL
    InstrAbortLowerEl = 0x20,
    /// Instruction Abort from same EL
    InstrAbortSameEl = 0x21,
    /// PC alignment fault
    PcAlignment = 0x22,
    /// Data Abort from lower EL
    DataAbortLowerEl = 0x24,
    /// Data Abort from same EL
    DataAbortSameEl = 0x25,
    /// SP alignment fault
    SpAlignment = 0x26,
    /// Trapped FP (AArch32)
    TrappedFpAarch32 = 0x28,
    /// Trapped FP (AArch64)
    TrappedFpAarch64 = 0x2c,
    /// SError interrupt
    SError = 0x2f,
    /// Breakpoint from lower EL
    BreakpointLowerEl = 0x30,
    /// Breakpoint from same EL
    BreakpointSameEl = 0x31,
    /// Software Step from lower EL
    SoftwareStepLowerEl = 0x32,
    /// Software Step from same EL
    SoftwareStepSameEl = 0x33,
    /// Watchpoint from lower EL
    WatchpointLowerEl = 0x34,
    /// Watchpoint from same EL
    WatchpointSameEl = 0x35,
    /// BKPT (AArch32)
    BkptAarch32 = 0x38,
    /// Vector Catch (AArch32)
    VectorCatch = 0x3a,
    /// BRK (AArch64)
    BrkAarch64 = 0x3c,
}

impl ExceptionClass {
    /// Parse from ESR value
    pub fn from_esr(esr: u64) -> Self {
        let ec = ((esr >> 26) & 0x3F) as u8;
        Self::from_raw(ec)
    }

    /// Create from raw EC value
    pub fn from_raw(ec: u8) -> Self {
        match ec {
            0x00 => Self::Unknown,
            0x01 => Self::TrappedWfx,
            0x03 => Self::TrappedMcrMrc,
            0x04 => Self::TrappedMcrrMrrc,
            0x05 => Self::TrappedMcrMrcCp14,
            0x06 => Self::TrappedLdcStc,
            0x07 => Self::SveSimdFp,
            0x0a => Self::TrappedLd64bSt64b,
            0x0c => Self::TrappedMrrcCp14,
            0x0d => Self::Bti,
            0x0e => Self::IllegalState,
            0x11 => Self::SvcAarch32,
            0x12 => Self::HvcAarch32,
            0x13 => Self::SmcAarch32,
            0x15 => Self::SvcAarch64,
            0x16 => Self::HvcAarch64,
            0x17 => Self::SmcAarch64,
            0x18 => Self::TrappedMsrMrsSys,
            0x19 => Self::Sve,
            0x1a => Self::TrappedEret,
            0x1c => Self::Pac,
            0x20 => Self::InstrAbortLowerEl,
            0x21 => Self::InstrAbortSameEl,
            0x22 => Self::PcAlignment,
            0x24 => Self::DataAbortLowerEl,
            0x25 => Self::DataAbortSameEl,
            0x26 => Self::SpAlignment,
            0x28 => Self::TrappedFpAarch32,
            0x2c => Self::TrappedFpAarch64,
            0x2f => Self::SError,
            0x30 => Self::BreakpointLowerEl,
            0x31 => Self::BreakpointSameEl,
            0x32 => Self::SoftwareStepLowerEl,
            0x33 => Self::SoftwareStepSameEl,
            0x34 => Self::WatchpointLowerEl,
            0x35 => Self::WatchpointSameEl,
            0x38 => Self::BkptAarch32,
            0x3a => Self::VectorCatch,
            0x3c => Self::BrkAarch64,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a data abort
    pub fn is_data_abort(&self) -> bool {
        matches!(self, Self::DataAbortLowerEl | Self::DataAbortSameEl)
    }

    /// Check if this is an instruction abort
    pub fn is_instr_abort(&self) -> bool {
        matches!(self, Self::InstrAbortLowerEl | Self::InstrAbortSameEl)
    }

    /// Check if this is a page fault
    pub fn is_page_fault(&self) -> bool {
        self.is_data_abort() || self.is_instr_abort()
    }

    /// Check if this is a system call
    pub fn is_syscall(&self) -> bool {
        matches!(self, Self::SvcAarch64 | Self::SvcAarch32)
    }

    /// Check if this is a breakpoint
    pub fn is_breakpoint(&self) -> bool {
        matches!(self,
            Self::BreakpointLowerEl | Self::BreakpointSameEl |
            Self::BkptAarch32 | Self::BrkAarch64
        )
    }

    /// Check if exception came from lower EL
    pub fn is_from_lower_el(&self) -> bool {
        matches!(self,
            Self::InstrAbortLowerEl | Self::DataAbortLowerEl |
            Self::BreakpointLowerEl | Self::SoftwareStepLowerEl |
            Self::WatchpointLowerEl
        )
    }
}

// =============================================================================
// Data/Instruction Abort ISS
// =============================================================================

/// Data Fault Status Code (DFSC/IFSC)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FaultStatusCode {
    /// Address size fault, level 0
    AddressSizeL0 = 0b000000,
    /// Address size fault, level 1
    AddressSizeL1 = 0b000001,
    /// Address size fault, level 2
    AddressSizeL2 = 0b000010,
    /// Address size fault, level 3
    AddressSizeL3 = 0b000011,
    /// Translation fault, level 0
    TranslationL0 = 0b000100,
    /// Translation fault, level 1
    TranslationL1 = 0b000101,
    /// Translation fault, level 2
    TranslationL2 = 0b000110,
    /// Translation fault, level 3
    TranslationL3 = 0b000111,
    /// Access flag fault, level 1
    AccessFlagL1 = 0b001001,
    /// Access flag fault, level 2
    AccessFlagL2 = 0b001010,
    /// Access flag fault, level 3
    AccessFlagL3 = 0b001011,
    /// Permission fault, level 1
    PermissionL1 = 0b001101,
    /// Permission fault, level 2
    PermissionL2 = 0b001110,
    /// Permission fault, level 3
    PermissionL3 = 0b001111,
    /// Synchronous External abort
    SyncExternal = 0b010000,
    /// Synchronous Tag Check Fault
    SyncTagCheck = 0b010001,
    /// Synchronous External abort on table walk, level 0
    SyncExternalL0 = 0b010100,
    /// Synchronous External abort on table walk, level 1
    SyncExternalL1 = 0b010101,
    /// Synchronous External abort on table walk, level 2
    SyncExternalL2 = 0b010110,
    /// Synchronous External abort on table walk, level 3
    SyncExternalL3 = 0b010111,
    /// Synchronous parity/ECC error
    SyncParity = 0b011000,
    /// Synchronous parity/ECC error on table walk, level 0
    SyncParityL0 = 0b011100,
    /// Synchronous parity/ECC error on table walk, level 1
    SyncParityL1 = 0b011101,
    /// Synchronous parity/ECC error on table walk, level 2
    SyncParityL2 = 0b011110,
    /// Synchronous parity/ECC error on table walk, level 3
    SyncParityL3 = 0b011111,
    /// Alignment fault
    Alignment = 0b100001,
    /// TLB conflict abort
    TlbConflict = 0b110000,
    /// Unsupported atomic hardware update
    AtomicHwUpdate = 0b110001,
    /// Implementation defined lockdown
    Lockdown = 0b110100,
    /// Implementation defined exclusive
    Exclusive = 0b110101,
    /// Unknown/Other
    Unknown = 0b111111,
}

impl FaultStatusCode {
    /// Create from ISS DFSC/IFSC field
    pub fn from_iss(iss: u32) -> Self {
        let fsc = (iss & 0x3F) as u8;
        match fsc {
            0b000000 => Self::AddressSizeL0,
            0b000001 => Self::AddressSizeL1,
            0b000010 => Self::AddressSizeL2,
            0b000011 => Self::AddressSizeL3,
            0b000100 => Self::TranslationL0,
            0b000101 => Self::TranslationL1,
            0b000110 => Self::TranslationL2,
            0b000111 => Self::TranslationL3,
            0b001001 => Self::AccessFlagL1,
            0b001010 => Self::AccessFlagL2,
            0b001011 => Self::AccessFlagL3,
            0b001101 => Self::PermissionL1,
            0b001110 => Self::PermissionL2,
            0b001111 => Self::PermissionL3,
            0b010000 => Self::SyncExternal,
            0b010001 => Self::SyncTagCheck,
            0b010100 => Self::SyncExternalL0,
            0b010101 => Self::SyncExternalL1,
            0b010110 => Self::SyncExternalL2,
            0b010111 => Self::SyncExternalL3,
            0b011000 => Self::SyncParity,
            0b011100 => Self::SyncParityL0,
            0b011101 => Self::SyncParityL1,
            0b011110 => Self::SyncParityL2,
            0b011111 => Self::SyncParityL3,
            0b100001 => Self::Alignment,
            0b110000 => Self::TlbConflict,
            0b110001 => Self::AtomicHwUpdate,
            0b110100 => Self::Lockdown,
            0b110101 => Self::Exclusive,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a translation fault (page not mapped)
    pub fn is_translation_fault(&self) -> bool {
        matches!(self,
            Self::TranslationL0 | Self::TranslationL1 |
            Self::TranslationL2 | Self::TranslationL3
        )
    }

    /// Check if this is a permission fault
    pub fn is_permission_fault(&self) -> bool {
        matches!(self,
            Self::PermissionL1 | Self::PermissionL2 | Self::PermissionL3
        )
    }

    /// Check if this is an access flag fault
    pub fn is_access_fault(&self) -> bool {
        matches!(self,
            Self::AccessFlagL1 | Self::AccessFlagL2 | Self::AccessFlagL3
        )
    }

    /// Get the translation level (0-3) if applicable
    pub fn level(&self) -> Option<u8> {
        let raw = *self as u8;
        if (raw & 0b111100) == 0b000100 || // Translation
           (raw & 0b111100) == 0b001000 || // Access flag
           (raw & 0b111100) == 0b001100    // Permission
        {
            Some(raw & 0x3)
        } else {
            None
        }
    }
}

// =============================================================================
// Data Abort ISS
// =============================================================================

/// Data Abort ISS (Instruction Specific Syndrome)
#[derive(Debug, Clone, Copy)]
pub struct DataAbortIss {
    /// Instruction Syndrome Valid
    pub isv: bool,
    /// Syndrome Access Size (if ISV)
    pub sas: u8,
    /// Syndrome Sign Extend (if ISV)
    pub sse: bool,
    /// Syndrome Register Transfer (if ISV)
    pub srt: u8,
    /// 64-bit register transfer
    pub sf: bool,
    /// Acquire/Release semantics
    pub ar: bool,
    /// FAR not Valid
    pub fnv: bool,
    /// External Abort Type
    pub ea: bool,
    /// Cache Maintenance operation
    pub cm: bool,
    /// Stage 2 fault for Stage 1 walk
    pub s1ptw: bool,
    /// Write not Read
    pub wnr: bool,
    /// Fault Status Code
    pub dfsc: FaultStatusCode,
}

impl DataAbortIss {
    /// Parse from ISS field
    pub fn from_iss(iss: u32) -> Self {
        Self {
            isv: (iss & (1 << 24)) != 0,
            sas: ((iss >> 22) & 0x3) as u8,
            sse: (iss & (1 << 21)) != 0,
            srt: ((iss >> 16) & 0x1F) as u8,
            sf: (iss & (1 << 15)) != 0,
            ar: (iss & (1 << 14)) != 0,
            fnv: (iss & (1 << 10)) != 0,
            ea: (iss & (1 << 9)) != 0,
            cm: (iss & (1 << 8)) != 0,
            s1ptw: (iss & (1 << 7)) != 0,
            wnr: (iss & (1 << 6)) != 0,
            dfsc: FaultStatusCode::from_iss(iss),
        }
    }

    /// Check if this was a write fault
    pub fn is_write(&self) -> bool {
        self.wnr
    }

    /// Get access size in bytes (if ISV)
    pub fn access_size(&self) -> Option<usize> {
        if self.isv {
            Some(1 << self.sas)
        } else {
            None
        }
    }
}

// =============================================================================
// Exception Type (High-Level)
// =============================================================================

/// High-level exception type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionType {
    /// Synchronous exception
    Synchronous,
    /// IRQ interrupt
    Irq,
    /// FIQ interrupt
    Fiq,
    /// System Error
    SError,
}

impl ExceptionType {
    /// Determine from vector offset
    pub fn from_vector_offset(offset: VectorOffset) -> Self {
        if offset.is_sync() {
            Self::Synchronous
        } else if offset.is_irq() {
            Self::Irq
        } else if offset.is_fiq() {
            Self::Fiq
        } else {
            Self::SError
        }
    }
}

// =============================================================================
// Exception Info
// =============================================================================

/// Complete exception information
#[derive(Debug, Clone)]
pub struct ExceptionInfo {
    /// Exception type
    pub exception_type: ExceptionType,
    /// Exception class
    pub class: ExceptionClass,
    /// Vector offset
    pub vector: VectorOffset,
    /// ESR value
    pub esr: u64,
    /// FAR value
    pub far: u64,
    /// From lower EL
    pub from_lower_el: bool,
}

impl ExceptionInfo {
    /// Create from trap frame
    pub fn from_trap_frame(frame: &TrapFrame, vector: VectorOffset) -> Self {
        Self {
            exception_type: ExceptionType::from_vector_offset(vector),
            class: ExceptionClass::from_esr(frame.esr),
            vector,
            esr: frame.esr,
            far: frame.far,
            from_lower_el: vector.is_from_lower_el(),
        }
    }

    /// Get instruction syndrome
    pub fn instruction_syndrome(&self) -> u32 {
        (self.esr & 0x1FFFFFF) as u32
    }

    /// Check if instruction length is 32-bit
    pub fn is_32bit_instruction(&self) -> bool {
        (self.esr & (1 << 25)) != 0
    }

    /// Get data abort info (if applicable)
    pub fn data_abort_info(&self) -> Option<DataAbortIss> {
        if self.class.is_data_abort() {
            Some(DataAbortIss::from_iss(self.instruction_syndrome()))
        } else {
            None
        }
    }

    /// Get fault status code (if applicable)
    pub fn fault_status(&self) -> Option<FaultStatusCode> {
        if self.class.is_page_fault() {
            Some(FaultStatusCode::from_iss(self.instruction_syndrome()))
        } else {
            None
        }
    }
}

// =============================================================================
// Exception Handler Type
// =============================================================================

/// Exception handler function signature
pub type ExceptionHandler = fn(&mut TrapFrame, ExceptionInfo);

/// Default exception handlers table
pub struct ExceptionHandlers {
    /// Synchronous exception handler
    pub sync: Option<ExceptionHandler>,
    /// IRQ handler
    pub irq: Option<ExceptionHandler>,
    /// FIQ handler
    pub fiq: Option<ExceptionHandler>,
    /// SError handler
    pub serror: Option<ExceptionHandler>,
    /// Page fault handler
    pub page_fault: Option<fn(&mut TrapFrame, u64, bool, bool) -> bool>,
    /// Syscall handler
    pub syscall: Option<fn(&mut TrapFrame) -> isize>,
}

impl ExceptionHandlers {
    /// Create with no handlers
    pub const fn new() -> Self {
        Self {
            sync: None,
            irq: None,
            fiq: None,
            serror: None,
            page_fault: None,
            syscall: None,
        }
    }
}

impl Default for ExceptionHandlers {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Exception Handler Dispatch
// =============================================================================

/// Dispatch exception to appropriate handler
///
/// This is called from the assembly vector entry code with the
/// trap frame pointer.
#[no_mangle]
pub extern "C" fn dispatch_exception(frame: &mut TrapFrame, vector_offset: u16) {
    let vector = VectorOffset::from_offset(vector_offset)
        .unwrap_or(VectorOffset::CurrentElSpxSync);

    let info = ExceptionInfo::from_trap_frame(frame, vector);

    // Route to specific handler based on type
    match info.exception_type {
        ExceptionType::Synchronous => handle_sync_exception(frame, info),
        ExceptionType::Irq => handle_irq(frame, info),
        ExceptionType::Fiq => handle_fiq(frame, info),
        ExceptionType::SError => handle_serror(frame, info),
    }
}

/// Handle synchronous exception
fn handle_sync_exception(frame: &mut TrapFrame, info: ExceptionInfo) {
    match info.class {
        ExceptionClass::SvcAarch64 => {
            // System call - handled separately
            handle_syscall(frame);
        }
        ExceptionClass::DataAbortLowerEl | ExceptionClass::DataAbortSameEl |
        ExceptionClass::InstrAbortLowerEl | ExceptionClass::InstrAbortSameEl => {
            // Page fault
            handle_page_fault(frame, &info);
        }
        ExceptionClass::BrkAarch64 | ExceptionClass::BreakpointLowerEl |
        ExceptionClass::BreakpointSameEl => {
            // Breakpoint
            handle_breakpoint(frame, &info);
        }
        ExceptionClass::PcAlignment | ExceptionClass::SpAlignment => {
            // Alignment fault
            handle_alignment_fault(frame, &info);
        }
        ExceptionClass::IllegalState => {
            // Illegal state
            handle_illegal_state(frame, &info);
        }
        _ => {
            // Unhandled synchronous exception
            handle_unhandled(frame, &info);
        }
    }
}

/// Handle IRQ
fn handle_irq(_frame: &mut TrapFrame, _info: ExceptionInfo) {
    // Placeholder - actual implementation will call GIC and dispatch
}

/// Handle FIQ
fn handle_fiq(_frame: &mut TrapFrame, _info: ExceptionInfo) {
    // Placeholder - typically used for secure interrupts
}

/// Handle SError
fn handle_serror(_frame: &mut TrapFrame, _info: ExceptionInfo) {
    // SError is typically fatal
    panic!("SError exception");
}

/// Handle system call
fn handle_syscall(_frame: &mut TrapFrame) {
    // Placeholder - will dispatch to syscall handler
}

/// Handle page fault
fn handle_page_fault(frame: &mut TrapFrame, info: &ExceptionInfo) {
    let _fault_addr = info.far;
    let _is_write = info.data_abort_info().map(|d| d.is_write()).unwrap_or(false);
    let _is_user = info.from_lower_el;

    // Placeholder - actual implementation will call VM subsystem
    let _ = frame;
}

/// Handle breakpoint
fn handle_breakpoint(_frame: &mut TrapFrame, _info: &ExceptionInfo) {
    // Placeholder - will integrate with debugger
}

/// Handle alignment fault
fn handle_alignment_fault(_frame: &mut TrapFrame, info: &ExceptionInfo) {
    panic!("Alignment fault at {:#x}", info.far);
}

/// Handle illegal execution state
fn handle_illegal_state(frame: &mut TrapFrame, _info: &ExceptionInfo) {
    panic!("Illegal execution state at {:#x}", frame.elr);
}

/// Handle unhandled exception
fn handle_unhandled(frame: &mut TrapFrame, info: &ExceptionInfo) {
    panic!(
        "Unhandled exception: {:?} at {:#x}, FAR={:#x}",
        info.class, frame.elr, info.far
    );
}

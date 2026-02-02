//! # RISC-V Control and Status Registers (CSRs)
//!
//! This module provides comprehensive CSR access for RISC-V 64-bit.
//! CSRs control CPU state, interrupts, virtual memory, and more.
//!
//! ## CSR Address Encoding
//!
//! CSR addresses are 12-bit values encoded as:
//! - Bits [11:10]: Read/Write access (00=RW, 01=RW, 10=RW, 11=RO)
//! - Bits [9:8]: Lowest privilege level that can access
//! - Bits [7:0]: Register index
//!
//! ## Privilege Levels
//!
//! - 0b00: User (U-mode)
//! - 0b01: Supervisor (S-mode)
//! - 0b10: Reserved (Hypervisor in H extension)
//! - 0b11: Machine (M-mode)

use core::arch::asm;

// ============================================================================
// CSR Address Constants
// ============================================================================

// ----------------------------------------------------------------------------
// Supervisor-level CSRs (accessible from S-mode and above)
// ----------------------------------------------------------------------------

/// Supervisor status register
pub const SSTATUS: u16 = 0x100;
/// Supervisor interrupt enable
pub const SIE: u16 = 0x104;
/// Supervisor trap handler base address
pub const STVEC: u16 = 0x105;
/// Supervisor counter enable
pub const SCOUNTEREN: u16 = 0x106;
/// Supervisor environment configuration
pub const SENVCFG: u16 = 0x10A;
/// Supervisor scratch register
pub const SSCRATCH: u16 = 0x140;
/// Supervisor exception program counter
pub const SEPC: u16 = 0x141;
/// Supervisor trap cause
pub const SCAUSE: u16 = 0x142;
/// Supervisor trap value
pub const STVAL: u16 = 0x143;
/// Supervisor interrupt pending
pub const SIP: u16 = 0x144;
/// Supervisor address translation and protection
pub const SATP: u16 = 0x180;
/// Supervisor context ID (for ASID)
pub const SCONTEXT: u16 = 0x5A8;

// ----------------------------------------------------------------------------
// Machine-level CSRs (only accessible from M-mode)
// ----------------------------------------------------------------------------

/// Machine status register
pub const MSTATUS: u16 = 0x300;
/// Machine ISA
pub const MISA: u16 = 0x301;
/// Machine exception delegation
pub const MEDELEG: u16 = 0x302;
/// Machine interrupt delegation
pub const MIDELEG: u16 = 0x303;
/// Machine interrupt enable
pub const MIE: u16 = 0x304;
/// Machine trap handler base address
pub const MTVEC: u16 = 0x305;
/// Machine counter enable
pub const MCOUNTEREN: u16 = 0x306;
/// Machine scratch register
pub const MSCRATCH: u16 = 0x340;
/// Machine exception program counter
pub const MEPC: u16 = 0x341;
/// Machine trap cause
pub const MCAUSE: u16 = 0x342;
/// Machine trap value
pub const MTVAL: u16 = 0x343;
/// Machine interrupt pending
pub const MIP: u16 = 0x344;
/// Machine configuration
pub const MENVCFG: u16 = 0x30A;
/// Machine security configuration
pub const MSECCFG: u16 = 0x747;

// Physical Memory Protection
/// PMP configuration 0
pub const PMPCFG0: u16 = 0x3A0;
/// PMP configuration 2
pub const PMPCFG2: u16 = 0x3A2;
/// PMP address 0
pub const PMPADDR0: u16 = 0x3B0;
/// PMP address 1
pub const PMPADDR1: u16 = 0x3B1;
/// PMP address 2
pub const PMPADDR2: u16 = 0x3B2;
/// PMP address 3
pub const PMPADDR3: u16 = 0x3B3;

// Machine Information
/// Vendor ID
pub const MVENDORID: u16 = 0xF11;
/// Architecture ID
pub const MARCHID: u16 = 0xF12;
/// Implementation ID
pub const MIMPID: u16 = 0xF13;
/// Hardware thread ID
pub const MHARTID: u16 = 0xF14;
/// Machine configuration pointer
pub const MCONFIGPTR: u16 = 0xF15;

// ----------------------------------------------------------------------------
// Counter/Timer CSRs
// ----------------------------------------------------------------------------

/// Cycle counter (low)
pub const CYCLE: u16 = 0xC00;
/// Timer (low)
pub const TIME: u16 = 0xC01;
/// Instructions retired (low)
pub const INSTRET: u16 = 0xC02;
/// Cycle counter (high) - RV32 only
pub const CYCLEH: u16 = 0xC80;
/// Timer (high) - RV32 only
pub const TIMEH: u16 = 0xC81;
/// Instructions retired (high) - RV32 only
pub const INSTRETH: u16 = 0xC82;

// Machine counter/timers
/// Machine cycle counter
pub const MCYCLE: u16 = 0xB00;
/// Machine instructions retired
pub const MINSTRET: u16 = 0xB02;

// ============================================================================
// CSR Read/Write Macros
// ============================================================================

/// Read a CSR by address
#[macro_export]
macro_rules! read_csr {
    ($csr:expr) => {{
        let value: u64;
        unsafe {
            core::arch::asm!(
                concat!("csrr {}, ", stringify!($csr)),
                out(reg) value,
                options(nomem, nostack, preserves_flags)
            );
        }
        value
    }};
}

/// Write a CSR by address
#[macro_export]
macro_rules! write_csr {
    ($csr:expr, $value:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrw ", stringify!($csr), ", {}"),
                in(reg) $value as u64,
                options(nomem, nostack)
            );
        }
    }};
}

/// Set bits in a CSR
#[macro_export]
macro_rules! set_csr_bits {
    ($csr:expr, $bits:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrs ", stringify!($csr), ", {}"),
                in(reg) $bits as u64,
                options(nomem, nostack)
            );
        }
    }};
}

/// Clear bits in a CSR
#[macro_export]
macro_rules! clear_csr_bits {
    ($csr:expr, $bits:expr) => {{
        unsafe {
            core::arch::asm!(
                concat!("csrc ", stringify!($csr), ", {}"),
                in(reg) $bits as u64,
                options(nomem, nostack)
            );
        }
    }};
}

/// Read and write a CSR atomically
#[macro_export]
macro_rules! swap_csr {
    ($csr:expr, $value:expr) => {{
        let old: u64;
        unsafe {
            core::arch::asm!(
                concat!("csrrw {}, ", stringify!($csr), ", {}"),
                out(reg) old,
                in(reg) $value as u64,
                options(nomem, nostack)
            );
        }
        old
    }};
}

/// Read a CSR and set bits atomically
#[macro_export]
macro_rules! read_set_csr {
    ($csr:expr, $bits:expr) => {{
        let old: u64;
        unsafe {
            core::arch::asm!(
                concat!("csrrs {}, ", stringify!($csr), ", {}"),
                out(reg) old,
                in(reg) $bits as u64,
                options(nomem, nostack)
            );
        }
        old
    }};
}

/// Read a CSR and clear bits atomically
#[macro_export]
macro_rules! read_clear_csr {
    ($csr:expr, $bits:expr) => {{
        let old: u64;
        unsafe {
            core::arch::asm!(
                concat!("csrrc {}, ", stringify!($csr), ", {}"),
                out(reg) old,
                in(reg) $bits as u64,
                options(nomem, nostack)
            );
        }
        old
    }};
}

// ============================================================================
// Status Register Bits (sstatus/mstatus)
// ============================================================================

/// Status register bits
pub mod status {
    /// Supervisor Interrupt Enable
    pub const SIE: u64 = 1 << 1;
    /// Machine Interrupt Enable
    pub const MIE: u64 = 1 << 3;
    /// Supervisor Previous Interrupt Enable
    pub const SPIE: u64 = 1 << 5;
    /// U-mode Big Endian
    pub const UBE: u64 = 1 << 6;
    /// Machine Previous Interrupt Enable
    pub const MPIE: u64 = 1 << 7;
    /// Supervisor Previous Privilege (1 bit)
    pub const SPP: u64 = 1 << 8;
    /// Vector extension state (2 bits)
    pub const VS_MASK: u64 = 0b11 << 9;
    /// Machine Previous Privilege (2 bits)
    pub const MPP_MASK: u64 = 0b11 << 11;
    /// FP extension state (2 bits)
    pub const FS_MASK: u64 = 0b11 << 13;
    /// User extension state (2 bits)
    pub const XS_MASK: u64 = 0b11 << 15;
    /// Modify Privilege (allow modification of lower privilege)
    pub const MPRV: u64 = 1 << 17;
    /// Supervisor User Memory access
    pub const SUM: u64 = 1 << 18;
    /// Make eXecutable Readable
    pub const MXR: u64 = 1 << 19;
    /// Trap Virtual Memory
    pub const TVM: u64 = 1 << 20;
    /// Timeout Wait
    pub const TW: u64 = 1 << 21;
    /// Trap SRET
    pub const TSR: u64 = 1 << 22;
    /// User XLEN (2 bits, RV64)
    pub const UXL_MASK: u64 = 0b11 << 32;
    /// Supervisor XLEN (2 bits, RV64)
    pub const SXL_MASK: u64 = 0b11 << 34;
    /// S-mode Big Endian
    pub const SBE: u64 = 1 << 36;
    /// M-mode Big Endian
    pub const MBE: u64 = 1 << 37;
    /// State Dirty (summary of FS, VS, XS)
    pub const SD: u64 = 1 << 63;

    /// MPP field shift
    pub const MPP_SHIFT: u64 = 11;
    /// SPP field shift
    pub const SPP_SHIFT: u64 = 8;
    /// FS field shift
    pub const FS_SHIFT: u64 = 13;
    /// VS field shift
    pub const VS_SHIFT: u64 = 9;

    /// Privilege levels for MPP/SPP
    pub const PRIV_USER: u64 = 0;
    pub const PRIV_SUPERVISOR: u64 = 1;
    pub const PRIV_MACHINE: u64 = 3;

    /// Extension states (for FS, VS, XS)
    pub const EXT_OFF: u64 = 0;
    pub const EXT_INITIAL: u64 = 1;
    pub const EXT_CLEAN: u64 = 2;
    pub const EXT_DIRTY: u64 = 3;
}

// ============================================================================
// Interrupt Enable/Pending Bits (sie/sip/mie/mip)
// ============================================================================

/// Interrupt bits
pub mod interrupt {
    /// Supervisor Software Interrupt
    pub const SSIP: u64 = 1 << 1;
    /// Machine Software Interrupt
    pub const MSIP: u64 = 1 << 3;
    /// Supervisor Timer Interrupt
    pub const STIP: u64 = 1 << 5;
    /// Machine Timer Interrupt
    pub const MTIP: u64 = 1 << 7;
    /// Supervisor External Interrupt
    pub const SEIP: u64 = 1 << 9;
    /// Machine External Interrupt
    pub const MEIP: u64 = 1 << 11;

    /// All supervisor interrupts
    pub const S_ALL: u64 = SSIP | STIP | SEIP;
    /// All machine interrupts
    pub const M_ALL: u64 = MSIP | MTIP | MEIP;
}

// ============================================================================
// Trap Cause Codes (scause/mcause)
// ============================================================================

/// Exception cause codes (bit 63 = 0)
pub mod exception {
    /// Instruction address misaligned
    pub const INSTRUCTION_MISALIGNED: u64 = 0;
    /// Instruction access fault
    pub const INSTRUCTION_ACCESS_FAULT: u64 = 1;
    /// Illegal instruction
    pub const ILLEGAL_INSTRUCTION: u64 = 2;
    /// Breakpoint
    pub const BREAKPOINT: u64 = 3;
    /// Load address misaligned
    pub const LOAD_MISALIGNED: u64 = 4;
    /// Load access fault
    pub const LOAD_ACCESS_FAULT: u64 = 5;
    /// Store/AMO address misaligned
    pub const STORE_MISALIGNED: u64 = 6;
    /// Store/AMO access fault
    pub const STORE_ACCESS_FAULT: u64 = 7;
    /// Environment call from U-mode
    pub const ECALL_FROM_U: u64 = 8;
    /// Environment call from S-mode
    pub const ECALL_FROM_S: u64 = 9;
    /// Reserved
    pub const RESERVED_10: u64 = 10;
    /// Environment call from M-mode
    pub const ECALL_FROM_M: u64 = 11;
    /// Instruction page fault
    pub const INSTRUCTION_PAGE_FAULT: u64 = 12;
    /// Load page fault
    pub const LOAD_PAGE_FAULT: u64 = 13;
    /// Reserved
    pub const RESERVED_14: u64 = 14;
    /// Store/AMO page fault
    pub const STORE_PAGE_FAULT: u64 = 15;
}

/// Interrupt cause codes (bit 63 = 1)
pub mod irq_cause {
    /// Supervisor software interrupt
    pub const SUPERVISOR_SOFTWARE: u64 = 1;
    /// Machine software interrupt
    pub const MACHINE_SOFTWARE: u64 = 3;
    /// Supervisor timer interrupt
    pub const SUPERVISOR_TIMER: u64 = 5;
    /// Machine timer interrupt
    pub const MACHINE_TIMER: u64 = 7;
    /// Supervisor external interrupt
    pub const SUPERVISOR_EXTERNAL: u64 = 9;
    /// Machine external interrupt
    pub const MACHINE_EXTERNAL: u64 = 11;
}

/// Interrupt bit in cause register
pub const CAUSE_INTERRUPT_BIT: u64 = 1 << 63;

// ============================================================================
// Trap Vector Modes (stvec/mtvec)
// ============================================================================

/// Trap vector modes
pub mod tvec {
    /// Direct mode: all traps go to BASE
    pub const MODE_DIRECT: u64 = 0;
    /// Vectored mode: async interrupts go to BASE + 4*cause
    pub const MODE_VECTORED: u64 = 1;
    /// Mode mask
    pub const MODE_MASK: u64 = 0b11;
    /// Base address mask (4-byte aligned)
    pub const BASE_MASK: u64 = !0b11;
}

// ============================================================================
// SATP Register (Supervisor Address Translation and Protection)
// ============================================================================

/// SATP modes and fields
pub mod satp {
    /// Bare mode (no translation)
    pub const MODE_BARE: u64 = 0;
    /// Sv39: 39-bit virtual address, 3-level page table
    pub const MODE_SV39: u64 = 8;
    /// Sv48: 48-bit virtual address, 4-level page table
    pub const MODE_SV48: u64 = 9;
    /// Sv57: 57-bit virtual address, 5-level page table (optional)
    pub const MODE_SV57: u64 = 10;
    /// Sv64: Reserved
    pub const MODE_SV64: u64 = 11;

    /// Mode field shift
    pub const MODE_SHIFT: u64 = 60;
    /// Mode field mask
    pub const MODE_MASK: u64 = 0xF << MODE_SHIFT;

    /// ASID field shift
    pub const ASID_SHIFT: u64 = 44;
    /// ASID field mask (16 bits for Sv39/48)
    pub const ASID_MASK: u64 = 0xFFFF << ASID_SHIFT;

    /// PPN field mask (44 bits)
    pub const PPN_MASK: u64 = (1 << 44) - 1;

    /// Build SATP value
    #[inline]
    pub const fn make(mode: u64, asid: u64, ppn: u64) -> u64 {
        (mode << MODE_SHIFT) | ((asid & 0xFFFF) << ASID_SHIFT) | (ppn & PPN_MASK)
    }

    /// Extract mode from SATP
    #[inline]
    pub const fn get_mode(satp: u64) -> u64 {
        (satp >> MODE_SHIFT) & 0xF
    }

    /// Extract ASID from SATP
    #[inline]
    pub const fn get_asid(satp: u64) -> u64 {
        (satp >> ASID_SHIFT) & 0xFFFF
    }

    /// Extract PPN from SATP
    #[inline]
    pub const fn get_ppn(satp: u64) -> u64 {
        satp & PPN_MASK
    }
}

// ============================================================================
// Supervisor CSR Access Functions
// ============================================================================

/// Read sstatus
#[inline]
pub fn read_sstatus() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, sstatus", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write sstatus
#[inline]
pub fn write_sstatus(value: u64) {
    unsafe {
        asm!("csrw sstatus, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Set bits in sstatus
#[inline]
pub fn set_sstatus(bits: u64) {
    unsafe {
        asm!("csrs sstatus, {}", in(reg) bits, options(nomem, nostack));
    }
}

/// Clear bits in sstatus
#[inline]
pub fn clear_sstatus(bits: u64) {
    unsafe {
        asm!("csrc sstatus, {}", in(reg) bits, options(nomem, nostack));
    }
}

/// Read sie (supervisor interrupt enable)
#[inline]
pub fn read_sie() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, sie", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write sie
#[inline]
pub fn write_sie(value: u64) {
    unsafe {
        asm!("csrw sie, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Enable supervisor interrupts
#[inline]
pub fn enable_sie(bits: u64) {
    unsafe {
        asm!("csrs sie, {}", in(reg) bits, options(nomem, nostack));
    }
}

/// Disable supervisor interrupts
#[inline]
pub fn disable_sie(bits: u64) {
    unsafe {
        asm!("csrc sie, {}", in(reg) bits, options(nomem, nostack));
    }
}

/// Read sip (supervisor interrupt pending)
#[inline]
pub fn read_sip() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, sip", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Clear supervisor interrupt pending
#[inline]
pub fn clear_sip(bits: u64) {
    unsafe {
        asm!("csrc sip, {}", in(reg) bits, options(nomem, nostack));
    }
}

/// Read stvec
#[inline]
pub fn read_stvec() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, stvec", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write stvec
#[inline]
pub fn write_stvec(value: u64) {
    unsafe {
        asm!("csrw stvec, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read sscratch
#[inline]
pub fn read_sscratch() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, sscratch", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write sscratch
#[inline]
pub fn write_sscratch(value: u64) {
    unsafe {
        asm!("csrw sscratch, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Swap sscratch (exchange with register)
#[inline]
pub fn swap_sscratch(value: u64) -> u64 {
    let old: u64;
    unsafe {
        asm!("csrrw {}, sscratch, {}", out(reg) old, in(reg) value, options(nomem, nostack));
    }
    old
}

/// Read sepc
#[inline]
pub fn read_sepc() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, sepc", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write sepc
#[inline]
pub fn write_sepc(value: u64) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read scause
#[inline]
pub fn read_scause() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, scause", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read stval
#[inline]
pub fn read_stval() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, stval", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read satp
#[inline]
pub fn read_satp() -> u64 {
    let value: u64;
    unsafe {
        asm!("csrr {}, satp", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write satp
#[inline]
pub fn write_satp(value: u64) {
    unsafe {
        asm!("csrw satp, {}", in(reg) value, options(nomem, nostack));
    }
}

// ============================================================================
// Counter CSR Access
// ============================================================================

/// Read cycle counter
#[inline]
pub fn read_cycle() -> u64 {
    let value: u64;
    unsafe {
        asm!("rdcycle {}", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read time counter
#[inline]
pub fn read_time() -> u64 {
    let value: u64;
    unsafe {
        asm!("rdtime {}", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read instructions retired counter
#[inline]
pub fn read_instret() -> u64 {
    let value: u64;
    unsafe {
        asm!("rdinstret {}", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

// ============================================================================
// Global Interrupt Control
// ============================================================================

/// Enable global interrupts in S-mode
#[inline]
pub fn enable_interrupts() {
    set_sstatus(status::SIE);
}

/// Disable global interrupts in S-mode
#[inline]
pub fn disable_interrupts() {
    clear_sstatus(status::SIE);
}

/// Check if interrupts are enabled
#[inline]
pub fn interrupts_enabled() -> bool {
    read_sstatus() & status::SIE != 0
}

/// Disable interrupts and return previous state
#[inline]
pub fn disable_interrupts_save() -> bool {
    let was_enabled = interrupts_enabled();
    disable_interrupts();
    was_enabled
}

/// Restore interrupt state
#[inline]
pub fn restore_interrupts(was_enabled: bool) {
    if was_enabled {
        enable_interrupts();
    }
}

// ============================================================================
// Cause Analysis
// ============================================================================

/// Parsed trap cause
#[derive(Debug, Clone, Copy)]
pub struct TrapCause {
    /// Is this an interrupt (true) or exception (false)?
    pub is_interrupt: bool,
    /// The cause code
    pub code: u64,
}

impl TrapCause {
    /// Parse from raw scause value
    pub const fn from_scause(scause: u64) -> Self {
        Self {
            is_interrupt: (scause & CAUSE_INTERRUPT_BIT) != 0,
            code: scause & !CAUSE_INTERRUPT_BIT,
        }
    }

    /// Get cause name
    pub fn name(&self) -> &'static str {
        if self.is_interrupt {
            match self.code {
                irq_cause::SUPERVISOR_SOFTWARE => "Supervisor Software Interrupt",
                irq_cause::MACHINE_SOFTWARE => "Machine Software Interrupt",
                irq_cause::SUPERVISOR_TIMER => "Supervisor Timer Interrupt",
                irq_cause::MACHINE_TIMER => "Machine Timer Interrupt",
                irq_cause::SUPERVISOR_EXTERNAL => "Supervisor External Interrupt",
                irq_cause::MACHINE_EXTERNAL => "Machine External Interrupt",
                _ => "Unknown Interrupt",
            }
        } else {
            match self.code {
                exception::INSTRUCTION_MISALIGNED => "Instruction Address Misaligned",
                exception::INSTRUCTION_ACCESS_FAULT => "Instruction Access Fault",
                exception::ILLEGAL_INSTRUCTION => "Illegal Instruction",
                exception::BREAKPOINT => "Breakpoint",
                exception::LOAD_MISALIGNED => "Load Address Misaligned",
                exception::LOAD_ACCESS_FAULT => "Load Access Fault",
                exception::STORE_MISALIGNED => "Store/AMO Address Misaligned",
                exception::STORE_ACCESS_FAULT => "Store/AMO Access Fault",
                exception::ECALL_FROM_U => "Environment Call from U-mode",
                exception::ECALL_FROM_S => "Environment Call from S-mode",
                exception::ECALL_FROM_M => "Environment Call from M-mode",
                exception::INSTRUCTION_PAGE_FAULT => "Instruction Page Fault",
                exception::LOAD_PAGE_FAULT => "Load Page Fault",
                exception::STORE_PAGE_FAULT => "Store/AMO Page Fault",
                _ => "Unknown Exception",
            }
        }
    }
}

// ============================================================================
// MISA Extension Detection
// ============================================================================

/// ISA extension bits in MISA
pub mod misa {
    pub const A: u64 = 1 << 0;  // Atomic
    pub const B: u64 = 1 << 1;  // Bit manipulation
    pub const C: u64 = 1 << 2;  // Compressed
    pub const D: u64 = 1 << 3;  // Double-precision FP
    pub const E: u64 = 1 << 4;  // RV32E base
    pub const F: u64 = 1 << 5;  // Single-precision FP
    pub const G: u64 = 1 << 6;  // Reserved
    pub const H: u64 = 1 << 7;  // Hypervisor
    pub const I: u64 = 1 << 8;  // Integer base
    pub const J: u64 = 1 << 9;  // Dynamically translated
    pub const K: u64 = 1 << 10; // Reserved
    pub const L: u64 = 1 << 11; // Reserved
    pub const M: u64 = 1 << 12; // Integer multiply/divide
    pub const N: u64 = 1 << 13; // User-level interrupts
    pub const O: u64 = 1 << 14; // Reserved
    pub const P: u64 = 1 << 15; // Packed SIMD
    pub const Q: u64 = 1 << 16; // Quad-precision FP
    pub const R: u64 = 1 << 17; // Reserved
    pub const S: u64 = 1 << 18; // Supervisor mode
    pub const T: u64 = 1 << 19; // Reserved
    pub const U: u64 = 1 << 20; // User mode
    pub const V: u64 = 1 << 21; // Vector
    pub const W: u64 = 1 << 22; // Reserved
    pub const X: u64 = 1 << 23; // Non-standard extensions
    pub const Y: u64 = 1 << 24; // Reserved
    pub const Z: u64 = 1 << 25; // Reserved

    /// XLEN in M-mode (bits 63:62)
    pub const MXL_SHIFT: u64 = 62;
    pub const MXL_MASK: u64 = 0b11 << MXL_SHIFT;
    pub const MXL_32: u64 = 1;
    pub const MXL_64: u64 = 2;
    pub const MXL_128: u64 = 3;
}

//! # AArch64 System Registers
//!
//! This module provides type-safe access to AArch64 system registers.
//!
//! ## Key System Registers
//!
//! - SCTLR_EL1: System Control Register
//! - TCR_EL1: Translation Control Register
//! - MAIR_EL1: Memory Attribute Indirection Register
//! - TTBR0_EL1/TTBR1_EL1: Translation Table Base Registers
//! - VBAR_EL1: Vector Base Address Register
//! - ESR_EL1: Exception Syndrome Register
//! - FAR_EL1: Fault Address Register
//! - ELR_EL1: Exception Link Register
//! - SPSR_EL1: Saved Processor State Register

use core::arch::asm;

// =============================================================================
// System Register Trait
// =============================================================================

/// Trait for system registers
pub trait SystemRegs {
    /// The register's value type
    type Value;

    /// Read the register
    fn read() -> Self::Value;

    /// Write to the register
    ///
    /// # Safety
    /// Writing to system registers can have immediate effects on system behavior
    unsafe fn write(value: Self::Value);
}

// =============================================================================
// SCTLR_EL1 - System Control Register
// =============================================================================

bitflags::bitflags! {
    /// SCTLR_EL1 flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Sctlr: u64 {
        /// MMU enable
        const M = 1 << 0;
        /// Alignment check enable
        const A = 1 << 1;
        /// Data cache enable
        const C = 1 << 2;
        /// Stack alignment check for EL0
        const SA = 1 << 3;
        /// Stack alignment check for EL1
        const SA0 = 1 << 4;
        /// CP15 barrier enable
        const CP15BEN = 1 << 5;
        /// Non-aligned access (0 = trap unaligned)
        const N_AA = 1 << 6;
        /// IT disable (reserved for EL1)
        const ITD = 1 << 7;
        /// SETEND disable
        const SED = 1 << 8;
        /// User mask access
        const UMA = 1 << 9;
        /// Exception endianness EL1
        const EE = 1 << 25;
        /// Exception endianness EL0
        const E0E = 1 << 24;
        /// Instruction cache enable
        const I = 1 << 12;
        /// Write permission implies XN
        const WXN = 1 << 19;
        /// Traps DC ZVA at EL0
        const DZE = 1 << 14;
        /// User cache maintenance
        const UCT = 1 << 15;
        /// No write-back
        const N_TWI = 1 << 16;
        /// No write-through
        const N_TWE = 1 << 18;
        /// LSMAOE
        const LSMAOE = 1 << 29;
        /// Exception entry is context synchronizing
        const EIS = 1 << 22;
        /// AArch32 at EL0
        const SPAN = 1 << 23;
        /// Force EL0 access to certain registers
        const IESB = 1 << 21;
        /// Trap SVE
        const TSCXT = 1 << 20;
    }
}

impl Sctlr {
    /// Default value for kernel mode (MMU off)
    pub const DEFAULT: Self = Self::empty();

    /// Value with MMU and caches enabled
    pub const MMU_ENABLED: Self = Self::M.union(Self::C).union(Self::I);
}

/// Read SCTLR_EL1
#[inline]
pub fn read_sctlr_el1() -> Sctlr {
    let value: u64;
    unsafe {
        asm!("mrs {}, SCTLR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Sctlr::from_bits_truncate(value)
}

/// Write SCTLR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the system is in a valid state for modifying this system register.
pub unsafe fn write_sctlr_el1(value: Sctlr) {
    asm!("msr SCTLR_EL1, {}", in(reg) value.bits(), options(nomem, nostack, preserves_flags));
}

// =============================================================================
// TCR_EL1 - Translation Control Register
// =============================================================================

bitflags::bitflags! {
    /// TCR_EL1 flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Tcr: u64 {
        /// TTBR0 region size (T0SZ field is bits 5:0)
        /// TTBR1 region size (T1SZ field is bits 21:16)

        /// Inner cacheability for TTBR0 walks
        const IRGN0_WBWA = 0b01 << 8;
        const IRGN0_WT = 0b10 << 8;
        const IRGN0_WB = 0b11 << 8;

        /// Outer cacheability for TTBR0 walks
        const ORGN0_WBWA = 0b01 << 10;
        const ORGN0_WT = 0b10 << 10;
        const ORGN0_WB = 0b11 << 10;

        /// Shareability for TTBR0 walks
        const SH0_OUTER = 0b10 << 12;
        const SH0_INNER = 0b11 << 12;

        /// Granule size for TTBR0 (TG0)
        const TG0_4KB = 0b00 << 14;
        const TG0_64KB = 0b01 << 14;
        const TG0_16KB = 0b10 << 14;

        /// Inner cacheability for TTBR1 walks
        const IRGN1_WBWA = 0b01 << 24;
        const IRGN1_WT = 0b10 << 24;
        const IRGN1_WB = 0b11 << 24;

        /// Outer cacheability for TTBR1 walks
        const ORGN1_WBWA = 0b01 << 26;
        const ORGN1_WT = 0b10 << 26;
        const ORGN1_WB = 0b11 << 26;

        /// Shareability for TTBR1 walks
        const SH1_OUTER = 0b10 << 28;
        const SH1_INNER = 0b11 << 28;

        /// Granule size for TTBR1 (TG1)
        const TG1_16KB = 0b01 << 30;
        const TG1_4KB = 0b10 << 30;
        const TG1_64KB = 0b11 << 30;

        /// Intermediate Physical Address Size (IPS)
        const IPS_32BIT = 0b000 << 32;
        const IPS_36BIT = 0b001 << 32;
        const IPS_40BIT = 0b010 << 32;
        const IPS_42BIT = 0b011 << 32;
        const IPS_44BIT = 0b100 << 32;
        const IPS_48BIT = 0b101 << 32;
        const IPS_52BIT = 0b110 << 32;

        /// Top byte ignore for TTBR0
        const TBI0 = 1 << 37;
        /// Top byte ignore for TTBR1
        const TBI1 = 1 << 38;

        /// ASID size (0 = 8-bit, 1 = 16-bit)
        const AS = 1 << 36;

        /// ASID selection (0 = TTBR0, 1 = TTBR1)
        const A1 = 1 << 22;

        /// Extended page-based invalidation
        const EPD0 = 1 << 7;
        const EPD1 = 1 << 23;
    }
}

impl Tcr {
    /// Create TCR value with given T0SZ and T1SZ
    pub const fn with_sizes(t0sz: u8, t1sz: u8) -> Self {
        Self::from_bits_truncate((t0sz as u64) | ((t1sz as u64) << 16))
    }

    /// Get T0SZ value
    pub fn t0sz(&self) -> u8 {
        (self.bits() & 0x3F) as u8
    }

    /// Get T1SZ value
    pub fn t1sz(&self) -> u8 {
        ((self.bits() >> 16) & 0x3F) as u8
    }

    /// Standard 48-bit virtual address with 4KB pages
    pub const STANDARD_4KB: Self = Self::from_bits_truncate(
        16 |                    // T0SZ = 16 (48-bit)
        (16 << 16) |           // T1SZ = 16 (48-bit)
        (0b01 << 8) |          // IRGN0 = Write-back, write-allocate
        (0b01 << 10) |         // ORGN0 = Write-back, write-allocate
        (0b11 << 12) |         // SH0 = Inner shareable
        (0b00 << 14) |         // TG0 = 4KB
        (0b01 << 24) |         // IRGN1 = Write-back, write-allocate
        (0b01 << 26) |         // ORGN1 = Write-back, write-allocate
        (0b11 << 28) |         // SH1 = Inner shareable
        (0b10 << 30) |         // TG1 = 4KB
        (0b101 << 32), // IPS = 48-bit
    );
}

/// Read TCR_EL1
#[inline]
pub fn read_tcr_el1() -> Tcr {
    let value: u64;
    unsafe {
        asm!("mrs {}, TCR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Tcr::from_bits_truncate(value)
}

/// Write TCR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure this MSR is valid for the current CPU and the value is appropriate.
pub unsafe fn write_tcr_el1(value: Tcr) {
    asm!("msr TCR_EL1, {}", in(reg) value.bits(), options(nomem, nostack, preserves_flags));
}

// =============================================================================
// MAIR_EL1 - Memory Attribute Indirection Register
// =============================================================================

/// MAIR_EL1 memory attributes
#[derive(Debug, Clone, Copy)]
pub struct Mair(pub u64);

impl Mair {
    /// Device-nGnRnE memory
    pub const DEVICE_NGNRNE: u8 = 0b0000_0000;
    /// Device-nGnRE memory
    pub const DEVICE_NGNRE: u8 = 0b0000_0100;
    /// Device-GRE memory
    pub const DEVICE_GRE: u8 = 0b0000_1100;
    /// Normal non-cacheable
    pub const NORMAL_NC: u8 = 0b0100_0100;
    /// Normal write-through
    pub const NORMAL_WT: u8 = 0b1011_1011;
    /// Normal write-back
    pub const NORMAL_WB: u8 = 0b1111_1111;
    /// Normal write-back non-transient
    pub const NORMAL_WBNT: u8 = 0b1110_1110;

    /// Create a new MAIR value
    pub const fn new() -> Self {
        Self(0)
    }

    /// Set attribute for given index (0-7)
    pub const fn with_attr(self, index: usize, attr: u8) -> Self {
        debug_assert!(index < 8);
        let shift = index * 8;
        let mask = 0xFF << shift;
        Self((self.0 & !mask) | ((attr as u64) << shift))
    }

    /// Get attribute at index
    pub const fn attr(&self, index: usize) -> u8 {
        debug_assert!(index < 8);
        ((self.0 >> (index * 8)) & 0xFF) as u8
    }

    /// Standard MAIR configuration
    /// Index 0: Device-nGnRnE
    /// Index 1: Normal non-cacheable
    /// Index 2: Normal write-back
    pub const STANDARD: Self = Self(
        (Self::DEVICE_NGNRNE as u64)
            | ((Self::NORMAL_NC as u64) << 8)
            | ((Self::NORMAL_WB as u64) << 16),
    );
}

impl Default for Mair {
    fn default() -> Self {
        Self::STANDARD
    }
}

/// Read MAIR_EL1
#[inline]
pub fn read_mair_el1() -> Mair {
    let value: u64;
    unsafe {
        asm!("mrs {}, MAIR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Mair(value)
}

/// Write MAIR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the system is in a valid state for modifying this system register.
pub unsafe fn write_mair_el1(value: Mair) {
    asm!("msr MAIR_EL1, {}", in(reg) value.0, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// TTBR0_EL1 / TTBR1_EL1 - Translation Table Base Registers
// =============================================================================

/// TTBR (Translation Table Base Register) value
#[derive(Debug, Clone, Copy)]
pub struct Ttbr {
    /// The raw register value
    value: u64,
}

impl Ttbr {
    /// Create a new TTBR value
    pub const fn new(base: u64, asid: u16) -> Self {
        // Bits [47:1] = BADDR (base address)
        // Bits [63:48] = ASID
        Self {
            value: (base & 0x0000_FFFF_FFFF_FFFE) | ((asid as u64) << 48),
        }
    }

    /// Create TTBR with CnP (Common not Private)
    pub const fn with_cnp(base: u64, asid: u16) -> Self {
        Self {
            value: (base & 0x0000_FFFF_FFFF_FFFE) | ((asid as u64) << 48) | 1,
        }
    }

    /// Get the base address
    pub const fn base(&self) -> u64 {
        self.value & 0x0000_FFFF_FFFF_FFFE
    }

    /// Get the ASID
    pub const fn asid(&self) -> u16 {
        (self.value >> 48) as u16
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.value
    }
}

/// Read TTBR0_EL1
#[inline]
pub fn read_ttbr0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, TTBR0_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TTBR0_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the page table pointer is valid and properly aligned.
pub unsafe fn write_ttbr0_el1(value: u64) {
    asm!("msr TTBR0_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

/// Read TTBR1_EL1
#[inline]
pub fn read_ttbr1_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, TTBR1_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TTBR1_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the page table pointer is valid and properly aligned.
pub unsafe fn write_ttbr1_el1(value: u64) {
    asm!("msr TTBR1_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// VBAR_EL1 - Vector Base Address Register
// =============================================================================

/// Read VBAR_EL1
#[inline]
pub fn read_vbar_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, VBAR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write VBAR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the vector table is properly set up and the address is valid.
pub unsafe fn write_vbar_el1(value: u64) {
    asm!("msr VBAR_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// ESR_EL1 - Exception Syndrome Register
// =============================================================================

/// Read ESR_EL1
#[inline]
pub fn read_esr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ESR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Extract exception class from ESR
#[inline]
pub fn esr_exception_class(esr: u64) -> u8 {
    ((esr >> 26) & 0x3F) as u8
}

/// Extract instruction length from ESR
#[inline]
pub fn esr_instruction_length(esr: u64) -> bool {
    (esr & (1 << 25)) != 0
}

/// Extract ISS from ESR
#[inline]
pub fn esr_iss(esr: u64) -> u32 {
    (esr & 0x01FF_FFFF) as u32
}

// =============================================================================
// FAR_EL1 - Fault Address Register
// =============================================================================

/// Read FAR_EL1
#[inline]
pub fn read_far_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, FAR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

// =============================================================================
// ELR_EL1 - Exception Link Register
// =============================================================================

/// Read ELR_EL1
#[inline]
pub fn read_elr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ELR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write ELR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the value is appropriate for the current exception level.
pub unsafe fn write_elr_el1(value: u64) {
    asm!("msr ELR_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// SPSR_EL1 - Saved Processor State Register
// =============================================================================

/// Read SPSR_EL1
#[inline]
pub fn read_spsr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, SPSR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write SPSR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the value is appropriate for the current exception level.
pub unsafe fn write_spsr_el1(value: u64) {
    asm!("msr SPSR_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// SP_EL0 - Stack Pointer for EL0
// =============================================================================

/// Read SP_EL0
#[inline]
pub fn read_sp_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, SP_EL0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write SP_EL0
#[inline]
///
/// # Safety
///
/// The caller must ensure the value is appropriate for the current exception level.
pub unsafe fn write_sp_el0(value: u64) {
    asm!("msr SP_EL0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// TPIDR_EL1 - Thread Pointer (Kernel)
// =============================================================================

/// Read TPIDR_EL1 (kernel per-CPU pointer)
#[inline]
pub fn read_tpidr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, TPIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TPIDR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure the thread pointer value is valid.
pub unsafe fn write_tpidr_el1(value: u64) {
    asm!("msr TPIDR_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// TPIDR_EL0 - Thread Pointer (User)
// =============================================================================

/// Read TPIDR_EL0 (user thread-local storage)
#[inline]
pub fn read_tpidr_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, TPIDR_EL0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TPIDR_EL0
#[inline]
///
/// # Safety
///
/// The caller must ensure the thread pointer value is valid.
pub unsafe fn write_tpidr_el0(value: u64) {
    asm!("msr TPIDR_EL0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

// =============================================================================
// MPIDR_EL1 - Multiprocessor Affinity Register
// =============================================================================

/// Read MPIDR_EL1
#[inline]
pub fn read_mpidr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, MPIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

// =============================================================================
// MIDR_EL1 - Main ID Register
// =============================================================================

/// Read MIDR_EL1
#[inline]
pub fn read_midr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, MIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

// =============================================================================
// CPACR_EL1 - Coprocessor Access Control Register
// =============================================================================

bitflags::bitflags! {
    /// CPACR_EL1 flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Cpacr: u64 {
        /// FPEN - FP/SIMD access from EL0/EL1
        /// 00 = Trap
        /// 01 = Trap from EL0
        /// 11 = No trap
        const FPEN_TRAP_NONE = 0b11 << 20;
        const FPEN_TRAP_EL0 = 0b01 << 20;
        const FPEN_TRAP_ALL = 0b00 << 20;

        /// ZEN - SVE access
        const ZEN_TRAP_NONE = 0b11 << 16;
        const ZEN_TRAP_EL0 = 0b01 << 16;
        const ZEN_TRAP_ALL = 0b00 << 16;
    }
}

/// Read CPACR_EL1
#[inline]
pub fn read_cpacr_el1() -> Cpacr {
    let value: u64;
    unsafe {
        asm!("mrs {}, CPACR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Cpacr::from_bits_truncate(value)
}

/// Write CPACR_EL1
#[inline]
///
/// # Safety
///
/// The caller must ensure this MSR is valid for the current CPU and the value is appropriate.
pub unsafe fn write_cpacr_el1(value: Cpacr) {
    asm!("msr CPACR_EL1, {}", in(reg) value.bits(), options(nomem, nostack, preserves_flags));
}

/// Enable FP/SIMD access
#[inline]
///
/// # Safety
///
/// The caller must ensure the CPU supports these features.
pub unsafe fn enable_fp_simd() {
    let cpacr = read_cpacr_el1() | Cpacr::FPEN_TRAP_NONE;
    write_cpacr_el1(cpacr);
    // Synchronization barrier
    core::arch::asm!("isb", options(nostack, preserves_flags));
}

//! # AArch64 Exception Level Management
//!
//! This module handles Exception Level (EL) detection and transitions.
//! AArch64 has four exception levels:
//!
//! - EL0: User/Application level
//! - EL1: Kernel/OS level
//! - EL2: Hypervisor level
//! - EL3: Secure Monitor level

use core::arch::asm;

// =============================================================================
// Exception Level
// =============================================================================

/// Exception Level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ExceptionLevel {
    /// EL0 - User/Application
    EL0 = 0,
    /// EL1 - Kernel/OS
    EL1 = 1,
    /// EL2 - Hypervisor
    EL2 = 2,
    /// EL3 - Secure Monitor
    EL3 = 3,
}

impl ExceptionLevel {
    /// Create from raw value
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::EL0),
            1 => Some(Self::EL1),
            2 => Some(Self::EL2),
            3 => Some(Self::EL3),
            _ => None,
        }
    }

    /// Check if this is a privileged level (EL1+)
    pub const fn is_privileged(&self) -> bool {
        (*self as u8) >= 1
    }

    /// Check if this is a secure level
    pub const fn is_secure(&self) -> bool {
        (*self as u8) == 3
    }

    /// Get the next higher EL
    pub const fn higher(&self) -> Option<Self> {
        match self {
            Self::EL0 => Some(Self::EL1),
            Self::EL1 => Some(Self::EL2),
            Self::EL2 => Some(Self::EL3),
            Self::EL3 => None,
        }
    }

    /// Get the next lower EL
    pub const fn lower(&self) -> Option<Self> {
        match self {
            Self::EL0 => None,
            Self::EL1 => Some(Self::EL0),
            Self::EL2 => Some(Self::EL1),
            Self::EL3 => Some(Self::EL2),
        }
    }
}

impl core::fmt::Display for ExceptionLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EL0 => write!(f, "EL0 (User)"),
            Self::EL1 => write!(f, "EL1 (Kernel)"),
            Self::EL2 => write!(f, "EL2 (Hypervisor)"),
            Self::EL3 => write!(f, "EL3 (Secure Monitor)"),
        }
    }
}

// =============================================================================
// Exception Level Detection
// =============================================================================

/// Read current exception level
#[inline]
pub fn current_el() -> ExceptionLevel {
    let value: u64;
    unsafe {
        asm!("mrs {}, CurrentEL", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    // CurrentEL[3:2] contains the EL
    let el = ((value >> 2) & 0x3) as u8;
    ExceptionLevel::from_raw(el).unwrap_or(ExceptionLevel::EL1)
}

/// Check if currently at EL0
#[inline]
pub fn in_el0() -> bool {
    current_el() == ExceptionLevel::EL0
}

/// Check if currently at EL1
#[inline]
pub fn in_el1() -> bool {
    current_el() == ExceptionLevel::EL1
}

/// Check if currently at EL2
#[inline]
pub fn in_el2() -> bool {
    current_el() == ExceptionLevel::EL2
}

/// Check if currently at EL3
#[inline]
pub fn in_el3() -> bool {
    current_el() == ExceptionLevel::EL3
}

// =============================================================================
// SPSR (Saved Program Status Register)
// =============================================================================

bitflags::bitflags! {
    /// Saved Program Status Register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Spsr: u64 {
        /// Negative condition flag
        const N = 1 << 31;
        /// Zero condition flag
        const Z = 1 << 30;
        /// Carry condition flag
        const C = 1 << 29;
        /// Overflow condition flag
        const V = 1 << 28;

        /// TCO - Top byte ignore for address checking
        const TCO = 1 << 25;
        /// DIT - Data Independent Timing
        const DIT = 1 << 24;
        /// UAO - User Access Override
        const UAO = 1 << 23;
        /// PAN - Privileged Access Never
        const PAN = 1 << 22;
        /// SS - Software Step
        const SS = 1 << 21;
        /// IL - Illegal Execution state
        const IL = 1 << 20;

        /// All exceptions (DAIF)
        const ALLINT = 1 << 13;

        /// Debug exception mask
        const D = 1 << 9;
        /// SError interrupt mask
        const A = 1 << 8;
        /// IRQ interrupt mask
        const I = 1 << 7;
        /// FIQ interrupt mask
        const F = 1 << 6;

        /// AArch64 mode (0 = AArch64, 1 = AArch32)
        const M4 = 1 << 4;

        // Mode bits [3:0] - EL and SP selection
        /// EL0t (EL0, SP_EL0)
        const EL0T = 0b0000;
        /// EL1t (EL1, SP_EL0)
        const EL1T = 0b0100;
        /// EL1h (EL1, SP_EL1)
        const EL1H = 0b0101;
        /// EL2t (EL2, SP_EL0)
        const EL2T = 0b1000;
        /// EL2h (EL2, SP_EL2)
        const EL2H = 0b1001;
        /// EL3t (EL3, SP_EL0)
        const EL3T = 0b1100;
        /// EL3h (EL3, SP_EL3)
        const EL3H = 0b1101;
    }
}

impl Spsr {
    /// Create SPSR for return to EL0
    pub const fn for_el0() -> Self {
        Self::from_bits_truncate(0b0000) // EL0t with interrupts enabled
    }

    /// Create SPSR for return to EL1 with SP_EL1
    pub const fn for_el1h() -> Self {
        Self::from_bits_truncate(0b0101) // EL1h
    }

    /// Create SPSR for return to EL1 with SP_EL0
    pub const fn for_el1t() -> Self {
        Self::from_bits_truncate(0b0100) // EL1t
    }

    /// Create SPSR for return to EL1 with interrupts masked
    pub const fn for_el1h_masked() -> Self {
        Self::from_bits_truncate(0b0101 | (1 << 7) | (1 << 6)) // EL1h + I + F
    }

    /// Get the EL from SPSR
    pub fn el(&self) -> ExceptionLevel {
        let mode = self.bits() & 0xF;
        let el = (mode >> 2) & 0x3;
        ExceptionLevel::from_raw(el as u8).unwrap_or(ExceptionLevel::EL0)
    }

    /// Check if uses dedicated SP (SP_ELx vs SP_EL0)
    pub fn uses_sp_elx(&self) -> bool {
        (self.bits() & 1) != 0
    }
}

// =============================================================================
// SPSR Register Access
// =============================================================================

/// Read SPSR_EL1
#[inline]
pub fn read_spsr_el1() -> Spsr {
    let value: u64;
    unsafe {
        asm!("mrs {}, SPSR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Spsr::from_bits_truncate(value)
}

/// Write SPSR_EL1
#[inline]
pub fn write_spsr_el1(spsr: Spsr) {
    unsafe {
        asm!("msr SPSR_EL1, {}", in(reg) spsr.bits(), options(nomem, nostack, preserves_flags));
    }
}

/// Read SPSR_EL2
#[inline]
pub fn read_spsr_el2() -> Spsr {
    let value: u64;
    unsafe {
        asm!("mrs {}, SPSR_EL2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Spsr::from_bits_truncate(value)
}

/// Write SPSR_EL2
#[inline]
pub fn write_spsr_el2(spsr: Spsr) {
    unsafe {
        asm!("msr SPSR_EL2, {}", in(reg) spsr.bits(), options(nomem, nostack, preserves_flags));
    }
}

// =============================================================================
// ELR (Exception Link Register)
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
pub fn write_elr_el1(value: u64) {
    unsafe {
        asm!("msr ELR_EL1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Read ELR_EL2
#[inline]
pub fn read_elr_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ELR_EL2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write ELR_EL2
#[inline]
pub fn write_elr_el2(value: u64) {
    unsafe {
        asm!("msr ELR_EL2, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

// =============================================================================
// Stack Pointer Selection
// =============================================================================

/// Stack Pointer selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpSelect {
    /// Use SP_EL0
    SpEl0 = 0,
    /// Use SP_ELx (dedicated SP for current EL)
    SpElx = 1,
}

/// Read SPSel
#[inline]
pub fn read_spsel() -> SpSelect {
    let value: u64;
    unsafe {
        asm!("mrs {}, SPSel", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    if value & 1 != 0 {
        SpSelect::SpElx
    } else {
        SpSelect::SpEl0
    }
}

/// Write SPSel
#[inline]
pub fn write_spsel(sp: SpSelect) {
    let value = sp as u64;
    unsafe {
        asm!("msr SPSel, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Switch to SP_EL0
#[inline]
pub fn use_sp_el0() {
    write_spsel(SpSelect::SpEl0);
}

/// Switch to SP_ELx
#[inline]
pub fn use_sp_elx() {
    write_spsel(SpSelect::SpElx);
}

// =============================================================================
// EL Transition
// =============================================================================

/// Prepare for ERET to target EL
///
/// Sets up ELR and SPSR for exception return.
///
/// # Arguments
/// * `target` - Target exception level
/// * `entry` - Entry point address
/// * `use_sp_elx` - Use dedicated stack pointer
/// * `mask_interrupts` - Mask IRQ/FIQ on entry
///
/// # Safety
/// Must be called from a higher exception level than target.
pub unsafe fn prepare_eret(
    target: ExceptionLevel,
    entry: u64,
    use_sp_elx: bool,
    mask_interrupts: bool,
) {
    // Build SPSR
    let mode = match (target, use_sp_elx) {
        (ExceptionLevel::EL0, _) => 0b0000, // EL0t
        (ExceptionLevel::EL1, false) => 0b0100, // EL1t
        (ExceptionLevel::EL1, true) => 0b0101, // EL1h
        (ExceptionLevel::EL2, false) => 0b1000, // EL2t
        (ExceptionLevel::EL2, true) => 0b1001, // EL2h
        (ExceptionLevel::EL3, false) => 0b1100, // EL3t
        (ExceptionLevel::EL3, true) => 0b1101, // EL3h
    };

    let mut spsr = mode as u64;
    if mask_interrupts {
        spsr |= (Spsr::I | Spsr::F).bits();
    }

    let current = current_el();

    match current {
        ExceptionLevel::EL2 => {
            asm!(
                "msr ELR_EL2, {elr}",
                "msr SPSR_EL2, {spsr}",
                elr = in(reg) entry,
                spsr = in(reg) spsr,
                options(nomem, nostack, preserves_flags)
            );
        }
        ExceptionLevel::EL3 => {
            asm!(
                "msr ELR_EL3, {elr}",
                "msr SPSR_EL3, {spsr}",
                elr = in(reg) entry,
                spsr = in(reg) spsr,
                options(nomem, nostack, preserves_flags)
            );
        }
        _ => {
            asm!(
                "msr ELR_EL1, {elr}",
                "msr SPSR_EL1, {spsr}",
                elr = in(reg) entry,
                spsr = in(reg) spsr,
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}

/// Execute exception return
///
/// # Safety
/// ELR and SPSR must be properly configured.
#[inline(never)]
pub unsafe fn eret() -> ! {
    asm!("eret", options(noreturn));
}

// =============================================================================
// Security State
// =============================================================================

/// Security state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityState {
    /// Secure state
    Secure,
    /// Non-secure state
    NonSecure,
}

/// Get current security state (requires EL3 or SCR_EL3 readable)
/// Returns NonSecure if not determinable
pub fn security_state() -> SecurityState {
    // This would require reading SCR_EL3.NS bit
    // For non-EL3, we assume non-secure
    if current_el() == ExceptionLevel::EL3 {
        SecurityState::Secure
    } else {
        SecurityState::NonSecure
    }
}

// =============================================================================
// HCR_EL2 for Hypervisor Configuration
// =============================================================================

bitflags::bitflags! {
    /// Hypervisor Configuration Register (EL2)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Hcr: u64 {
        /// VM - Virtualization enable
        const VM = 1 << 0;
        /// SWIO - Set/Way Invalidation Override
        const SWIO = 1 << 1;
        /// PTW - Protected Table Walk
        const PTW = 1 << 2;
        /// FMO - FIQ Mask Override
        const FMO = 1 << 3;
        /// IMO - IRQ Mask Override
        const IMO = 1 << 4;
        /// AMO - SError Mask Override
        const AMO = 1 << 5;
        /// VF - Virtual FIQ
        const VF = 1 << 6;
        /// VI - Virtual IRQ
        const VI = 1 << 7;
        /// VSE - Virtual SError
        const VSE = 1 << 8;
        /// FB - Force Broadcast
        const FB = 1 << 9;
        /// BSU - Barrier Shareability Upgrade
        const BSU_INNER = 1 << 10;
        const BSU_OUTER = 2 << 10;
        const BSU_FULL = 3 << 10;
        /// DC - Default Cacheable
        const DC = 1 << 12;
        /// TWI - Trap WFI
        const TWI = 1 << 13;
        /// TWE - Trap WFE
        const TWE = 1 << 14;
        /// TID0-3 - Trap ID register groups
        const TID0 = 1 << 15;
        const TID1 = 1 << 16;
        const TID2 = 1 << 17;
        const TID3 = 1 << 18;
        /// TSC - Trap SMC
        const TSC = 1 << 19;
        /// TIDCP - Trap implementation-defined
        const TIDCP = 1 << 20;
        /// TACR - Trap ACTLR
        const TACR = 1 << 21;
        /// TSW - Trap Set/Way operations
        const TSW = 1 << 22;
        /// TPCP - Trap DC operations
        const TPCP = 1 << 23;
        /// TPU - Trap TLB operations
        const TPU = 1 << 24;
        /// TTLB - Trap TLB maintenance
        const TTLB = 1 << 25;
        /// TVM - Trap virtual memory controls
        const TVM = 1 << 26;
        /// TGE - Trap General Exceptions
        const TGE = 1 << 27;
        /// TDZ - Trap DC ZVA
        const TDZ = 1 << 28;
        /// HCD - Hypervisor Call Disable
        const HCD = 1 << 29;
        /// TRVM - Trap Read Virtual Memory
        const TRVM = 1 << 30;
        /// RW - Register Width (1 = AArch64)
        const RW = 1 << 31;
        /// CD - Stage 2 cacheability disable
        const CD = 1 << 32;
        /// ID - Stage 2 instruction cacheability disable
        const ID = 1 << 33;
        /// E2H - EL2 Host mode
        const E2H = 1 << 34;
        /// TLOR - Trap LOR registers
        const TLOR = 1 << 35;
        /// TERR - Trap Error record accesses
        const TERR = 1 << 36;
        /// TEA - Trap External Abort
        const TEA = 1 << 37;
        /// APK - Trap APDAKey/APDBKey
        const APK = 1 << 40;
        /// API - Trap APIAKey/APIBKey
        const API = 1 << 41;
        /// NV - Nested Virtualization
        const NV = 1 << 42;
        /// NV1 - Nested Virtualization EL1
        const NV1 = 1 << 43;
        /// AT - Address Translation instructions
        const AT = 1 << 44;
        /// NV2 - Enhanced Nested Virtualization
        const NV2 = 1 << 45;
    }
}

/// Read HCR_EL2
#[inline]
pub fn read_hcr_el2() -> Hcr {
    let value: u64;
    unsafe {
        asm!("mrs {}, HCR_EL2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Hcr::from_bits_truncate(value)
}

/// Write HCR_EL2
#[inline]
pub fn write_hcr_el2(hcr: Hcr) {
    unsafe {
        asm!("msr HCR_EL2, {}", in(reg) hcr.bits(), options(nomem, nostack, preserves_flags));
    }
}

// =============================================================================
// SCR_EL3 for Secure Configuration
// =============================================================================

bitflags::bitflags! {
    /// Secure Configuration Register (EL3)
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Scr: u64 {
        /// NS - Non-secure bit
        const NS = 1 << 0;
        /// IRQ - IRQ routing to EL3
        const IRQ = 1 << 1;
        /// FIQ - FIQ routing to EL3
        const FIQ = 1 << 2;
        /// EA - External Abort routing
        const EA = 1 << 3;
        /// SMD - SMC Disable
        const SMD = 1 << 7;
        /// HCE - Hypervisor Call Enable
        const HCE = 1 << 8;
        /// SIF - Secure Instruction Fetch
        const SIF = 1 << 9;
        /// RW - Register Width for EL2 (1 = AArch64)
        const RW = 1 << 10;
        /// ST - Secure Timer
        const ST = 1 << 11;
        /// TWI - Trap WFI
        const TWI = 1 << 12;
        /// TWE - Trap WFE
        const TWE = 1 << 13;
        /// TLOR - Trap LOR registers
        const TLOR = 1 << 14;
        /// TERR - Trap Error records
        const TERR = 1 << 15;
        /// APK - Trap APDAKey/APDBKey
        const APK = 1 << 16;
        /// API - Trap APIAKey/APIBKey
        const API = 1 << 17;
        /// EEL2 - Enable Secure EL2
        const EEL2 = 1 << 18;
        /// EASE - External Abort SError
        const EASE = 1 << 19;
        /// NMEA - Non-maskable External Abort
        const NMEA = 1 << 20;
    }
}

/// Read SCR_EL3 (only valid at EL3)
#[inline]
pub fn read_scr_el3() -> Scr {
    let value: u64;
    unsafe {
        asm!("mrs {}, SCR_EL3", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    Scr::from_bits_truncate(value)
}

/// Write SCR_EL3 (only valid at EL3)
#[inline]
pub fn write_scr_el3(scr: Scr) {
    unsafe {
        asm!("msr SCR_EL3, {}", in(reg) scr.bits(), options(nomem, nostack, preserves_flags));
    }
}

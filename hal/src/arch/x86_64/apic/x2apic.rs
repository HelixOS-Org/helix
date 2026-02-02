//! # x2APIC Module
//!
//! This module implements the x2APIC mode for the Local APIC.
//! x2APIC provides MSR-based access to APIC registers, which is
//! faster and supports larger APIC IDs (32-bit).
//!
//! ## x2APIC vs xAPIC Comparison
//!
//! | Feature          | xAPIC           | x2APIC          |
//! |------------------|-----------------|-----------------|
//! | Access Method    | Memory-mapped   | MSR             |
//! | APIC ID Width    | 8 bits          | 32 bits         |
//! | Max CPUs         | 255             | 2^32            |
//! | ICR Write        | Two 32-bit      | Single 64-bit   |
//! | Performance      | Slower          | Faster          |
//!
//! ## Register Access
//!
//! x2APIC registers are accessed via MSRs starting at 0x800:
//! - MSR 0x800 + (MMIO_Offset >> 4) = x2APIC register
//!
//! ## Enabling x2APIC
//!
//! x2APIC is enabled via the IA32_APIC_BASE MSR:
//! - Set bit 10 (x2APIC enable) along with bit 11 (APIC enable)
//! - Once enabled, cannot return to xAPIC without full APIC disable

use core::sync::atomic::{AtomicBool, Ordering};

use super::registers;

// =============================================================================
// x2APIC MSR Addresses
// =============================================================================

/// x2APIC MSR base and register offsets
pub mod x2apic_msr {
    /// Base MSR address for x2APIC
    pub const BASE: u32 = 0x800;

    /// APIC ID Register (read-only)
    pub const ID: u32 = BASE + 0x02;

    /// APIC Version Register (read-only)
    pub const VERSION: u32 = BASE + 0x03;

    /// Task Priority Register
    pub const TPR: u32 = BASE + 0x08;

    /// Processor Priority Register (read-only)
    pub const PPR: u32 = BASE + 0x0A;

    /// End Of Interrupt Register (write-only)
    pub const EOI: u32 = BASE + 0x0B;

    /// Logical Destination Register (read-only in x2APIC)
    pub const LDR: u32 = BASE + 0x0D;

    /// Spurious Interrupt Vector Register
    pub const SVR: u32 = BASE + 0x0F;

    /// In-Service Register (256 bits, 8 MSRs)
    pub const ISR_BASE: u32 = BASE + 0x10;

    /// Trigger Mode Register (256 bits, 8 MSRs)
    pub const TMR_BASE: u32 = BASE + 0x18;

    /// Interrupt Request Register (256 bits, 8 MSRs)
    pub const IRR_BASE: u32 = BASE + 0x20;

    /// Error Status Register
    pub const ESR: u32 = BASE + 0x28;

    /// LVT CMCI (Corrected Machine Check Interrupt)
    pub const LVT_CMCI: u32 = BASE + 0x2F;

    /// Interrupt Command Register (64-bit in x2APIC)
    pub const ICR: u32 = BASE + 0x30;

    /// LVT Timer Register
    pub const LVT_TIMER: u32 = BASE + 0x32;

    /// LVT Thermal Sensor Register
    pub const LVT_THERMAL: u32 = BASE + 0x33;

    /// LVT Performance Monitoring Register
    pub const LVT_PERF: u32 = BASE + 0x34;

    /// LVT LINT0 Register
    pub const LVT_LINT0: u32 = BASE + 0x35;

    /// LVT LINT1 Register
    pub const LVT_LINT1: u32 = BASE + 0x36;

    /// LVT Error Register
    pub const LVT_ERROR: u32 = BASE + 0x37;

    /// Timer Initial Count Register
    pub const TIMER_ICR: u32 = BASE + 0x38;

    /// Timer Current Count Register (read-only)
    pub const TIMER_CCR: u32 = BASE + 0x39;

    /// Timer Divide Configuration Register
    pub const TIMER_DCR: u32 = BASE + 0x3E;

    /// Self IPI Register (write-only in x2APIC)
    pub const SELF_IPI: u32 = BASE + 0x3F;
}

// Re-export for use by other modules
pub use x2apic_msr::*;

// =============================================================================
// x2APIC State
// =============================================================================

/// Whether x2APIC mode is active
static X2APIC_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Check if x2APIC mode is currently active
#[inline]
pub fn is_active() -> bool {
    X2APIC_ACTIVE.load(Ordering::Acquire)
}

/// Set x2APIC active state
#[inline]
fn set_active(active: bool) {
    X2APIC_ACTIVE.store(active, Ordering::Release);
}

// =============================================================================
// MSR Access
// =============================================================================

/// Read an MSR
#[inline]
unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nostack, preserves_flags),
    );
    ((high as u64) << 32) | (low as u64)
}

/// Write an MSR
#[inline]
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nostack, preserves_flags),
    );
}

// =============================================================================
// x2APIC Detection
// =============================================================================

/// IA32_APIC_BASE MSR
const IA32_APIC_BASE_MSR: u32 = 0x1B;

/// APIC enable bit in IA32_APIC_BASE
const APIC_ENABLE_BIT: u64 = 1 << 11;

/// x2APIC enable bit in IA32_APIC_BASE
const X2APIC_ENABLE_BIT: u64 = 1 << 10;

/// Check if x2APIC is supported (via CPUID)
pub fn is_supported() -> bool {
    let result: u32;
    unsafe {
        core::arch::asm!(
            "mov eax, 1",
            "cpuid",
            out("ecx") result,
            out("eax") _,
            out("ebx") _,
            out("edx") _,
            options(nostack, preserves_flags),
        );
    }
    result & (1 << 21) != 0
}

/// Check if x2APIC is currently enabled in hardware
pub fn is_enabled_in_hardware() -> bool {
    unsafe {
        let base = rdmsr(IA32_APIC_BASE_MSR);
        (base & APIC_ENABLE_BIT != 0) && (base & X2APIC_ENABLE_BIT != 0)
    }
}

// =============================================================================
// x2APIC Initialization
// =============================================================================

/// Enable x2APIC mode
///
/// # Safety
///
/// Must be called during early boot before interrupts are enabled.
/// Cannot be disabled without full system reset.
pub unsafe fn enable() -> Result<(), X2ApicError> {
    if !is_supported() {
        return Err(X2ApicError::NotSupported);
    }

    if is_active() {
        return Ok(()); // Already enabled
    }

    // Read current APIC base
    let mut base = rdmsr(IA32_APIC_BASE_MSR);

    // Enable APIC and x2APIC
    base |= APIC_ENABLE_BIT | X2APIC_ENABLE_BIT;
    wrmsr(IA32_APIC_BASE_MSR, base);

    // Verify x2APIC is enabled
    let base = rdmsr(IA32_APIC_BASE_MSR);
    if base & X2APIC_ENABLE_BIT == 0 {
        return Err(X2ApicError::EnableFailed);
    }

    set_active(true);
    Ok(())
}

/// Disable x2APIC mode (returns to xAPIC)
///
/// # Safety
///
/// This requires disabling the APIC entirely first, which is
/// generally not recommended on a running system.
pub unsafe fn disable() -> Result<(), X2ApicError> {
    if !is_active() {
        return Ok(());
    }

    // Read current APIC base
    let mut base = rdmsr(IA32_APIC_BASE_MSR);

    // Must first disable APIC entirely
    base &= !(APIC_ENABLE_BIT | X2APIC_ENABLE_BIT);
    wrmsr(IA32_APIC_BASE_MSR, base);

    // Then re-enable in xAPIC mode
    base |= APIC_ENABLE_BIT;
    wrmsr(IA32_APIC_BASE_MSR, base);

    set_active(false);
    Ok(())
}

// =============================================================================
// x2APIC Error Type
// =============================================================================

/// x2APIC error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X2ApicError {
    /// x2APIC not supported by CPU
    NotSupported,
    /// Failed to enable x2APIC
    EnableFailed,
}

impl core::fmt::Display for X2ApicError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            X2ApicError::NotSupported => write!(f, "x2APIC not supported"),
            X2ApicError::EnableFailed => write!(f, "Failed to enable x2APIC"),
        }
    }
}

// =============================================================================
// x2APIC Register Access
// =============================================================================

/// x2APIC register abstraction
pub struct X2Apic;

impl X2Apic {
    /// Read the APIC ID
    #[inline]
    pub fn id() -> u32 {
        unsafe { rdmsr(x2apic_msr::ID) as u32 }
    }

    /// Read the APIC version
    #[inline]
    pub fn version() -> u32 {
        unsafe { rdmsr(x2apic_msr::VERSION) as u32 }
    }

    /// Get the maximum LVT entries
    #[inline]
    pub fn max_lvt() -> u8 {
        ((Self::version() >> 16) & 0xFF) as u8 + 1
    }

    /// Read Task Priority Register
    #[inline]
    pub fn read_tpr() -> u8 {
        unsafe { rdmsr(x2apic_msr::TPR) as u8 }
    }

    /// Write Task Priority Register
    #[inline]
    pub fn write_tpr(priority: u8) {
        unsafe {
            wrmsr(x2apic_msr::TPR, priority as u64);
        }
    }

    /// Read Processor Priority Register
    #[inline]
    pub fn read_ppr() -> u8 {
        unsafe { rdmsr(x2apic_msr::PPR) as u8 }
    }

    /// Send End-Of-Interrupt
    #[inline]
    pub fn eoi() {
        unsafe {
            wrmsr(x2apic_msr::EOI, 0);
        }
    }

    /// Read Logical Destination Register
    #[inline]
    pub fn read_ldr() -> u32 {
        unsafe { rdmsr(x2apic_msr::LDR) as u32 }
    }

    /// Read Spurious Vector Register
    #[inline]
    pub fn read_svr() -> u32 {
        unsafe { rdmsr(x2apic_msr::SVR) as u32 }
    }

    /// Write Spurious Vector Register
    #[inline]
    pub fn write_svr(value: u32) {
        unsafe {
            wrmsr(x2apic_msr::SVR, value as u64);
        }
    }

    /// Enable APIC with spurious vector
    #[inline]
    pub fn enable_apic(spurious_vector: u8) {
        let svr = (spurious_vector as u32) | (1 << 8); // Enable bit
        Self::write_svr(svr);
    }

    /// Disable APIC (software disable)
    #[inline]
    pub fn disable_apic() {
        let svr = Self::read_svr() & !(1 << 8);
        Self::write_svr(svr);
    }

    /// Read Error Status Register
    #[inline]
    pub fn read_esr() -> u32 {
        unsafe {
            wrmsr(x2apic_msr::ESR, 0);
            rdmsr(x2apic_msr::ESR) as u32
        }
    }

    /// Read LVT Timer
    #[inline]
    pub fn read_lvt_timer() -> u32 {
        unsafe { rdmsr(x2apic_msr::LVT_TIMER) as u32 }
    }

    /// Write LVT Timer
    #[inline]
    pub fn write_lvt_timer(value: u32) {
        unsafe {
            wrmsr(x2apic_msr::LVT_TIMER, value as u64);
        }
    }

    /// Read Timer Initial Count
    #[inline]
    pub fn read_timer_icr() -> u32 {
        unsafe { rdmsr(x2apic_msr::TIMER_ICR) as u32 }
    }

    /// Write Timer Initial Count
    #[inline]
    pub fn write_timer_icr(count: u32) {
        unsafe {
            wrmsr(x2apic_msr::TIMER_ICR, count as u64);
        }
    }

    /// Read Timer Current Count
    #[inline]
    pub fn read_timer_ccr() -> u32 {
        unsafe { rdmsr(x2apic_msr::TIMER_CCR) as u32 }
    }

    /// Read Timer Divide Configuration
    #[inline]
    pub fn read_timer_dcr() -> u32 {
        unsafe { rdmsr(x2apic_msr::TIMER_DCR) as u32 }
    }

    /// Write Timer Divide Configuration
    #[inline]
    pub fn write_timer_dcr(divide: u32) {
        unsafe {
            wrmsr(x2apic_msr::TIMER_DCR, divide as u64);
        }
    }

    /// Send IPI using ICR
    ///
    /// In x2APIC mode, the ICR is a single 64-bit MSR.
    /// - Bits 0-7: Vector
    /// - Bits 8-10: Delivery Mode
    /// - Bit 11: Destination Mode
    /// - Bit 12: Delivery Status (read-only)
    /// - Bit 14: Level
    /// - Bit 15: Trigger Mode
    /// - Bits 18-19: Destination Shorthand
    /// - Bits 32-63: Destination APIC ID
    pub fn send_ipi(dest: u32, vector: u8, delivery_mode: u8, shorthand: u8) {
        let icr = (dest as u64) << 32
            | (vector as u64)
            | ((delivery_mode as u64) << 8)
            | (1 << 14)  // Level: Assert
            | ((shorthand as u64) << 18);

        unsafe {
            wrmsr(x2apic_msr::ICR, icr);
        }
    }

    /// Send self-IPI (faster than regular IPI to self)
    #[inline]
    pub fn send_self_ipi(vector: u8) {
        unsafe {
            wrmsr(x2apic_msr::SELF_IPI, vector as u64);
        }
    }

    /// Read ISR (In-Service Register) bit
    pub fn is_in_service(vector: u8) -> bool {
        let index = vector / 32;
        let bit = vector % 32;
        let isr = unsafe { rdmsr(x2apic_msr::ISR_BASE + index as u32) as u32 };
        isr & (1 << bit) != 0
    }

    /// Read IRR (Interrupt Request Register) bit
    pub fn is_pending(vector: u8) -> bool {
        let index = vector / 32;
        let bit = vector % 32;
        let irr = unsafe { rdmsr(x2apic_msr::IRR_BASE + index as u32) as u32 };
        irr & (1 << bit) != 0
    }

    /// Read TMR (Trigger Mode Register) bit
    pub fn is_level_triggered(vector: u8) -> bool {
        let index = vector / 32;
        let bit = vector % 32;
        let tmr = unsafe { rdmsr(x2apic_msr::TMR_BASE + index as u32) as u32 };
        tmr & (1 << bit) != 0
    }
}

// =============================================================================
// LVT Configuration
// =============================================================================

/// Configure LVT Timer in x2APIC mode
pub fn configure_timer(vector: u8, mode: TimerMode, masked: bool) {
    let mut lvt = vector as u32;

    lvt |= match mode {
        TimerMode::OneShot => 0,
        TimerMode::Periodic => 1 << 17,
        TimerMode::TscDeadline => 2 << 17,
    };

    if masked {
        lvt |= 1 << 16;
    }

    X2Apic::write_lvt_timer(lvt);
}

/// Timer Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    OneShot,
    Periodic,
    TscDeadline,
}

// =============================================================================
// IPI Support
// =============================================================================

/// Destination shorthand values
pub mod shorthand {
    /// No shorthand - use destination field
    pub const NONE: u8 = 0b00;
    /// Self
    pub const SELF: u8 = 0b01;
    /// All including self
    pub const ALL_INCLUDING_SELF: u8 = 0b10;
    /// All excluding self
    pub const ALL_EXCLUDING_SELF: u8 = 0b11;
}

/// Delivery mode values
pub mod delivery_mode {
    /// Fixed
    pub const FIXED: u8 = 0b000;
    /// Lowest priority
    pub const LOWEST_PRIORITY: u8 = 0b001;
    /// SMI
    pub const SMI: u8 = 0b010;
    /// NMI
    pub const NMI: u8 = 0b100;
    /// INIT
    pub const INIT: u8 = 0b101;
    /// Startup IPI
    pub const SIPI: u8 = 0b110;
}

/// Send a fixed IPI to a specific APIC ID
#[inline]
pub fn send_fixed_ipi(dest: u32, vector: u8) {
    X2Apic::send_ipi(dest, vector, delivery_mode::FIXED, shorthand::NONE);
}

/// Send an NMI to a specific APIC ID
#[inline]
pub fn send_nmi(dest: u32) {
    X2Apic::send_ipi(dest, 0, delivery_mode::NMI, shorthand::NONE);
}

/// Send an INIT IPI to a specific APIC ID
#[inline]
pub fn send_init(dest: u32) {
    X2Apic::send_ipi(dest, 0, delivery_mode::INIT, shorthand::NONE);
}

/// Send a SIPI (Startup IPI) to a specific APIC ID
///
/// The vector specifies the startup address as vector * 0x1000
#[inline]
pub fn send_sipi(dest: u32, vector: u8) {
    X2Apic::send_ipi(dest, vector, delivery_mode::SIPI, shorthand::NONE);
}

/// Broadcast an IPI to all CPUs (excluding self)
#[inline]
pub fn broadcast_ipi(vector: u8) {
    X2Apic::send_ipi(
        0,
        vector,
        delivery_mode::FIXED,
        shorthand::ALL_EXCLUDING_SELF,
    );
}

/// Broadcast an NMI to all CPUs (excluding self)
#[inline]
pub fn broadcast_nmi() {
    X2Apic::send_ipi(0, 0, delivery_mode::NMI, shorthand::ALL_EXCLUDING_SELF);
}

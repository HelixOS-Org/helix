//! # Local APIC
//!
//! This module implements the Local APIC functionality for both
//! xAPIC (memory-mapped) and x2APIC (MSR-based) modes.

use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use super::ipi::IpiDestination;
use super::{is_x2apic_enabled, registers, x2apic_msr, ERROR_VECTOR, SPURIOUS_VECTOR};

// =============================================================================
// Error Type
// =============================================================================

/// APIC Error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicError {
    /// APIC is not present
    NotPresent,
    /// APIC already initialized
    AlreadyInitialized,
    /// APIC not initialized
    NotInitialized,
    /// Invalid APIC ID
    InvalidId,
    /// IPI send failed
    IpiSendFailed,
    /// Timer configuration error
    TimerError,
    /// I/O APIC error
    IoApicError,
}

impl fmt::Display for ApicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApicError::NotPresent => write!(f, "APIC not present"),
            ApicError::AlreadyInitialized => write!(f, "APIC already initialized"),
            ApicError::NotInitialized => write!(f, "APIC not initialized"),
            ApicError::InvalidId => write!(f, "Invalid APIC ID"),
            ApicError::IpiSendFailed => write!(f, "IPI send failed"),
            ApicError::TimerError => write!(f, "Timer configuration error"),
            ApicError::IoApicError => write!(f, "I/O APIC error"),
        }
    }
}

// =============================================================================
// Local APIC Mode
// =============================================================================

/// Local APIC operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalApicMode {
    /// xAPIC (memory-mapped)
    XApic,
    /// x2APIC (MSR-based)
    X2Apic,
}

// =============================================================================
// LAPIC Base Address
// =============================================================================

/// Virtual base address for Local APIC memory-mapped registers.
static LAPIC_VIRT_BASE: AtomicU64 = AtomicU64::new(0);

/// Set the virtual address for LAPIC access
///
/// # Safety
///
/// The address must be a valid mapping of the LAPIC registers.
#[inline]
pub unsafe fn set_lapic_base(virt_addr: u64) {
    LAPIC_VIRT_BASE.store(virt_addr, Ordering::SeqCst);
}

/// Get the current LAPIC virtual base address
#[inline]
fn lapic_base() -> u64 {
    LAPIC_VIRT_BASE.load(Ordering::Relaxed)
}

// =============================================================================
// Register Access
// =============================================================================

/// Read a Local APIC register (xAPIC mode)
#[inline]
unsafe fn read_xapic(offset: u32) -> u32 {
    let addr = lapic_base() + offset as u64;
    unsafe { core::ptr::read_volatile(addr as *const u32) }
}

/// Write a Local APIC register (xAPIC mode)
#[inline]
unsafe fn write_xapic(offset: u32, value: u32) {
    let addr = lapic_base() + offset as u64;
    unsafe {
        core::ptr::write_volatile(addr as *mut u32, value);
    }
}

/// Read an MSR (x2APIC mode)
#[inline]
unsafe fn read_msr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags),
        );
    }
    ((high as u64) << 32) | (low as u64)
}

/// Write an MSR (x2APIC mode)
#[inline]
unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nostack, preserves_flags),
        );
    }
}

/// Read a Local APIC register (auto-detects mode)
///
/// # Safety
///
/// - The Local APIC must be properly initialized and mapped.
/// - The offset must be a valid LAPIC register offset.
#[inline]
pub unsafe fn read_lapic(offset: u32) -> u32 {
    if is_x2apic_enabled() {
        // Convert offset to MSR (offset / 16 + 0x800)
        let msr = x2apic_msr::BASE + (offset >> 4);
        unsafe { read_msr(msr) as u32 }
    } else {
        unsafe { read_xapic(offset) }
    }
}

/// Write a Local APIC register (auto-detects mode)
///
/// # Safety
///
/// - The Local APIC must be properly initialized and mapped.
/// - The offset must be a valid LAPIC register offset.
/// - Writing invalid values may cause undefined hardware behavior.
#[inline]
pub unsafe fn write_lapic(offset: u32, value: u32) {
    if is_x2apic_enabled() {
        let msr = x2apic_msr::BASE + (offset >> 4);
        unsafe {
            write_msr(msr, value as u64);
        }
    } else {
        unsafe {
            write_xapic(offset, value);
        }
    }
}

// =============================================================================
// Local APIC Structure
// =============================================================================

/// Local APIC abstraction for managing the CPU-local APIC.
///
/// Supports both xAPIC (memory-mapped) and x2APIC (MSR-based) modes.
pub struct LocalApic {
    /// Base virtual address for xAPIC memory-mapped access.
    base: u64,
    /// Current operating mode (xAPIC or x2APIC).
    mode: LocalApicMode,
}

impl LocalApic {
    /// Create a new Local APIC instance
    ///
    /// # Safety
    ///
    /// The base address must be valid if using xAPIC mode.
    pub unsafe fn new(base: u64) -> Self {
        let mode = if is_x2apic_enabled() {
            LocalApicMode::X2Apic
        } else {
            LocalApicMode::XApic
        };

        Self { base, mode }
    }

    /// Get the operating mode
    #[inline]
    pub fn mode(&self) -> LocalApicMode {
        self.mode
    }

    /// Get the APIC ID
    #[inline]
    pub fn id(&self) -> u32 {
        unsafe {
            if self.mode == LocalApicMode::X2Apic {
                read_msr(x2apic_msr::ID) as u32
            } else {
                (read_xapic(registers::ID) >> 24) & 0xFF
            }
        }
    }

    /// Get the APIC version
    #[inline]
    pub fn version(&self) -> u32 {
        unsafe { read_lapic(registers::VERSION) }
    }

    /// Get the maximum LVT entries
    #[inline]
    pub fn max_lvt(&self) -> u8 {
        ((unsafe { read_lapic(registers::VERSION) } >> 16) & 0xFF) as u8 + 1
    }

    /// Set the spurious interrupt vector and enable APIC
    pub fn enable(&mut self, spurious_vector: u8) {
        unsafe {
            let mut svr = read_lapic(registers::SVR);
            // Set vector and enable bit
            svr = (svr & 0xFFFF_FF00) | (spurious_vector as u32) | (1 << 8);
            write_lapic(registers::SVR, svr);
        }
    }

    /// Disable the Local APIC
    pub fn disable(&mut self) {
        unsafe {
            let mut svr = read_lapic(registers::SVR);
            svr &= !(1 << 8); // Clear enable bit
            write_lapic(registers::SVR, svr);
        }
    }

    /// Send End-Of-Interrupt
    #[inline]
    pub fn eoi(&self) {
        unsafe {
            write_lapic(registers::EOI, 0);
        }
    }

    /// Set Task Priority Register
    #[inline]
    pub fn set_tpr(&self, priority: u8) {
        unsafe {
            write_lapic(registers::TPR, priority as u32);
        }
    }

    /// Get Task Priority Register
    #[inline]
    pub fn get_tpr(&self) -> u8 {
        unsafe { read_lapic(registers::TPR) as u8 }
    }

    /// Get Processor Priority Register
    #[inline]
    pub fn get_ppr(&self) -> u8 {
        unsafe { read_lapic(registers::PPR) as u8 }
    }

    /// Read Error Status Register
    #[inline]
    pub fn read_error_status(&self) -> u32 {
        unsafe {
            // Write before read to update
            write_lapic(registers::ESR, 0);
            read_lapic(registers::ESR)
        }
    }

    /// Configure LVT Timer
    pub fn configure_timer(&self, vector: u8, mode: TimerMode, masked: bool) {
        let mut lvt = vector as u32;

        lvt |= match mode {
            TimerMode::OneShot => 0,
            TimerMode::Periodic => 1 << 17,
            TimerMode::TscDeadline => 2 << 17,
        };

        if masked {
            lvt |= 1 << 16;
        }

        unsafe {
            write_lapic(registers::LVT_TIMER, lvt);
        }
    }

    /// Set timer divide configuration
    pub fn set_timer_divide(&self, divide: TimerDivide) {
        unsafe {
            write_lapic(registers::TIMER_DCR, divide as u32);
        }
    }

    /// Set timer initial count
    pub fn set_timer_initial_count(&self, count: u32) {
        unsafe {
            write_lapic(registers::TIMER_ICR, count);
        }
    }

    /// Get timer current count
    #[inline]
    pub fn get_timer_current_count(&self) -> u32 {
        unsafe { read_lapic(registers::TIMER_CCR) }
    }

    /// Configure an LVT entry
    pub fn configure_lvt(&self, lvt: LvtEntry, vector: u8, masked: bool) {
        let offset = match lvt {
            LvtEntry::Timer => registers::LVT_TIMER,
            LvtEntry::Thermal => registers::LVT_THERMAL,
            LvtEntry::PerfCounter => registers::LVT_PERF,
            LvtEntry::Lint0 => registers::LVT_LINT0,
            LvtEntry::Lint1 => registers::LVT_LINT1,
            LvtEntry::Error => registers::LVT_ERROR,
            LvtEntry::Cmci => registers::LVT_CMCI,
        };

        let mut value = vector as u32;
        if masked {
            value |= 1 << 16;
        }

        unsafe {
            write_lapic(offset, value);
        }
    }
}

impl fmt::Debug for LocalApic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalApic")
            .field("base", &format_args!("{:#x}", self.base))
            .field("mode", &self.mode)
            .field("id", &self.id())
            .field("version", &format_args!("{:#x}", self.version()))
            .finish()
    }
}

// =============================================================================
// Timer Configuration
// =============================================================================

/// APIC Timer Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerMode {
    /// One-shot mode
    OneShot,
    /// Periodic mode
    Periodic,
    /// TSC-Deadline mode (requires CPUID.01H:ECX.TSC_Deadline[bit 24])
    TscDeadline,
}

/// APIC Timer Divide Configuration
///
/// Controls the divisor applied to the bus/core clock for the APIC timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum TimerDivide {
    /// Divide by 1 (no division).
    By1   = 0b1011,
    /// Divide by 2.
    By2   = 0b0000,
    /// Divide by 4.
    By4   = 0b0001,
    /// Divide by 8.
    By8   = 0b0010,
    /// Divide by 16.
    By16  = 0b0011,
    /// Divide by 32.
    By32  = 0b1000,
    /// Divide by 64.
    By64  = 0b1001,
    /// Divide by 128.
    By128 = 0b1010,
}

// =============================================================================
// LVT Entries
// =============================================================================

/// Local Vector Table entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LvtEntry {
    /// Timer
    Timer,
    /// Thermal sensor
    Thermal,
    /// Performance counter
    PerfCounter,
    /// Local Interrupt 0
    Lint0,
    /// Local Interrupt 1
    Lint1,
    /// Error
    Error,
    /// Corrected Machine Check Interrupt
    Cmci,
}

// =============================================================================
// Module-level Functions
// =============================================================================

/// Initialize the Local APIC
///
/// # Safety
///
/// Must be called during early boot.
pub unsafe fn init(base: u64) -> Result<(), ApicError> {
    // Set the base address for register access
    unsafe {
        set_lapic_base(base);
    }

    // Enable the APIC with spurious vector
    let mut svr = unsafe { read_lapic(registers::SVR) };
    svr |= (SPURIOUS_VECTOR as u32) | (1 << 8); // Enable + spurious vector
    unsafe {
        write_lapic(registers::SVR, svr);
    }

    // Set TPR to 0 to accept all interrupts
    unsafe {
        write_lapic(registers::TPR, 0);
    }

    // Configure error LVT
    unsafe {
        write_lapic(registers::LVT_ERROR, ERROR_VECTOR as u32);
    }

    // Clear any pending errors
    unsafe {
        write_lapic(registers::ESR, 0);
    }
    unsafe {
        write_lapic(registers::ESR, 0);
    }

    // Mask all LVT entries initially
    unsafe {
        write_lapic(registers::LVT_TIMER, 1 << 16);
    }
    unsafe {
        write_lapic(registers::LVT_THERMAL, 1 << 16);
    }
    unsafe {
        write_lapic(registers::LVT_PERF, 1 << 16);
    }

    // For xAPIC, set flat destination mode
    if !is_x2apic_enabled() {
        unsafe {
            write_lapic(registers::DFR, 0xFFFF_FFFF);
        } // Flat model
        let id = unsafe { (read_lapic(registers::ID) >> 24) & 0xFF };
        unsafe {
            write_lapic(registers::LDR, id << 24);
        }
    }

    Ok(())
}

/// Get the current CPU's APIC ID
#[inline]
pub fn get_apic_id() -> u32 {
    unsafe {
        if is_x2apic_enabled() {
            read_msr(x2apic_msr::ID) as u32
        } else {
            (read_xapic(registers::ID) >> 24) & 0xFF
        }
    }
}

/// Send End-Of-Interrupt
#[inline]
pub fn send_eoi() {
    unsafe {
        write_lapic(registers::EOI, 0);
    }
}

/// Send an IPI
///
/// # Safety
///
/// - The Local APIC must be properly initialized.
/// - The destination CPU must be capable of receiving the interrupt.
/// - The vector must be a valid interrupt vector (32-255).
pub unsafe fn send_ipi(destination: IpiDestination, vector: u8) {
    if is_x2apic_enabled() {
        unsafe {
            send_ipi_x2apic(destination, vector);
        }
    } else {
        unsafe {
            send_ipi_xapic(destination, vector);
        }
    }
}

/// Send IPI in xAPIC mode
unsafe fn send_ipi_xapic(destination: IpiDestination, vector: u8) {
    // Wait for previous IPI to complete
    while unsafe { read_xapic(registers::ICR_LOW) } & (1 << 12) != 0 {
        core::hint::spin_loop();
    }

    let (icr_high, dest_shorthand) = match destination {
        IpiDestination::Single(id) => (id << 24, 0b00),
        IpiDestination::Myself => (0, 0b01),
        IpiDestination::AllIncludingSelf => (0, 0b10),
        IpiDestination::AllExcludingSelf => (0, 0b11),
    };

    // ICR High: destination
    unsafe {
        write_xapic(registers::ICR_HIGH, icr_high);
    }

    // ICR Low: vector, delivery mode (fixed), level (assert), trigger (edge), shorthand
    let icr_low = (vector as u32)         // Physical destination mode
        | (1 << 14)         // Trigger: Edge
        | ((dest_shorthand as u32) << 18);

    unsafe {
        write_xapic(registers::ICR_LOW, icr_low);
    }
}

/// Send IPI in x2APIC mode
unsafe fn send_ipi_x2apic(destination: IpiDestination, vector: u8) {
    let (dest_id, dest_shorthand) = match destination {
        IpiDestination::Single(id) => (id, 0b00),
        IpiDestination::Myself => (0, 0b01),
        IpiDestination::AllIncludingSelf => (0, 0b10),
        IpiDestination::AllExcludingSelf => (0, 0b11),
    };

    // x2APIC ICR is a single 64-bit MSR
    let icr = ((dest_id as u64) << 32)
        | (vector as u64)         // Physical destination mode
        | (1 << 14)         // Trigger: Edge
        | ((dest_shorthand as u64) << 18);

    unsafe {
        write_msr(x2apic_msr::ICR, icr);
    }
}

/// Check if an interrupt is pending
#[inline]
pub fn is_interrupt_pending() -> bool {
    unsafe {
        // Check IRR (Interrupt Request Register)
        for i in 0..8 {
            if read_lapic(registers::IRR_BASE + i * 0x10) != 0 {
                return true;
            }
        }
        false
    }
}

/// Get the highest priority pending interrupt vector.
///
/// Returns the vector number of the highest priority interrupt waiting
/// in the Interrupt Request Register (IRR), or `None` if no interrupts are pending.
pub fn get_highest_priority_pending() -> Option<u8> {
    unsafe {
        for i in (0..8).rev() {
            let irr = read_lapic(registers::IRR_BASE + i * 0x10);
            if irr != 0 {
                // Find highest set bit
                let bit = 31 - irr.leading_zeros();
                return Some((i * 32 + bit) as u8);
            }
        }
        None
    }
}

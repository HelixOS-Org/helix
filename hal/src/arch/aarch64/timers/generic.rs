//! # Generic Timer Counter Access
//!
//! This module provides access to the ARM Generic Timer system counter,
//! including frequency detection and counter reading.
//!
//! ## System Registers
//!
//! | Register      | Description                                        |
//! |---------------|----------------------------------------------------|
//! | CNTFRQ_EL0    | Counter frequency (set by firmware)               |
//! | CNTPCT_EL0    | Physical counter value (64-bit)                   |
//! | CNTVCT_EL0    | Virtual counter value (64-bit)                    |
//! | CNTKCTL_EL1   | Kernel control (EL0 access permissions)           |
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │                    System Counter Architecture                        │
//! ├───────────────────────────────────────────────────────────────────────┤
//! │                                                                       │
//! │   System Counter (always running)                                     │
//! │   ┌──────────────────────────────────────────────────────────────┐   │
//! │   │                                                              │   │
//! │   │   CNTPCT (Physical)                                          │   │
//! │   │   ┌─────────────────────────────────────────────────────┐   │   │
//! │   │   │  64-bit monotonic counter                           │   │   │
//! │   │   │  Always incrementing at CNTFRQ frequency           │   │   │
//! │   │   │  Cannot be stopped or reset                        │   │   │
//! │   │   └─────────────────────────────────────────────────────┘   │   │
//! │   │                       │                                      │   │
//! │   │                       ▼                                      │   │
//! │   │   CNTVCT (Virtual) = CNTPCT - CNTVOFF                       │   │
//! │   │   ┌─────────────────────────────────────────────────────┐   │   │
//! │   │   │  64-bit virtual counter                             │   │   │
//! │   │   │  Offset by hypervisor-controlled CNTVOFF_EL2        │   │   │
//! │   │   │  Used for VM-isolated timekeeping                   │   │   │
//! │   │   └─────────────────────────────────────────────────────┘   │   │
//! │   │                                                              │   │
//! │   └──────────────────────────────────────────────────────────────┘   │
//! │                                                                       │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```

use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// System Register Access
// ============================================================================

/// Read CNTFRQ_EL0 (Counter Frequency)
///
/// Returns the frequency of the system counter in Hz.
/// This value is typically set by firmware and is read-only.
#[inline]
pub fn read_cntfrq_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntfrq_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read CNTPCT_EL0 (Physical Counter)
///
/// Returns the current value of the physical counter.
/// This counter is always running and cannot be stopped.
#[inline]
pub fn read_cntpct_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntpct_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read CNTVCT_EL0 (Virtual Counter)
///
/// Returns the current value of the virtual counter.
/// Virtual count = Physical count - CNTVOFF_EL2
#[inline]
pub fn read_cntvct_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntvct_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read CNTKCTL_EL1 (Counter-timer Kernel Control)
///
/// Controls EL0 access to timer registers.
#[inline]
pub fn read_cntkctl_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntkctl_el1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTKCTL_EL1
#[inline]
pub fn write_cntkctl_el1(value: u64) {
    unsafe {
        asm!("msr cntkctl_el1, {}", in(reg) value, options(nomem, nostack));
    }
}

// EL2 registers (available when running at EL2)

/// Read CNTVOFF_EL2 (Virtual Offset)
///
/// Only accessible at EL2 or EL3.
#[inline]
pub fn read_cntvoff_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntvoff_el2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTVOFF_EL2 (Virtual Offset)
///
/// Only accessible at EL2 or EL3.
#[inline]
pub fn write_cntvoff_el2(value: u64) {
    unsafe {
        asm!("msr cntvoff_el2, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTHCTL_EL2 (Counter-timer Hypervisor Control)
#[inline]
pub fn read_cnthctl_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cnthctl_el2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTHCTL_EL2
#[inline]
pub fn write_cnthctl_el2(value: u64) {
    unsafe {
        asm!("msr cnthctl_el2, {}", in(reg) value, options(nomem, nostack));
    }
}

// ============================================================================
// CNTKCTL_EL1 Bits
// ============================================================================

/// Bit definitions for CNTKCTL_EL1
pub mod cntkctl {
    /// EL0 Physical counter access enable
    pub const EL0PCTEN: u64 = 1 << 0;
    /// EL0 Virtual counter access enable
    pub const EL0VCTEN: u64 = 1 << 1;
    /// Event stream generation enable
    pub const EVNTEN: u64 = 1 << 2;
    /// Event stream transition direction (0=low-to-high, 1=high-to-low)
    pub const EVNTDIR: u64 = 1 << 3;
    /// Event stream trigger bit position [7:4]
    pub const EVNTI_SHIFT: u64 = 4;
    pub const EVNTI_MASK: u64 = 0xF << EVNTI_SHIFT;
    /// EL0 Physical timer register access enable
    pub const EL0PTEN: u64 = 1 << 9;
    /// EL0 Virtual timer register access enable
    pub const EL0VTEN: u64 = 1 << 8;
}

// ============================================================================
// Timer Interface
// ============================================================================

/// Cached timer frequency
static TIMER_FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// Generic timer interface
pub struct Timer;

impl Timer {
    /// Get the timer frequency in Hz
    ///
    /// Reads CNTFRQ_EL0 and caches the value.
    /// Returns 0 if the frequency register is not set (error condition).
    pub fn frequency() -> u64 {
        let cached = TIMER_FREQUENCY.load(Ordering::Relaxed);
        if cached != 0 {
            return cached;
        }

        let freq = read_cntfrq_el0();
        if freq != 0 {
            TIMER_FREQUENCY.store(freq, Ordering::Relaxed);
        }
        freq
    }

    /// Read the physical counter value
    #[inline]
    pub fn counter() -> u64 {
        read_cntpct_el0()
    }

    /// Read the physical counter value (alias for clarity)
    #[inline]
    pub fn physical_counter() -> u64 {
        read_cntpct_el0()
    }

    /// Read the virtual counter value
    #[inline]
    pub fn virtual_counter() -> u64 {
        read_cntvct_el0()
    }

    /// Convert ticks to nanoseconds
    pub fn ticks_to_ns(ticks: u64) -> u64 {
        let freq = Self::frequency();
        if freq == 0 {
            return 0;
        }
        // Use 128-bit math to avoid overflow
        // result = (ticks * 1_000_000_000) / freq
        let (hi, lo) = Self::mul_u64(ticks, 1_000_000_000);
        Self::div_u128_u64(hi, lo, freq)
    }

    /// Convert nanoseconds to ticks
    pub fn ns_to_ticks(ns: u64) -> u64 {
        let freq = Self::frequency();
        if freq == 0 {
            return 0;
        }
        // result = (ns * freq) / 1_000_000_000
        let (hi, lo) = Self::mul_u64(ns, freq);
        Self::div_u128_u64(hi, lo, 1_000_000_000)
    }

    /// Convert ticks to microseconds
    pub fn ticks_to_us(ticks: u64) -> u64 {
        let freq = Self::frequency();
        if freq == 0 {
            return 0;
        }
        (ticks * 1_000_000) / freq
    }

    /// Convert microseconds to ticks
    pub fn us_to_ticks(us: u64) -> u64 {
        let freq = Self::frequency();
        (us * freq) / 1_000_000
    }

    /// Convert ticks to milliseconds
    pub fn ticks_to_ms(ticks: u64) -> u64 {
        let freq = Self::frequency();
        if freq == 0 {
            return 0;
        }
        (ticks * 1_000) / freq
    }

    /// Convert milliseconds to ticks
    pub fn ms_to_ticks(ms: u64) -> u64 {
        let freq = Self::frequency();
        (ms * freq) / 1_000
    }

    /// Multiply two u64 values and return 128-bit result as (hi, lo)
    #[inline]
    fn mul_u64(a: u64, b: u64) -> (u64, u64) {
        let result = (a as u128) * (b as u128);
        ((result >> 64) as u64, result as u64)
    }

    /// Divide 128-bit (hi, lo) by u64 divisor
    #[inline]
    fn div_u128_u64(hi: u64, lo: u64, divisor: u64) -> u64 {
        let dividend = ((hi as u128) << 64) | (lo as u128);
        (dividend / divisor as u128) as u64
    }

    /// Get time since boot in nanoseconds (requires boot_counter)
    pub fn uptime_ns(boot_counter: u64) -> u64 {
        let now = Self::counter();
        let elapsed = now.saturating_sub(boot_counter);
        Self::ticks_to_ns(elapsed)
    }

    /// Busy-wait for a number of nanoseconds
    ///
    /// # Warning
    /// This is a busy wait and will consume CPU. Use sparingly and only
    /// for short delays during early boot or interrupt-sensitive code.
    pub fn delay_ns(ns: u64) {
        let start = Self::counter();
        let target = start + Self::ns_to_ticks(ns);
        while Self::counter() < target {
            core::hint::spin_loop();
        }
    }

    /// Busy-wait for a number of microseconds
    pub fn delay_us(us: u64) {
        let start = Self::counter();
        let target = start + Self::us_to_ticks(us);
        while Self::counter() < target {
            core::hint::spin_loop();
        }
    }

    /// Busy-wait for a number of milliseconds
    pub fn delay_ms(ms: u64) {
        let start = Self::counter();
        let target = start + Self::ms_to_ticks(ms);
        while Self::counter() < target {
            core::hint::spin_loop();
        }
    }
}

// ============================================================================
// EL0 Access Control
// ============================================================================

/// Timer access control for EL0
pub struct TimerAccess;

impl TimerAccess {
    /// Enable EL0 access to physical counter
    pub fn enable_el0_physical_counter() {
        let mut ctl = read_cntkctl_el1();
        ctl |= cntkctl::EL0PCTEN;
        write_cntkctl_el1(ctl);
    }

    /// Disable EL0 access to physical counter
    pub fn disable_el0_physical_counter() {
        let mut ctl = read_cntkctl_el1();
        ctl &= !cntkctl::EL0PCTEN;
        write_cntkctl_el1(ctl);
    }

    /// Enable EL0 access to virtual counter
    pub fn enable_el0_virtual_counter() {
        let mut ctl = read_cntkctl_el1();
        ctl |= cntkctl::EL0VCTEN;
        write_cntkctl_el1(ctl);
    }

    /// Disable EL0 access to virtual counter
    pub fn disable_el0_virtual_counter() {
        let mut ctl = read_cntkctl_el1();
        ctl &= !cntkctl::EL0VCTEN;
        write_cntkctl_el1(ctl);
    }

    /// Enable EL0 access to physical timer registers
    pub fn enable_el0_physical_timer() {
        let mut ctl = read_cntkctl_el1();
        ctl |= cntkctl::EL0PTEN;
        write_cntkctl_el1(ctl);
    }

    /// Disable EL0 access to physical timer registers
    pub fn disable_el0_physical_timer() {
        let mut ctl = read_cntkctl_el1();
        ctl &= !cntkctl::EL0PTEN;
        write_cntkctl_el1(ctl);
    }

    /// Enable EL0 access to virtual timer registers
    pub fn enable_el0_virtual_timer() {
        let mut ctl = read_cntkctl_el1();
        ctl |= cntkctl::EL0VTEN;
        write_cntkctl_el1(ctl);
    }

    /// Disable EL0 access to virtual timer registers
    pub fn disable_el0_virtual_timer() {
        let mut ctl = read_cntkctl_el1();
        ctl &= !cntkctl::EL0VTEN;
        write_cntkctl_el1(ctl);
    }

    /// Configure standard kernel access (no EL0 access to timers)
    pub fn configure_kernel_only() {
        // Allow EL0 to read counters but not configure timers
        write_cntkctl_el1(cntkctl::EL0PCTEN | cntkctl::EL0VCTEN);
    }

    /// Configure userspace access (EL0 can read counters and virtual timer)
    pub fn configure_userspace_virtual() {
        write_cntkctl_el1(
            cntkctl::EL0PCTEN | cntkctl::EL0VCTEN | cntkctl::EL0VTEN
        );
    }
}

// ============================================================================
// Event Stream
// ============================================================================

/// Event stream configuration
///
/// The event stream generates periodic events that can be used for WFE-based
/// spinning with timeout.
pub struct EventStream;

impl EventStream {
    /// Enable event stream generation
    ///
    /// # Parameters
    /// - `bit`: Which counter bit to monitor (0-15). Higher bits = longer period.
    ///   - Bit 0: Every 2 counter ticks
    ///   - Bit 4: Every 32 counter ticks
    ///   - Bit 15: Every 65536 counter ticks
    /// - `high_to_low`: Event on high-to-low transition (else low-to-high)
    pub fn enable(bit: u8, high_to_low: bool) {
        let mut ctl = read_cntkctl_el1();
        ctl &= !cntkctl::EVNTI_MASK;
        ctl |= ((bit as u64) & 0xF) << cntkctl::EVNTI_SHIFT;
        if high_to_low {
            ctl |= cntkctl::EVNTDIR;
        } else {
            ctl &= !cntkctl::EVNTDIR;
        }
        ctl |= cntkctl::EVNTEN;
        write_cntkctl_el1(ctl);
    }

    /// Disable event stream
    pub fn disable() {
        let mut ctl = read_cntkctl_el1();
        ctl &= !cntkctl::EVNTEN;
        write_cntkctl_el1(ctl);
    }

    /// Check if event stream is enabled
    pub fn is_enabled() -> bool {
        (read_cntkctl_el1() & cntkctl::EVNTEN) != 0
    }
}

// ============================================================================
// High-Resolution Timestamp
// ============================================================================

/// High-resolution timestamp for profiling
#[derive(Debug, Clone, Copy)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Capture current timestamp
    #[inline]
    pub fn now() -> Self {
        Self(Timer::counter())
    }

    /// Get raw counter value
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Duration since this timestamp in ticks
    #[inline]
    pub fn elapsed_ticks(&self) -> u64 {
        Timer::counter().saturating_sub(self.0)
    }

    /// Duration since this timestamp in nanoseconds
    #[inline]
    pub fn elapsed_ns(&self) -> u64 {
        Timer::ticks_to_ns(self.elapsed_ticks())
    }

    /// Duration since this timestamp in microseconds
    #[inline]
    pub fn elapsed_us(&self) -> u64 {
        Timer::ticks_to_us(self.elapsed_ticks())
    }

    /// Duration since this timestamp in milliseconds
    #[inline]
    pub fn elapsed_ms(&self) -> u64 {
        Timer::ticks_to_ms(self.elapsed_ticks())
    }

    /// Time between two timestamps in ticks
    pub fn difference(&self, other: &Timestamp) -> u64 {
        if self.0 > other.0 {
            self.0 - other.0
        } else {
            other.0 - self.0
        }
    }
}

impl core::ops::Sub for Timestamp {
    type Output = u64;

    fn sub(self, rhs: Self) -> u64 {
        self.0.saturating_sub(rhs.0)
    }
}

// ============================================================================
// Frequency Detection
// ============================================================================

/// Known timer frequencies for common platforms
pub mod frequencies {
    /// ARM Juno / FVP (62.5 MHz)
    pub const ARM_JUNO: u64 = 62_500_000;

    /// Raspberry Pi 4 (54 MHz)
    pub const RPI4: u64 = 54_000_000;

    /// QEMU virt (62.5 MHz typically, but configurable)
    pub const QEMU_VIRT: u64 = 62_500_000;

    /// AWS Graviton (25 MHz)
    pub const AWS_GRAVITON: u64 = 25_000_000;

    /// Ampere Altra (25 MHz)
    pub const AMPERE_ALTRA: u64 = 25_000_000;

    /// Apple Silicon (24 MHz)
    pub const APPLE_M1: u64 = 24_000_000;
}

/// Detect and validate timer frequency
pub fn detect_frequency() -> Result<u64, super::TimerError> {
    let freq = read_cntfrq_el0();

    if freq == 0 {
        return Err(super::TimerError::FrequencyNotAvailable);
    }

    // Sanity check: frequency should be between 1 MHz and 1 GHz
    if freq < 1_000_000 || freq > 1_000_000_000 {
        // Unusual but might be valid, just return it
    }

    // Cache it
    TIMER_FREQUENCY.store(freq, Ordering::Relaxed);

    Ok(freq)
}

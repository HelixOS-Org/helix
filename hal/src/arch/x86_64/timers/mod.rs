//! # x86_64 Timer Framework
//!
//! This module provides comprehensive timer support for x86_64 systems,
//! including high-precision timing, calibration, and periodic interrupts.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         Timer Subsystem                                  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │   ┌─────────────────────────────────────────────────────────────────┐   │
//! │   │                    High-Level Timer API                          │   │
//! │   │  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────────┐          │   │
//! │   │  │ Delay   │  │ Timeout │  │ Deadline │  │ Periodic │          │   │
//! │   │  └─────────┘  └─────────┘  └──────────┘  └──────────┘          │   │
//! │   └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                           │
//! │   ┌─────────────────────────────────────────────────────────────────┐   │
//! │   │                    Timer Abstraction Layer                       │   │
//! │   │                                                                   │   │
//! │   │  Primary Clock Source    │    Event Timers                       │   │
//! │   │  ┌──────────────────┐    │    ┌──────────────────┐              │   │
//! │   │  │ TSC (preferred)  │    │    │ APIC Timer       │              │   │
//! │   │  │ HPET Counter     │    │    │ HPET Comparator  │              │   │
//! │   │  │ ACPI PM Timer    │    │    │ PIT              │              │   │
//! │   │  └──────────────────┘    │    └──────────────────┘              │   │
//! │   └─────────────────────────────────────────────────────────────────┘   │
//! │                              │                                           │
//! │   ┌─────────────────────────────────────────────────────────────────┐   │
//! │   │                    Hardware Drivers                              │   │
//! │   │                                                                   │   │
//! │   │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐ │   │
//! │   │  │    TSC     │  │   HPET     │  │ APIC Timer │  │    PIT     │ │   │
//! │   │  │            │  │            │  │            │  │            │ │   │
//! │   │  │ 64-bit     │  │ 64-bit     │  │ 32-bit     │  │ 16-bit     │ │   │
//! │   │  │ Per-CPU    │  │ Global     │  │ Per-CPU    │  │ Global     │ │   │
//! │   │  │ ~3GHz      │  │ ~25MHz     │  │ Variable   │  │ 1.193MHz   │ │   │
//! │   │  └────────────┘  └────────────┘  └────────────┘  └────────────┘ │   │
//! │   └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Timer Hierarchy
//!
//! 1. **TSC (Time Stamp Counter)** - Preferred for timing
//!    - Highest resolution (CPU frequency)
//!    - Per-CPU, requires synchronization for SMP
//!    - May not be invariant on older CPUs
//!
//! 2. **HPET (High Precision Event Timer)** - Fallback
//!    - Good resolution (~25 MHz typically)
//!    - Global, shared across all CPUs
//!    - Both counter and comparator functionality
//!
//! 3. **APIC Timer** - Per-CPU events
//!    - Local to each CPU
//!    - Periodic and one-shot modes
//!    - TSC-Deadline mode on modern CPUs
//!
//! 4. **PIT (Programmable Interval Timer)** - Legacy
//!    - Used for early boot calibration
//!    - Low resolution (1.193182 MHz)
//!    - Always available
//!
//! ## Features
//!
//! - Automatic timer source selection
//! - TSC calibration using multiple methods
//! - Invariant TSC detection
//! - Per-CPU timer management
//! - High-precision delays (nanosecond resolution)
//! - Periodic interrupt scheduling

#![allow(dead_code)]

pub mod apic_timer;
pub mod calibration;
pub mod hpet;
pub mod pit;
pub mod tsc;

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub use apic_timer::{ApicTimer, ApicTimerMode};
pub use calibration::{CalibrationMethod, CalibrationResult};
pub use hpet::{Hpet, HpetTimer};
pub use pit::{Pit, PitChannel};
pub use tsc::{Tsc, TscFeatures};

// =============================================================================
// Constants
// =============================================================================

/// Nanoseconds per second
pub const NS_PER_SEC: u64 = 1_000_000_000;

/// Nanoseconds per millisecond
pub const NS_PER_MS: u64 = 1_000_000;

/// Nanoseconds per microsecond
pub const NS_PER_US: u64 = 1_000;

/// PIT base frequency (1.193182 MHz)
pub const PIT_FREQUENCY: u64 = 1_193_182;

/// Default APIC timer frequency assumption (100 MHz for calibration)
pub const APIC_TIMER_DEFAULT_FREQ: u64 = 100_000_000;

// =============================================================================
// Timer Source Selection
// =============================================================================

/// Available timer sources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerSource {
    /// Time Stamp Counter
    Tsc,
    /// High Precision Event Timer
    Hpet,
    /// ACPI PM Timer
    AcpiPm,
    /// Programmable Interval Timer
    Pit,
}

/// Current primary clock source
static PRIMARY_CLOCK: AtomicU64 = AtomicU64::new(0); // 0 = TSC, 1 = HPET, etc.

/// Timer subsystem initialized
static TIMER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// TSC frequency in Hz
static TSC_FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// TSC invariant flag
static TSC_INVARIANT: AtomicBool = AtomicBool::new(false);

// =============================================================================
// Timer Initialization
// =============================================================================

/// Initialize the timer subsystem
///
/// This function:
/// 1. Detects available timer sources
/// 2. Calibrates the TSC
/// 3. Selects the best clock source
/// 4. Initializes per-CPU timers
///
/// # Safety
///
/// Must be called once during early boot after APIC initialization.
pub unsafe fn init() -> Result<(), TimerError> {
    if TIMER_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(TimerError::AlreadyInitialized);
    }

    // Initialize PIT for calibration
    pit::init();

    // Detect TSC features
    let tsc_features = tsc::detect_features();
    TSC_INVARIANT.store(tsc_features.invariant, Ordering::SeqCst);

    // Calibrate TSC
    let tsc_freq = if tsc_features.invariant {
        // Try to get TSC frequency from CPUID first
        if let Some(freq) = tsc::get_frequency_from_cpuid() {
            freq
        } else {
            // Fall back to PIT calibration
            calibration::calibrate_tsc_with_pit()?
        }
    } else {
        // Non-invariant TSC - still calibrate but mark it
        calibration::calibrate_tsc_with_pit()?
    };

    TSC_FREQUENCY.store(tsc_freq, Ordering::SeqCst);

    log::info!(
        "Timers: TSC calibrated at {} MHz (invariant={})",
        tsc_freq / 1_000_000,
        tsc_features.invariant
    );

    // Select primary clock source
    if tsc_features.invariant && tsc_freq > 0 {
        PRIMARY_CLOCK.store(TimerSource::Tsc as u64, Ordering::SeqCst);
        log::info!("Timers: Using TSC as primary clock source");
    } else {
        // Try HPET
        if hpet::is_available() {
            PRIMARY_CLOCK.store(TimerSource::Hpet as u64, Ordering::SeqCst);
            log::info!("Timers: Using HPET as primary clock source");
        } else {
            PRIMARY_CLOCK.store(TimerSource::Pit as u64, Ordering::SeqCst);
            log::warn!("Timers: Falling back to PIT (low precision)");
        }
    }

    Ok(())
}

/// Initialize per-CPU timer for an Application Processor
///
/// # Safety
///
/// Must be called after `init()` on the BSP.
pub unsafe fn init_for_ap() -> Result<(), TimerError> {
    if !TIMER_INITIALIZED.load(Ordering::Acquire) {
        return Err(TimerError::NotInitialized);
    }

    // Each AP needs its own APIC timer configured
    // The TSC should be synchronized on modern systems

    Ok(())
}

// =============================================================================
// Error Type
// =============================================================================

/// Timer error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerError {
    /// Timer subsystem not initialized
    NotInitialized,
    /// Timer subsystem already initialized
    AlreadyInitialized,
    /// Calibration failed
    CalibrationFailed,
    /// Timer not available
    NotAvailable,
    /// Invalid configuration
    InvalidConfiguration,
    /// Timeout expired
    Timeout,
}

impl core::fmt::Display for TimerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TimerError::NotInitialized => write!(f, "Timer subsystem not initialized"),
            TimerError::AlreadyInitialized => write!(f, "Timer subsystem already initialized"),
            TimerError::CalibrationFailed => write!(f, "Timer calibration failed"),
            TimerError::NotAvailable => write!(f, "Timer not available"),
            TimerError::InvalidConfiguration => write!(f, "Invalid timer configuration"),
            TimerError::Timeout => write!(f, "Timeout expired"),
        }
    }
}

// =============================================================================
// Public Interface
// =============================================================================

/// Get the current primary clock source
#[inline]
pub fn primary_clock_source() -> TimerSource {
    match PRIMARY_CLOCK.load(Ordering::Relaxed) {
        0 => TimerSource::Tsc,
        1 => TimerSource::Hpet,
        2 => TimerSource::AcpiPm,
        _ => TimerSource::Pit,
    }
}

/// Get the TSC frequency in Hz
#[inline]
pub fn tsc_frequency() -> u64 {
    TSC_FREQUENCY.load(Ordering::Relaxed)
}

/// Check if TSC is invariant
#[inline]
pub fn is_tsc_invariant() -> bool {
    TSC_INVARIANT.load(Ordering::Relaxed)
}

/// Read the current timestamp (nanoseconds since boot)
#[inline]
pub fn read_ns() -> u64 {
    match primary_clock_source() {
        TimerSource::Tsc => {
            let freq = tsc_frequency();
            if freq > 0 {
                let tsc = tsc::read();
                // Convert TSC to nanoseconds: tsc * NS_PER_SEC / freq
                // Use 128-bit math to avoid overflow
                ((tsc as u128 * NS_PER_SEC as u128) / freq as u128) as u64
            } else {
                0
            }
        },
        TimerSource::Hpet => hpet::read_ns(),
        TimerSource::Pit | TimerSource::AcpiPm => {
            // Low precision fallback
            0
        },
    }
}

/// Read the current timestamp (microseconds since boot)
#[inline]
pub fn read_us() -> u64 {
    read_ns() / NS_PER_US
}

/// Read the current timestamp (milliseconds since boot)
#[inline]
pub fn read_ms() -> u64 {
    read_ns() / NS_PER_MS
}

/// Delay for a specified number of nanoseconds
#[inline]
pub fn delay_ns(ns: u64) {
    let start = read_ns();
    while read_ns() - start < ns {
        core::hint::spin_loop();
    }
}

/// Delay for a specified number of microseconds
#[inline]
pub fn delay_us(us: u64) {
    delay_ns(us * NS_PER_US);
}

/// Delay for a specified number of milliseconds
#[inline]
pub fn delay_ms(ms: u64) {
    delay_ns(ms * NS_PER_MS);
}

/// Convert TSC ticks to nanoseconds
#[inline]
pub fn tsc_to_ns(ticks: u64) -> u64 {
    let freq = tsc_frequency();
    if freq > 0 {
        ((ticks as u128 * NS_PER_SEC as u128) / freq as u128) as u64
    } else {
        0
    }
}

/// Convert nanoseconds to TSC ticks
#[inline]
pub fn ns_to_tsc(ns: u64) -> u64 {
    let freq = tsc_frequency();
    if freq > 0 {
        ((ns as u128 * freq as u128) / NS_PER_SEC as u128) as u64
    } else {
        0
    }
}

// =============================================================================
// Deadline Support
// =============================================================================

/// A point in time (for deadline-based waiting)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    /// TSC value at this instant
    tsc: u64,
}

impl Instant {
    /// Get the current instant
    #[inline]
    pub fn now() -> Self {
        Self { tsc: tsc::read() }
    }

    /// Create an instant from a TSC value
    #[inline]
    pub const fn from_tsc(tsc: u64) -> Self {
        Self { tsc }
    }

    /// Get the TSC value
    #[inline]
    pub const fn tsc(&self) -> u64 {
        self.tsc
    }

    /// Calculate duration since another instant
    #[inline]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        Duration::from_ticks(self.tsc.saturating_sub(earlier.tsc))
    }

    /// Calculate elapsed time since this instant
    #[inline]
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(*self)
    }

    /// Add a duration to this instant
    #[inline]
    pub fn add_duration(&self, duration: Duration) -> Self {
        Self {
            tsc: self.tsc + duration.ticks(),
        }
    }

    /// Check if deadline has passed
    #[inline]
    pub fn has_passed(&self) -> bool {
        tsc::read() >= self.tsc
    }
}

/// A duration of time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    /// Duration in TSC ticks
    ticks: u64,
}

impl Duration {
    /// Zero duration
    pub const ZERO: Duration = Duration { ticks: 0 };

    /// Maximum duration
    pub const MAX: Duration = Duration { ticks: u64::MAX };

    /// Create a duration from TSC ticks
    #[inline]
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    /// Create a duration from nanoseconds
    #[inline]
    pub fn from_nanos(ns: u64) -> Self {
        Self {
            ticks: ns_to_tsc(ns),
        }
    }

    /// Create a duration from microseconds
    #[inline]
    pub fn from_micros(us: u64) -> Self {
        Self::from_nanos(us * NS_PER_US)
    }

    /// Create a duration from milliseconds
    #[inline]
    pub fn from_millis(ms: u64) -> Self {
        Self::from_nanos(ms * NS_PER_MS)
    }

    /// Create a duration from seconds
    #[inline]
    pub fn from_secs(secs: u64) -> Self {
        Self::from_nanos(secs * NS_PER_SEC)
    }

    /// Get duration in TSC ticks
    #[inline]
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Get duration in nanoseconds
    #[inline]
    pub fn as_nanos(&self) -> u64 {
        tsc_to_ns(self.ticks)
    }

    /// Get duration in microseconds
    #[inline]
    pub fn as_micros(&self) -> u64 {
        self.as_nanos() / NS_PER_US
    }

    /// Get duration in milliseconds
    #[inline]
    pub fn as_millis(&self) -> u64 {
        self.as_nanos() / NS_PER_MS
    }

    /// Get duration in seconds
    #[inline]
    pub fn as_secs(&self) -> u64 {
        self.as_nanos() / NS_PER_SEC
    }

    /// Check if duration is zero
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.ticks == 0
    }
}

impl core::ops::Add for Duration {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            ticks: self.ticks.saturating_add(rhs.ticks),
        }
    }
}

impl core::ops::Sub for Duration {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            ticks: self.ticks.saturating_sub(rhs.ticks),
        }
    }
}

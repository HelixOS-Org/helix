//! # AArch64 Timer Framework
//!
//! This module provides comprehensive support for the ARM Generic Timer architecture,
//! including counter access, physical and virtual timers.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │                        ARM Generic Timer Architecture                    │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │                       System Counter                                │ │
//! │  │                                                                     │ │
//! │  │   CNTFRQ_EL0: Counter frequency (read-only from EL0)               │ │
//! │  │   ┌──────────────────────────────────────────────────────────┐     │ │
//! │  │   │  64-bit monotonic counter @ configured frequency         │     │ │
//! │  │   │  (e.g., 62.5 MHz on ARM Juno, 54 MHz on RPi 4)          │     │ │
//! │  │   └──────────────────────────────────────────────────────────┘     │ │
//! │  └────────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │                    Physical Timer (CNTP_*)                         │ │
//! │  │                                                                     │ │
//! │  │   CNTP_CVAL_EL0  │ Compare value (64-bit absolute)                 │ │
//! │  │   CNTP_TVAL_EL0  │ Timer value (32-bit relative, signed)           │ │
//! │  │   CNTP_CTL_EL0   │ Control (Enable, IMASK, ISTATUS)               │ │
//! │  │                                                                     │ │
//! │  │   IRQ: PPI 30 (Non-secure EL1 Physical Timer)                     │ │
//! │  └────────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │                    Virtual Timer (CNTV_*)                          │ │
//! │  │                                                                     │ │
//! │  │   CNTV_CVAL_EL0  │ Compare value (64-bit absolute)                 │ │
//! │  │   CNTV_TVAL_EL0  │ Timer value (32-bit relative, signed)           │ │
//! │  │   CNTV_CTL_EL0   │ Control (Enable, IMASK, ISTATUS)               │ │
//! │  │   CNTVOFF_EL2    │ Virtual offset (set by hypervisor)             │ │
//! │  │                                                                     │ │
//! │  │   Virtual Count = Physical Count - CNTVOFF_EL2                    │ │
//! │  │   IRQ: PPI 27 (EL1 Virtual Timer)                                 │ │
//! │  └────────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! │  Timer IRQ Numbers (PPI):                                                │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │  ID   │ Timer                                                      │ │
//! │  │  29   │ EL1 Physical Timer (Secure)                               │ │
//! │  │  30   │ EL1 Physical Timer (Non-secure)                           │ │
//! │  │  27   │ EL1 Virtual Timer                                         │ │
//! │  │  26   │ EL2 Physical Timer                                        │ │
//! │  │  28   │ EL2 Virtual Timer (ARMv8.1-VHE)                           │ │
//! │  │  25   │ EL3 Physical Timer                                        │ │
//! │  └────────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage Examples
//!
//! ### Basic Timer Setup
//!
//! ```ignore
//! use crate::arch::aarch64::timers::{Timer, PhysicalTimer, VirtualTimer};
//!
//! // Initialize timer subsystem
//! let freq = Timer::frequency();
//! log!("Timer frequency: {} Hz", freq);
//!
//! // Read current counter
//! let count = Timer::counter();
//!
//! // Set up a one-shot timer (1ms from now)
//! let phys = PhysicalTimer::new();
//! phys.set_deadline(Timer::counter() + Timer::frequency() / 1000);
//! phys.enable();
//! ```

pub mod generic;
pub mod physical;
pub mod virtual_timer;

pub use generic::*;
pub use physical::*;
pub use virtual_timer::*;

// ============================================================================
// Timer Constants
// ============================================================================

/// Physical Timer PPI ID (Non-secure EL1)
pub const TIMER_PPI_PHYS_NS: u32 = 30;

/// Physical Timer PPI ID (Secure EL1)
pub const TIMER_PPI_PHYS_S: u32 = 29;

/// Virtual Timer PPI ID (EL1)
pub const TIMER_PPI_VIRT: u32 = 27;

/// Hypervisor Physical Timer PPI ID (EL2)
pub const TIMER_PPI_HYP_PHYS: u32 = 26;

/// Hypervisor Virtual Timer PPI ID (EL2, VHE)
pub const TIMER_PPI_HYP_VIRT: u32 = 28;

/// Secure Physical Timer PPI ID (EL3)
pub const TIMER_PPI_EL3_PHYS: u32 = 25;

// ============================================================================
// Common Timer Types
// ============================================================================

/// Timer error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerError {
    /// Timer frequency not available
    FrequencyNotAvailable,
    /// Invalid timer value
    InvalidValue,
    /// Timer already enabled
    AlreadyEnabled,
    /// Timer not enabled
    NotEnabled,
    /// Timer expired before configuration completed
    Expired,
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    /// Timer is disabled
    Disabled,
    /// Timer is enabled and waiting
    Enabled,
    /// Timer has fired (ISTATUS set)
    Fired,
    /// Timer is masked (IMASK set)
    Masked,
}

/// Timer type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    /// EL1 Non-secure Physical Timer
    PhysicalNonSecure,
    /// EL1 Secure Physical Timer
    PhysicalSecure,
    /// EL1 Virtual Timer
    Virtual,
    /// EL2 Physical Timer
    HypervisorPhysical,
    /// EL2 Virtual Timer
    HypervisorVirtual,
}

// ============================================================================
// Timer Operations Trait
// ============================================================================

/// Common timer operations
pub trait TimerOperations {
    /// Get timer control value
    fn control(&self) -> u64;

    /// Set timer control value
    fn set_control(&mut self, value: u64);

    /// Get timer compare value
    fn compare_value(&self) -> u64;

    /// Set timer compare value
    fn set_compare_value(&mut self, value: u64);

    /// Get timer value (countdown)
    fn timer_value(&self) -> i32;

    /// Set timer value (relative)
    fn set_timer_value(&mut self, value: i32);

    /// Enable the timer
    fn enable(&mut self) {
        let ctl = self.control();
        self.set_control((ctl & !0x2) | 0x1); // Clear IMASK, set ENABLE
    }

    /// Disable the timer
    fn disable(&mut self) {
        let ctl = self.control();
        self.set_control(ctl & !0x1); // Clear ENABLE
    }

    /// Mask the timer interrupt
    fn mask_interrupt(&mut self) {
        let ctl = self.control();
        self.set_control(ctl | 0x2); // Set IMASK
    }

    /// Unmask the timer interrupt
    fn unmask_interrupt(&mut self) {
        let ctl = self.control();
        self.set_control(ctl & !0x2); // Clear IMASK
    }

    /// Check if timer interrupt is pending
    fn is_pending(&self) -> bool {
        (self.control() & 0x4) != 0 // ISTATUS bit
    }

    /// Check if timer is enabled
    fn is_enabled(&self) -> bool {
        (self.control() & 0x1) != 0
    }

    /// Check if timer is masked
    fn is_masked(&self) -> bool {
        (self.control() & 0x2) != 0
    }

    /// Get timer state
    fn state(&self) -> TimerState {
        let ctl = self.control();
        if (ctl & 0x1) == 0 {
            TimerState::Disabled
        } else if (ctl & 0x2) != 0 {
            TimerState::Masked
        } else if (ctl & 0x4) != 0 {
            TimerState::Fired
        } else {
            TimerState::Enabled
        }
    }

    /// Set deadline (absolute counter value)
    fn set_deadline(&mut self, deadline: u64) {
        self.set_compare_value(deadline);
    }

    /// Set deadline from now (relative delay)
    fn set_delay(&mut self, ticks: u64) {
        let now = Timer::counter();
        self.set_compare_value(now + ticks);
    }

    /// Set deadline in nanoseconds from now
    fn set_delay_ns(&mut self, ns: u64) {
        let freq = Timer::frequency();
        let ticks = (ns * freq) / 1_000_000_000;
        self.set_delay(ticks);
    }

    /// Set deadline in microseconds from now
    fn set_delay_us(&mut self, us: u64) {
        let freq = Timer::frequency();
        let ticks = (us * freq) / 1_000_000;
        self.set_delay(ticks);
    }

    /// Set deadline in milliseconds from now
    fn set_delay_ms(&mut self, ms: u64) {
        let freq = Timer::frequency();
        let ticks = (ms * freq) / 1_000;
        self.set_delay(ticks);
    }
}

// ============================================================================
// Timer Initialization
// ============================================================================

/// Timer subsystem state
#[derive(Debug, Clone, Copy)]
pub struct TimerSubsystem {
    /// Timer frequency
    frequency: u64,
    /// Boot counter value
    boot_counter: u64,
    /// Preferred timer type
    preferred_timer: TimerType,
}

impl TimerSubsystem {
    /// Create timer subsystem (call once during boot)
    pub fn init() -> Self {
        let frequency = Timer::frequency();
        let boot_counter = Timer::counter();

        // Prefer virtual timer (works in VMs, at EL1 without HYP)
        let preferred_timer = TimerType::Virtual;

        Self {
            frequency,
            boot_counter,
            preferred_timer,
        }
    }

    /// Get timer frequency
    pub fn frequency(&self) -> u64 {
        self.frequency
    }

    /// Get boot counter value
    pub fn boot_counter(&self) -> u64 {
        self.boot_counter
    }

    /// Get time since boot in nanoseconds
    pub fn time_since_boot_ns(&self) -> u64 {
        let now = Timer::counter();
        let elapsed = now.saturating_sub(self.boot_counter);
        (elapsed * 1_000_000_000) / self.frequency
    }

    /// Get time since boot in microseconds
    pub fn time_since_boot_us(&self) -> u64 {
        let now = Timer::counter();
        let elapsed = now.saturating_sub(self.boot_counter);
        (elapsed * 1_000_000) / self.frequency
    }

    /// Get time since boot in milliseconds
    pub fn time_since_boot_ms(&self) -> u64 {
        let now = Timer::counter();
        let elapsed = now.saturating_sub(self.boot_counter);
        (elapsed * 1_000) / self.frequency
    }

    /// Convert ticks to nanoseconds
    pub fn ticks_to_ns(&self, ticks: u64) -> u64 {
        (ticks * 1_000_000_000) / self.frequency
    }

    /// Convert nanoseconds to ticks
    pub fn ns_to_ticks(&self, ns: u64) -> u64 {
        (ns * self.frequency) / 1_000_000_000
    }
}

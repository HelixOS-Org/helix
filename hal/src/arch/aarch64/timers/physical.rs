//! # Physical Timer (CNTP_*)
//!
//! This module provides access to the EL1 physical timer, which is commonly
//! used for kernel tick timers and scheduling.
//!
//! ## Register Set
//!
//! | Register      | Description                                        |
//! |---------------|----------------------------------------------------|
//! | CNTP_CTL_EL0  | Physical timer control                            |
//! | CNTP_CVAL_EL0 | Physical timer compare value (64-bit absolute)   |
//! | CNTP_TVAL_EL0 | Physical timer value (32-bit signed relative)    |
//!
//! ## Control Register (CNTP_CTL_EL0)
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │ 63                                    3     2      1      0         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │ RES0                                 │ISTATUS│IMASK│ENABLE│         │
//! │                                      │  (ro) │     │      │         │
//! └─────────────────────────────────────────────────────────────────────┘
//!
//! ENABLE   - Enable timer (1 = enabled)
//! IMASK    - Interrupt mask (1 = masked)
//! ISTATUS  - Interrupt status (1 = condition met, read-only)
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! // Set up a one-shot timer for 1ms
//! let mut timer = PhysicalTimer::new();
//! timer.set_delay_ms(1);
//! timer.enable();
//!
//! // In interrupt handler:
//! if timer.is_pending() {
//!     timer.disable(); // Or set new deadline for periodic
//!     // Handle tick...
//! }
//! ```

use core::arch::asm;

use super::{Timer, TimerOperations, TimerState};

// ============================================================================
// System Register Access
// ============================================================================

/// Read CNTP_CTL_EL0 (Physical Timer Control)
#[inline]
pub fn read_cntp_ctl_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntp_ctl_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTP_CTL_EL0
#[inline]
pub fn write_cntp_ctl_el0(value: u64) {
    unsafe {
        asm!("msr cntp_ctl_el0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTP_CVAL_EL0 (Physical Timer Compare Value)
#[inline]
pub fn read_cntp_cval_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntp_cval_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTP_CVAL_EL0
#[inline]
pub fn write_cntp_cval_el0(value: u64) {
    unsafe {
        asm!("msr cntp_cval_el0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTP_TVAL_EL0 (Physical Timer Value)
#[inline]
pub fn read_cntp_tval_el0() -> i32 {
    let value: i64;
    unsafe {
        asm!("mrs {}, cntp_tval_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value as i32
}

/// Write CNTP_TVAL_EL0
#[inline]
pub fn write_cntp_tval_el0(value: i32) {
    unsafe {
        asm!("msr cntp_tval_el0, {}", in(reg) value as i64, options(nomem, nostack));
    }
}

// ============================================================================
// Control Register Bits
// ============================================================================

/// Control register bit definitions
pub mod ctl {
    /// Timer enable bit
    pub const ENABLE: u64 = 1 << 0;
    /// Interrupt mask bit (1 = masked)
    pub const IMASK: u64 = 1 << 1;
    /// Interrupt status bit (read-only)
    pub const ISTATUS: u64 = 1 << 2;
}

// ============================================================================
// Physical Timer
// ============================================================================

/// EL1 Physical Timer (Non-secure)
///
/// This timer uses CNTP_* registers and generates PPI 30.
#[derive(Debug)]
pub struct PhysicalTimer {
    /// Cached frequency for calculations
    frequency: u64,
}

impl PhysicalTimer {
    /// Create a new physical timer instance
    pub fn new() -> Self {
        Self {
            frequency: Timer::frequency(),
        }
    }

    /// Initialize the physical timer
    ///
    /// Disables the timer and unmasks interrupts.
    pub fn init(&mut self) {
        // Disable timer, unmask interrupt
        write_cntp_ctl_el0(0);
    }

    /// Get the PPI interrupt number for this timer
    pub const fn irq_number() -> u32 {
        super::TIMER_PPI_PHYS_NS
    }

    /// Get remaining time in nanoseconds (or 0 if expired)
    pub fn remaining_ns(&self) -> u64 {
        let tval = read_cntp_tval_el0();
        if tval <= 0 {
            0
        } else {
            Timer::ticks_to_ns(tval as u64)
        }
    }

    /// Check if timer has expired
    pub fn has_expired(&self) -> bool {
        read_cntp_tval_el0() <= 0
    }

    /// Configure for periodic interrupts
    ///
    /// After each interrupt, call `reload_periodic()` to set the next deadline.
    pub fn configure_periodic(&mut self, period_ns: u64) {
        let period_ticks = Timer::ns_to_ticks(period_ns);
        write_cntp_tval_el0(period_ticks as i32);
    }

    /// Reload for next periodic interrupt
    pub fn reload_periodic(&mut self, period_ticks: u64) {
        // Use TVAL for relative reload
        write_cntp_tval_el0(period_ticks as i32);
    }
}

impl Default for PhysicalTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerOperations for PhysicalTimer {
    fn control(&self) -> u64 {
        read_cntp_ctl_el0()
    }

    fn set_control(&mut self, value: u64) {
        write_cntp_ctl_el0(value);
    }

    fn compare_value(&self) -> u64 {
        read_cntp_cval_el0()
    }

    fn set_compare_value(&mut self, value: u64) {
        write_cntp_cval_el0(value);
    }

    fn timer_value(&self) -> i32 {
        read_cntp_tval_el0()
    }

    fn set_timer_value(&mut self, value: i32) {
        write_cntp_tval_el0(value);
    }
}

// ============================================================================
// Secure Physical Timer
// ============================================================================

/// EL1 Physical Timer (Secure)
///
/// This timer uses CNTPS_* registers and generates PPI 29.
/// Only accessible from Secure world (EL3 or Secure EL1).
#[derive(Debug)]
pub struct SecurePhysicalTimer {
    frequency: u64,
}

/// Read CNTPS_CTL_EL1 (Secure Physical Timer Control)
#[inline]
pub fn read_cntps_ctl_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntps_ctl_el1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTPS_CTL_EL1
#[inline]
pub fn write_cntps_ctl_el1(value: u64) {
    unsafe {
        asm!("msr cntps_ctl_el1, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTPS_CVAL_EL1
#[inline]
pub fn read_cntps_cval_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntps_cval_el1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTPS_CVAL_EL1
#[inline]
pub fn write_cntps_cval_el1(value: u64) {
    unsafe {
        asm!("msr cntps_cval_el1, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTPS_TVAL_EL1
#[inline]
pub fn read_cntps_tval_el1() -> i32 {
    let value: i64;
    unsafe {
        asm!("mrs {}, cntps_tval_el1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value as i32
}

/// Write CNTPS_TVAL_EL1
#[inline]
pub fn write_cntps_tval_el1(value: i32) {
    unsafe {
        asm!("msr cntps_tval_el1, {}", in(reg) value as i64, options(nomem, nostack));
    }
}

impl SecurePhysicalTimer {
    /// Create a new secure physical timer instance
    ///
    /// # Safety
    /// Only call this when running in Secure world.
    pub unsafe fn new() -> Self {
        Self {
            frequency: Timer::frequency(),
        }
    }

    /// Get the PPI interrupt number for this timer
    pub const fn irq_number() -> u32 {
        super::TIMER_PPI_PHYS_S
    }

    /// Initialize the secure physical timer
    pub fn init(&mut self) {
        write_cntps_ctl_el1(0);
    }
}

impl TimerOperations for SecurePhysicalTimer {
    fn control(&self) -> u64 {
        read_cntps_ctl_el1()
    }

    fn set_control(&mut self, value: u64) {
        write_cntps_ctl_el1(value);
    }

    fn compare_value(&self) -> u64 {
        read_cntps_cval_el1()
    }

    fn set_compare_value(&mut self, value: u64) {
        write_cntps_cval_el1(value);
    }

    fn timer_value(&self) -> i32 {
        read_cntps_tval_el1()
    }

    fn set_timer_value(&mut self, value: i32) {
        write_cntps_tval_el1(value);
    }
}

// ============================================================================
// EL2 Physical Timer (Hypervisor)
// ============================================================================

/// EL2 Physical Timer
///
/// Uses CNTHP_* registers, generates PPI 26.
/// Only accessible at EL2.
#[derive(Debug)]
pub struct HypervisorPhysicalTimer {
    frequency: u64,
}

/// Read CNTHP_CTL_EL2
#[inline]
pub fn read_cnthp_ctl_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cnthp_ctl_el2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTHP_CTL_EL2
#[inline]
pub fn write_cnthp_ctl_el2(value: u64) {
    unsafe {
        asm!("msr cnthp_ctl_el2, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTHP_CVAL_EL2
#[inline]
pub fn read_cnthp_cval_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cnthp_cval_el2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTHP_CVAL_EL2
#[inline]
pub fn write_cnthp_cval_el2(value: u64) {
    unsafe {
        asm!("msr cnthp_cval_el2, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTHP_TVAL_EL2
#[inline]
pub fn read_cnthp_tval_el2() -> i32 {
    let value: i64;
    unsafe {
        asm!("mrs {}, cnthp_tval_el2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value as i32
}

/// Write CNTHP_TVAL_EL2
#[inline]
pub fn write_cnthp_tval_el2(value: i32) {
    unsafe {
        asm!("msr cnthp_tval_el2, {}", in(reg) value as i64, options(nomem, nostack));
    }
}

impl HypervisorPhysicalTimer {
    /// Create a new EL2 physical timer instance
    ///
    /// # Safety
    /// Only call this when running at EL2.
    pub unsafe fn new() -> Self {
        Self {
            frequency: Timer::frequency(),
        }
    }

    /// Get the PPI interrupt number for this timer
    pub const fn irq_number() -> u32 {
        super::TIMER_PPI_HYP_PHYS
    }

    /// Initialize the hypervisor physical timer
    pub fn init(&mut self) {
        write_cnthp_ctl_el2(0);
    }
}

impl TimerOperations for HypervisorPhysicalTimer {
    fn control(&self) -> u64 {
        read_cnthp_ctl_el2()
    }

    fn set_control(&mut self, value: u64) {
        write_cnthp_ctl_el2(value);
    }

    fn compare_value(&self) -> u64 {
        read_cnthp_cval_el2()
    }

    fn set_compare_value(&mut self, value: u64) {
        write_cnthp_cval_el2(value);
    }

    fn timer_value(&self) -> i32 {
        read_cnthp_tval_el2()
    }

    fn set_timer_value(&mut self, value: i32) {
        write_cnthp_tval_el2(value);
    }
}

// ============================================================================
// Timer Manager
// ============================================================================

/// Physical timer manager for high-level operations
pub struct PhysicalTimerManager {
    timer: PhysicalTimer,
    period_ticks: u64,
    is_periodic: bool,
}

impl PhysicalTimerManager {
    /// Create a new timer manager
    pub fn new() -> Self {
        Self {
            timer: PhysicalTimer::new(),
            period_ticks: 0,
            is_periodic: false,
        }
    }

    /// Set up a one-shot timer
    pub fn oneshot_ns(&mut self, delay_ns: u64) {
        self.is_periodic = false;
        self.timer.set_delay_ns(delay_ns);
        self.timer.enable();
    }

    /// Set up a one-shot timer in milliseconds
    pub fn oneshot_ms(&mut self, delay_ms: u64) {
        self.is_periodic = false;
        self.timer.set_delay_ms(delay_ms);
        self.timer.enable();
    }

    /// Set up a periodic timer
    pub fn periodic_ns(&mut self, period_ns: u64) {
        self.is_periodic = true;
        self.period_ticks = Timer::ns_to_ticks(period_ns);
        self.timer.set_delay(self.period_ticks);
        self.timer.enable();
    }

    /// Set up a periodic timer in milliseconds
    pub fn periodic_ms(&mut self, period_ms: u64) {
        self.is_periodic = true;
        self.period_ticks = Timer::ms_to_ticks(period_ms);
        self.timer.set_delay(self.period_ticks);
        self.timer.enable();
    }

    /// Handle timer interrupt
    ///
    /// Returns true if this was a valid timer interrupt.
    /// For periodic timers, automatically reloads.
    pub fn handle_interrupt(&mut self) -> bool {
        if !self.timer.is_pending() {
            return false;
        }

        if self.is_periodic {
            // Reload for next period
            self.timer.reload_periodic(self.period_ticks);
        } else {
            // One-shot: disable
            self.timer.disable();
        }

        true
    }

    /// Stop the timer
    pub fn stop(&mut self) {
        self.timer.disable();
    }

    /// Check if timer is running
    pub fn is_running(&self) -> bool {
        self.timer.is_enabled()
    }

    /// Get remaining time in nanoseconds
    pub fn remaining_ns(&self) -> u64 {
        self.timer.remaining_ns()
    }
}

impl Default for PhysicalTimerManager {
    fn default() -> Self {
        Self::new()
    }
}

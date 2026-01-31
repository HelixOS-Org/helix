//! # Virtual Timer (CNTV_*)
//!
//! This module provides access to the EL1 virtual timer, which is commonly
//! used in virtualized environments and provides VM-isolated timekeeping.
//!
//! ## Register Set
//!
//! | Register      | Description                                        |
//! |---------------|----------------------------------------------------|
//! | CNTV_CTL_EL0  | Virtual timer control                             |
//! | CNTV_CVAL_EL0 | Virtual timer compare value (64-bit absolute)    |
//! | CNTV_TVAL_EL0 | Virtual timer value (32-bit signed relative)     |
//!
//! ## Virtual Timer Behavior
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │                    Virtual Timer Architecture                         │
//! ├───────────────────────────────────────────────────────────────────────┤
//! │                                                                       │
//! │   Physical Counter (CNTPCT_EL0)                                       │
//! │   ┌─────────────────────────────────────────────────────────────┐    │
//! │   │  Always incrementing at system counter frequency            │    │
//! │   └─────────────────────────────────────────────────────────────┘    │
//! │                           │                                           │
//! │                           ▼                                           │
//! │   Virtual Offset (CNTVOFF_EL2)                                        │
//! │   ┌─────────────────────────────────────────────────────────────┐    │
//! │   │  Set by hypervisor, subtracted from physical count          │    │
//! │   │  Allows VMs to have isolated view of time                   │    │
//! │   └─────────────────────────────────────────────────────────────┘    │
//! │                           │                                           │
//! │                           ▼                                           │
//! │   Virtual Counter (CNTVCT_EL0) = CNTPCT_EL0 - CNTVOFF_EL2            │
//! │   ┌─────────────────────────────────────────────────────────────┐    │
//! │   │  VM-local view of time                                       │    │
//! │   │  Starts from 0 when VM is created (if CNTVOFF set)          │    │
//! │   └─────────────────────────────────────────────────────────────┘    │
//! │                           │                                           │
//! │                           ▼                                           │
//! │   Virtual Timer Compare (CNTV_CVAL_EL0)                              │
//! │   ┌─────────────────────────────────────────────────────────────┐    │
//! │   │  Timer fires when CNTVCT_EL0 >= CNTV_CVAL_EL0                │    │
//! │   │  PPI 27 is asserted                                          │    │
//! │   └─────────────────────────────────────────────────────────────┘    │
//! │                                                                       │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Advantages of Virtual Timer
//!
//! 1. Works correctly in VMs (uses virtual counter)
//! 2. Always accessible from EL1 (no traps to hypervisor)
//! 3. Hypervisor can migrate VMs without breaking timekeeping
//!
//! ## Usage
//!
//! ```ignore
//! // Virtual timer is preferred for most kernel uses
//! let mut timer = VirtualTimer::new();
//! timer.set_delay_ms(10);
//! timer.enable();
//! ```

use super::{Timer, TimerOperations, TimerState};
use core::arch::asm;

// ============================================================================
// System Register Access
// ============================================================================

/// Read CNTV_CTL_EL0 (Virtual Timer Control)
#[inline]
pub fn read_cntv_ctl_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntv_ctl_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTV_CTL_EL0
#[inline]
pub fn write_cntv_ctl_el0(value: u64) {
    unsafe {
        asm!("msr cntv_ctl_el0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTV_CVAL_EL0 (Virtual Timer Compare Value)
#[inline]
pub fn read_cntv_cval_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, cntv_cval_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTV_CVAL_EL0
#[inline]
pub fn write_cntv_cval_el0(value: u64) {
    unsafe {
        asm!("msr cntv_cval_el0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTV_TVAL_EL0 (Virtual Timer Value)
#[inline]
pub fn read_cntv_tval_el0() -> i32 {
    let value: i64;
    unsafe {
        asm!("mrs {}, cntv_tval_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value as i32
}

/// Write CNTV_TVAL_EL0
#[inline]
pub fn write_cntv_tval_el0(value: i32) {
    unsafe {
        asm!("msr cntv_tval_el0, {}", in(reg) value as i64, options(nomem, nostack));
    }
}

// ============================================================================
// Control Register Bits (same as physical timer)
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
// Virtual Timer
// ============================================================================

/// EL1 Virtual Timer
///
/// This timer uses CNTV_* registers and generates PPI 27.
/// Preferred for most kernel uses as it works correctly in VMs.
#[derive(Debug)]
pub struct VirtualTimer {
    /// Cached frequency for calculations
    frequency: u64,
}

impl VirtualTimer {
    /// Create a new virtual timer instance
    pub fn new() -> Self {
        Self {
            frequency: Timer::frequency(),
        }
    }

    /// Initialize the virtual timer
    ///
    /// Disables the timer and unmasks interrupts.
    pub fn init(&mut self) {
        // Disable timer, unmask interrupt
        write_cntv_ctl_el0(0);
    }

    /// Get the PPI interrupt number for this timer
    pub const fn irq_number() -> u32 {
        super::TIMER_PPI_VIRT
    }

    /// Get the current virtual counter value
    #[inline]
    pub fn counter(&self) -> u64 {
        super::generic::read_cntvct_el0()
    }

    /// Get remaining time in nanoseconds (or 0 if expired)
    pub fn remaining_ns(&self) -> u64 {
        let tval = read_cntv_tval_el0();
        if tval <= 0 {
            0
        } else {
            Timer::ticks_to_ns(tval as u64)
        }
    }

    /// Check if timer has expired
    pub fn has_expired(&self) -> bool {
        read_cntv_tval_el0() <= 0
    }

    /// Configure for periodic interrupts
    ///
    /// After each interrupt, call `reload_periodic()` to set the next deadline.
    pub fn configure_periodic(&mut self, period_ns: u64) {
        let period_ticks = Timer::ns_to_ticks(period_ns);
        write_cntv_tval_el0(period_ticks as i32);
    }

    /// Reload for next periodic interrupt
    pub fn reload_periodic(&mut self, period_ticks: u64) {
        // Use TVAL for relative reload
        write_cntv_tval_el0(period_ticks as i32);
    }
}

impl Default for VirtualTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerOperations for VirtualTimer {
    fn control(&self) -> u64 {
        read_cntv_ctl_el0()
    }

    fn set_control(&mut self, value: u64) {
        write_cntv_ctl_el0(value);
    }

    fn compare_value(&self) -> u64 {
        read_cntv_cval_el0()
    }

    fn set_compare_value(&mut self, value: u64) {
        write_cntv_cval_el0(value);
    }

    fn timer_value(&self) -> i32 {
        read_cntv_tval_el0()
    }

    fn set_timer_value(&mut self, value: i32) {
        write_cntv_tval_el0(value);
    }
}

// ============================================================================
// EL2 Virtual Timer (ARMv8.1-VHE)
// ============================================================================

/// EL2 Virtual Timer
///
/// Uses CNTHV_* registers, generates PPI 28.
/// Only accessible at EL2 with VHE enabled.
#[derive(Debug)]
pub struct HypervisorVirtualTimer {
    frequency: u64,
}

/// Read CNTHV_CTL_EL2
#[inline]
pub fn read_cnthv_ctl_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_4_C14_C3_1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTHV_CTL_EL2
#[inline]
pub fn write_cnthv_ctl_el2(value: u64) {
    unsafe {
        asm!("msr S3_4_C14_C3_1, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTHV_CVAL_EL2
#[inline]
pub fn read_cnthv_cval_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_4_C14_C3_2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write CNTHV_CVAL_EL2
#[inline]
pub fn write_cnthv_cval_el2(value: u64) {
    unsafe {
        asm!("msr S3_4_C14_C3_2, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read CNTHV_TVAL_EL2
#[inline]
pub fn read_cnthv_tval_el2() -> i32 {
    let value: i64;
    unsafe {
        asm!("mrs {}, S3_4_C14_C3_0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value as i32
}

/// Write CNTHV_TVAL_EL2
#[inline]
pub fn write_cnthv_tval_el2(value: i32) {
    unsafe {
        asm!("msr S3_4_C14_C3_0, {}", in(reg) value as i64, options(nomem, nostack));
    }
}

impl HypervisorVirtualTimer {
    /// Create a new EL2 virtual timer instance
    ///
    /// # Safety
    /// Only call this when running at EL2 with VHE enabled.
    pub unsafe fn new() -> Self {
        Self {
            frequency: Timer::frequency(),
        }
    }

    /// Get the PPI interrupt number for this timer
    pub const fn irq_number() -> u32 {
        super::TIMER_PPI_HYP_VIRT
    }

    /// Initialize the hypervisor virtual timer
    pub fn init(&mut self) {
        write_cnthv_ctl_el2(0);
    }
}

impl TimerOperations for HypervisorVirtualTimer {
    fn control(&self) -> u64 {
        read_cnthv_ctl_el2()
    }

    fn set_control(&mut self, value: u64) {
        write_cnthv_ctl_el2(value);
    }

    fn compare_value(&self) -> u64 {
        read_cnthv_cval_el2()
    }

    fn set_compare_value(&mut self, value: u64) {
        write_cnthv_cval_el2(value);
    }

    fn timer_value(&self) -> i32 {
        read_cnthv_tval_el2()
    }

    fn set_timer_value(&mut self, value: i32) {
        write_cnthv_tval_el2(value);
    }
}

// ============================================================================
// Virtual Timer Manager
// ============================================================================

/// Virtual timer manager for high-level operations
pub struct VirtualTimerManager {
    timer: VirtualTimer,
    period_ticks: u64,
    is_periodic: bool,
}

impl VirtualTimerManager {
    /// Create a new timer manager
    pub fn new() -> Self {
        Self {
            timer: VirtualTimer::new(),
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

    /// Set up periodic timer with Hz frequency
    pub fn periodic_hz(&mut self, hz: u64) {
        if hz == 0 {
            return;
        }
        let period_ns = 1_000_000_000 / hz;
        self.periodic_ns(period_ns);
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

    /// Get the underlying timer for direct access
    pub fn timer(&mut self) -> &mut VirtualTimer {
        &mut self.timer
    }
}

impl Default for VirtualTimerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Kernel Tick Timer
// ============================================================================

/// Kernel tick timer configuration
///
/// Standard tick frequencies:
/// - 100 Hz: 10ms ticks (traditional Unix)
/// - 250 Hz: 4ms ticks (common Linux default)
/// - 1000 Hz: 1ms ticks (low latency)
pub struct KernelTickTimer {
    manager: VirtualTimerManager,
    tick_count: u64,
    hz: u64,
}

impl KernelTickTimer {
    /// Create a new kernel tick timer
    pub fn new(hz: u64) -> Self {
        Self {
            manager: VirtualTimerManager::new(),
            tick_count: 0,
            hz,
        }
    }

    /// Create with 100 Hz (10ms ticks)
    pub fn hz_100() -> Self {
        Self::new(100)
    }

    /// Create with 250 Hz (4ms ticks)
    pub fn hz_250() -> Self {
        Self::new(250)
    }

    /// Create with 1000 Hz (1ms ticks)
    pub fn hz_1000() -> Self {
        Self::new(1000)
    }

    /// Start the tick timer
    pub fn start(&mut self) {
        self.tick_count = 0;
        self.manager.periodic_hz(self.hz);
    }

    /// Stop the tick timer
    pub fn stop(&mut self) {
        self.manager.stop();
    }

    /// Handle tick interrupt
    ///
    /// Returns true if a tick occurred, false for spurious interrupt.
    pub fn handle_tick(&mut self) -> bool {
        if self.manager.handle_interrupt() {
            self.tick_count += 1;
            true
        } else {
            false
        }
    }

    /// Get current tick count
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Get tick frequency in Hz
    pub fn frequency_hz(&self) -> u64 {
        self.hz
    }

    /// Get milliseconds since start
    pub fn uptime_ms(&self) -> u64 {
        (self.tick_count * 1000) / self.hz
    }

    /// Get seconds since start
    pub fn uptime_secs(&self) -> u64 {
        self.tick_count / self.hz
    }
}

// ============================================================================
// Watchdog-style Timer
// ============================================================================

/// Watchdog timer using virtual timer
///
/// Must be "pet" regularly or it will fire.
pub struct WatchdogTimer {
    timer: VirtualTimer,
    timeout_ticks: u64,
    enabled: bool,
}

impl WatchdogTimer {
    /// Create a new watchdog timer
    pub fn new(timeout_ms: u64) -> Self {
        let timeout_ticks = Timer::ms_to_ticks(timeout_ms);
        Self {
            timer: VirtualTimer::new(),
            timeout_ticks,
            enabled: false,
        }
    }

    /// Start the watchdog
    pub fn start(&mut self) {
        self.timer.set_delay(self.timeout_ticks);
        self.timer.enable();
        self.enabled = true;
    }

    /// Stop the watchdog
    pub fn stop(&mut self) {
        self.timer.disable();
        self.enabled = false;
    }

    /// Pet the watchdog (reset timeout)
    pub fn pet(&mut self) {
        if self.enabled {
            self.timer.set_delay(self.timeout_ticks);
        }
    }

    /// Check if watchdog has fired
    pub fn has_fired(&self) -> bool {
        self.enabled && self.timer.is_pending()
    }

    /// Change timeout (takes effect on next pet)
    pub fn set_timeout_ms(&mut self, timeout_ms: u64) {
        self.timeout_ticks = Timer::ms_to_ticks(timeout_ms);
    }
}

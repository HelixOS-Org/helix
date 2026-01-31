//! # AArch64 Generic Timer Driver
//!
//! Implements the ARM Generic Timer for timing and delays.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::BootResult;

// =============================================================================
// TIMER SYSTEM REGISTERS
// =============================================================================

/// Counter-timer Frequency register
pub const CNTFRQ_EL0: u32 = 0;
/// Counter-timer Physical Count register
pub const CNTPCT_EL0: u32 = 1;
/// Counter-timer Virtual Count register
pub const CNTVCT_EL0: u32 = 2;
/// Counter-timer Physical Timer Control register
pub const CNTP_CTL_EL0: u32 = 3;
/// Counter-timer Physical Timer CompareValue register
pub const CNTP_CVAL_EL0: u32 = 4;
/// Counter-timer Physical Timer TimerValue register
pub const CNTP_TVAL_EL0: u32 = 5;
/// Counter-timer Virtual Timer Control register
pub const CNTV_CTL_EL0: u32 = 6;
/// Counter-timer Virtual Timer CompareValue register
pub const CNTV_CVAL_EL0: u32 = 7;
/// Counter-timer Virtual Timer TimerValue register
pub const CNTV_TVAL_EL0: u32 = 8;

// =============================================================================
// TIMER CONTROL FLAGS
// =============================================================================

/// Timer enable
pub const CTL_ENABLE: u64 = 1 << 0;
/// Timer interrupt mask
pub const CTL_IMASK: u64 = 1 << 1;
/// Timer status (interrupt pending)
pub const CTL_ISTATUS: u64 = 1 << 2;

// =============================================================================
// MEMORY-MAPPED TIMER REGISTERS
// =============================================================================

/// CNTCTL frame base (common default)
pub const CNTCTL_BASE_DEFAULT: u64 = 0x2A430000;
/// CNTCTLBase offset: Physical counter control
pub const CNTCR: u64 = 0x000;
/// CNTCTLBase offset: Counter status
pub const CNTSR: u64 = 0x004;
/// CNTCTLBase offset: Counter value (low)
pub const CNTCVL: u64 = 0x008;
/// CNTCTLBase offset: Counter value (high)
pub const CNTCVU: u64 = 0x00C;
/// CNTCTLBase offset: Counter frequency
pub const CNTFID0: u64 = 0x020;

// =============================================================================
// TIMER STATE
// =============================================================================

/// Timer frequency in Hz
static TIMER_FREQ: AtomicU64 = AtomicU64::new(0);
/// Ticks per microsecond
static TICKS_PER_US: AtomicU64 = AtomicU64::new(0);
/// Ticks per nanosecond (fixed point, 16.16)
static TICKS_PER_NS_FP: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// SYSTEM REGISTER ACCESS
// =============================================================================

/// Read CNTFRQ_EL0
fn read_cntfrq() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTFRQ_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write CNTFRQ_EL0 (EL1 or higher)
fn write_cntfrq(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr CNTFRQ_EL0, {}",
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Read CNTPCT_EL0 (physical count)
pub fn read_cntpct() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTPCT_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Read CNTVCT_EL0 (virtual count)
pub fn read_cntvct() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTVCT_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Read CNTP_CTL_EL0
fn read_cntp_ctl() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTP_CTL_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write CNTP_CTL_EL0
fn write_cntp_ctl(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr CNTP_CTL_EL0, {}",
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Read CNTP_CVAL_EL0
fn read_cntp_cval() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTP_CVAL_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write CNTP_CVAL_EL0
fn write_cntp_cval(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr CNTP_CVAL_EL0, {}",
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Read CNTP_TVAL_EL0
fn read_cntp_tval() -> i32 {
    let value: i64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTP_TVAL_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value as i32
}

/// Write CNTP_TVAL_EL0
fn write_cntp_tval(value: i32) {
    unsafe {
        core::arch::asm!(
            "msr CNTP_TVAL_EL0, {}",
            in(reg) value as i64,
            options(nomem, nostack)
        );
    }
}

/// Read CNTV_CTL_EL0
fn read_cntv_ctl() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTV_CTL_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write CNTV_CTL_EL0
fn write_cntv_ctl(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr CNTV_CTL_EL0, {}",
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Read CNTV_CVAL_EL0
fn read_cntv_cval() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTV_CVAL_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write CNTV_CVAL_EL0
fn write_cntv_cval(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr CNTV_CVAL_EL0, {}",
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Read CNTV_TVAL_EL0
fn read_cntv_tval() -> i32 {
    let value: i64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTV_TVAL_EL0",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value as i32
}

/// Write CNTV_TVAL_EL0
fn write_cntv_tval(value: i32) {
    unsafe {
        core::arch::asm!(
            "msr CNTV_TVAL_EL0, {}",
            in(reg) value as i64,
            options(nomem, nostack)
        );
    }
}

// =============================================================================
// HIGHER EL REGISTERS
// =============================================================================

/// Read CNTHCTL_EL2
fn read_cnthctl_el2() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTHCTL_EL2",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write CNTHCTL_EL2
fn write_cnthctl_el2(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr CNTHCTL_EL2, {}",
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

/// Read CNTVOFF_EL2
fn read_cntvoff_el2() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, CNTVOFF_EL2",
            out(reg) value,
            options(nomem, nostack)
        );
    }
    value
}

/// Write CNTVOFF_EL2
fn write_cntvoff_el2(value: u64) {
    unsafe {
        core::arch::asm!(
            "msr CNTVOFF_EL2, {}",
            in(reg) value,
            options(nomem, nostack)
        );
    }
}

// =============================================================================
// TIMER INITIALIZATION
// =============================================================================

/// Initialize the generic timer
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Get current exception level
    let el = get_current_el();

    // If we're at EL2, configure timer access for EL1
    if el >= 2 {
        // Enable physical and virtual counter access from EL1/EL0
        let cnthctl = read_cnthctl_el2() | (1 << 0) | (1 << 1); // EL1PCTEN, EL1PCEN
        write_cnthctl_el2(cnthctl);

        // Clear virtual offset
        write_cntvoff_el2(0);
    }

    // Read timer frequency
    let freq = read_cntfrq();

    // If frequency is 0, try to read from memory-mapped register or use default
    let freq = if freq == 0 {
        // Try QEMU default (62.5 MHz)
        62_500_000
    } else {
        freq
    };

    TIMER_FREQ.store(freq, Ordering::SeqCst);

    // Calculate ticks per microsecond
    let ticks_per_us = freq / 1_000_000;
    TICKS_PER_US.store(ticks_per_us, Ordering::SeqCst);

    // Calculate ticks per nanosecond (fixed point 16.16)
    let ticks_per_ns_fp = (freq << 16) / 1_000_000_000;
    TICKS_PER_NS_FP.store(ticks_per_ns_fp, Ordering::SeqCst);

    // Disable physical timer interrupt initially
    write_cntp_ctl(CTL_IMASK);

    // Disable virtual timer interrupt initially
    write_cntv_ctl(CTL_IMASK);

    // Store timer info in context
    ctx.arch_data.arm.timer_frequency = freq;

    Ok(())
}

/// Initialize timer for AP (secondary CPU)
pub unsafe fn init_ap() {
    // Disable physical timer interrupt
    write_cntp_ctl(CTL_IMASK);

    // Disable virtual timer interrupt
    write_cntv_ctl(CTL_IMASK);
}

// =============================================================================
// TIME RETRIEVAL
// =============================================================================

/// Get current time in nanoseconds
pub fn get_time_ns() -> u64 {
    let count = read_cntpct();
    let freq = TIMER_FREQ.load(Ordering::SeqCst);
    if freq == 0 {
        return 0;
    }

    // Use 128-bit arithmetic to avoid overflow
    let ns = (count as u128 * 1_000_000_000) / (freq as u128);
    ns as u64
}

/// Get current time in microseconds
pub fn get_time_us() -> u64 {
    let count = read_cntpct();
    let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
    if ticks_per_us == 0 {
        return 0;
    }
    count / ticks_per_us
}

/// Get current time in milliseconds
pub fn get_time_ms() -> u64 {
    get_time_us() / 1000
}

/// Get raw counter value
pub fn get_counter() -> u64 {
    read_cntpct()
}

/// Get timer frequency
pub fn get_frequency() -> u64 {
    TIMER_FREQ.load(Ordering::SeqCst)
}

// =============================================================================
// DELAYS
// =============================================================================

/// Delay for specified nanoseconds
pub fn delay_ns(ns: u64) {
    let freq = TIMER_FREQ.load(Ordering::SeqCst);
    if freq == 0 {
        return;
    }

    // Calculate ticks to wait
    let ticks = (ns as u128 * freq as u128) / 1_000_000_000;
    let start = read_cntpct();
    let end = start + ticks as u64;

    while read_cntpct() < end {
        core::hint::spin_loop();
    }
}

/// Delay for specified microseconds
pub fn delay_us(us: u64) {
    let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
    if ticks_per_us == 0 {
        return;
    }

    let ticks = us * ticks_per_us;
    let start = read_cntpct();
    let end = start + ticks;

    while read_cntpct() < end {
        core::hint::spin_loop();
    }
}

/// Delay for specified milliseconds
pub fn delay_ms(ms: u64) {
    delay_us(ms * 1000);
}

/// Delay for specified seconds
pub fn delay_s(s: u64) {
    delay_ms(s * 1000);
}

// =============================================================================
// TIMER INTERRUPTS
// =============================================================================

/// Physical timer IRQ number
pub const PHYS_TIMER_IRQ: u32 = 30; // PPI 14
/// Virtual timer IRQ number
pub const VIRT_TIMER_IRQ: u32 = 27; // PPI 11
/// Hypervisor physical timer IRQ number
pub const HYP_TIMER_IRQ: u32 = 26; // PPI 10
/// Secure physical timer IRQ number
pub const SEC_TIMER_IRQ: u32 = 29; // PPI 13

/// Enable physical timer
pub fn enable_phys_timer() {
    let ctl = read_cntp_ctl();
    write_cntp_ctl((ctl & !CTL_IMASK) | CTL_ENABLE);
}

/// Disable physical timer
pub fn disable_phys_timer() {
    let ctl = read_cntp_ctl();
    write_cntp_ctl(ctl & !CTL_ENABLE);
}

/// Set physical timer compare value (absolute)
pub fn set_phys_timer_cval(cval: u64) {
    write_cntp_cval(cval);
}

/// Set physical timer countdown value (relative)
pub fn set_phys_timer_tval(tval: i32) {
    write_cntp_tval(tval);
}

/// Check if physical timer interrupt is pending
pub fn phys_timer_pending() -> bool {
    (read_cntp_ctl() & CTL_ISTATUS) != 0
}

/// Acknowledge physical timer interrupt
pub fn ack_phys_timer() {
    // Clear by writing TVAL or CVAL in the future
    set_phys_timer_tval(i32::MAX);
}

/// Enable virtual timer
pub fn enable_virt_timer() {
    let ctl = read_cntv_ctl();
    write_cntv_ctl((ctl & !CTL_IMASK) | CTL_ENABLE);
}

/// Disable virtual timer
pub fn disable_virt_timer() {
    let ctl = read_cntv_ctl();
    write_cntv_ctl(ctl & !CTL_ENABLE);
}

/// Set virtual timer compare value (absolute)
pub fn set_virt_timer_cval(cval: u64) {
    write_cntv_cval(cval);
}

/// Set virtual timer countdown value (relative)
pub fn set_virt_timer_tval(tval: i32) {
    write_cntv_tval(tval);
}

/// Check if virtual timer interrupt is pending
pub fn virt_timer_pending() -> bool {
    (read_cntv_ctl() & CTL_ISTATUS) != 0
}

/// Acknowledge virtual timer interrupt
pub fn ack_virt_timer() {
    set_virt_timer_tval(i32::MAX);
}

// =============================================================================
// ONE-SHOT TIMER
// =============================================================================

/// Schedule one-shot timer interrupt after specified microseconds
pub fn schedule_interrupt_us(us: u64) {
    let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
    if ticks_per_us == 0 {
        return;
    }

    let ticks = us * ticks_per_us;
    let current = read_cntpct();
    let target = current + ticks;

    write_cntp_cval(target);
    enable_phys_timer();
}

/// Schedule one-shot timer interrupt after specified milliseconds
pub fn schedule_interrupt_ms(ms: u64) {
    schedule_interrupt_us(ms * 1000);
}

// =============================================================================
// PERIODIC TIMER
// =============================================================================

/// Periodic timer state
struct PeriodicTimer {
    enabled: bool,
    period_ticks: u64,
    next_trigger: u64,
}

static mut PERIODIC_TIMER: PeriodicTimer = PeriodicTimer {
    enabled: false,
    period_ticks: 0,
    next_trigger: 0,
};

/// Start periodic timer with given period in microseconds
pub fn start_periodic_us(period_us: u64) {
    let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
    if ticks_per_us == 0 {
        return;
    }

    let period_ticks = period_us * ticks_per_us;
    let current = read_cntpct();
    let next = current + period_ticks;

    unsafe {
        PERIODIC_TIMER.enabled = true;
        PERIODIC_TIMER.period_ticks = period_ticks;
        PERIODIC_TIMER.next_trigger = next;
    }

    write_cntp_cval(next);
    enable_phys_timer();
}

/// Start periodic timer with given period in milliseconds
pub fn start_periodic_ms(period_ms: u64) {
    start_periodic_us(period_ms * 1000);
}

/// Handle periodic timer interrupt (call from IRQ handler)
pub fn handle_periodic_interrupt() {
    unsafe {
        if !PERIODIC_TIMER.enabled {
            return;
        }

        // Schedule next interrupt
        PERIODIC_TIMER.next_trigger += PERIODIC_TIMER.period_ticks;
        write_cntp_cval(PERIODIC_TIMER.next_trigger);
    }
}

/// Stop periodic timer
pub fn stop_periodic() {
    unsafe {
        PERIODIC_TIMER.enabled = false;
    }
    disable_phys_timer();
}

// =============================================================================
// TIMESTAMP
// =============================================================================

/// High-precision timestamp
#[derive(Clone, Copy, Debug)]
pub struct Timestamp {
    /// Raw counter value
    pub ticks: u64,
    /// Timer frequency
    pub frequency: u64,
}

impl Timestamp {
    /// Create new timestamp from current counter value
    pub fn now() -> Self {
        Self {
            ticks: read_cntpct(),
            frequency: TIMER_FREQ.load(Ordering::SeqCst),
        }
    }

    /// Get elapsed time since this timestamp in nanoseconds
    pub fn elapsed_ns(&self) -> u64 {
        let current = read_cntpct();
        let elapsed_ticks = current.saturating_sub(self.ticks);
        if self.frequency == 0 {
            return 0;
        }
        (elapsed_ticks as u128 * 1_000_000_000 / self.frequency as u128) as u64
    }

    /// Get elapsed time since this timestamp in microseconds
    pub fn elapsed_us(&self) -> u64 {
        self.elapsed_ns() / 1000
    }

    /// Get elapsed time since this timestamp in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_us() / 1000
    }

    /// Convert to nanoseconds since boot
    pub fn as_ns(&self) -> u64 {
        if self.frequency == 0 {
            return 0;
        }
        (self.ticks as u128 * 1_000_000_000 / self.frequency as u128) as u64
    }

    /// Convert to microseconds since boot
    pub fn as_us(&self) -> u64 {
        self.as_ns() / 1000
    }

    /// Convert to milliseconds since boot
    pub fn as_ms(&self) -> u64 {
        self.as_us() / 1000
    }
}

// =============================================================================
// WATCHDOG TIMER (Generic Timer based)
// =============================================================================

/// Simple software watchdog using generic timer
pub struct SoftwareWatchdog {
    timeout_ticks: u64,
    last_pet: u64,
}

impl SoftwareWatchdog {
    /// Create new watchdog with timeout in milliseconds
    pub fn new(timeout_ms: u64) -> Self {
        let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
        Self {
            timeout_ticks: timeout_ms * 1000 * ticks_per_us,
            last_pet: read_cntpct(),
        }
    }

    /// Pet (reset) the watchdog
    pub fn pet(&mut self) {
        self.last_pet = read_cntpct();
    }

    /// Check if watchdog has expired
    pub fn expired(&self) -> bool {
        let current = read_cntpct();
        current.saturating_sub(self.last_pet) > self.timeout_ticks
    }

    /// Get remaining time before timeout in milliseconds
    pub fn remaining_ms(&self) -> u64 {
        let current = read_cntpct();
        let elapsed = current.saturating_sub(self.last_pet);
        if elapsed >= self.timeout_ticks {
            0
        } else {
            let remaining_ticks = self.timeout_ticks - elapsed;
            let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
            if ticks_per_us == 0 {
                0
            } else {
                remaining_ticks / ticks_per_us / 1000
            }
        }
    }
}

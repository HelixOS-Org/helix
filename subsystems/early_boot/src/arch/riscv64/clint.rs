//! # RISC-V CLINT (Core Local Interruptor)
//!
//! Timer and software interrupt controller for RISC-V.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::BootResult;

// =============================================================================
// CLINT CONSTANTS
// =============================================================================

/// Default CLINT base address (QEMU virt)
pub const CLINT_BASE_DEFAULT: u64 = 0x0200_0000;

/// Timer frequency (typical, platform-specific)
pub const TIMER_FREQ_DEFAULT: u64 = 10_000_000; // 10 MHz

// =============================================================================
// CLINT REGISTER OFFSETS
// =============================================================================

/// Machine software interrupt pending (MSIP)
/// One 32-bit register per hart at offset hartid * 4
pub const CLINT_MSIP_BASE: u64 = 0x0000;

/// Machine timer compare (MTIMECMP)
/// One 64-bit register per hart at offset 0x4000 + hartid * 8
pub const CLINT_MTIMECMP_BASE: u64 = 0x4000;

/// Machine timer (MTIME)
/// Single 64-bit register at offset 0xBFF8
pub const CLINT_MTIME: u64 = 0xBFF8;

// =============================================================================
// CLINT STATE
// =============================================================================

/// CLINT base address
static CLINT_BASE: AtomicU64 = AtomicU64::new(CLINT_BASE_DEFAULT);
/// Timer frequency
static TIMER_FREQ: AtomicU64 = AtomicU64::new(TIMER_FREQ_DEFAULT);
/// Ticks per microsecond
static TICKS_PER_US: AtomicU64 = AtomicU64::new(10);

// =============================================================================
// REGISTER ACCESS
// =============================================================================

/// Read 32-bit CLINT register
#[inline]
unsafe fn clint_read32(offset: u64) -> u32 {
    let addr = CLINT_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::read_volatile(addr as *const u32)
}

/// Write 32-bit CLINT register
#[inline]
unsafe fn clint_write32(offset: u64, value: u32) {
    let addr = CLINT_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::write_volatile(addr as *mut u32, value);
}

/// Read 64-bit CLINT register
#[inline]
unsafe fn clint_read64(offset: u64) -> u64 {
    let addr = CLINT_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::read_volatile(addr as *const u64)
}

/// Write 64-bit CLINT register
#[inline]
unsafe fn clint_write64(offset: u64, value: u64) {
    let addr = CLINT_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::write_volatile(addr as *mut u64, value);
}

// =============================================================================
// TIMER FUNCTIONS
// =============================================================================

/// Get current time value
pub fn get_mtime() -> u64 {
    unsafe { clint_read64(CLINT_MTIME) }
}

/// Set time value (requires M-mode)
pub unsafe fn set_mtime(value: u64) {
    clint_write64(CLINT_MTIME, value);
}

/// Get timer compare value for hart
pub fn get_mtimecmp(hartid: u64) -> u64 {
    unsafe { clint_read64(CLINT_MTIMECMP_BASE + hartid * 8) }
}

/// Set timer compare value for hart
pub unsafe fn set_mtimecmp(hartid: u64, value: u64) {
    clint_write64(CLINT_MTIMECMP_BASE + hartid * 8, value);
}

/// Set timer to fire after specified ticks
pub unsafe fn set_timer_relative(hartid: u64, ticks: u64) {
    let current = get_mtime();
    set_mtimecmp(hartid, current.wrapping_add(ticks));
}

/// Disable timer interrupt for hart
pub unsafe fn disable_timer(hartid: u64) {
    // Set compare to max value to effectively disable
    set_mtimecmp(hartid, u64::MAX);
}

/// Clear timer interrupt by setting new compare value
pub unsafe fn clear_timer_interrupt(hartid: u64) {
    // Must set mtimecmp > mtime to clear interrupt
    let current = get_mtime();
    set_mtimecmp(
        hartid,
        current.wrapping_add(TIMER_FREQ.load(Ordering::SeqCst)),
    );
}

// =============================================================================
// SOFTWARE INTERRUPT FUNCTIONS
// =============================================================================

/// Get software interrupt pending status for hart
pub fn get_msip(hartid: u64) -> bool {
    unsafe { clint_read32(CLINT_MSIP_BASE + hartid * 4) != 0 }
}

/// Set software interrupt pending for hart (send IPI)
pub unsafe fn set_msip(hartid: u64) {
    clint_write32(CLINT_MSIP_BASE + hartid * 4, 1);
}

/// Clear software interrupt pending for hart
pub unsafe fn clear_msip(hartid: u64) {
    clint_write32(CLINT_MSIP_BASE + hartid * 4, 0);
}

/// Send IPI to hart
pub unsafe fn send_ipi(hartid: u64) {
    set_msip(hartid);
}

/// Clear IPI for current hart
pub unsafe fn clear_ipi() {
    let hartid = read_mhartid();
    clear_msip(hartid);
}

// =============================================================================
// TIME FUNCTIONS
// =============================================================================

/// Get current time in nanoseconds
pub fn get_time_ns() -> u64 {
    let mtime = get_mtime();
    let freq = TIMER_FREQ.load(Ordering::SeqCst);
    if freq == 0 {
        return 0;
    }
    (mtime as u128 * 1_000_000_000 / freq as u128) as u64
}

/// Get current time in microseconds
pub fn get_time_us() -> u64 {
    let mtime = get_mtime();
    let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
    if ticks_per_us == 0 {
        return 0;
    }
    mtime / ticks_per_us
}

/// Get current time in milliseconds
pub fn get_time_ms() -> u64 {
    get_time_us() / 1000
}

/// Get timer frequency
pub fn get_frequency() -> u64 {
    TIMER_FREQ.load(Ordering::SeqCst)
}

// =============================================================================
// DELAY FUNCTIONS
// =============================================================================

/// Delay for specified ticks
pub fn delay_ticks(ticks: u64) {
    let start = get_mtime();
    let end = start.wrapping_add(ticks);

    if end > start {
        while get_mtime() < end {
            core::hint::spin_loop();
        }
    } else {
        // Handle wraparound
        while get_mtime() >= start {
            core::hint::spin_loop();
        }
        while get_mtime() < end {
            core::hint::spin_loop();
        }
    }
}

/// Delay for specified nanoseconds
pub fn delay_ns(ns: u64) {
    let freq = TIMER_FREQ.load(Ordering::SeqCst);
    let ticks = (ns as u128 * freq as u128 / 1_000_000_000) as u64;
    delay_ticks(ticks);
}

/// Delay for specified microseconds
pub fn delay_us(us: u64) {
    let ticks = us * TICKS_PER_US.load(Ordering::SeqCst);
    delay_ticks(ticks);
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
// PERIODIC TIMER
// =============================================================================

/// Periodic timer state
struct PeriodicTimer {
    enabled: bool,
    period_ticks: u64,
    hartid: u64,
}

static mut PERIODIC_TIMERS: [PeriodicTimer; 16] =
    unsafe { core::mem::MaybeUninit::zeroed().assume_init() };

/// Start periodic timer for hart
pub unsafe fn start_periodic(hartid: u64, period_us: u64) {
    if hartid >= 16 {
        return;
    }

    let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
    let period_ticks = period_us * ticks_per_us;

    PERIODIC_TIMERS[hartid as usize] = PeriodicTimer {
        enabled: true,
        period_ticks,
        hartid,
    };

    // Set first timer
    set_timer_relative(hartid, period_ticks);

    // Enable timer interrupt
    let mie = read_mie();
    write_mie(mie | MIE_MTIE);
}

/// Handle periodic timer interrupt
pub unsafe fn handle_periodic(hartid: u64) {
    if hartid >= 16 {
        return;
    }

    let timer = &PERIODIC_TIMERS[hartid as usize];
    if timer.enabled {
        // Schedule next interrupt
        let current = get_mtimecmp(hartid);
        set_mtimecmp(hartid, current.wrapping_add(timer.period_ticks));
    }
}

/// Stop periodic timer for hart
pub unsafe fn stop_periodic(hartid: u64) {
    if hartid >= 16 {
        return;
    }

    PERIODIC_TIMERS[hartid as usize].enabled = false;
    disable_timer(hartid);
}

// =============================================================================
// ONE-SHOT TIMER
// =============================================================================

/// Schedule one-shot timer interrupt after microseconds
pub unsafe fn schedule_oneshot_us(hartid: u64, us: u64) {
    let ticks = us * TICKS_PER_US.load(Ordering::SeqCst);
    set_timer_relative(hartid, ticks);

    // Enable timer interrupt
    let mie = read_mie();
    write_mie(mie | MIE_MTIE);
}

/// Schedule one-shot timer interrupt after milliseconds
pub unsafe fn schedule_oneshot_ms(hartid: u64, ms: u64) {
    schedule_oneshot_us(hartid, ms * 1000);
}

// =============================================================================
// TIMESTAMP
// =============================================================================

/// High-precision timestamp
#[derive(Clone, Copy, Debug)]
pub struct Timestamp {
    pub ticks: u64,
    pub frequency: u64,
}

impl Timestamp {
    /// Get current timestamp
    pub fn now() -> Self {
        Self {
            ticks: get_mtime(),
            frequency: TIMER_FREQ.load(Ordering::SeqCst),
        }
    }

    /// Get elapsed nanoseconds since timestamp
    pub fn elapsed_ns(&self) -> u64 {
        let current = get_mtime();
        let elapsed = current.wrapping_sub(self.ticks);
        if self.frequency == 0 {
            return 0;
        }
        (elapsed as u128 * 1_000_000_000 / self.frequency as u128) as u64
    }

    /// Get elapsed microseconds since timestamp
    pub fn elapsed_us(&self) -> u64 {
        self.elapsed_ns() / 1000
    }

    /// Get elapsed milliseconds since timestamp
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_us() / 1000
    }

    /// Convert timestamp to nanoseconds since boot
    pub fn as_ns(&self) -> u64 {
        if self.frequency == 0 {
            return 0;
        }
        (self.ticks as u128 * 1_000_000_000 / self.frequency as u128) as u64
    }
}

// =============================================================================
// WATCHDOG (Software)
// =============================================================================

/// Simple software watchdog
pub struct Watchdog {
    timeout_ticks: u64,
    last_pet: u64,
}

impl Watchdog {
    /// Create new watchdog with timeout in milliseconds
    pub fn new(timeout_ms: u64) -> Self {
        let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
        Self {
            timeout_ticks: timeout_ms * 1000 * ticks_per_us,
            last_pet: get_mtime(),
        }
    }

    /// Pet (reset) the watchdog
    pub fn pet(&mut self) {
        self.last_pet = get_mtime();
    }

    /// Check if watchdog has expired
    pub fn expired(&self) -> bool {
        let current = get_mtime();
        current.wrapping_sub(self.last_pet) > self.timeout_ticks
    }

    /// Get remaining time before timeout in milliseconds
    pub fn remaining_ms(&self) -> u64 {
        let current = get_mtime();
        let elapsed = current.wrapping_sub(self.last_pet);
        if elapsed >= self.timeout_ticks {
            0
        } else {
            let remaining = self.timeout_ticks - elapsed;
            let ticks_per_us = TICKS_PER_US.load(Ordering::SeqCst);
            if ticks_per_us == 0 {
                0
            } else {
                remaining / ticks_per_us / 1000
            }
        }
    }
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize CLINT
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Get CLINT base from device tree or use default
    let base = if let Some(ref dt_info) = ctx.boot_info.device_tree {
        // TODO: Parse device tree
        CLINT_BASE_DEFAULT
    } else {
        CLINT_BASE_DEFAULT
    };

    CLINT_BASE.store(base, Ordering::SeqCst);

    // Get timer frequency (may come from device tree or SBI)
    let freq = if ctx.arch_data.riscv.timer_frequency != 0 {
        ctx.arch_data.riscv.timer_frequency
    } else {
        TIMER_FREQ_DEFAULT
    };

    TIMER_FREQ.store(freq, Ordering::SeqCst);
    TICKS_PER_US.store(freq / 1_000_000, Ordering::SeqCst);

    // Disable timer interrupt for boot hart
    let hartid = read_mhartid();
    disable_timer(hartid);

    // Clear any pending software interrupt
    clear_msip(hartid);

    // Store CLINT info
    ctx.arch_data.riscv.clint_base = base;
    ctx.arch_data.riscv.timer_frequency = freq;

    Ok(())
}

/// Initialize CLINT for secondary hart
pub unsafe fn init_secondary(hartid: u64) {
    // Disable timer interrupt
    disable_timer(hartid);

    // Clear software interrupt
    clear_msip(hartid);
}

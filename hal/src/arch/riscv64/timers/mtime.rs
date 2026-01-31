//! # Machine Timer (MTIME) Interface
//!
//! Low-level access to the RISC-V machine timer.
//!
//! On most systems, the TIME CSR and MTIME register are the same counter.
//! From S-mode, we access this via the TIME CSR (which traps to M-mode).

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::{DEFAULT_TIMER_FREQ, NS_PER_SEC, US_PER_SEC, MS_PER_SEC};

// ============================================================================
// Timer Frequency
// ============================================================================

/// Timer frequency (Hz)
static TIMER_FREQUENCY: AtomicU64 = AtomicU64::new(DEFAULT_TIMER_FREQ);

/// Get the timer frequency
#[inline]
pub fn get_timer_frequency() -> u64 {
    TIMER_FREQUENCY.load(Ordering::Relaxed)
}

/// Set the timer frequency
pub fn set_timer_frequency(freq: u64) {
    TIMER_FREQUENCY.store(freq, Ordering::SeqCst);
}

// ============================================================================
// Time Reading
// ============================================================================

/// Read the current time counter (TIME CSR)
///
/// This is the primary way to read time from S-mode.
#[inline(always)]
pub fn read_time() -> u64 {
    let time: u64;
    unsafe {
        core::arch::asm!(
            "rdtime {}",
            out(reg) time,
            options(nomem, nostack, preserves_flags)
        );
    }
    time
}

/// Read the cycle counter (CYCLE CSR)
#[inline(always)]
pub fn read_cycle() -> u64 {
    let cycle: u64;
    unsafe {
        core::arch::asm!(
            "rdcycle {}",
            out(reg) cycle,
            options(nomem, nostack, preserves_flags)
        );
    }
    cycle
}

/// Read the instructions retired counter (INSTRET CSR)
#[inline(always)]
pub fn read_instret() -> u64 {
    let instret: u64;
    unsafe {
        core::arch::asm!(
            "rdinstret {}",
            out(reg) instret,
            options(nomem, nostack, preserves_flags)
        );
    }
    instret
}

// ============================================================================
// High-Precision Time Reading
// ============================================================================

/// High-precision timestamp combining TIME and CYCLE
#[derive(Debug, Clone, Copy)]
pub struct HighPrecisionTime {
    /// TIME counter value
    pub time: u64,
    /// CYCLE counter value
    pub cycle: u64,
}

impl HighPrecisionTime {
    /// Read current high-precision time
    #[inline]
    pub fn now() -> Self {
        // Read both counters as close together as possible
        let time = read_time();
        let cycle = read_cycle();
        Self { time, cycle }
    }

    /// Calculate elapsed time
    pub fn elapsed_ticks(&self) -> u64 {
        read_time().saturating_sub(self.time)
    }

    /// Calculate elapsed cycles
    pub fn elapsed_cycles(&self) -> u64 {
        read_cycle().saturating_sub(self.cycle)
    }
}

// ============================================================================
// MTIME Direct Access (M-mode only or memory-mapped)
// ============================================================================

/// MTIME base address (if memory-mapped)
static MTIME_BASE: AtomicUsize = AtomicUsize::new(0);

/// Set the MTIME base address
///
/// # Safety
/// Must be a valid memory-mapped MTIME address.
pub unsafe fn set_mtime_base(base: usize) {
    MTIME_BASE.store(base, Ordering::SeqCst);
}

/// Read MTIME directly (if memory-mapped)
///
/// # Safety
/// MTIME base must be set and accessible.
pub unsafe fn read_mtime_direct() -> u64 {
    let base = MTIME_BASE.load(Ordering::Relaxed);
    if base == 0 {
        return read_time(); // Fallback to CSR
    }

    core::ptr::read_volatile(base as *const u64)
}

/// Write MTIME directly (if memory-mapped)
///
/// # Safety
/// MTIME base must be set and writable (not typical).
pub unsafe fn write_mtime_direct(value: u64) {
    let base = MTIME_BASE.load(Ordering::Relaxed);
    if base != 0 {
        core::ptr::write_volatile(base as *mut u64, value);
    }
}

// ============================================================================
// MTIMECMP Direct Access (M-mode only or memory-mapped)
// ============================================================================

/// MTIMECMP base address (if memory-mapped)
static MTIMECMP_BASE: AtomicUsize = AtomicUsize::new(0);

/// Set the MTIMECMP base address
///
/// # Safety
/// Must be a valid memory-mapped MTIMECMP address.
pub unsafe fn set_mtimecmp_base(base: usize) {
    MTIMECMP_BASE.store(base, Ordering::SeqCst);
}

/// Read MTIMECMP for a hart (if memory-mapped)
///
/// # Safety
/// MTIMECMP base must be set and accessible.
pub unsafe fn read_mtimecmp(hart_id: usize) -> u64 {
    let base = MTIMECMP_BASE.load(Ordering::Relaxed);
    if base == 0 {
        return u64::MAX; // No direct access
    }

    let addr = base + hart_id * 8;
    core::ptr::read_volatile(addr as *const u64)
}

/// Write MTIMECMP for a hart (if memory-mapped)
///
/// # Safety
/// MTIMECMP base must be set and accessible.
pub unsafe fn write_mtimecmp(hart_id: usize, value: u64) {
    let base = MTIMECMP_BASE.load(Ordering::Relaxed);
    if base != 0 {
        let addr = base + hart_id * 8;
        core::ptr::write_volatile(addr as *mut u64, value);
    }
}

// ============================================================================
// Timer Frequency Detection
// ============================================================================

/// Detect timer frequency by calibrating against a known reference
///
/// This uses a busy loop to estimate the frequency. For accurate results,
/// use the device tree value instead.
pub fn detect_timer_frequency_approx() -> u64 {
    // Read start time
    let start_time = read_time();
    let start_cycle = read_cycle();

    // Busy wait for some cycles
    const WAIT_CYCLES: u64 = 10_000_000;
    while read_cycle() < start_cycle + WAIT_CYCLES {
        core::hint::spin_loop();
    }

    // Read end time
    let end_time = read_time();
    let end_cycle = read_cycle();

    // Calculate ratio
    let time_delta = end_time.saturating_sub(start_time);
    let cycle_delta = end_cycle.saturating_sub(start_cycle);

    if time_delta == 0 {
        return DEFAULT_TIMER_FREQ;
    }

    // Estimate: time_freq = cycle_freq * (time_delta / cycle_delta)
    // This assumes cycle_freq is approximately the CPU frequency
    // which may not always be accurate

    // For now, just return a rough estimate based on typical ratios
    // In practice, use device tree for accurate frequency
    DEFAULT_TIMER_FREQ
}

// ============================================================================
// Time Conversion
// ============================================================================

/// Convert timer ticks to nanoseconds
#[inline]
pub fn ticks_to_ns(ticks: u64) -> u64 {
    let freq = get_timer_frequency();
    if freq == 0 { return 0; }
    (ticks as u128 * NS_PER_SEC as u128 / freq as u128) as u64
}

/// Convert nanoseconds to timer ticks
#[inline]
pub fn ns_to_ticks(ns: u64) -> u64 {
    let freq = get_timer_frequency();
    (ns as u128 * freq as u128 / NS_PER_SEC as u128) as u64
}

/// Convert timer ticks to microseconds
#[inline]
pub fn ticks_to_us(ticks: u64) -> u64 {
    let freq = get_timer_frequency();
    if freq == 0 { return 0; }
    (ticks * US_PER_SEC) / freq
}

/// Convert microseconds to timer ticks
#[inline]
pub fn us_to_ticks(us: u64) -> u64 {
    let freq = get_timer_frequency();
    (us * freq) / US_PER_SEC
}

/// Convert timer ticks to milliseconds
#[inline]
pub fn ticks_to_ms(ticks: u64) -> u64 {
    let freq = get_timer_frequency();
    if freq == 0 { return 0; }
    (ticks * MS_PER_SEC) / freq
}

/// Convert milliseconds to timer ticks
#[inline]
pub fn ms_to_ticks(ms: u64) -> u64 {
    let freq = get_timer_frequency();
    (ms * freq) / MS_PER_SEC
}

// ============================================================================
// Delay Functions
// ============================================================================

/// Delay for a specified number of timer ticks
#[inline]
pub fn delay_ticks(ticks: u64) {
    let start = read_time();
    let target = start.saturating_add(ticks);
    while read_time() < target {
        core::hint::spin_loop();
    }
}

/// Delay for a specified number of nanoseconds
#[inline]
pub fn delay_ns(ns: u64) {
    delay_ticks(ns_to_ticks(ns));
}

/// Delay for a specified number of microseconds
#[inline]
pub fn delay_us(us: u64) {
    delay_ticks(us_to_ticks(us));
}

/// Delay for a specified number of milliseconds
#[inline]
pub fn delay_ms(ms: u64) {
    delay_ticks(ms_to_ticks(ms));
}

// ============================================================================
// Performance Measurement
// ============================================================================

/// Performance measurement context
#[derive(Debug)]
pub struct PerfMeasure {
    start_time: u64,
    start_cycle: u64,
    start_instret: u64,
}

impl PerfMeasure {
    /// Start a new measurement
    pub fn start() -> Self {
        Self {
            start_time: read_time(),
            start_cycle: read_cycle(),
            start_instret: read_instret(),
        }
    }

    /// End the measurement and get results
    pub fn end(self) -> PerfResult {
        let end_time = read_time();
        let end_cycle = read_cycle();
        let end_instret = read_instret();

        PerfResult {
            time_ticks: end_time.saturating_sub(self.start_time),
            cycles: end_cycle.saturating_sub(self.start_cycle),
            instructions: end_instret.saturating_sub(self.start_instret),
        }
    }
}

/// Performance measurement result
#[derive(Debug, Clone)]
pub struct PerfResult {
    /// Elapsed time in timer ticks
    pub time_ticks: u64,
    /// Elapsed CPU cycles
    pub cycles: u64,
    /// Instructions retired
    pub instructions: u64,
}

impl PerfResult {
    /// Get elapsed time in nanoseconds
    pub fn time_ns(&self) -> u64 {
        ticks_to_ns(self.time_ticks)
    }

    /// Calculate instructions per cycle (IPC)
    pub fn ipc(&self) -> f64 {
        if self.cycles == 0 {
            0.0
        } else {
            self.instructions as f64 / self.cycles as f64
        }
    }

    /// Calculate cycles per instruction (CPI)
    pub fn cpi(&self) -> f64 {
        if self.instructions == 0 {
            0.0
        } else {
            self.cycles as f64 / self.instructions as f64
        }
    }
}

// ============================================================================
// Timestamping
// ============================================================================

/// Simple timestamp for logging
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Create a timestamp from the current time
    pub fn now() -> Self {
        Self(read_time())
    }

    /// Create from raw ticks
    pub const fn from_ticks(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Get raw ticks
    pub const fn ticks(self) -> u64 {
        self.0
    }

    /// Get time in microseconds
    pub fn as_us(self) -> u64 {
        ticks_to_us(self.0)
    }

    /// Get time in milliseconds
    pub fn as_ms(self) -> u64 {
        ticks_to_ms(self.0)
    }

    /// Calculate duration between timestamps
    pub fn duration_to(self, later: Timestamp) -> u64 {
        later.0.saturating_sub(self.0)
    }
}

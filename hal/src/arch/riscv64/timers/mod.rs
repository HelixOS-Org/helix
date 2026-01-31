//! # Timer Framework
//!
//! Timer support for RISC-V systems including machine timer (MTIME)
//! and supervisor timer (accessed via SBI).
//!
//! ## Submodules
//!
//! - `mtime`: Machine timer interface
//! - `sstimer`: Supervisor timer interface

pub mod mtime;
pub mod sstimer;

// Re-export commonly used items
pub use mtime::{read_time, read_cycle, get_timer_frequency, set_timer_frequency};
pub use sstimer::{set_timer, clear_timer, arm_oneshot, arm_periodic};

use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

// ============================================================================
// Timer Constants
// ============================================================================

/// Default timer frequency (10 MHz for QEMU)
pub const DEFAULT_TIMER_FREQ: u64 = 10_000_000;

/// Nanoseconds per second
pub const NS_PER_SEC: u64 = 1_000_000_000;

/// Microseconds per second
pub const US_PER_SEC: u64 = 1_000_000;

/// Milliseconds per second
pub const MS_PER_SEC: u64 = 1_000;

// ============================================================================
// Timer State
// ============================================================================

/// Timer initialized flag
static TIMER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Timer frequency (Hz)
static TIMER_FREQUENCY: AtomicU64 = AtomicU64::new(DEFAULT_TIMER_FREQ);

/// Boot time (timer ticks at boot)
static BOOT_TIME_TICKS: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// Timer Configuration
// ============================================================================

/// Timer configuration
#[derive(Debug, Clone, Copy)]
pub struct TimerConfig {
    /// Timer frequency in Hz
    pub frequency: u64,
    /// Enable periodic tick
    pub periodic_tick: bool,
    /// Tick interval in microseconds
    pub tick_interval_us: u64,
}

impl TimerConfig {
    /// Default configuration
    pub const fn default() -> Self {
        Self {
            frequency: DEFAULT_TIMER_FREQ,
            periodic_tick: true,
            tick_interval_us: 10_000, // 10ms = 100 Hz
        }
    }

    /// Calculate ticks per tick interval
    pub const fn ticks_per_interval(&self) -> u64 {
        (self.frequency * self.tick_interval_us) / US_PER_SEC
    }
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self::default()
    }
}

// ============================================================================
// Timer Initialization
// ============================================================================

/// Initialize the timer subsystem
///
/// # Safety
/// Must be called once during boot.
pub unsafe fn init(config: TimerConfig) {
    TIMER_FREQUENCY.store(config.frequency, Ordering::SeqCst);
    BOOT_TIME_TICKS.store(read_time(), Ordering::SeqCst);

    // Initialize supervisor timer
    sstimer::init(config);

    TIMER_INITIALIZED.store(true, Ordering::SeqCst);
}

/// Initialize timer for secondary hart
///
/// # Safety
/// Must be called on each secondary hart.
pub unsafe fn init_secondary(_hart_id: usize, config: TimerConfig) {
    sstimer::init(config);
}

/// Check if timer is initialized
pub fn is_initialized() -> bool {
    TIMER_INITIALIZED.load(Ordering::Acquire)
}

// ============================================================================
// Time Queries
// ============================================================================

/// Get time since boot in nanoseconds
pub fn time_since_boot_ns() -> u64 {
    let now = read_time();
    let boot = BOOT_TIME_TICKS.load(Ordering::Relaxed);
    let elapsed = now.saturating_sub(boot);
    ticks_to_ns(elapsed)
}

/// Get time since boot in microseconds
pub fn time_since_boot_us() -> u64 {
    let now = read_time();
    let boot = BOOT_TIME_TICKS.load(Ordering::Relaxed);
    let elapsed = now.saturating_sub(boot);
    ticks_to_us(elapsed)
}

/// Get time since boot in milliseconds
pub fn time_since_boot_ms() -> u64 {
    let now = read_time();
    let boot = BOOT_TIME_TICKS.load(Ordering::Relaxed);
    let elapsed = now.saturating_sub(boot);
    ticks_to_ms(elapsed)
}

/// Get time since boot in seconds
pub fn time_since_boot_s() -> u64 {
    let now = read_time();
    let boot = BOOT_TIME_TICKS.load(Ordering::Relaxed);
    let elapsed = now.saturating_sub(boot);
    ticks_to_s(elapsed)
}

// ============================================================================
// Time Conversion Functions
// ============================================================================

/// Convert nanoseconds to timer ticks
#[inline]
pub fn ns_to_ticks(ns: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    (ns * freq) / NS_PER_SEC
}

/// Convert microseconds to timer ticks
#[inline]
pub fn us_to_ticks(us: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    (us * freq) / US_PER_SEC
}

/// Convert milliseconds to timer ticks
#[inline]
pub fn ms_to_ticks(ms: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    (ms * freq) / MS_PER_SEC
}

/// Convert seconds to timer ticks
#[inline]
pub fn s_to_ticks(s: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    s * freq
}

/// Convert timer ticks to nanoseconds
#[inline]
pub fn ticks_to_ns(ticks: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    if freq == 0 { return 0; }
    (ticks * NS_PER_SEC) / freq
}

/// Convert timer ticks to microseconds
#[inline]
pub fn ticks_to_us(ticks: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    if freq == 0 { return 0; }
    (ticks * US_PER_SEC) / freq
}

/// Convert timer ticks to milliseconds
#[inline]
pub fn ticks_to_ms(ticks: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    if freq == 0 { return 0; }
    (ticks * MS_PER_SEC) / freq
}

/// Convert timer ticks to seconds
#[inline]
pub fn ticks_to_s(ticks: u64) -> u64 {
    let freq = TIMER_FREQUENCY.load(Ordering::Relaxed);
    if freq == 0 { return 0; }
    ticks / freq
}

// ============================================================================
// Delay Functions
// ============================================================================

/// Busy-wait for a number of nanoseconds
pub fn delay_ns(ns: u64) {
    delay_ticks(ns_to_ticks(ns));
}

/// Busy-wait for a number of microseconds
pub fn delay_us(us: u64) {
    delay_ticks(us_to_ticks(us));
}

/// Busy-wait for a number of milliseconds
pub fn delay_ms(ms: u64) {
    delay_ticks(ms_to_ticks(ms));
}

/// Busy-wait for a number of seconds
pub fn delay_s(s: u64) {
    delay_ticks(s_to_ticks(s));
}

/// Busy-wait for a number of timer ticks
pub fn delay_ticks(ticks: u64) {
    let start = read_time();
    let target = start.saturating_add(ticks);

    while read_time() < target {
        core::hint::spin_loop();
    }
}

// ============================================================================
// Timer Callback Interface
// ============================================================================

/// Timer callback function type
pub type TimerCallback = fn(u64);

/// Timer callback entry
#[derive(Clone, Copy)]
struct TimerCallbackEntry {
    callback: Option<TimerCallback>,
    deadline: u64,
    period: u64, // 0 for one-shot
    active: bool,
}

impl TimerCallbackEntry {
    const fn empty() -> Self {
        Self {
            callback: None,
            deadline: 0,
            period: 0,
            active: false,
        }
    }
}

/// Maximum number of timer callbacks
const MAX_CALLBACKS: usize = 32;

/// Timer callback table
static mut TIMER_CALLBACKS: [TimerCallbackEntry; MAX_CALLBACKS] = [TimerCallbackEntry::empty(); MAX_CALLBACKS];

/// Register a one-shot timer callback
///
/// # Safety
/// Callback must be safe to call from interrupt context.
pub unsafe fn register_oneshot_callback(
    deadline: u64,
    callback: TimerCallback,
) -> Option<TimerHandle> {
    for i in 0..MAX_CALLBACKS {
        if !TIMER_CALLBACKS[i].active {
            TIMER_CALLBACKS[i] = TimerCallbackEntry {
                callback: Some(callback),
                deadline,
                period: 0,
                active: true,
            };
            return Some(TimerHandle(i));
        }
    }
    None
}

/// Register a periodic timer callback
///
/// # Safety
/// Callback must be safe to call from interrupt context.
pub unsafe fn register_periodic_callback(
    first_deadline: u64,
    period: u64,
    callback: TimerCallback,
) -> Option<TimerHandle> {
    for i in 0..MAX_CALLBACKS {
        if !TIMER_CALLBACKS[i].active {
            TIMER_CALLBACKS[i] = TimerCallbackEntry {
                callback: Some(callback),
                deadline: first_deadline,
                period,
                active: true,
            };
            return Some(TimerHandle(i));
        }
    }
    None
}

/// Cancel a timer callback
pub fn cancel_callback(handle: TimerHandle) {
    if handle.0 < MAX_CALLBACKS {
        unsafe {
            TIMER_CALLBACKS[handle.0].active = false;
        }
    }
}

/// Timer handle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerHandle(usize);

impl TimerHandle {
    /// Get the index
    pub fn index(self) -> usize {
        self.0
    }
}

/// Process expired timers (called from timer interrupt handler)
pub(crate) fn process_expired_timers(now: u64) {
    unsafe {
        for entry in TIMER_CALLBACKS.iter_mut() {
            if entry.active && now >= entry.deadline {
                if let Some(callback) = entry.callback {
                    callback(now);
                }

                if entry.period > 0 {
                    // Reschedule periodic timer
                    entry.deadline = now.saturating_add(entry.period);
                } else {
                    // One-shot timer, deactivate
                    entry.active = false;
                }
            }
        }
    }
}

/// Get the next timer deadline
pub fn next_deadline() -> u64 {
    let mut next = u64::MAX;

    unsafe {
        for entry in TIMER_CALLBACKS.iter() {
            if entry.active && entry.deadline < next {
                next = entry.deadline;
            }
        }
    }

    next
}

// ============================================================================
// High-Resolution Timer
// ============================================================================

/// High-resolution timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant {
    ticks: u64,
}

impl Instant {
    /// Get current instant
    pub fn now() -> Self {
        Self { ticks: read_time() }
    }

    /// Get the raw tick count
    pub fn ticks(self) -> u64 {
        self.ticks
    }

    /// Duration since another instant
    pub fn duration_since(self, earlier: Instant) -> Duration {
        Duration::from_ticks(self.ticks.saturating_sub(earlier.ticks))
    }

    /// Elapsed time since this instant
    pub fn elapsed(self) -> Duration {
        Self::now().duration_since(self)
    }

    /// Check if deadline has passed
    pub fn has_passed(self) -> bool {
        read_time() >= self.ticks
    }

    /// Add a duration
    pub fn add_duration(self, duration: Duration) -> Self {
        Self {
            ticks: self.ticks.saturating_add(duration.ticks),
        }
    }
}

/// Duration type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    ticks: u64,
}

impl Duration {
    /// Zero duration
    pub const ZERO: Self = Self { ticks: 0 };

    /// Maximum duration
    pub const MAX: Self = Self { ticks: u64::MAX };

    /// Create from ticks
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    /// Create from nanoseconds
    pub fn from_nanos(ns: u64) -> Self {
        Self { ticks: ns_to_ticks(ns) }
    }

    /// Create from microseconds
    pub fn from_micros(us: u64) -> Self {
        Self { ticks: us_to_ticks(us) }
    }

    /// Create from milliseconds
    pub fn from_millis(ms: u64) -> Self {
        Self { ticks: ms_to_ticks(ms) }
    }

    /// Create from seconds
    pub fn from_secs(s: u64) -> Self {
        Self { ticks: s_to_ticks(s) }
    }

    /// Get as ticks
    pub const fn as_ticks(self) -> u64 {
        self.ticks
    }

    /// Get as nanoseconds
    pub fn as_nanos(self) -> u64 {
        ticks_to_ns(self.ticks)
    }

    /// Get as microseconds
    pub fn as_micros(self) -> u64 {
        ticks_to_us(self.ticks)
    }

    /// Get as milliseconds
    pub fn as_millis(self) -> u64 {
        ticks_to_ms(self.ticks)
    }

    /// Get as seconds
    pub fn as_secs(self) -> u64 {
        ticks_to_s(self.ticks)
    }

    /// Check if zero
    pub fn is_zero(self) -> bool {
        self.ticks == 0
    }
}

impl core::ops::Add for Duration {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            ticks: self.ticks.saturating_add(rhs.ticks),
        }
    }
}

impl core::ops::Sub for Duration {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            ticks: self.ticks.saturating_sub(rhs.ticks),
        }
    }
}

// ============================================================================
// Timeout Support
// ============================================================================

/// Timeout for operations
#[derive(Debug, Clone, Copy)]
pub enum Timeout {
    /// No timeout (wait forever)
    Never,
    /// Immediate (don't wait)
    Immediate,
    /// Wait for a duration
    After(Duration),
    /// Wait until a deadline
    Until(Instant),
}

impl Timeout {
    /// Check if the timeout has expired
    pub fn has_expired(self) -> bool {
        match self {
            Self::Never => false,
            Self::Immediate => true,
            Self::After(d) => d.is_zero(),
            Self::Until(i) => i.has_passed(),
        }
    }

    /// Get the deadline instant
    pub fn deadline(self) -> Option<Instant> {
        match self {
            Self::Never => None,
            Self::Immediate => Some(Instant::now()),
            Self::After(d) => Some(Instant::now().add_duration(d)),
            Self::Until(i) => Some(i),
        }
    }
}

impl From<Duration> for Timeout {
    fn from(d: Duration) -> Self {
        Self::After(d)
    }
}

impl From<Instant> for Timeout {
    fn from(i: Instant) -> Self {
        Self::Until(i)
    }
}

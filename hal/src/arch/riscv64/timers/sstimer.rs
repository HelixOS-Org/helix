//! # Supervisor Timer Interface
//!
//! Timer management for S-mode using SBI calls.
//!
//! From S-mode, we cannot directly access MTIMECMP. Instead,
//! we use the SBI timer extension to set timer deadlines.

use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};

use super::mtime::read_time;
use super::{TimerConfig, us_to_ticks, process_expired_timers, next_deadline};

// ============================================================================
// SBI Timer Extension
// ============================================================================

/// SBI Timer Extension ID
const TIMER_EID: usize = 0x54494D45;

/// Set the timer via SBI
#[inline]
fn sbi_set_timer(stime_value: u64) {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") TIMER_EID,
            in("a6") 0usize, // FID = 0 for set_timer
            in("a0") stime_value,
            options(nomem, nostack)
        );
    }
}

// ============================================================================
// Timer State
// ============================================================================

/// Timer initialized
static TIMER_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Periodic tick enabled
static PERIODIC_TICK_ENABLED: AtomicBool = AtomicBool::new(false);

/// Tick interval in ticks
static TICK_INTERVAL: AtomicU64 = AtomicU64::new(0);

/// Next tick deadline
static NEXT_TICK: AtomicU64 = AtomicU64::new(u64::MAX);

/// Tick count
static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the supervisor timer
pub fn init(config: TimerConfig) {
    let interval = us_to_ticks(config.tick_interval_us);
    TICK_INTERVAL.store(interval, Ordering::SeqCst);
    PERIODIC_TICK_ENABLED.store(config.periodic_tick, Ordering::SeqCst);

    if config.periodic_tick {
        // Set up first tick
        let deadline = read_time().saturating_add(interval);
        NEXT_TICK.store(deadline, Ordering::SeqCst);
        sbi_set_timer(deadline);
    }

    TIMER_INITIALIZED.store(true, Ordering::SeqCst);
}

/// Check if timer is initialized
pub fn is_initialized() -> bool {
    TIMER_INITIALIZED.load(Ordering::Acquire)
}

// ============================================================================
// Timer Control
// ============================================================================

/// Set an absolute timer deadline
pub fn set_timer(deadline: u64) {
    sbi_set_timer(deadline);
}

/// Set a relative timer (deadline = now + ticks)
pub fn set_timer_relative(ticks: u64) {
    let deadline = read_time().saturating_add(ticks);
    sbi_set_timer(deadline);
}

/// Clear the timer (disable by setting far future)
pub fn clear_timer() {
    sbi_set_timer(u64::MAX);
}

/// Arm a one-shot timer
pub fn arm_oneshot(ticks: u64) {
    let deadline = read_time().saturating_add(ticks);

    // Only set if earlier than current deadline
    let current = NEXT_TICK.load(Ordering::Relaxed);
    if deadline < current {
        NEXT_TICK.store(deadline, Ordering::SeqCst);
        sbi_set_timer(deadline);
    }
}

/// Arm a periodic timer
pub fn arm_periodic(interval_ticks: u64) {
    TICK_INTERVAL.store(interval_ticks, Ordering::SeqCst);
    PERIODIC_TICK_ENABLED.store(true, Ordering::SeqCst);

    let deadline = read_time().saturating_add(interval_ticks);
    NEXT_TICK.store(deadline, Ordering::SeqCst);
    sbi_set_timer(deadline);
}

/// Disable periodic timer
pub fn disable_periodic() {
    PERIODIC_TICK_ENABLED.store(false, Ordering::SeqCst);
}

/// Enable periodic timer
pub fn enable_periodic() {
    PERIODIC_TICK_ENABLED.store(true, Ordering::SeqCst);
}

// ============================================================================
// Timer Interrupt Handler
// ============================================================================

/// Timer tick handler
pub type TickHandler = fn(u64);

/// Registered tick handler
static mut TICK_HANDLER: Option<TickHandler> = None;

/// Register a tick handler
///
/// # Safety
/// Handler must be safe to call from interrupt context.
pub unsafe fn register_tick_handler(handler: TickHandler) {
    TICK_HANDLER = Some(handler);
}

/// Handle a timer interrupt
///
/// Returns true if rescheduling should occur.
pub fn handle_timer_interrupt() -> bool {
    let now = read_time();

    // Increment tick count
    let tick = TICK_COUNT.fetch_add(1, Ordering::SeqCst);

    // Process expired timer callbacks
    process_expired_timers(now);

    // Call registered handler
    unsafe {
        if let Some(handler) = TICK_HANDLER {
            handler(tick);
        }
    }

    // Set up next periodic tick
    if PERIODIC_TICK_ENABLED.load(Ordering::Relaxed) {
        let interval = TICK_INTERVAL.load(Ordering::Relaxed);
        let mut next = NEXT_TICK.load(Ordering::Relaxed);

        // Advance to next tick (handle missed ticks)
        while next <= now {
            next = next.saturating_add(interval);
        }

        // Check if there's an earlier callback deadline
        let callback_deadline = next_deadline();
        if callback_deadline < next {
            next = callback_deadline;
        }

        NEXT_TICK.store(next, Ordering::SeqCst);
        sbi_set_timer(next);
    } else {
        // Check for callback deadlines
        let callback_deadline = next_deadline();
        if callback_deadline < u64::MAX {
            sbi_set_timer(callback_deadline);
        } else {
            clear_timer();
        }
    }

    // Always trigger reschedule on tick
    true
}

// ============================================================================
// Tick Statistics
// ============================================================================

/// Get the current tick count
pub fn get_tick_count() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

/// Reset the tick count
pub fn reset_tick_count() {
    TICK_COUNT.store(0, Ordering::SeqCst);
}

/// Get the tick interval in ticks
pub fn get_tick_interval() -> u64 {
    TICK_INTERVAL.load(Ordering::Relaxed)
}

/// Get the tick rate in Hz
pub fn get_tick_rate() -> u64 {
    let interval = TICK_INTERVAL.load(Ordering::Relaxed);
    if interval == 0 { return 0; }
    super::get_timer_frequency() / interval
}

// ============================================================================
// Time Queries
// ============================================================================

/// Get time until next tick
pub fn time_until_tick() -> u64 {
    let now = read_time();
    let next = NEXT_TICK.load(Ordering::Relaxed);
    next.saturating_sub(now)
}

/// Check if timer interrupt is pending
pub fn is_timer_pending() -> bool {
    let now = read_time();
    let next = NEXT_TICK.load(Ordering::Relaxed);
    now >= next
}

// ============================================================================
// Per-Hart Timer State
// ============================================================================

/// Per-hart timer state
#[derive(Debug, Clone, Copy)]
pub struct HartTimerState {
    /// Next deadline for this hart
    pub deadline: u64,
    /// Is periodic enabled
    pub periodic: bool,
    /// Interval for periodic
    pub interval: u64,
    /// Tick count for this hart
    pub ticks: u64,
}

impl HartTimerState {
    /// New timer state
    pub const fn new() -> Self {
        Self {
            deadline: u64::MAX,
            periodic: false,
            interval: 0,
            ticks: 0,
        }
    }
}

impl Default for HartTimerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-hart timer states
const MAX_HARTS: usize = 256;
static mut HART_TIMER_STATES: [HartTimerState; MAX_HARTS] = [HartTimerState::new(); MAX_HARTS];

/// Get timer state for a hart
pub fn get_hart_timer_state(hart_id: usize) -> Option<HartTimerState> {
    if hart_id < MAX_HARTS {
        Some(unsafe { HART_TIMER_STATES[hart_id] })
    } else {
        None
    }
}

/// Set timer state for a hart
pub fn set_hart_timer_state(hart_id: usize, state: HartTimerState) {
    if hart_id < MAX_HARTS {
        unsafe { HART_TIMER_STATES[hart_id] = state };
    }
}

// ============================================================================
// Scheduler Integration
// ============================================================================

/// Time slice for scheduler (in timer ticks)
static TIME_SLICE: AtomicU64 = AtomicU64::new(0);

/// Set the scheduler time slice
pub fn set_time_slice(ticks: u64) {
    TIME_SLICE.store(ticks, Ordering::SeqCst);
}

/// Get the scheduler time slice
pub fn get_time_slice() -> u64 {
    TIME_SLICE.load(Ordering::Relaxed)
}

/// Set up a time slice for the current task
pub fn arm_time_slice() {
    let slice = TIME_SLICE.load(Ordering::Relaxed);
    if slice > 0 {
        arm_oneshot(slice);
    }
}

/// Calculate remaining time slice
pub fn remaining_time_slice() -> u64 {
    let now = read_time();
    let next = NEXT_TICK.load(Ordering::Relaxed);
    next.saturating_sub(now)
}

// ============================================================================
// Deadline-based Scheduling Support
// ============================================================================

/// Deadline structure for EDF scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline {
    /// Absolute deadline in timer ticks
    pub absolute: u64,
    /// Relative deadline (period)
    pub relative: u64,
}

impl Deadline {
    /// Create a new deadline
    pub fn new(relative: u64) -> Self {
        Self {
            absolute: read_time().saturating_add(relative),
            relative,
        }
    }

    /// Check if deadline has passed
    pub fn has_passed(&self) -> bool {
        read_time() >= self.absolute
    }

    /// Time until deadline
    pub fn time_remaining(&self) -> u64 {
        let now = read_time();
        self.absolute.saturating_sub(now)
    }

    /// Reset deadline (for periodic tasks)
    pub fn reset(&mut self) {
        self.absolute = read_time().saturating_add(self.relative);
    }
}

// ============================================================================
// Watchdog Timer Support
// ============================================================================

/// Watchdog state
static WATCHDOG_DEADLINE: AtomicU64 = AtomicU64::new(u64::MAX);
static WATCHDOG_ENABLED: AtomicBool = AtomicBool::new(false);

/// Watchdog handler
static mut WATCHDOG_HANDLER: Option<fn()> = None;

/// Enable watchdog with a timeout
pub fn enable_watchdog(timeout_ticks: u64) {
    let deadline = read_time().saturating_add(timeout_ticks);
    WATCHDOG_DEADLINE.store(deadline, Ordering::SeqCst);
    WATCHDOG_ENABLED.store(true, Ordering::SeqCst);
}

/// Disable watchdog
pub fn disable_watchdog() {
    WATCHDOG_ENABLED.store(false, Ordering::SeqCst);
    WATCHDOG_DEADLINE.store(u64::MAX, Ordering::SeqCst);
}

/// Pet (reset) the watchdog
pub fn pet_watchdog(timeout_ticks: u64) {
    if WATCHDOG_ENABLED.load(Ordering::Relaxed) {
        let deadline = read_time().saturating_add(timeout_ticks);
        WATCHDOG_DEADLINE.store(deadline, Ordering::SeqCst);
    }
}

/// Register watchdog handler
///
/// # Safety
/// Handler must be safe to call from interrupt context.
pub unsafe fn register_watchdog_handler(handler: fn()) {
    WATCHDOG_HANDLER = Some(handler);
}

/// Check watchdog (called from timer interrupt)
pub(super) fn check_watchdog() {
    if WATCHDOG_ENABLED.load(Ordering::Relaxed) {
        let now = read_time();
        let deadline = WATCHDOG_DEADLINE.load(Ordering::Relaxed);

        if now >= deadline {
            unsafe {
                if let Some(handler) = WATCHDOG_HANDLER {
                    handler();
                }
            }
        }
    }
}

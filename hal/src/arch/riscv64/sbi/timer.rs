//! # SBI Timer Extension
//!
//! Timer management via SBI.

use super::eid;
use super::base::sbi_call_1;

// ============================================================================
// Timer Extension
// ============================================================================

/// Set the timer deadline
///
/// Programs the timer to generate an interrupt when the time reaches `stime_value`.
/// To clear the timer, set a value in the far future (e.g., u64::MAX).
pub fn set_timer(stime_value: u64) {
    let _ = sbi_call_1(eid::TIME, 0, stime_value as usize);
}

/// Clear the timer by setting it to the maximum value
pub fn clear_timer() {
    set_timer(u64::MAX);
}

/// Set a relative timer (current time + delay)
pub fn set_timer_relative(delay: u64) {
    let current = read_time();
    set_timer(current.saturating_add(delay));
}

// ============================================================================
// Time Reading
// ============================================================================

/// Read the current time counter
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

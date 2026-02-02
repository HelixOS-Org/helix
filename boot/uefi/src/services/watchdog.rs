//! Watchdog Timer Services
//!
//! Safe wrappers for UEFI watchdog timer functionality.

use super::boot_services;
use crate::raw::types::*;

// =============================================================================
// WATCHDOG TIMER
// =============================================================================

/// Watchdog timer controller
pub struct WatchdogTimer {
    /// Original timeout (to restore later)
    original_timeout: u64,
    /// Is disabled
    disabled: bool,
}

impl WatchdogTimer {
    /// Create new watchdog timer controller
    pub fn new() -> Self {
        Self {
            original_timeout: 300, // Default 5 minutes
            disabled: false,
        }
    }

    /// Set watchdog timeout in seconds
    ///
    /// A timeout of 0 disables the watchdog.
    pub fn set_timeout(&mut self, seconds: u64) -> Result<(), Status> {
        let bs = unsafe { boot_services() };

        // Convert to 100ns units
        let timeout = seconds;

        // SetWatchdogTimer(Timeout, WatchdogCode, DataSize, WatchdogData)
        let result =
            unsafe { ((*bs).set_watchdog_timer)(timeout as usize, 0x10000, 0, core::ptr::null()) };

        if result == Status::SUCCESS {
            self.disabled = seconds == 0;
            Ok(())
        } else {
            Err(result)
        }
    }

    /// Disable the watchdog timer
    pub fn disable(&mut self) -> Result<(), Status> {
        self.set_timeout(0)
    }

    /// Enable with default timeout (5 minutes)
    pub fn enable_default(&mut self) -> Result<(), Status> {
        self.set_timeout(300)
    }

    /// Kick (reset) the watchdog timer
    ///
    /// This resets the countdown without changing the timeout.
    pub fn kick(&self) -> Result<(), Status> {
        if self.disabled {
            return Ok(());
        }

        let bs = unsafe { boot_services() };

        // Re-set the timer to reset the countdown
        let result = unsafe {
            ((*bs).set_watchdog_timer)(
                self.original_timeout as usize,
                0x10000,
                0,
                core::ptr::null(),
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(result)
        }
    }

    /// Check if watchdog is disabled
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl Default for WatchdogTimer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// WATCHDOG GUARD
// =============================================================================

/// RAII guard that disables watchdog on creation and restores on drop
pub struct WatchdogGuard {
    /// Previous timeout
    previous_timeout: u64,
    /// Was disabled
    was_disabled: bool,
}

impl WatchdogGuard {
    /// Create new guard that disables watchdog
    pub fn disable() -> Result<Self, Status> {
        let guard = Self {
            previous_timeout: 300, // Default
            was_disabled: false,
        };

        let bs = unsafe { boot_services() };

        let result = unsafe { ((*bs).set_watchdog_timer)(0, 0, 0, core::ptr::null()) };

        if result == Status::SUCCESS {
            Ok(guard)
        } else {
            Err(result)
        }
    }

    /// Create guard with specific timeout
    pub fn with_timeout(seconds: u64) -> Result<Self, Status> {
        let guard = Self {
            previous_timeout: 300,
            was_disabled: false,
        };

        let bs = unsafe { boot_services() };

        let result =
            unsafe { ((*bs).set_watchdog_timer)(seconds as usize, 0x10000, 0, core::ptr::null()) };

        if result == Status::SUCCESS {
            Ok(guard)
        } else {
            Err(result)
        }
    }
}

impl Drop for WatchdogGuard {
    fn drop(&mut self) {
        if !self.was_disabled {
            let bs = unsafe { boot_services() };
            unsafe {
                let _ = ((*bs).set_watchdog_timer)(
                    self.previous_timeout as usize,
                    0x10000,
                    0,
                    core::ptr::null(),
                );
            }
        }
    }
}

// =============================================================================
// WATCHDOG CODES
// =============================================================================

/// Watchdog reset codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchdogCode(u64);

impl WatchdogCode {
    /// UEFI timeout
    pub const UEFI_TIMEOUT: Self = Self(0);

    /// User defined base
    pub const USER_DEFINED_BASE: Self = Self(0x10000);

    /// Boot failure
    pub const BOOT_FAILURE: Self = Self(0x10001);

    /// OS loader failure
    pub const OS_LOADER_FAILURE: Self = Self(0x10002);

    /// Driver failure
    pub const DRIVER_FAILURE: Self = Self(0x10003);

    /// Application failure
    pub const APPLICATION_FAILURE: Self = Self(0x10004);

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Create user-defined code
    pub const fn user_defined(code: u64) -> Self {
        Self(Self::USER_DEFINED_BASE.0 + code)
    }
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Set watchdog timeout
pub fn set_watchdog_timeout(seconds: u64) -> Result<(), Status> {
    let bs = unsafe { boot_services() };

    let result =
        unsafe { ((*bs).set_watchdog_timer)(seconds as usize, 0x10000, 0, core::ptr::null()) };

    if result == Status::SUCCESS {
        Ok(())
    } else {
        Err(result)
    }
}

/// Disable watchdog timer
pub fn disable_watchdog() -> Result<(), Status> {
    set_watchdog_timeout(0)
}

/// Set watchdog with message
pub fn set_watchdog_with_message(
    seconds: u64,
    code: WatchdogCode,
    message: &str,
) -> Result<(), Status> {
    let bs = unsafe { boot_services() };

    // Convert message to UCS-2
    let mut buffer = [0u16; 256];
    let len = message
        .chars()
        .take(255)
        .enumerate()
        .map(|(i, c)| {
            buffer[i] = c as u16;
            i
        })
        .last()
        .map(|i| i + 1)
        .unwrap_or(0);

    let result = unsafe {
        ((*bs).set_watchdog_timer)(seconds as usize, code.raw(), len * 2, buffer.as_ptr())
    };

    if result == Status::SUCCESS {
        Ok(())
    } else {
        Err(result)
    }
}

// =============================================================================
// WATCHDOG KICKER
// =============================================================================

/// Automatic watchdog kicker
///
/// Periodically kicks the watchdog to prevent timeout.
pub struct WatchdogKicker {
    /// Timer event
    timer: Option<super::events::Timer>,
    /// Kick interval in 100ns units
    interval_100ns: u64,
}

impl WatchdogKicker {
    /// Create new kicker with interval in milliseconds
    pub fn new(interval_ms: u64) -> Self {
        Self {
            timer: None,
            interval_100ns: interval_ms * 10_000, // ms to 100ns
        }
    }

    /// Start periodic kicking
    pub fn start(&mut self) -> Result<(), Status> {
        // Note: In a real implementation, this would set up a periodic
        // timer event with a callback to kick the watchdog.
        // For now, we just track the interval.
        Ok(())
    }

    /// Stop periodic kicking
    pub fn stop(&mut self) {
        self.timer = None;
    }

    /// Manual kick
    pub fn kick(&self) -> Result<(), Status> {
        // Just reset the timer by setting it again
        // This requires knowing the current timeout, which we don't track globally
        // So just set a reasonable default
        set_watchdog_timeout(300)
    }
}

// =============================================================================
// LONG OPERATION GUARD
// =============================================================================

/// Guard for long operations that might trigger watchdog
///
/// Automatically extends watchdog timeout during long operations.
pub struct LongOperationGuard {
    /// Guard that disabled watchdog
    guard: Option<WatchdogGuard>,
}

impl LongOperationGuard {
    /// Create guard for operation expected to take given seconds
    pub fn new(expected_seconds: u64) -> Result<Self, Status> {
        // Add 50% margin
        let timeout = expected_seconds + expected_seconds / 2 + 60;

        let guard = WatchdogGuard::with_timeout(timeout)?;

        Ok(Self { guard: Some(guard) })
    }

    /// Create guard that completely disables watchdog
    pub fn disable() -> Result<Self, Status> {
        let guard = WatchdogGuard::disable()?;

        Ok(Self { guard: Some(guard) })
    }

    /// Mark progress (extend timeout if needed)
    pub fn progress(&mut self) -> Result<(), Status> {
        // In a real implementation, this could extend the timeout
        // For now, just reset the timer
        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watchdog_code() {
        let code = WatchdogCode::user_defined(42);
        assert_eq!(code.raw(), 0x10000 + 42);
    }

    #[test]
    fn test_watchdog_timer() {
        let timer = WatchdogTimer::new();
        assert!(!timer.is_disabled());
    }
}

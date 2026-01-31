//! # SBI Hart State Management (HSM) Extension
//!
//! The HSM extension provides hart lifecycle management.
//!
//! ## Functions
//!
//! - `hart_start`: Start a stopped hart
//! - `hart_stop`: Stop the calling hart
//! - `hart_get_status`: Get hart status
//! - `hart_suspend`: Suspend the calling hart

use super::{eid, hsm_fid, SbiError};
use super::base::{sbi_call_0, sbi_call_1, sbi_call_3, SbiRet};

// ============================================================================
// Hart State
// ============================================================================

/// Hart state as reported by SBI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum HartState {
    /// Hart has been started and is running
    Started = 0,
    /// Hart is stopped
    Stopped = 1,
    /// Hart start request pending
    StartPending = 2,
    /// Hart stop request pending
    StopPending = 3,
    /// Hart is suspended
    Suspended = 4,
    /// Hart suspend request pending
    SuspendPending = 5,
    /// Hart resume request pending
    ResumePending = 6,
}

impl HartState {
    /// Convert from raw SBI value
    pub fn from_raw(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::Started),
            1 => Some(Self::Stopped),
            2 => Some(Self::StartPending),
            3 => Some(Self::StopPending),
            4 => Some(Self::Suspended),
            5 => Some(Self::SuspendPending),
            6 => Some(Self::ResumePending),
            _ => None,
        }
    }

    /// Check if hart is in a running state
    pub fn is_running(self) -> bool {
        matches!(self, Self::Started)
    }

    /// Check if hart is in a stopped state
    pub fn is_stopped(self) -> bool {
        matches!(self, Self::Stopped)
    }

    /// Check if hart is in a transition state
    pub fn is_pending(self) -> bool {
        matches!(
            self,
            Self::StartPending | Self::StopPending | Self::SuspendPending | Self::ResumePending
        )
    }

    /// Check if hart can be started
    pub fn can_start(self) -> bool {
        matches!(self, Self::Stopped)
    }

    /// Get state name
    pub fn name(self) -> &'static str {
        match self {
            Self::Started => "Started",
            Self::Stopped => "Stopped",
            Self::StartPending => "Start Pending",
            Self::StopPending => "Stop Pending",
            Self::Suspended => "Suspended",
            Self::SuspendPending => "Suspend Pending",
            Self::ResumePending => "Resume Pending",
        }
    }
}

// ============================================================================
// Suspend Type
// ============================================================================

/// Hart suspend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SuspendType {
    /// Retentive suspend (state preserved)
    DefaultRetentive = 0x0000_0000,
    /// Non-retentive suspend (state lost)
    DefaultNonRetentive = 0x8000_0000,
}

impl SuspendType {
    /// Check if this is a retentive suspend
    pub fn is_retentive(self) -> bool {
        (self as u32) & 0x8000_0000 == 0
    }

    /// Check if this is a non-retentive suspend
    pub fn is_non_retentive(self) -> bool {
        (self as u32) & 0x8000_0000 != 0
    }
}

// ============================================================================
// HSM Functions
// ============================================================================

/// Start a hart
///
/// # Arguments
/// * `hartid` - The hart ID to start
/// * `start_addr` - The address where the hart should start executing
/// * `opaque` - An opaque value passed to the hart in register a1
///
/// # Safety
/// The start_addr must point to valid code.
pub unsafe fn hart_start(hartid: usize, start_addr: usize, opaque: usize) -> Result<(), SbiError> {
    let ret = sbi_call_3(eid::HSM, hsm_fid::HART_START, hartid, start_addr, opaque);

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Stop the calling hart
///
/// This function does not return on success.
pub fn hart_stop() -> Result<(), SbiError> {
    let ret = sbi_call_0(eid::HSM, hsm_fid::HART_STOP);

    // Should not return on success
    Err(SbiError::from_raw(ret.error))
}

/// Get the status of a hart
pub fn hart_get_status(hartid: usize) -> Result<HartState, SbiError> {
    let ret = sbi_call_1(eid::HSM, hsm_fid::HART_GET_STATUS, hartid);

    if ret.is_success() {
        HartState::from_raw(ret.value).ok_or(SbiError::Failed)
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

/// Suspend the calling hart
///
/// # Arguments
/// * `suspend_type` - The type of suspension
/// * `resume_addr` - Address to resume at (for non-retentive)
/// * `opaque` - Opaque value passed on resume
///
/// For retentive suspend, execution continues after this call on resume.
/// For non-retentive suspend, execution resumes at resume_addr.
pub fn hart_suspend(
    suspend_type: SuspendType,
    resume_addr: usize,
    opaque: usize,
) -> Result<(), SbiError> {
    let ret = sbi_call_3(
        eid::HSM,
        hsm_fid::HART_SUSPEND,
        suspend_type as usize,
        resume_addr,
        opaque,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SbiError::from_raw(ret.error))
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Check if a hart is available (exists and is in a valid state)
pub fn hart_is_available(hartid: usize) -> bool {
    hart_get_status(hartid).is_ok()
}

/// Wait for a hart to reach a specific state
pub fn wait_for_hart_state(hartid: usize, expected: HartState, timeout_loops: Option<u64>) -> bool {
    let mut remaining = timeout_loops;

    loop {
        match hart_get_status(hartid) {
            Ok(state) if state == expected => return true,
            Err(_) => return false,
            _ => {}
        }

        core::hint::spin_loop();

        if let Some(ref mut t) = remaining {
            if *t == 0 {
                return false;
            }
            *t -= 1;
        }
    }
}

/// Wait for a hart to start
pub fn wait_for_hart_start(hartid: usize, timeout_loops: Option<u64>) -> bool {
    wait_for_hart_state(hartid, HartState::Started, timeout_loops)
}

/// Wait for a hart to stop
pub fn wait_for_hart_stop(hartid: usize, timeout_loops: Option<u64>) -> bool {
    wait_for_hart_state(hartid, HartState::Stopped, timeout_loops)
}

// ============================================================================
// Multi-Hart Operations
// ============================================================================

/// Start multiple harts
///
/// # Safety
/// The start_addr must point to valid code.
pub unsafe fn start_harts(
    hart_mask: u64,
    hart_mask_base: usize,
    start_addr: usize,
) -> usize {
    let mut started = 0;

    for i in 0..64 {
        if (hart_mask >> i) & 1 != 0 {
            let hartid = hart_mask_base + i;
            if hart_start(hartid, start_addr, hartid).is_ok() {
                started += 1;
            }
        }
    }

    started
}

/// Stop the calling hart (does not return)
pub fn stop_self() -> ! {
    let _ = hart_stop();

    // Should not reach here, but just in case
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
    }
}

/// Suspend for a specific duration (approximate)
///
/// Uses retentive suspend if available.
pub fn suspend_retentive() -> Result<(), SbiError> {
    hart_suspend(SuspendType::DefaultRetentive, 0, 0)
}

// ============================================================================
// Hart Information
// ============================================================================

/// Collect status of all harts
pub fn collect_hart_status(max_harts: usize) -> alloc::vec::Vec<(usize, HartState)> {
    let mut status = alloc::vec::Vec::new();

    for hartid in 0..max_harts {
        if let Ok(state) = hart_get_status(hartid) {
            status.push((hartid, state));
        }
    }

    status
}

extern crate alloc;

/// Count harts in a specific state
pub fn count_harts_in_state(max_harts: usize, state: HartState) -> usize {
    let mut count = 0;

    for hartid in 0..max_harts {
        if let Ok(s) = hart_get_status(hartid) {
            if s == state {
                count += 1;
            }
        }
    }

    count
}

/// Count running harts
pub fn count_running_harts(max_harts: usize) -> usize {
    count_harts_in_state(max_harts, HartState::Started)
}

/// Count stopped harts
pub fn count_stopped_harts(max_harts: usize) -> usize {
    count_harts_in_state(max_harts, HartState::Stopped)
}

/// Find the next stopped hart
pub fn find_stopped_hart(max_harts: usize) -> Option<usize> {
    for hartid in 0..max_harts {
        if let Ok(HartState::Stopped) = hart_get_status(hartid) {
            return Some(hartid);
        }
    }
    None
}

/// Find all stopped harts
pub fn find_all_stopped_harts(max_harts: usize) -> alloc::vec::Vec<usize> {
    let mut harts = alloc::vec::Vec::new();

    for hartid in 0..max_harts {
        if let Ok(HartState::Stopped) = hart_get_status(hartid) {
            harts.push(hartid);
        }
    }

    harts
}

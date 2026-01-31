//! # Secondary Hart Startup
//!
//! Handles bringing up secondary harts using SBI HSM extension.
//!
//! ## Boot Protocol
//!
//! 1. Primary hart calls `start_hart()` for each secondary
//! 2. Secondary hart starts at the provided entry point
//! 3. Secondary hart initializes its per-CPU data
//! 4. Secondary hart signals ready
//! 5. Primary hart waits for all secondaries

use core::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};

use super::{MAX_HARTS, increment_online};
use super::hartid::{set_hart_state, HartState, mark_hart_online, mark_ready};
use super::percpu::init_secondary_percpu;

// ============================================================================
// Hart Status
// ============================================================================

/// Hart status from SBI HSM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum HartStatus {
    /// Hart is started (running)
    Started = 0,
    /// Hart is stopped
    Stopped = 1,
    /// Hart start is pending
    StartPending = 2,
    /// Hart stop is pending
    StopPending = 3,
    /// Hart is suspended
    Suspended = 4,
    /// Hart suspend is pending
    SuspendPending = 5,
    /// Hart resume is pending
    ResumePending = 6,
}

impl HartStatus {
    /// Convert from SBI return value
    pub fn from_sbi(value: i64) -> Option<Self> {
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

    /// Is the hart in a running state?
    pub fn is_running(self) -> bool {
        matches!(self, Self::Started)
    }

    /// Is the hart in a stopped state?
    pub fn is_stopped(self) -> bool {
        matches!(self, Self::Stopped)
    }
}

// ============================================================================
// SBI HSM Interface
// ============================================================================

/// SBI extension IDs
mod sbi {
    /// HSM (Hart State Management) extension ID
    pub const HSM_EID: usize = 0x48534D;

    /// HSM function IDs
    pub mod hsm {
        pub const HART_START: usize = 0;
        pub const HART_STOP: usize = 1;
        pub const HART_GET_STATUS: usize = 2;
        pub const HART_SUSPEND: usize = 3;
    }
}

/// SBI return structure
#[derive(Debug, Clone, Copy)]
pub struct SbiRet {
    pub error: i64,
    pub value: i64,
}

impl SbiRet {
    /// Check if the call succeeded
    pub fn is_success(&self) -> bool {
        self.error == 0
    }
}

/// Make an SBI call
#[inline]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let error: i64;
    let value: i64;

    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") eid,
            in("a6") fid,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            lateout("a0") error,
            lateout("a1") value,
            options(nostack)
        );
    }

    SbiRet { error, value }
}

// ============================================================================
// Hart Control
// ============================================================================

/// Start a secondary hart
///
/// # Arguments
/// * `hart_id` - Hart ID to start
/// * `start_addr` - Entry point address
/// * `opaque` - Value passed to the entry point in a1
///
/// # Safety
/// The start_addr must point to valid code that handles hart startup.
pub unsafe fn start_hart(hart_id: usize, start_addr: usize, opaque: usize) -> Result<(), StartError> {
    if hart_id >= MAX_HARTS {
        return Err(StartError::InvalidHart);
    }

    set_hart_state(hart_id, HartState::Starting);

    let ret = sbi_call(
        sbi::HSM_EID,
        sbi::hsm::HART_START,
        hart_id,
        start_addr,
        opaque,
    );

    if ret.is_success() {
        Ok(())
    } else {
        set_hart_state(hart_id, HartState::Stopped);
        Err(StartError::from_sbi(ret.error))
    }
}

/// Stop the current hart
pub fn stop_current_hart() -> ! {
    let hart_id = super::hartid::get_hart_id();
    set_hart_state(hart_id, HartState::Stopping);
    super::hartid::mark_hart_offline(hart_id);
    super::decrement_online();

    let _ = sbi_call(
        sbi::HSM_EID,
        sbi::hsm::HART_STOP,
        0, 0, 0,
    );

    // Should not return
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
    }
}

/// Get the status of a hart
pub fn get_hart_status(hart_id: usize) -> Option<HartStatus> {
    let ret = sbi_call(
        sbi::HSM_EID,
        sbi::hsm::HART_GET_STATUS,
        hart_id,
        0, 0,
    );

    if ret.is_success() {
        HartStatus::from_sbi(ret.value)
    } else {
        None
    }
}

/// Suspend the current hart
pub fn suspend_hart(suspend_type: SuspendType, resume_addr: usize, opaque: usize) -> Result<(), SuspendError> {
    let ret = sbi_call(
        sbi::HSM_EID,
        sbi::hsm::HART_SUSPEND,
        suspend_type as usize,
        resume_addr,
        opaque,
    );

    if ret.is_success() {
        Ok(())
    } else {
        Err(SuspendError::from_sbi(ret.error))
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Hart start error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartError {
    /// Invalid hart ID
    InvalidHart,
    /// Hart is already started
    AlreadyStarted,
    /// Invalid address
    InvalidAddress,
    /// Generic failure
    Failed,
    /// Feature not supported
    NotSupported,
    /// Unknown error
    Unknown(i64),
}

impl StartError {
    fn from_sbi(error: i64) -> Self {
        match error {
            -1 => Self::Failed,
            -2 => Self::NotSupported,
            -3 => Self::InvalidHart,
            -4 => Self::InvalidAddress,
            -5 => Self::AlreadyStarted,
            e => Self::Unknown(e),
        }
    }
}

/// Hart suspend error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuspendError {
    /// Invalid suspend type
    InvalidType,
    /// Not supported
    NotSupported,
    /// Invalid address
    InvalidAddress,
    /// Generic failure
    Failed,
    /// Unknown error
    Unknown(i64),
}

impl SuspendError {
    fn from_sbi(error: i64) -> Self {
        match error {
            -1 => Self::Failed,
            -2 => Self::NotSupported,
            -3 => Self::InvalidType,
            -4 => Self::InvalidAddress,
            e => Self::Unknown(e),
        }
    }
}

/// Suspend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SuspendType {
    /// Default retentive suspend
    DefaultRetentive = 0x0000_0000,
    /// Default non-retentive suspend
    DefaultNonRetentive = 0x8000_0000,
}

// ============================================================================
// Wait Functions
// ============================================================================

/// Wait for a hart to reach a specific status
pub fn wait_for_hart(hart_id: usize, expected: HartStatus, timeout_us: Option<u64>) -> bool {
    let mut remaining = timeout_us;

    loop {
        match get_hart_status(hart_id) {
            Some(status) if status == expected => return true,
            _ => {}
        }

        // Simple delay
        for _ in 0..100 {
            core::hint::spin_loop();
        }

        if let Some(ref mut t) = remaining {
            if *t == 0 {
                return false;
            }
            *t = t.saturating_sub(1);
        }
    }
}

/// Wait for a hart to be started
pub fn wait_for_hart_started(hart_id: usize, timeout_us: Option<u64>) -> bool {
    wait_for_hart(hart_id, HartStatus::Started, timeout_us)
}

/// Wait for a hart to be stopped
pub fn wait_for_hart_stopped(hart_id: usize, timeout_us: Option<u64>) -> bool {
    wait_for_hart(hart_id, HartStatus::Stopped, timeout_us)
}

// ============================================================================
// Multi-Hart Startup
// ============================================================================

/// Start all secondary harts
///
/// # Safety
/// Entry point must be valid for all harts.
pub unsafe fn start_all_secondary_harts(
    entry_point: usize,
    hart_count: usize,
    boot_hart: usize,
) -> usize {
    let mut started = 0;

    for hart_id in 0..hart_count.min(MAX_HARTS) {
        if hart_id == boot_hart {
            continue; // Skip boot hart
        }

        // Pass hart_id as opaque value
        if start_hart(hart_id, entry_point, hart_id).is_ok() {
            started += 1;
        }
    }

    started
}

/// Wait for all harts to be ready
pub fn wait_for_all_harts_ready(hart_count: usize, timeout_us: Option<u64>) -> usize {
    let mut ready = 0;

    for hart_id in 0..hart_count.min(MAX_HARTS) {
        if super::hartid::wait_for_hart_ready(hart_id, timeout_us) {
            ready += 1;
        }
    }

    ready
}

// ============================================================================
// Secondary Hart Entry
// ============================================================================

/// Startup data passed to secondary harts
#[repr(C)]
pub struct SecondaryStartupData {
    /// Stack pointer for this hart
    pub stack_ptr: usize,
    /// Entry function to call after initialization
    pub entry_fn: usize,
    /// Argument to pass to entry function
    pub entry_arg: usize,
    /// Synchronization flag
    pub ready: AtomicUsize,
}

impl SecondaryStartupData {
    /// Create new startup data
    pub const fn new() -> Self {
        Self {
            stack_ptr: 0,
            entry_fn: 0,
            entry_arg: 0,
            ready: AtomicUsize::new(0),
        }
    }
}

/// Storage for startup data
static mut STARTUP_DATA: [SecondaryStartupData; MAX_HARTS] = {
    const INIT: SecondaryStartupData = SecondaryStartupData::new();
    [INIT; MAX_HARTS]
};

/// Prepare startup data for a secondary hart
///
/// # Safety
/// Must be called before starting the hart.
pub unsafe fn prepare_secondary_startup(
    hart_id: usize,
    stack_ptr: usize,
    entry_fn: fn(usize),
    entry_arg: usize,
) {
    if hart_id >= MAX_HARTS {
        return;
    }

    STARTUP_DATA[hart_id] = SecondaryStartupData {
        stack_ptr,
        entry_fn: entry_fn as usize,
        entry_arg,
        ready: AtomicUsize::new(0),
    };
}

/// Get startup data for a hart
///
/// # Safety
/// Must be called from the correct hart.
pub unsafe fn get_startup_data(hart_id: usize) -> &'static SecondaryStartupData {
    &STARTUP_DATA[hart_id.min(MAX_HARTS - 1)]
}

/// Secondary hart entry point (called from assembly)
///
/// # Safety
/// Must be called from the secondary hart entry stub.
#[no_mangle]
pub unsafe extern "C" fn secondary_hart_entry(hart_id: usize) {
    // Initialize per-CPU data
    init_secondary_percpu(hart_id);

    // Mark as online and running
    mark_hart_online(hart_id);
    set_hart_state(hart_id, HartState::Running);
    increment_online();

    // Signal ready
    mark_ready();

    // Get startup data
    let data = get_startup_data(hart_id);
    data.ready.store(1, Ordering::Release);

    // Call the entry function
    let entry: fn(usize) = core::mem::transmute(data.entry_fn);
    entry(data.entry_arg);

    // If entry returns, stop the hart
    stop_current_hart();
}

/// Assembly trampoline for secondary hart entry
///
/// This is the actual entry point passed to SBI.
/// It sets up the stack and calls secondary_hart_entry.
#[naked]
#[no_mangle]
pub unsafe extern "C" fn secondary_hart_trampoline() -> ! {
    core::arch::naked_asm!(
        // a0 = hart_id (from opaque parameter)
        // Set up stack
        "la t0, {startup_data}",
        "slli t1, a0, 5",         // t1 = hart_id * 32 (sizeof SecondaryStartupData)
        "add t0, t0, t1",
        "ld sp, 0(t0)",           // Load stack pointer

        // Set TP to hart_id temporarily
        "mv tp, a0",

        // Call Rust entry point
        "call {entry}",

        // Should not return
        "1: wfi",
        "j 1b",

        startup_data = sym STARTUP_DATA,
        entry = sym secondary_hart_entry,
    );
}

// ============================================================================
// Boot Synchronization
// ============================================================================

/// Barrier for boot synchronization
static BOOT_BARRIER_COUNT: AtomicUsize = AtomicUsize::new(0);
static BOOT_BARRIER_GENERATION: AtomicUsize = AtomicUsize::new(0);

/// Wait at the boot barrier
pub fn boot_barrier(expected_harts: usize) {
    let gen = BOOT_BARRIER_GENERATION.load(Ordering::Acquire);

    let arrived = BOOT_BARRIER_COUNT.fetch_add(1, Ordering::AcqRel) + 1;

    if arrived == expected_harts {
        // Last to arrive - reset and advance
        BOOT_BARRIER_COUNT.store(0, Ordering::Release);
        BOOT_BARRIER_GENERATION.store(gen.wrapping_add(1), Ordering::Release);
    } else {
        // Wait for generation to advance
        while BOOT_BARRIER_GENERATION.load(Ordering::Acquire) == gen {
            core::hint::spin_loop();
        }
    }
}

/// Reset the boot barrier
pub fn reset_boot_barrier() {
    BOOT_BARRIER_COUNT.store(0, Ordering::SeqCst);
    BOOT_BARRIER_GENERATION.store(0, Ordering::SeqCst);
}

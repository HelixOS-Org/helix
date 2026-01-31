//! # Hart ID Management
//!
//! Functions for reading and managing RISC-V Hart IDs.
//!
//! On RISC-V, the hart ID is typically stored in a CSR (mhartid)
//! and cached in the TP (thread pointer) register for fast access.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use super::MAX_HARTS;

// ============================================================================
// Hart ID Types
// ============================================================================

/// Hart ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HartId(usize);

impl HartId {
    /// Create a new HartId
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// Get the raw hart ID value
    pub const fn as_usize(self) -> usize {
        self.0
    }

    /// Get the current hart's ID
    pub fn current() -> Self {
        Self(get_hart_id())
    }

    /// Check if this is the boot hart
    pub fn is_boot(self) -> bool {
        self.0 == get_boot_hart_id()
    }

    /// Check if this hart is online
    pub fn is_online(self) -> bool {
        is_hart_online(self.0)
    }
}

impl From<usize> for HartId {
    fn from(id: usize) -> Self {
        Self(id)
    }
}

impl From<HartId> for usize {
    fn from(id: HartId) -> Self {
        id.0
    }
}

// ============================================================================
// Hart ID Access
// ============================================================================

/// Read the hart ID from the TP register (fast path)
///
/// This assumes the TP register has been set up with the hart ID.
#[inline(always)]
pub fn get_hart_id() -> usize {
    let tp: usize;
    unsafe {
        core::arch::asm!("mv {}, tp", out(reg) tp, options(nomem, nostack, preserves_flags));
    }
    // The hart ID is stored at offset 0 in the per-CPU structure
    // For simplicity, we use TP directly as the hart ID during early boot
    tp
}

/// Read the hart ID from the mhartid CSR
///
/// This traps to M-mode on most systems (only works from M-mode directly).
/// Use `get_hart_id()` for the fast path.
#[inline]
pub fn read_mhartid() -> usize {
    // mhartid is not directly accessible from S-mode
    // We need to use a different method or rely on TP
    get_hart_id()
}

/// Set the hart ID in the TP register
///
/// # Safety
/// Must be called during early hart initialization.
#[inline]
pub unsafe fn set_hart_id(hart_id: usize) {
    core::arch::asm!("mv tp, {}", in(reg) hart_id, options(nomem, nostack, preserves_flags));
}

/// Set the TP register to point to per-CPU data
///
/// # Safety
/// Must be called with a valid per-CPU data pointer.
#[inline]
pub unsafe fn set_tp(ptr: usize) {
    core::arch::asm!("mv tp, {}", in(reg) ptr, options(nomem, nostack, preserves_flags));
}

/// Read the TP register value
#[inline]
pub fn get_tp() -> usize {
    let tp: usize;
    unsafe {
        core::arch::asm!("mv {}, tp", out(reg) tp, options(nomem, nostack, preserves_flags));
    }
    tp
}

// ============================================================================
// Boot Hart ID
// ============================================================================

/// Boot hart ID (detected at startup)
static BOOT_HART_ID: AtomicUsize = AtomicUsize::new(0);

/// Get the boot hart ID
pub fn get_boot_hart_id() -> usize {
    BOOT_HART_ID.load(Ordering::Relaxed)
}

/// Set the boot hart ID
///
/// # Safety
/// Should only be called once during early initialization.
pub unsafe fn set_boot_hart_id(hart_id: usize) {
    BOOT_HART_ID.store(hart_id, Ordering::SeqCst);
}

/// Check if the current hart is the boot hart
pub fn is_boot_hart() -> bool {
    get_hart_id() == get_boot_hart_id()
}

// ============================================================================
// Hart Online Status
// ============================================================================

/// Online status for each hart
static HART_ONLINE: [AtomicBool; MAX_HARTS] = {
    const FALSE: AtomicBool = AtomicBool::new(false);
    [FALSE; MAX_HARTS]
};

/// Mark a hart as online
pub fn mark_hart_online(hart_id: usize) {
    if hart_id < MAX_HARTS {
        HART_ONLINE[hart_id].store(true, Ordering::SeqCst);
    }
}

/// Mark a hart as offline
pub fn mark_hart_offline(hart_id: usize) {
    if hart_id < MAX_HARTS {
        HART_ONLINE[hart_id].store(false, Ordering::SeqCst);
    }
}

/// Check if a hart is online
pub fn is_hart_online(hart_id: usize) -> bool {
    if hart_id < MAX_HARTS {
        HART_ONLINE[hart_id].load(Ordering::Relaxed)
    } else {
        false
    }
}

/// Get the number of online harts
pub fn count_online_harts() -> usize {
    HART_ONLINE.iter()
        .filter(|h| h.load(Ordering::Relaxed))
        .count()
}

// ============================================================================
// Hart Ready Status (for synchronization)
// ============================================================================

/// Ready status for each hart (used during initialization)
static HART_READY: [AtomicBool; MAX_HARTS] = {
    const FALSE: AtomicBool = AtomicBool::new(false);
    [FALSE; MAX_HARTS]
};

/// Mark the current hart as ready
pub fn mark_ready() {
    let hart_id = get_hart_id();
    if hart_id < MAX_HARTS {
        HART_READY[hart_id].store(true, Ordering::SeqCst);
    }
}

/// Check if a hart is ready
pub fn is_hart_ready(hart_id: usize) -> bool {
    if hart_id < MAX_HARTS {
        HART_READY[hart_id].load(Ordering::Acquire)
    } else {
        false
    }
}

/// Wait for a hart to become ready
pub fn wait_for_hart_ready(hart_id: usize, timeout: Option<u64>) -> bool {
    if hart_id >= MAX_HARTS {
        return false;
    }

    let mut remaining = timeout;

    while !HART_READY[hart_id].load(Ordering::Acquire) {
        core::hint::spin_loop();

        if let Some(ref mut t) = remaining {
            if *t == 0 {
                return false;
            }
            *t -= 1;
        }
    }

    true
}

/// Wait for all harts up to a count to be ready
pub fn wait_for_all_ready(hart_count: usize, timeout: Option<u64>) -> bool {
    for hart_id in 0..hart_count.min(MAX_HARTS) {
        if !wait_for_hart_ready(hart_id, timeout) {
            return false;
        }
    }
    true
}

// ============================================================================
// Hart State
// ============================================================================

/// Hart execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HartState {
    /// Hart is stopped
    Stopped = 0,
    /// Hart is starting
    Starting = 1,
    /// Hart is running
    Running = 2,
    /// Hart is suspending
    Suspending = 3,
    /// Hart is suspended
    Suspended = 4,
    /// Hart is stopping
    Stopping = 5,
}

impl HartState {
    /// Is the hart in an active state?
    pub fn is_active(self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Suspending | Self::Stopping)
    }

    /// Is the hart runnable?
    pub fn is_runnable(self) -> bool {
        matches!(self, Self::Running)
    }
}

impl From<u8> for HartState {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Stopped,
            1 => Self::Starting,
            2 => Self::Running,
            3 => Self::Suspending,
            4 => Self::Suspended,
            5 => Self::Stopping,
            _ => Self::Stopped,
        }
    }
}

/// Hart state storage
static HART_STATE: [AtomicUsize; MAX_HARTS] = {
    const STOPPED: AtomicUsize = AtomicUsize::new(HartState::Stopped as usize);
    [STOPPED; MAX_HARTS]
};

/// Get the state of a hart
pub fn get_hart_state(hart_id: usize) -> HartState {
    if hart_id < MAX_HARTS {
        HartState::from(HART_STATE[hart_id].load(Ordering::Acquire) as u8)
    } else {
        HartState::Stopped
    }
}

/// Set the state of a hart
pub fn set_hart_state(hart_id: usize, state: HartState) {
    if hart_id < MAX_HARTS {
        HART_STATE[hart_id].store(state as usize, Ordering::Release);
    }
}

/// Try to transition hart state
pub fn try_transition_state(
    hart_id: usize,
    from: HartState,
    to: HartState,
) -> Result<(), HartState> {
    if hart_id >= MAX_HARTS {
        return Err(HartState::Stopped);
    }

    match HART_STATE[hart_id].compare_exchange(
        from as usize,
        to as usize,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => Ok(()),
        Err(current) => Err(HartState::from(current as u8)),
    }
}

// ============================================================================
// Hart Features
// ============================================================================

/// Hart feature flags
#[derive(Debug, Clone, Copy, Default)]
pub struct HartFeatures {
    /// Supports floating point
    pub has_float: bool,
    /// Supports double precision
    pub has_double: bool,
    /// Supports compressed instructions
    pub has_compressed: bool,
    /// Supports atomic instructions
    pub has_atomic: bool,
    /// Supports vector extension
    pub has_vector: bool,
    /// Supports hypervisor extension
    pub has_hypervisor: bool,
}

/// Per-hart feature storage
static mut HART_FEATURES: [HartFeatures; MAX_HARTS] = [HartFeatures {
    has_float: false,
    has_double: false,
    has_compressed: false,
    has_atomic: false,
    has_vector: false,
    has_hypervisor: false,
}; MAX_HARTS];

/// Get features for a hart
pub fn get_hart_features(hart_id: usize) -> HartFeatures {
    if hart_id < MAX_HARTS {
        unsafe { HART_FEATURES[hart_id] }
    } else {
        HartFeatures::default()
    }
}

/// Set features for a hart
///
/// # Safety
/// Should only be called during hart initialization.
pub unsafe fn set_hart_features(hart_id: usize, features: HartFeatures) {
    if hart_id < MAX_HARTS {
        HART_FEATURES[hart_id] = features;
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get a list of all hart IDs
pub fn get_all_hart_ids() -> impl Iterator<Item = usize> {
    (0..MAX_HARTS).filter(|&id| is_hart_online(id))
}

/// Get the next online hart after the given one (wrapping)
pub fn next_online_hart(current: usize) -> Option<usize> {
    let total = super::total_harts();

    // Search forward from current+1
    for offset in 1..total {
        let id = (current + offset) % total;
        if is_hart_online(id) {
            return Some(id);
        }
    }

    None
}

/// Get any online hart other than the current one
pub fn any_other_online_hart() -> Option<usize> {
    let current = get_hart_id();
    next_online_hart(current)
}

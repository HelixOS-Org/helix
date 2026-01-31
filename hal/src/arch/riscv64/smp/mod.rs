//! # SMP (Symmetric Multi-Processing) Framework
//!
//! Multi-hart support for RISC-V systems.
//!
//! ## Submodules
//!
//! - `hartid`: Hart ID management
//! - `percpu`: Per-hart data structures
//! - `startup`: Secondary hart startup
//! - `ipi`: Inter-Processor Interrupts

pub mod hartid;
pub mod percpu;
pub mod startup;
pub mod ipi;

// Re-export commonly used items
pub use hartid::{get_hart_id, get_boot_hart_id, HartId};
pub use percpu::{PerCpu, get_percpu, PerCpuRef};
pub use startup::{start_hart, wait_for_hart, HartStatus};
pub use ipi::{send_ipi, broadcast_ipi, IpiType};

use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

// ============================================================================
// SMP Constants
// ============================================================================

/// Maximum supported number of harts
pub const MAX_HARTS: usize = 256;

/// Boot hart ID (set during initialization)
static BOOT_HART_ID: AtomicUsize = AtomicUsize::new(0);

/// Number of online harts
static ONLINE_HARTS: AtomicUsize = AtomicUsize::new(1);

/// Total number of harts in the system
static TOTAL_HARTS: AtomicUsize = AtomicUsize::new(1);

/// SMP initialized flag
static SMP_INITIALIZED: AtomicBool = AtomicBool::new(false);

// ============================================================================
// SMP Configuration
// ============================================================================

/// SMP configuration
#[derive(Debug, Clone, Copy)]
pub struct SmpConfig {
    /// Boot hart ID
    pub boot_hart: usize,
    /// Total number of harts
    pub total_harts: usize,
    /// Stack size per hart
    pub stack_size: usize,
    /// Stack base address
    pub stack_base: usize,
}

impl SmpConfig {
    /// Default configuration
    pub const fn default() -> Self {
        Self {
            boot_hart: 0,
            total_harts: 1,
            stack_size: 64 * 1024, // 64KB per hart
            stack_base: 0,
        }
    }

    /// Calculate stack pointer for a hart
    pub const fn stack_for_hart(&self, hart_id: usize) -> usize {
        self.stack_base + (hart_id + 1) * self.stack_size
    }
}

// ============================================================================
// SMP Initialization
// ============================================================================

/// Initialize SMP subsystem
///
/// # Safety
/// Must be called once from the boot hart.
pub unsafe fn init(config: SmpConfig) {
    BOOT_HART_ID.store(config.boot_hart, Ordering::SeqCst);
    TOTAL_HARTS.store(config.total_harts, Ordering::SeqCst);
    ONLINE_HARTS.store(1, Ordering::SeqCst); // Boot hart is online

    // Mark boot hart as online
    hartid::mark_hart_online(config.boot_hart);

    // Initialize per-CPU data for boot hart
    percpu::init_boot_percpu(config.boot_hart);

    SMP_INITIALIZED.store(true, Ordering::SeqCst);
}

/// Check if SMP is initialized
pub fn is_initialized() -> bool {
    SMP_INITIALIZED.load(Ordering::Acquire)
}

/// Get the boot hart ID
pub fn boot_hart_id() -> usize {
    BOOT_HART_ID.load(Ordering::Relaxed)
}

/// Get number of online harts
pub fn online_harts() -> usize {
    ONLINE_HARTS.load(Ordering::Relaxed)
}

/// Get total number of harts
pub fn total_harts() -> usize {
    TOTAL_HARTS.load(Ordering::Relaxed)
}

/// Increment online hart count
pub(crate) fn increment_online() {
    ONLINE_HARTS.fetch_add(1, Ordering::SeqCst);
}

/// Decrement online hart count
pub(crate) fn decrement_online() {
    ONLINE_HARTS.fetch_sub(1, Ordering::SeqCst);
}

// ============================================================================
// Hart Iteration
// ============================================================================

/// Iterator over all harts
pub struct HartIterator {
    current: usize,
    total: usize,
}

impl HartIterator {
    /// Create new iterator
    pub fn new() -> Self {
        Self {
            current: 0,
            total: total_harts(),
        }
    }

    /// Create iterator over online harts only
    pub fn online_only() -> OnlineHartIterator {
        OnlineHartIterator { current: 0 }
    }
}

impl Default for HartIterator {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for HartIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.total {
            let id = self.current;
            self.current += 1;
            Some(id)
        } else {
            None
        }
    }
}

/// Iterator over online harts only
pub struct OnlineHartIterator {
    current: usize,
}

impl Iterator for OnlineHartIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let total = total_harts();
        while self.current < total {
            let id = self.current;
            self.current += 1;
            if hartid::is_hart_online(id) {
                return Some(id);
            }
        }
        None
    }
}

/// Iterate over all harts
pub fn all_harts() -> HartIterator {
    HartIterator::new()
}

/// Iterate over online harts
pub fn online_hart_ids() -> OnlineHartIterator {
    HartIterator::online_only()
}

// ============================================================================
// Hart Mask Operations
// ============================================================================

/// Hart mask for multi-hart operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HartMask {
    /// Low 64 harts
    pub low: u64,
    /// High 64 harts (if supported)
    pub high: u64,
    /// Mask base (for SBI compatibility)
    pub base: usize,
}

impl HartMask {
    /// Empty mask (no harts)
    pub const fn empty() -> Self {
        Self {
            low: 0,
            high: 0,
            base: 0,
        }
    }

    /// All harts mask
    pub const fn all() -> Self {
        Self {
            low: u64::MAX,
            high: u64::MAX,
            base: 0,
        }
    }

    /// Single hart
    pub const fn single(hart: usize) -> Self {
        if hart < 64 {
            Self {
                low: 1 << hart,
                high: 0,
                base: 0,
            }
        } else if hart < 128 {
            Self {
                low: 0,
                high: 1 << (hart - 64),
                base: 0,
            }
        } else {
            Self {
                low: 1,
                high: 0,
                base: hart,
            }
        }
    }

    /// All harts except one
    pub fn all_except(hart: usize) -> Self {
        let mut mask = Self::all();
        mask.clear(hart);
        mask
    }

    /// Set a hart in the mask
    pub fn set(&mut self, hart: usize) {
        let adjusted = hart.saturating_sub(self.base);
        if adjusted < 64 {
            self.low |= 1 << adjusted;
        } else if adjusted < 128 {
            self.high |= 1 << (adjusted - 64);
        }
    }

    /// Clear a hart from the mask
    pub fn clear(&mut self, hart: usize) {
        let adjusted = hart.saturating_sub(self.base);
        if adjusted < 64 {
            self.low &= !(1 << adjusted);
        } else if adjusted < 128 {
            self.high &= !(1 << (adjusted - 64));
        }
    }

    /// Check if a hart is in the mask
    pub fn contains(&self, hart: usize) -> bool {
        let adjusted = hart.saturating_sub(self.base);
        if adjusted < 64 {
            (self.low >> adjusted) & 1 != 0
        } else if adjusted < 128 {
            (self.high >> (adjusted - 64)) & 1 != 0
        } else {
            false
        }
    }

    /// Check if mask is empty
    pub fn is_empty(&self) -> bool {
        self.low == 0 && self.high == 0
    }

    /// Count set bits
    pub fn count(&self) -> u32 {
        self.low.count_ones() + self.high.count_ones()
    }

    /// Get mask for online harts
    pub fn online() -> Self {
        let mut mask = Self::empty();
        for hart in online_hart_ids() {
            mask.set(hart);
        }
        mask
    }
}

impl Default for HartMask {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Cross-Hart Calls
// ============================================================================

/// Function to call on remote harts
pub type CrossCallFn = fn(arg: usize);

/// Cross-hart call request
struct CrossCallRequest {
    func: CrossCallFn,
    arg: usize,
    completed: AtomicBool,
}

/// Pending cross-call requests per hart
static mut CROSS_CALL_REQUESTS: [Option<CrossCallRequest>; MAX_HARTS] = {
    const NONE: Option<CrossCallRequest> = None;
    [NONE; MAX_HARTS]
};

/// Call a function on another hart
///
/// # Safety
/// Function must be safe to call from interrupt context.
pub unsafe fn call_on_hart(target_hart: usize, func: CrossCallFn, arg: usize) {
    if target_hart >= MAX_HARTS {
        return;
    }

    // Set up the request
    CROSS_CALL_REQUESTS[target_hart] = Some(CrossCallRequest {
        func,
        arg,
        completed: AtomicBool::new(false),
    });

    // Send IPI to wake the hart
    ipi::send_ipi(target_hart, ipi::IpiType::FunctionCall);

    // Wait for completion (with timeout)
    let mut timeout = 1_000_000u32;
    while timeout > 0 {
        if let Some(ref req) = CROSS_CALL_REQUESTS[target_hart] {
            if req.completed.load(Ordering::Acquire) {
                break;
            }
        }
        core::hint::spin_loop();
        timeout -= 1;
    }

    // Clear request
    CROSS_CALL_REQUESTS[target_hart] = None;
}

/// Call a function on all other harts
///
/// # Safety
/// Function must be safe to call from interrupt context.
pub unsafe fn call_on_all_others(func: CrossCallFn, arg: usize) {
    let current = hartid::get_hart_id();
    for hart in online_hart_ids() {
        if hart != current {
            call_on_hart(hart, func, arg);
        }
    }
}

/// Handle incoming cross-call (called from IPI handler)
pub(crate) fn handle_cross_call(hart_id: usize) {
    unsafe {
        if let Some(ref req) = CROSS_CALL_REQUESTS[hart_id] {
            (req.func)(req.arg);
            req.completed.store(true, Ordering::Release);
        }
    }
}

// ============================================================================
// Hart Synchronization Barrier
// ============================================================================

/// Multi-hart barrier
pub struct HartBarrier {
    count: AtomicUsize,
    generation: AtomicUsize,
    expected: usize,
}

impl HartBarrier {
    /// Create a new barrier
    pub const fn new(expected: usize) -> Self {
        Self {
            count: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            expected,
        }
    }

    /// Wait at the barrier
    pub fn wait(&self) {
        let gen = self.generation.load(Ordering::Acquire);

        let arrived = self.count.fetch_add(1, Ordering::AcqRel) + 1;

        if arrived == self.expected {
            // Last to arrive - reset and advance
            self.count.store(0, Ordering::Release);
            self.generation.store(gen.wrapping_add(1), Ordering::Release);
        } else {
            // Wait for generation to advance
            while self.generation.load(Ordering::Acquire) == gen {
                core::hint::spin_loop();
            }
        }
    }

    /// Reset the barrier
    pub fn reset(&self) {
        self.count.store(0, Ordering::SeqCst);
    }
}

// ============================================================================
// Hart Parking
// ============================================================================

/// Hart parking state
static PARKED_HARTS: [AtomicBool; MAX_HARTS] = {
    const FALSE: AtomicBool = AtomicBool::new(false);
    [FALSE; MAX_HARTS]
};

/// Park the current hart
pub fn park() {
    let hart = hartid::get_hart_id();
    if hart < MAX_HARTS {
        PARKED_HARTS[hart].store(true, Ordering::SeqCst);

        // Wait to be unparked
        while PARKED_HARTS[hart].load(Ordering::Acquire) {
            // WFI to save power
            unsafe { core::arch::asm!("wfi", options(nomem, nostack)) };
        }
    }
}

/// Unpark a specific hart
pub fn unpark(hart: usize) {
    if hart < MAX_HARTS {
        PARKED_HARTS[hart].store(false, Ordering::SeqCst);
        // Send IPI to wake
        ipi::send_ipi(hart, ipi::IpiType::Wakeup);
    }
}

/// Check if a hart is parked
pub fn is_parked(hart: usize) -> bool {
    if hart < MAX_HARTS {
        PARKED_HARTS[hart].load(Ordering::Relaxed)
    } else {
        false
    }
}

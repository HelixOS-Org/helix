//! # Per-CPU (Per-Hart) Data Structures
//!
//! Provides per-hart data storage using the TP (thread pointer) register.
//!
//! ## Design
//!
//! On RISC-V, the TP register is conventionally used to point to thread-local
//! or per-CPU data. In kernel mode, we use it to point to a `PerCpu` structure
//! that contains hart-specific state.

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use super::MAX_HARTS;
use super::hartid::{get_hart_id, get_tp};

// ============================================================================
// Per-CPU Data Structure
// ============================================================================

/// Per-hart data structure
///
/// This structure is stored at the address pointed to by the TP register.
/// The hart_id field MUST be at offset 0 for fast access.
#[repr(C)]
pub struct PerCpu {
    // === Cache line 0 (64 bytes) - Hot data ===
    /// Hart ID (MUST be at offset 0)
    pub hart_id: usize,

    /// Current kernel stack pointer
    pub kernel_sp: usize,

    /// User stack pointer (saved during trap)
    pub user_sp: usize,

    /// Current trap frame pointer
    pub trap_frame: usize,

    /// Scratch space for trap handler
    pub scratch: [usize; 4],

    // === Cache line 1 (64 bytes) - Scheduling ===
    /// Current thread pointer
    pub current_thread: usize,

    /// Idle thread for this hart
    pub idle_thread: usize,

    /// Preemption count (>0 means preemption disabled)
    pub preempt_count: usize,

    /// Interrupt nesting depth
    pub interrupt_depth: usize,

    /// Need reschedule flag
    pub need_resched: AtomicBool,

    /// Padding for alignment
    _pad1: [u8; 23],

    // === Cache line 2 (64 bytes) - Timing ===
    /// Timer deadline
    pub timer_deadline: u64,

    /// Time in kernel (for accounting)
    pub time_in_kernel: u64,

    /// Time in user (for accounting)
    pub time_in_user: u64,

    /// Time in interrupt
    pub time_in_interrupt: u64,

    /// Last timestamp for timing
    pub last_timestamp: u64,

    /// Padding
    _pad2: [u8; 24],

    // === Cache line 3+ - Less frequently accessed ===
    /// FPU state pointer
    pub fpu_state: usize,

    /// Is FPU currently enabled?
    pub fpu_enabled: bool,

    /// Current ASID
    pub current_asid: u16,

    /// CPU features
    pub features: u32,

    /// CPU frequency (Hz)
    pub cpu_freq: u64,

    /// IPI pending flags
    pub ipi_pending: AtomicUsize,

    /// Deferred work pending
    pub deferred_pending: AtomicBool,

    /// Private data pointer
    pub private: usize,
}

impl PerCpu {
    /// Create a new PerCpu structure
    pub const fn new(hart_id: usize) -> Self {
        Self {
            hart_id,
            kernel_sp: 0,
            user_sp: 0,
            trap_frame: 0,
            scratch: [0; 4],
            current_thread: 0,
            idle_thread: 0,
            preempt_count: 0,
            interrupt_depth: 0,
            need_resched: AtomicBool::new(false),
            _pad1: [0; 23],
            timer_deadline: u64::MAX,
            time_in_kernel: 0,
            time_in_user: 0,
            time_in_interrupt: 0,
            last_timestamp: 0,
            _pad2: [0; 24],
            fpu_state: 0,
            fpu_enabled: false,
            current_asid: 0,
            features: 0,
            cpu_freq: 0,
            ipi_pending: AtomicUsize::new(0),
            deferred_pending: AtomicBool::new(false),
            private: 0,
        }
    }

    /// Get a reference to the current hart's PerCpu
    #[inline]
    pub fn current() -> &'static Self {
        unsafe { &*(get_tp() as *const Self) }
    }

    /// Get a mutable reference to the current hart's PerCpu
    ///
    /// # Safety
    /// Caller must ensure no other references exist.
    #[inline]
    pub unsafe fn current_mut() -> &'static mut Self {
        &mut *(get_tp() as *mut Self)
    }

    /// Check if we're in an interrupt context
    #[inline]
    pub fn in_interrupt(&self) -> bool {
        self.interrupt_depth > 0
    }

    /// Check if preemption is enabled
    #[inline]
    pub fn preemption_enabled(&self) -> bool {
        self.preempt_count == 0
    }

    /// Disable preemption
    #[inline]
    pub fn preempt_disable(&mut self) {
        self.preempt_count += 1;
    }

    /// Enable preemption
    #[inline]
    pub fn preempt_enable(&mut self) {
        debug_assert!(self.preempt_count > 0);
        self.preempt_count -= 1;
    }

    /// Set need reschedule flag
    #[inline]
    pub fn set_need_resched(&self) {
        self.need_resched.store(true, Ordering::Release);
    }

    /// Clear and check need reschedule flag
    #[inline]
    pub fn check_resched(&self) -> bool {
        self.need_resched.swap(false, Ordering::AcqRel)
    }
}

// ============================================================================
// Per-CPU Allocation
// ============================================================================

/// Storage for per-CPU structures
static mut PERCPU_STORAGE: [MaybeUninit<PerCpu>; MAX_HARTS] = {
    const UNINIT: MaybeUninit<PerCpu> = MaybeUninit::uninit();
    [UNINIT; MAX_HARTS]
};

/// Per-CPU pointers
static PERCPU_PTRS: [AtomicPtr<PerCpu>; MAX_HARTS] = {
    const NULL: AtomicPtr<PerCpu> = AtomicPtr::new(core::ptr::null_mut());
    [NULL; MAX_HARTS]
};

/// Initialized flag for each per-CPU
static PERCPU_INIT: [AtomicBool; MAX_HARTS] = {
    const FALSE: AtomicBool = AtomicBool::new(false);
    [FALSE; MAX_HARTS]
};

/// Initialize per-CPU for the boot hart
///
/// # Safety
/// Must be called once during early boot.
pub unsafe fn init_boot_percpu(hart_id: usize) {
    if hart_id >= MAX_HARTS {
        return;
    }

    // Initialize the structure
    let percpu = PERCPU_STORAGE[hart_id].as_mut_ptr();
    percpu.write(PerCpu::new(hart_id));

    // Store the pointer
    PERCPU_PTRS[hart_id].store(percpu, Ordering::SeqCst);
    PERCPU_INIT[hart_id].store(true, Ordering::SeqCst);

    // Set TP to point to our per-CPU data
    super::hartid::set_tp(percpu as usize);
}

/// Initialize per-CPU for a secondary hart
///
/// # Safety
/// Must be called once per hart during startup.
pub unsafe fn init_secondary_percpu(hart_id: usize) {
    if hart_id >= MAX_HARTS {
        return;
    }

    // Initialize the structure
    let percpu = PERCPU_STORAGE[hart_id].as_mut_ptr();
    percpu.write(PerCpu::new(hart_id));

    // Store the pointer
    PERCPU_PTRS[hart_id].store(percpu, Ordering::SeqCst);
    PERCPU_INIT[hart_id].store(true, Ordering::SeqCst);

    // Set TP to point to our per-CPU data
    super::hartid::set_tp(percpu as usize);
}

/// Get per-CPU structure for a specific hart
pub fn get_percpu(hart_id: usize) -> Option<&'static PerCpu> {
    if hart_id >= MAX_HARTS {
        return None;
    }

    if !PERCPU_INIT[hart_id].load(Ordering::Acquire) {
        return None;
    }

    let ptr = PERCPU_PTRS[hart_id].load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &*ptr })
    }
}

/// Get mutable per-CPU structure for a specific hart
///
/// # Safety
/// Caller must ensure exclusive access.
pub unsafe fn get_percpu_mut(hart_id: usize) -> Option<&'static mut PerCpu> {
    if hart_id >= MAX_HARTS {
        return None;
    }

    if !PERCPU_INIT[hart_id].load(Ordering::Acquire) {
        return None;
    }

    let ptr = PERCPU_PTRS[hart_id].load(Ordering::Acquire);
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

// ============================================================================
// Per-CPU Reference Type
// ============================================================================

/// Reference to per-CPU data
pub struct PerCpuRef<T> {
    offset: usize,
    _marker: core::marker::PhantomData<T>,
}

impl<T> PerCpuRef<T> {
    /// Create a new per-CPU reference from an offset
    pub const fn from_offset(offset: usize) -> Self {
        Self {
            offset,
            _marker: core::marker::PhantomData,
        }
    }

    /// Get a reference to the data for the current hart
    #[inline]
    pub fn get(&self) -> &T {
        let base = get_tp();
        unsafe { &*((base + self.offset) as *const T) }
    }

    /// Get a mutable reference to the data for the current hart
    ///
    /// # Safety
    /// Caller must ensure no other references exist.
    #[inline]
    pub unsafe fn get_mut(&self) -> &mut T {
        let base = get_tp();
        &mut *((base + self.offset) as *mut T)
    }

    /// Get a reference to the data for a specific hart
    pub fn get_for(&self, hart_id: usize) -> Option<&T> {
        get_percpu(hart_id).map(|percpu| {
            let base = percpu as *const PerCpu as usize;
            unsafe { &*((base + self.offset) as *const T) }
        })
    }
}

// ============================================================================
// Preemption Control
// ============================================================================

/// Disable preemption and return a guard
pub fn preempt_disable() -> PreemptGuard {
    let percpu = unsafe { PerCpu::current_mut() };
    percpu.preempt_disable();
    PreemptGuard { _private: () }
}

/// Guard that re-enables preemption when dropped
pub struct PreemptGuard {
    _private: (),
}

impl Drop for PreemptGuard {
    fn drop(&mut self) {
        let percpu = unsafe { PerCpu::current_mut() };
        percpu.preempt_enable();
    }
}

// ============================================================================
// Per-CPU Variable Macro Support
// ============================================================================

/// Per-CPU variable storage
pub struct PerCpuVar<T> {
    data: UnsafeCell<[MaybeUninit<T>; MAX_HARTS]>,
    init: [AtomicBool; MAX_HARTS],
}

impl<T> PerCpuVar<T> {
    /// Create a new uninitialized per-CPU variable
    pub const fn new() -> Self {
        const FALSE: AtomicBool = AtomicBool::new(false);
        Self {
            data: UnsafeCell::new({
                // Safety: array of MaybeUninit doesn't need initialization
                unsafe { MaybeUninit::uninit().assume_init() }
            }),
            init: [FALSE; MAX_HARTS],
        }
    }

    /// Initialize for the current hart
    ///
    /// # Safety
    /// Must only be called once per hart.
    pub unsafe fn init(&self, value: T) {
        let hart_id = get_hart_id();
        if hart_id < MAX_HARTS {
            (*self.data.get())[hart_id].write(value);
            self.init[hart_id].store(true, Ordering::Release);
        }
    }

    /// Get a reference for the current hart
    pub fn get(&self) -> Option<&T> {
        let hart_id = get_hart_id();
        if hart_id < MAX_HARTS && self.init[hart_id].load(Ordering::Acquire) {
            Some(unsafe { (*self.data.get())[hart_id].assume_init_ref() })
        } else {
            None
        }
    }

    /// Get a mutable reference for the current hart
    ///
    /// # Safety
    /// Caller must ensure exclusive access.
    pub unsafe fn get_mut(&self) -> Option<&mut T> {
        let hart_id = get_hart_id();
        if hart_id < MAX_HARTS && self.init[hart_id].load(Ordering::Acquire) {
            Some((*self.data.get())[hart_id].assume_init_mut())
        } else {
            None
        }
    }
}

// SAFETY: PerCpuVar is Sync because each hart only accesses its own slot
unsafe impl<T: Send> Sync for PerCpuVar<T> {}
unsafe impl<T: Send> Send for PerCpuVar<T> {}

impl<T> Default for PerCpuVar<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Statistics and Debugging
// ============================================================================

/// Dump per-CPU state for debugging
pub fn dump_percpu_state(hart_id: usize) {
    if let Some(percpu) = get_percpu(hart_id) {
        // In a real implementation, this would log the state
        let _ = percpu;
    }
}

/// Get per-CPU statistics
#[derive(Debug, Clone)]
pub struct PerCpuStats {
    pub hart_id: usize,
    pub time_in_kernel: u64,
    pub time_in_user: u64,
    pub time_in_interrupt: u64,
    pub interrupt_depth: usize,
    pub preempt_count: usize,
}

/// Get stats for a specific hart
pub fn get_percpu_stats(hart_id: usize) -> Option<PerCpuStats> {
    get_percpu(hart_id).map(|percpu| PerCpuStats {
        hart_id: percpu.hart_id,
        time_in_kernel: percpu.time_in_kernel,
        time_in_user: percpu.time_in_user,
        time_in_interrupt: percpu.time_in_interrupt,
        interrupt_depth: percpu.interrupt_depth,
        preempt_count: percpu.preempt_count,
    })
}

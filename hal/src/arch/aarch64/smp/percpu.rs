//! # Per-CPU Data Management
//!
//! This module provides infrastructure for per-CPU data storage on AArch64
//! systems, utilizing the TPIDR_EL1 register for efficient access.
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────────┐
//! │                      Per-CPU Data Architecture                        │
//! ├───────────────────────────────────────────────────────────────────────┤
//! │                                                                       │
//! │   CPU 0                  CPU 1                  CPU N                 │
//! │  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐     │
//! │  │  TPIDR_EL1 ────▶│   │  TPIDR_EL1 ────▶│   │  TPIDR_EL1 ────▶│     │
//! │  │                 │   │                 │   │                 │     │
//! │  │ ┌─────────────┐ │   │ ┌─────────────┐ │   │ ┌─────────────┐ │     │
//! │  │ │ PerCpuData  │ │   │ │ PerCpuData  │ │   │ │ PerCpuData  │ │     │
//! │  │ │             │ │   │ │             │ │   │ │             │ │     │
//! │  │ │ cpu_id      │ │   │ │ cpu_id      │ │   │ │ cpu_id      │ │     │
//! │  │ │ mpidr       │ │   │ │ mpidr       │ │   │ │ mpidr       │ │     │
//! │  │ │ stack_top   │ │   │ │ stack_top   │ │   │ │ stack_top   │ │     │
//! │  │ │ current_task│ │   │ │ current_task│ │   │ │ current_task│ │     │
//! │  │ │ irq_count   │ │   │ │ irq_count   │ │   │ │ irq_count   │ │     │
//! │  │ │ preempt_cnt │ │   │ │ preempt_cnt │ │   │ │ preempt_cnt │ │     │
//! │  │ │ ...         │ │   │ │ ...         │ │   │ │ ...         │ │     │
//! │  │ └─────────────┘ │   │ └─────────────┘ │   │ └─────────────┘ │     │
//! │  └─────────────────┘   └─────────────────┘   └─────────────────┘     │
//! │                                                                       │
//! │  TPIDR_EL1: Thread Pointer ID Register (EL1)                         │
//! │  - Fast single-instruction access                                     │
//! │  - Per-CPU by design (each CPU has its own)                          │
//! │  - Not affected by context switches                                  │
//! │                                                                       │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage Patterns
//!
//! ### Static Per-CPU Variables
//!
//! ```ignore
//! // Define a per-CPU variable
//! per_cpu_define!(MY_COUNTER: AtomicU64 = AtomicU64::new(0));
//!
//! // Access current CPU's counter
//! MY_COUNTER.with(|c| c.fetch_add(1, Ordering::Relaxed));
//! ```
//!
//! ### Direct Access
//!
//! ```ignore
//! // Get current CPU's data
//! let percpu = PerCpuData::current();
//! let cpu_id = percpu.cpu_id;
//! ```

use core::arch::asm;
use core::cell::UnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

use super::Mpidr;

// ============================================================================
// TPIDR Register Access
// ============================================================================

/// Read TPIDR_EL1 (Thread Pointer ID Register for EL1)
#[inline]
pub fn read_tpidr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, tpidr_el1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TPIDR_EL1
#[inline]
pub fn write_tpidr_el1(value: u64) {
    unsafe {
        asm!("msr tpidr_el1, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Read TPIDR_EL0 (Thread Pointer ID Register for EL0, user-accessible)
#[inline]
pub fn read_tpidr_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, tpidr_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TPIDR_EL0
#[inline]
pub fn write_tpidr_el0(value: u64) {
    unsafe {
        asm!("msr tpidr_el0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

/// Read TPIDRRO_EL0 (Read-Only Thread Pointer for EL0)
#[inline]
pub fn read_tpidrro_el0() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, tpidrro_el0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Write TPIDRRO_EL0 (from EL1)
#[inline]
pub fn write_tpidrro_el0(value: u64) {
    unsafe {
        asm!("msr tpidrro_el0, {}", in(reg) value, options(nomem, nostack, preserves_flags));
    }
}

// ============================================================================
// Per-CPU Data Structure
// ============================================================================

/// Per-CPU data block
///
/// This structure contains all per-CPU state. It is allocated for each CPU
/// and pointed to by TPIDR_EL1.
#[repr(C)]
pub struct PerCpuData {
    // ========================================================================
    // Critical fields (accessed frequently, keep at top)
    // ========================================================================
    /// Self pointer for validation
    pub self_ptr: *mut PerCpuData,

    /// CPU ID (linear, 0-based)
    pub cpu_id: u32,

    /// MPIDR value for this CPU
    pub mpidr: u64,

    /// Current task pointer (kernel-dependent)
    pub current_task: AtomicUsize,

    /// Preemption disable count
    pub preempt_count: AtomicU32,

    /// IRQ nesting count
    pub irq_count: AtomicU32,

    // ========================================================================
    // Stack information
    // ========================================================================
    /// Kernel stack top (initial SP)
    pub stack_top: usize,

    /// Kernel stack bottom (guard page boundary)
    pub stack_bottom: usize,

    /// IRQ stack top (separate stack for interrupts)
    pub irq_stack_top: usize,

    // ========================================================================
    // Scheduler state
    // ========================================================================
    /// CPU is idle
    pub is_idle: bool,

    /// Need to reschedule
    pub need_resched: bool,

    /// Scheduler ticks
    pub tick_count: AtomicU64,

    // ========================================================================
    // Interrupt handling
    // ========================================================================
    /// Last acknowledged interrupt ID
    pub last_irq: u32,

    /// Interrupt statistics
    pub irq_stats: IrqStats,

    // ========================================================================
    // Timer state
    // ========================================================================
    /// Timer deadline (timer compare value)
    pub timer_deadline: u64,

    /// Timer frequency (from CNTFRQ_EL0)
    pub timer_freq: u64,

    // ========================================================================
    // FPU/SIMD state
    // ========================================================================
    /// Current FPU owner (task pointer)
    pub fpu_owner: AtomicUsize,

    /// FPU is enabled for current task
    pub fpu_enabled: bool,

    // ========================================================================
    // TLB shootdown
    // ========================================================================
    /// TLB shootdown pending
    pub tlb_shootdown_pending: bool,

    /// TLB flush address (or 0 for full flush)
    pub tlb_flush_addr: AtomicU64,

    // ========================================================================
    // Platform-specific extension
    // ========================================================================
    /// Platform-specific data pointer
    pub platform_data: *mut u8,

    /// Padding to cache line alignment
    _pad: [u8; 16],
}

/// IRQ statistics per CPU
#[repr(C)]
#[derive(Default)]
pub struct IrqStats {
    /// Total interrupts handled
    pub total_count: AtomicU64,
    /// SGIs received
    pub sgi_count: AtomicU64,
    /// PPIs received
    pub ppi_count: AtomicU64,
    /// SPIs received
    pub spi_count: AtomicU64,
    /// Spurious interrupts
    pub spurious_count: AtomicU64,
}

impl PerCpuData {
    /// Size of per-CPU data (aligned to cache line)
    pub const SIZE: usize = 256; // Adjust based on actual struct size

    /// Create a new per-CPU data block (uninitialized)
    pub const fn new() -> Self {
        Self {
            self_ptr: core::ptr::null_mut(),
            cpu_id: 0,
            mpidr: 0,
            current_task: AtomicUsize::new(0),
            preempt_count: AtomicU32::new(0),
            irq_count: AtomicU32::new(0),
            stack_top: 0,
            stack_bottom: 0,
            irq_stack_top: 0,
            is_idle: true,
            need_resched: false,
            tick_count: AtomicU64::new(0),
            last_irq: 0,
            irq_stats: IrqStats {
                total_count: AtomicU64::new(0),
                sgi_count: AtomicU64::new(0),
                ppi_count: AtomicU64::new(0),
                spi_count: AtomicU64::new(0),
                spurious_count: AtomicU64::new(0),
            },
            timer_deadline: 0,
            timer_freq: 0,
            fpu_owner: AtomicUsize::new(0),
            fpu_enabled: false,
            tlb_shootdown_pending: false,
            tlb_flush_addr: AtomicU64::new(0),
            platform_data: core::ptr::null_mut(),
            _pad: [0; 16],
        }
    }

    /// Initialize per-CPU data for a specific CPU
    pub fn init(&mut self, cpu_id: u32, stack_top: usize, stack_size: usize) {
        let mpidr = if cpu_id == 0 {
            Mpidr::current().value()
        } else {
            0 // Will be set when CPU starts
        };

        self.self_ptr = self as *mut PerCpuData;
        self.cpu_id = cpu_id;
        self.mpidr = mpidr;
        self.stack_top = stack_top;
        self.stack_bottom = stack_top.saturating_sub(stack_size);
        self.is_idle = true;
        self.need_resched = false;

        // Read timer frequency
        let freq: u64;
        unsafe {
            asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack));
        }
        self.timer_freq = freq;
    }

    /// Get the current CPU's per-CPU data
    ///
    /// # Safety
    ///
    /// Per-CPU data must have been initialized for this CPU.
    #[inline]
    pub unsafe fn current() -> &'static mut Self {
        let ptr = read_tpidr_el1() as *mut PerCpuData;
        debug_assert!(!ptr.is_null(), "Per-CPU data not initialized");
        &mut *ptr
    }

    /// Try to get current per-CPU data (returns None if not initialized)
    #[inline]
    pub fn try_current() -> Option<&'static mut Self> {
        let ptr = read_tpidr_el1() as *mut PerCpuData;
        if ptr.is_null() {
            None
        } else {
            unsafe {
                let data = &mut *ptr;
                // Validate self pointer
                if data.self_ptr == ptr {
                    Some(data)
                } else {
                    None
                }
            }
        }
    }

    /// Check if per-CPU data is initialized
    #[inline]
    pub fn is_initialized() -> bool {
        let ptr = read_tpidr_el1();
        ptr != 0
    }

    /// Get the current CPU ID
    #[inline]
    pub fn current_cpu_id() -> u32 {
        if Self::is_initialized() {
            unsafe { Self::current().cpu_id }
        } else {
            // Fall back to MPIDR
            Mpidr::current().flat_id()
        }
    }

    // ========================================================================
    // Preemption Control
    // ========================================================================

    /// Disable preemption
    #[inline]
    pub fn preempt_disable(&self) {
        self.preempt_count.fetch_add(1, Ordering::Relaxed);
        // Barrier to ensure the increment is visible before continuing
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
    }

    /// Enable preemption
    #[inline]
    pub fn preempt_enable(&self) {
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
        let old = self.preempt_count.fetch_sub(1, Ordering::Relaxed);
        debug_assert!(old > 0, "Preemption count underflow");
    }

    /// Check if preemption is enabled
    #[inline]
    pub fn preemption_enabled(&self) -> bool {
        self.preempt_count.load(Ordering::Relaxed) == 0
    }

    /// Get preemption count
    #[inline]
    pub fn preempt_count(&self) -> u32 {
        self.preempt_count.load(Ordering::Relaxed)
    }

    // ========================================================================
    // IRQ Tracking
    // ========================================================================

    /// Enter IRQ context
    #[inline]
    pub fn irq_enter(&self) {
        self.irq_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Exit IRQ context
    #[inline]
    pub fn irq_exit(&self) {
        let old = self.irq_count.fetch_sub(1, Ordering::Relaxed);
        debug_assert!(old > 0, "IRQ count underflow");
    }

    /// Check if in IRQ context
    #[inline]
    pub fn in_irq(&self) -> bool {
        self.irq_count.load(Ordering::Relaxed) > 0
    }

    /// Get IRQ nesting depth
    #[inline]
    pub fn irq_depth(&self) -> u32 {
        self.irq_count.load(Ordering::Relaxed)
    }

    // ========================================================================
    // Scheduler Helpers
    // ========================================================================

    /// Request a reschedule
    #[inline]
    pub fn set_need_resched(&mut self) {
        self.need_resched = true;
    }

    /// Clear reschedule flag
    #[inline]
    pub fn clear_need_resched(&mut self) {
        self.need_resched = false;
    }

    /// Increment tick counter
    #[inline]
    pub fn tick(&self) {
        self.tick_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get tick count
    #[inline]
    pub fn ticks(&self) -> u64 {
        self.tick_count.load(Ordering::Relaxed)
    }
}

impl Default for PerCpuData {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: PerCpuData is only accessed from its owning CPU
unsafe impl Send for PerCpuData {}
unsafe impl Sync for PerCpuData {}

// ============================================================================
// Per-CPU Array
// ============================================================================

/// Maximum CPUs supported
pub const MAX_CPUS: usize = super::MAX_CPUS;

/// Array of per-CPU data pointers
static mut PERCPU_PTRS: [*mut PerCpuData; MAX_CPUS] = [core::ptr::null_mut(); MAX_CPUS];

/// Get per-CPU data for a specific CPU
///
/// # Safety
///
/// The CPU must have been initialized.
pub unsafe fn get_percpu(cpu_id: u32) -> Option<&'static mut PerCpuData> {
    let ptr = PERCPU_PTRS.get(cpu_id as usize).copied()?;
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

/// Register per-CPU data for a CPU
///
/// # Safety
///
/// Must only be called once per CPU during initialization.
pub unsafe fn register_percpu(cpu_id: u32, data: *mut PerCpuData) {
    if (cpu_id as usize) < MAX_CPUS {
        PERCPU_PTRS[cpu_id as usize] = data;
    }
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize per-CPU data for the BSP (Boot Processor)
pub fn init_percpu_bsp() {
    // For BSP, we use a static allocation or early memory allocation
    // In a real kernel, this would allocate from early boot memory

    static mut BSP_PERCPU: PerCpuData = PerCpuData::new();

    unsafe {
        // Initialize BSP per-CPU data
        BSP_PERCPU.init(0, 0, 0); // Stack info will be set separately
        BSP_PERCPU.mpidr = Mpidr::current().value();

        // Register and activate
        register_percpu(0, &mut BSP_PERCPU);
        write_tpidr_el1(&BSP_PERCPU as *const _ as u64);
    }
}

/// Initialize per-CPU data for an AP (Application Processor)
///
/// # Safety
///
/// Must be called from the AP's initialization path with its CPU ID.
pub unsafe fn init_percpu_ap(cpu_id: u32) {
    if let Some(data) = get_percpu(cpu_id) {
        // Set MPIDR now that we're running on this CPU
        data.mpidr = Mpidr::current().value();
        data.self_ptr = data as *mut PerCpuData;

        // Activate per-CPU data
        write_tpidr_el1(data as *const _ as u64);
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Get the current CPU ID
#[inline]
pub fn current_cpu_id() -> u32 {
    PerCpuData::current_cpu_id()
}

/// Check if we're in interrupt context
#[inline]
pub fn in_interrupt() -> bool {
    if let Some(data) = PerCpuData::try_current() {
        data.in_irq()
    } else {
        false
    }
}

/// Check if preemption is disabled
#[inline]
pub fn preempt_disabled() -> bool {
    if let Some(data) = PerCpuData::try_current() {
        !data.preemption_enabled()
    } else {
        true // Assume disabled if not initialized
    }
}

/// Disable preemption
#[inline]
pub fn preempt_disable() {
    if let Some(data) = PerCpuData::try_current() {
        data.preempt_disable();
    }
}

/// Enable preemption
#[inline]
pub fn preempt_enable() {
    if let Some(data) = PerCpuData::try_current() {
        data.preempt_enable();
    }
}

// ============================================================================
// Per-CPU Variable Macro Support
// ============================================================================

/// A per-CPU variable wrapper
pub struct PerCpuVar<T> {
    /// Array of values, one per CPU
    values: UnsafeCell<[T; MAX_CPUS]>,
}

impl<T: Copy + Default> PerCpuVar<T> {
    /// Create a new per-CPU variable with default values
    pub const fn new() -> Self {
        // This is a const fn workaround for array initialization
        Self {
            values: UnsafeCell::new(unsafe { core::mem::zeroed() }),
        }
    }

    /// Get the value for the current CPU
    #[inline]
    pub fn get(&self) -> T {
        let cpu_id = current_cpu_id() as usize;
        unsafe { (*self.values.get())[cpu_id] }
    }

    /// Set the value for the current CPU
    #[inline]
    pub fn set(&self, value: T) {
        let cpu_id = current_cpu_id() as usize;
        unsafe { (*self.values.get())[cpu_id] = value };
    }

    /// Get the value for a specific CPU
    #[inline]
    pub fn get_cpu(&self, cpu_id: u32) -> T {
        unsafe { (*self.values.get())[cpu_id as usize] }
    }

    /// Set the value for a specific CPU
    #[inline]
    pub fn set_cpu(&self, cpu_id: u32, value: T) {
        unsafe { (*self.values.get())[cpu_id as usize] = value };
    }

    /// Apply a function to the current CPU's value
    #[inline]
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let cpu_id = current_cpu_id() as usize;
        unsafe { f(&mut (*self.values.get())[cpu_id]) }
    }
}

// Safety: Each CPU only accesses its own element
unsafe impl<T: Send> Send for PerCpuVar<T> {}
unsafe impl<T: Send> Sync for PerCpuVar<T> {}

/// Macro to define a per-CPU variable
#[macro_export]
macro_rules! per_cpu {
    ($name:ident : $ty:ty = $init:expr) => {
        #[allow(non_upper_case_globals)]
        static $name: $crate::arch::aarch64::smp::percpu::PerCpuVar<$ty> =
            $crate::arch::aarch64::smp::percpu::PerCpuVar::new();
    };
}

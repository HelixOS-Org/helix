//! # Per-CPU Data Management
//!
//! This module provides mechanisms for storing and accessing per-CPU data
//! using the GS segment base register.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Per-CPU Data Layout                      │
//! ├─────────────────────────────────────────────────────────────┤
//! │ GS:0x00  │ Self pointer (for verification)                  │
//! │ GS:0x08  │ CPU ID                                           │
//! │ GS:0x10  │ APIC ID                                          │
//! │ GS:0x18  │ Current task pointer                             │
//! │ GS:0x20  │ Kernel stack top                                 │
//! │ GS:0x28  │ User stack (saved during syscall)                │
//! │ GS:0x30  │ TSS pointer                                      │
//! │ GS:0x38  │ Interrupt nesting level                          │
//! │ GS:0x40  │ Preemption count                                 │
//! │ GS:0x48  │ Flags (in_irq, need_resched, etc.)               │
//! │ GS:0x50+ │ Extension area                                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::{SmpError, MAX_CPUS};

// =============================================================================
// Constants
// =============================================================================

/// Per-CPU area magic value
const PERCPU_MAGIC: u64 = 0x5045_5243_5055_5F5F; // "PERCPU__"

/// Per-CPU area size
pub const PERCPU_AREA_SIZE: usize = 4096;

/// Offset of self pointer
pub const PERCPU_SELF_OFFSET: usize = 0x00;
/// Offset of CPU ID
pub const PERCPU_CPU_ID_OFFSET: usize = 0x08;
/// Offset of APIC ID
pub const PERCPU_APIC_ID_OFFSET: usize = 0x10;
/// Offset of current task pointer
pub const PERCPU_CURRENT_TASK_OFFSET: usize = 0x18;
/// Offset of kernel stack top
pub const PERCPU_KERNEL_STACK_OFFSET: usize = 0x20;
/// Offset of user stack (saved)
pub const PERCPU_USER_STACK_OFFSET: usize = 0x28;
/// Offset of TSS pointer
pub const PERCPU_TSS_OFFSET: usize = 0x30;
/// Offset of interrupt nesting level
pub const PERCPU_IRQ_NESTING_OFFSET: usize = 0x38;
/// Offset of preemption count
pub const PERCPU_PREEMPT_COUNT_OFFSET: usize = 0x40;
/// Offset of flags
pub const PERCPU_FLAGS_OFFSET: usize = 0x48;
/// Offset of extension area
pub const PERCPU_EXTENSION_OFFSET: usize = 0x50;

// =============================================================================
// Per-CPU Data Structure
// =============================================================================

/// Per-CPU data block
#[repr(C, align(4096))]
pub struct PerCpuData {
    /// Self pointer (GS:0x00)
    pub self_ptr: u64,
    /// CPU ID (GS:0x08)
    pub cpu_id: u64,
    /// APIC ID (GS:0x10)
    pub apic_id: u64,
    /// Current task pointer (GS:0x18)
    pub current_task: AtomicU64,
    /// Kernel stack top (GS:0x20)
    pub kernel_stack: u64,
    /// User stack saved during syscall (GS:0x28)
    pub user_stack: AtomicU64,
    /// TSS pointer (GS:0x30)
    pub tss_ptr: u64,
    /// Interrupt nesting level (GS:0x38)
    pub irq_nesting: AtomicU64,
    /// Preemption count (GS:0x40)
    pub preempt_count: AtomicU64,
    /// Flags (GS:0x48)
    pub flags: AtomicU64,
    /// Magic value for validation
    pub magic: u64,
    /// Reserved/padding
    _reserved: [u8; PERCPU_AREA_SIZE - 0x60],
}

// Per-CPU flags
bitflags::bitflags! {
    /// Per-CPU flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PerCpuFlags: u64 {
        /// Currently in interrupt handler
        const IN_IRQ = 1 << 0;
        /// Need to reschedule
        const NEED_RESCHED = 1 << 1;
        /// In NMI handler
        const IN_NMI = 1 << 2;
        /// In exception handler
        const IN_EXCEPTION = 1 << 3;
        /// Preemption disabled
        const PREEMPT_DISABLED = 1 << 4;
        /// CPU is idle
        const IDLE = 1 << 5;
        /// CPU is halted
        const HALTED = 1 << 6;
    }
}

impl PerCpuData {
    /// Create a new per-CPU data block
    pub const fn new() -> Self {
        Self {
            self_ptr: 0,
            cpu_id: 0,
            apic_id: 0,
            current_task: AtomicU64::new(0),
            kernel_stack: 0,
            user_stack: AtomicU64::new(0),
            tss_ptr: 0,
            irq_nesting: AtomicU64::new(0),
            preempt_count: AtomicU64::new(0),
            flags: AtomicU64::new(0),
            magic: PERCPU_MAGIC,
            _reserved: [0; PERCPU_AREA_SIZE - 0x60],
        }
    }

    /// Initialize per-CPU data
    pub fn init(&mut self, cpu_id: u32, apic_id: u32) {
        self.self_ptr = self as *const _ as u64;
        self.cpu_id = cpu_id as u64;
        self.apic_id = apic_id as u64;
        self.magic = PERCPU_MAGIC;
    }

    /// Validate the per-CPU data
    pub fn is_valid(&self) -> bool {
        self.magic == PERCPU_MAGIC && self.self_ptr == self as *const _ as u64
    }

    /// Get flags
    pub fn get_flags(&self) -> PerCpuFlags {
        PerCpuFlags::from_bits_truncate(self.flags.load(Ordering::Relaxed))
    }

    /// Set flag
    pub fn set_flag(&self, flag: PerCpuFlags) {
        self.flags.fetch_or(flag.bits(), Ordering::SeqCst);
    }

    /// Clear flag
    pub fn clear_flag(&self, flag: PerCpuFlags) {
        self.flags.fetch_and(!flag.bits(), Ordering::SeqCst);
    }

    /// Enter interrupt
    pub fn enter_irq(&self) {
        self.irq_nesting.fetch_add(1, Ordering::SeqCst);
        self.set_flag(PerCpuFlags::IN_IRQ);
    }

    /// Exit interrupt
    pub fn exit_irq(&self) {
        let prev = self.irq_nesting.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            self.clear_flag(PerCpuFlags::IN_IRQ);
        }
    }

    /// Check if in interrupt context
    pub fn in_irq(&self) -> bool {
        self.irq_nesting.load(Ordering::Relaxed) > 0
    }

    /// Disable preemption
    pub fn preempt_disable(&self) {
        self.preempt_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Enable preemption
    pub fn preempt_enable(&self) {
        let prev = self.preempt_count.fetch_sub(1, Ordering::SeqCst);
        debug_assert!(prev > 0, "preempt_enable() without preempt_disable()");
    }

    /// Check if preemption is enabled
    pub fn preemptible(&self) -> bool {
        self.preempt_count.load(Ordering::Relaxed) == 0 && !self.in_irq()
    }
}

impl Default for PerCpuData {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Global Per-CPU Array
// =============================================================================

/// Global per-CPU data array
static mut PERCPU_DATA: [PerCpuData; MAX_CPUS] = [const { PerCpuData::new() }; MAX_CPUS];

/// Per-CPU initialization status
static PERCPU_INITIALIZED: [AtomicU32; MAX_CPUS] = [const { AtomicU32::new(0) }; MAX_CPUS];

/// Get per-CPU data for a specific CPU
///
/// # Safety
/// This returns a mutable reference. Caller must ensure proper synchronization.
pub unsafe fn get_percpu_data(cpu_id: usize) -> Option<&'static mut PerCpuData> {
    if cpu_id < MAX_CPUS && PERCPU_INITIALIZED[cpu_id].load(Ordering::Acquire) != 0 {
        Some(&mut PERCPU_DATA[cpu_id])
    } else {
        None
    }
}

/// Get per-CPU data pointer for a specific CPU
pub fn get_percpu_ptr(cpu_id: usize) -> Option<*mut PerCpuData> {
    if cpu_id < MAX_CPUS {
        Some(unsafe { &raw mut PERCPU_DATA[cpu_id] })
    } else {
        None
    }
}

/// Initialize per-CPU data for BSP
pub fn init_bsp(apic_id: u32) -> Result<(), SmpError> {
    init_percpu(0, apic_id)?;
    set_gs_base(unsafe { &raw mut PERCPU_DATA[0] } as u64);
    Ok(())
}

/// Initialize per-CPU data for an AP
pub fn init_ap(cpu_id: usize, apic_id: u32) -> Result<(), SmpError> {
    init_percpu(cpu_id, apic_id)?;
    set_gs_base(unsafe { &raw mut PERCPU_DATA[cpu_id] } as u64);
    Ok(())
}

/// Initialize per-CPU data
fn init_percpu(cpu_id: usize, apic_id: u32) -> Result<(), SmpError> {
    if cpu_id >= MAX_CPUS {
        return Err(SmpError::InvalidCpuId);
    }

    unsafe {
        PERCPU_DATA[cpu_id].init(cpu_id as u32, apic_id);
    }

    PERCPU_INITIALIZED[cpu_id].store(1, Ordering::Release);

    Ok(())
}

// =============================================================================
// GS Base Management
// =============================================================================

/// IA32_GS_BASE MSR
const IA32_GS_BASE: u32 = 0xC000_0101;

/// IA32_KERNEL_GS_BASE MSR
const IA32_KERNEL_GS_BASE: u32 = 0xC000_0102;

/// Set GS base register
pub fn set_gs_base(base: u64) {
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") IA32_GS_BASE,
            in("eax") base as u32,
            in("edx") (base >> 32) as u32,
            options(nostack, preserves_flags),
        );
    }
}

/// Get GS base register
pub fn get_gs_base() -> u64 {
    let (lo, hi): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") IA32_GS_BASE,
            out("eax") lo,
            out("edx") hi,
            options(nostack, preserves_flags),
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

/// Set kernel GS base (for SWAPGS)
pub fn set_kernel_gs_base(base: u64) {
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") IA32_KERNEL_GS_BASE,
            in("eax") base as u32,
            in("edx") (base >> 32) as u32,
            options(nostack, preserves_flags),
        );
    }
}

/// Get kernel GS base
pub fn get_kernel_gs_base() -> u64 {
    let (lo, hi): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") IA32_KERNEL_GS_BASE,
            out("eax") lo,
            out("edx") hi,
            options(nostack, preserves_flags),
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

/// Swap GS and kernel GS base
#[inline]
pub fn swap_gs() {
    unsafe {
        core::arch::asm!("swapgs", options(nostack, preserves_flags));
    }
}

// =============================================================================
// Fast Per-CPU Access
// =============================================================================

/// Get current CPU ID from GS segment
#[inline]
pub fn current_cpu_id() -> u32 {
    let id: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, gs:[{}]",
            out(reg) id,
            const PERCPU_CPU_ID_OFFSET,
            options(nostack, readonly),
        );
    }
    id as u32
}

/// Get current APIC ID from GS segment
#[inline]
pub fn current_apic_id() -> u32 {
    let id: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, gs:[{}]",
            out(reg) id,
            const PERCPU_APIC_ID_OFFSET,
            options(nostack, readonly),
        );
    }
    id as u32
}

/// Get current task pointer from GS segment
#[inline]
pub fn current_task_ptr() -> u64 {
    let ptr: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, gs:[{}]",
            out(reg) ptr,
            const PERCPU_CURRENT_TASK_OFFSET,
            options(nostack, readonly),
        );
    }
    ptr
}

/// Set current task pointer
#[inline]
pub fn set_current_task_ptr(ptr: u64) {
    unsafe {
        core::arch::asm!(
            "mov gs:[{}], {}",
            const PERCPU_CURRENT_TASK_OFFSET,
            in(reg) ptr,
            options(nostack),
        );
    }
}

/// Get current per-CPU data
#[inline]
pub fn current_percpu() -> Option<&'static PerCpuData> {
    let ptr: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, gs:[{}]",
            out(reg) ptr,
            const PERCPU_SELF_OFFSET,
            options(nostack, readonly),
        );
    }

    if ptr != 0 {
        let data = unsafe { &*(ptr as *const PerCpuData) };
        if data.is_valid() {
            return Some(data);
        }
    }
    None
}

// =============================================================================
// Per-CPU Variable
// =============================================================================

/// Per-CPU variable wrapper
///
/// Provides safe access to per-CPU data with type safety.
pub struct PerCpu<T> {
    /// Per-CPU values
    data: [UnsafeCell<T>; MAX_CPUS],
}

unsafe impl<T: Send> Send for PerCpu<T> {}
unsafe impl<T: Send> Sync for PerCpu<T> {}

impl<T: Default + Copy> PerCpu<T> {
    /// Create a new per-CPU variable with default values
    pub const fn new() -> Self
    where
        T: ~const Default,
    {
        Self {
            data: [const { UnsafeCell::new(T::default()) }; MAX_CPUS],
        }
    }

    /// Create a new per-CPU variable with a specific value
    pub const fn with_value(value: T) -> Self {
        Self {
            data: [const { UnsafeCell::new(value) }; MAX_CPUS],
        }
    }
}

impl<T> PerCpu<T> {
    /// Get reference to current CPU's value
    #[inline]
    pub fn get(&self) -> &T {
        let cpu_id = current_cpu_id() as usize;
        debug_assert!(cpu_id < MAX_CPUS);
        unsafe { &*self.data[cpu_id].get() }
    }

    /// Get mutable reference to current CPU's value
    ///
    /// # Safety
    /// Caller must ensure exclusive access (e.g., via preemption disable)
    #[inline]
    pub unsafe fn get_mut(&self) -> &mut T {
        let cpu_id = current_cpu_id() as usize;
        debug_assert!(cpu_id < MAX_CPUS);
        &mut *self.data[cpu_id].get()
    }

    /// Get reference to a specific CPU's value
    pub fn get_cpu(&self, cpu_id: usize) -> Option<&T> {
        if cpu_id < MAX_CPUS {
            Some(unsafe { &*self.data[cpu_id].get() })
        } else {
            None
        }
    }

    /// Get mutable reference to a specific CPU's value
    ///
    /// # Safety
    /// Caller must ensure exclusive access
    pub unsafe fn get_cpu_mut(&self, cpu_id: usize) -> Option<&mut T> {
        if cpu_id < MAX_CPUS {
            Some(&mut *self.data[cpu_id].get())
        } else {
            None
        }
    }
}

/// Per-CPU reference guard
///
/// RAII guard for per-CPU access with preemption disabled
pub struct PerCpuRef<'a, T> {
    value: &'a T,
    _marker: PhantomData<*const ()>,
}

impl<'a, T> PerCpuRef<'a, T> {
    /// Create a new per-CPU reference with preemption disabled
    pub fn new(percpu: &'a PerCpu<T>) -> Self {
        // Disable preemption
        if let Some(data) = current_percpu() {
            data.preempt_disable();
        }

        Self {
            value: percpu.get(),
            _marker: PhantomData,
        }
    }
}

impl<T> core::ops::Deref for PerCpuRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> Drop for PerCpuRef<'_, T> {
    fn drop(&mut self) {
        // Re-enable preemption
        if let Some(data) = current_percpu() {
            data.preempt_enable();
        }
    }
}

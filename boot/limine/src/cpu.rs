//! # CPU Topology and SMP Support
//!
//! This module provides CPU abstraction and symmetric multi-processing
//! support for multi-core initialization and management.
//!
//! ## Features
//!
//! - CPU enumeration and identification
//! - Application Processor (AP) startup
//! - Per-CPU data management
//! - CPU synchronization primitives

use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use crate::requests::SmpResponse;

/// Maximum supported CPUs
pub const MAX_CPUS: usize = 256;

/// CPU state storage
static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);
static BSP_ID: AtomicU64 = AtomicU64::new(0);
static CPUS_STARTED: AtomicUsize = AtomicUsize::new(0);

/// Initialize CPU subsystem from SMP response
pub fn init_from_smp(smp: &SmpResponse) {
    CPU_COUNT.store(smp.cpu_count(), Ordering::Release);
    BSP_ID.store(smp.bsp_lapic_id(), Ordering::Release);
    CPUS_STARTED.store(1, Ordering::Release); // BSP is started
}

/// Get the total CPU count
pub fn cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Acquire)
}

/// Get the BSP LAPIC ID
pub fn bsp_id() -> u64 {
    BSP_ID.load(Ordering::Acquire)
}

/// Get the number of CPUs that have started
pub fn cpus_started() -> usize {
    CPUS_STARTED.load(Ordering::Acquire)
}

/// Increment the started CPU counter
pub fn mark_cpu_started() {
    CPUS_STARTED.fetch_add(1, Ordering::AcqRel);
}

/// Check if all CPUs have started
pub fn all_cpus_started() -> bool {
    cpus_started() >= cpu_count()
}

// =============================================================================
// CPU Identification
// =============================================================================

/// Get the current CPU ID
///
/// On x86_64, this uses CPUID to get the APIC ID.
#[cfg(target_arch = "x86_64")]
pub fn current_cpu_id() -> u32 {
    // Read LAPIC ID from CPUID leaf 1
    // We need to save/restore rbx because LLVM uses it internally
    let result: u32;
    unsafe {
        core::arch::asm!(
            "push rbx",        // Save rbx
            "mov eax, 1",      // CPUID leaf 1
            "cpuid",           // Execute CPUID
            "shr ebx, 24",     // Extract APIC ID (bits 31:24)
            "mov {0:e}, ebx",  // Move to output
            "pop rbx",         // Restore rbx
            out(reg) result,
            out("eax") _,
            out("ecx") _,
            out("edx") _,
            options(nostack),
        );
    }
    result
}

#[cfg(target_arch = "aarch64")]
pub fn current_cpu_id() -> u32 {
    let mpidr: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, mpidr_el1",
            out(reg) mpidr,
        );
    }
    (mpidr & 0xFF) as u32
}

#[cfg(target_arch = "riscv64")]
pub fn current_cpu_id() -> u32 {
    let hartid: u64;
    unsafe {
        core::arch::asm!(
            "csrr {}, mhartid",
            out(reg) hartid,
        );
    }
    hartid as u32
}

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
)))]
pub fn current_cpu_id() -> u32 {
    0
}

/// Check if we're running on the BSP
pub fn is_bsp() -> bool {
    u64::from(current_cpu_id()) == bsp_id()
}

// =============================================================================
// Per-CPU Data
// =============================================================================

/// Per-CPU data storage
///
/// This provides thread-local-like storage for CPU-specific data.
pub struct PerCpu<T: Sized> {
    data: [core::cell::UnsafeCell<core::mem::MaybeUninit<T>>; MAX_CPUS],
    init: [AtomicBool; MAX_CPUS],
}

impl<T: Sized> PerCpu<T> {
    /// Create a new per-CPU storage
    pub const fn new() -> Self {
        const UNINIT_BOOL: AtomicBool = AtomicBool::new(false);
        Self {
            data: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
            init: [UNINIT_BOOL; MAX_CPUS],
        }
    }

    /// Get a reference to the current CPU's data
    pub fn get(&self) -> Option<&T> {
        let cpu_id = current_cpu_id() as usize;
        if cpu_id >= MAX_CPUS {
            return None;
        }

        if !self.init[cpu_id].load(Ordering::Acquire) {
            return None;
        }

        // Safety: We've confirmed the data is initialized
        unsafe { Some((*self.data[cpu_id].get()).assume_init_ref()) }
    }

    /// Get a mutable reference to the current CPU's data
    ///
    /// # Safety
    ///
    /// Caller must ensure no other references to this data exist.
    pub unsafe fn get_mut(&self) -> Option<&mut T> {
        let cpu_id = current_cpu_id() as usize;
        if cpu_id >= MAX_CPUS {
            return None;
        }

        if !self.init[cpu_id].load(Ordering::Acquire) {
            return None;
        }

        unsafe { Some((*self.data[cpu_id].get()).assume_init_mut()) }
    }

    /// Set the current CPU's data
    pub fn set(&self, value: T) {
        let cpu_id = current_cpu_id() as usize;
        if cpu_id >= MAX_CPUS {
            return;
        }

        // Safety: We only access our own CPU's data
        unsafe {
            (*self.data[cpu_id].get()).write(value);
        }
        self.init[cpu_id].store(true, Ordering::Release);
    }

    /// Take the current CPU's data
    pub fn take(&self) -> Option<T> {
        let cpu_id = current_cpu_id() as usize;
        if cpu_id >= MAX_CPUS {
            return None;
        }

        if !self.init[cpu_id].swap(false, Ordering::AcqRel) {
            return None;
        }

        // Safety: We've confirmed the data was initialized and we're taking ownership
        unsafe { Some((*self.data[cpu_id].get()).assume_init_read()) }
    }
}

// Safety: Each CPU only accesses its own data
unsafe impl<T: Send> Sync for PerCpu<T> {}

// =============================================================================
// CPU Synchronization
// =============================================================================

/// Simple spin barrier for CPU synchronization
pub struct CpuBarrier {
    count: AtomicUsize,
    generation: AtomicUsize,
    target: usize,
}

impl CpuBarrier {
    /// Create a new barrier for the given number of CPUs
    pub const fn new(target: usize) -> Self {
        Self {
            count: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            target,
        }
    }

    /// Wait at the barrier
    ///
    /// Returns true for exactly one CPU (the "leader").
    pub fn wait(&self) -> bool {
        let gen = self.generation.load(Ordering::Acquire);
        let arrived = self.count.fetch_add(1, Ordering::AcqRel) + 1;

        if arrived == self.target {
            // Last to arrive - reset and notify
            self.count.store(0, Ordering::Release);
            self.generation.fetch_add(1, Ordering::Release);
            true
        } else {
            // Wait for generation to change
            while self.generation.load(Ordering::Acquire) == gen {
                core::hint::spin_loop();
            }
            false
        }
    }

    /// Get the number of CPUs currently waiting
    pub fn waiting(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }
}

/// One-shot synchronization flag
pub struct OnceFlag {
    done: AtomicBool,
}

impl OnceFlag {
    /// Create a new one-shot flag
    pub const fn new() -> Self {
        Self {
            done: AtomicBool::new(false),
        }
    }

    /// Try to claim the flag
    ///
    /// Returns true exactly once (for the first caller).
    pub fn claim(&self) -> bool {
        !self.done.swap(true, Ordering::AcqRel)
    }

    /// Check if the flag has been claimed
    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    /// Reset the flag
    pub fn reset(&self) {
        self.done.store(false, Ordering::Release);
    }
}

// =============================================================================
// AP Startup
// =============================================================================

/// AP (Application Processor) entry point signature
pub type ApEntry = extern "C" fn() -> !;

/// Start all application processors
///
/// This function iterates through all CPUs and starts each AP at the
/// given entry point.
#[allow(clippy::fn_to_numeric_cast)]
pub fn start_all_aps(smp: &SmpResponse, entry: ApEntry, arg: u64) {
    for cpu in smp.cpus() {
        if !cpu.is_bsp() {
            cpu.start(entry as u64, arg);
        }
    }
}

/// Wait for all CPUs to start
pub fn wait_for_all_cpus() {
    while !all_cpus_started() {
        core::hint::spin_loop();
    }
}

/// Wait for all CPUs with timeout
///
/// Returns false if timeout occurred.
pub fn wait_for_all_cpus_timeout(max_iterations: usize) -> bool {
    for _ in 0..max_iterations {
        if all_cpus_started() {
            return true;
        }
        core::hint::spin_loop();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_once_flag() {
        let flag = OnceFlag::new();
        assert!(!flag.is_done());
        assert!(flag.claim());
        assert!(flag.is_done());
        assert!(!flag.claim());
    }

    #[test]
    fn test_barrier_single() {
        let barrier = CpuBarrier::new(1);
        assert!(barrier.wait());
    }
}

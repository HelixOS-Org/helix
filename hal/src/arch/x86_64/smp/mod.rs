//! # x86_64 Symmetric Multi-Processing (SMP) Framework
//!
//! This module provides comprehensive SMP support for multi-core x86_64 systems,
//! including CPU enumeration, AP startup, per-CPU data management, and
//! synchronization primitives.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         SMP Architecture                                 │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │   ┌───────────────────────────────────────────────────────────────────┐ │
//! │   │                        BSP (Bootstrap Processor)                   │ │
//! │   │                                                                    │ │
//! │   │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐│ │
//! │   │  │ ACPI/MP     │  │ Memory      │  │ AP Startup                  ││ │
//! │   │  │ Table Parse │─►│ Allocation  │─►│ INIT-SIPI-SIPI              ││ │
//! │   │  └─────────────┘  └─────────────┘  └─────────────────────────────┘│ │
//! │   └───────────────────────────────────────────────────────────────────┘ │
//! │                              │                                           │
//! │                              ▼                                           │
//! │   ┌───────────────────────────────────────────────────────────────────┐ │
//! │   │                      AP (Application Processors)                   │ │
//! │   │                                                                    │ │
//! │   │  ┌─────────┐  ┌─────────┐  ┌─────────┐        ┌─────────┐        │ │
//! │   │  │  CPU 1  │  │  CPU 2  │  │  CPU 3  │  ...   │  CPU n  │        │ │
//! │   │  │         │  │         │  │         │        │         │        │ │
//! │   │  │Per-CPU  │  │Per-CPU  │  │Per-CPU  │        │Per-CPU  │        │ │
//! │   │  │Data     │  │Data     │  │Data     │        │Data     │        │ │
//! │   │  └─────────┘  └─────────┘  └─────────┘        └─────────┘        │ │
//! │   └───────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! │   Synchronization:                                                       │
//! │   ┌─────────────────────────────────────────────────────────────────┐   │
//! │   │  Barriers  │  Spinlocks  │  RwLocks  │  Atomic Ops  │  IPI      │   │
//! │   └─────────────────────────────────────────────────────────────────┘   │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Boot Sequence
//!
//! 1. BSP initializes core systems (GDT, IDT, paging, APIC)
//! 2. BSP parses ACPI/MP tables to enumerate CPUs
//! 3. BSP allocates per-CPU structures and stacks
//! 4. BSP broadcasts INIT-SIPI-SIPI to wake APs
//! 5. Each AP initializes its local state
//! 6. All CPUs synchronize at a barrier
//! 7. System is ready for multi-threaded operation
//!
//! ## Features
//!
//! - CPU enumeration from ACPI MADT
//! - INIT-SIPI-SIPI AP startup protocol
//! - Per-CPU data areas with GS base
//! - Various synchronization primitives
//! - CPU hotplug support (future)

#![allow(dead_code)]

pub mod startup;
pub mod cpu_info;
pub mod per_cpu;
pub mod barriers;

pub use startup::{TrampolineData, start_ap, start_all_aps, set_tsc_frequency, ApEntryFn};
pub use cpu_info::{CpuInfo, CpuTopology, CpuState, detect_topology, enumerate_cpus, get_cpu_info, register_cpu};
pub use per_cpu::{PerCpu, PerCpuRef, PerCpuData, PerCpuFlags, init_bsp, init_ap, current_cpu_id, current_apic_id, current_percpu};
pub use barriers::{Barrier, SpinBarrier, SeqLock, SpinLock, TicketLock, RwLock, memory_barrier, mfence, lfence, sfence};

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of supported CPUs
pub const MAX_CPUS: usize = 256;

/// Stack size per CPU (64KB)
pub const CPU_STACK_SIZE: usize = 64 * 1024;

/// Per-CPU data area size
pub const PER_CPU_SIZE: usize = 4096;

/// AP startup trampoline address (must be < 1MB, 4K aligned)
pub const AP_TRAMPOLINE_ADDR: u64 = 0x8000;

// =============================================================================
// Global State
// =============================================================================

/// SMP system initialized
static SMP_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Number of CPUs in the system
static CPU_COUNT: AtomicUsize = AtomicUsize::new(1);

/// Number of online CPUs
static ONLINE_CPU_COUNT: AtomicUsize = AtomicUsize::new(1);

/// BSP APIC ID
static BSP_APIC_ID: AtomicU32 = AtomicU32::new(0);

/// CPU presence bitmap (up to 256 CPUs)
static CPU_PRESENT: [AtomicU32; 8] = [
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
];

/// CPU online bitmap
static CPU_ONLINE: [AtomicU32; 8] = [
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
    AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
];

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the SMP subsystem
///
/// This must be called on the BSP after basic system initialization
/// (GDT, IDT, APIC, memory manager).
///
/// # Safety
///
/// Must be called exactly once during boot.
pub unsafe fn init() -> Result<(), SmpError> {
    if SMP_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(SmpError::AlreadyInitialized);
    }

    // Get BSP APIC ID
    let bsp_id = get_current_apic_id();
    BSP_APIC_ID.store(bsp_id, Ordering::SeqCst);

    // Mark BSP as present and online
    set_cpu_present(bsp_id as usize, true);
    set_cpu_online(bsp_id as usize, true);

    // Initialize per-CPU for BSP
    per_cpu::init_bsp()?;

    // Enumerate CPUs (from ACPI or MP tables)
    let cpu_count = cpu_info::enumerate_cpus()?;
    CPU_COUNT.store(cpu_count, Ordering::SeqCst);

    log::info!(
        "SMP: Initialized on BSP (APIC ID {}), {} CPU(s) detected",
        bsp_id,
        cpu_count
    );

    Ok(())
}

/// Start all Application Processors
///
/// # Safety
///
/// Must be called after `init()` and after the memory subsystem
/// is ready to allocate AP stacks.
pub unsafe fn start_all_aps() -> Result<usize, SmpError> {
    if !SMP_INITIALIZED.load(Ordering::Acquire) {
        return Err(SmpError::NotInitialized);
    }

    let started = startup::start_aps()?;

    log::info!("SMP: Started {} application processor(s)", started);

    Ok(started)
}

// =============================================================================
// CPU Information
// =============================================================================

/// Get the current CPU's APIC ID
#[inline]
pub fn get_current_apic_id() -> u32 {
    // Try x2APIC first, then fall back to xAPIC
    let (_, ebx, _, _) = cpuid(0x0B);
    if ebx != 0 {
        // x2APIC: EDX contains the APIC ID
        let (_, _, _, edx) = cpuid_subleaf(0x0B, 0);
        edx
    } else {
        // xAPIC: APIC ID from CPUID.01H.EBX[31:24]
        let (_, ebx, _, _) = cpuid(1);
        ebx >> 24
    }
}

/// Get the current CPU's index (0-based)
#[inline]
pub fn current_cpu_id() -> usize {
    per_cpu::current_cpu_id()
}

/// Get the total number of CPUs in the system
#[inline]
pub fn cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Relaxed)
}

/// Get the number of online CPUs
#[inline]
pub fn online_cpu_count() -> usize {
    ONLINE_CPU_COUNT.load(Ordering::Relaxed)
}

/// Get the BSP's APIC ID
#[inline]
pub fn bsp_apic_id() -> u32 {
    BSP_APIC_ID.load(Ordering::Relaxed)
}

/// Check if current CPU is the BSP
#[inline]
pub fn is_bsp() -> bool {
    get_current_apic_id() == bsp_apic_id()
}

/// Check if a CPU is present
#[inline]
pub fn is_cpu_present(cpu_id: usize) -> bool {
    if cpu_id >= MAX_CPUS {
        return false;
    }
    let word = cpu_id / 32;
    let bit = cpu_id % 32;
    CPU_PRESENT[word].load(Ordering::Relaxed) & (1 << bit) != 0
}

/// Check if a CPU is online
#[inline]
pub fn is_cpu_online(cpu_id: usize) -> bool {
    if cpu_id >= MAX_CPUS {
        return false;
    }
    let word = cpu_id / 32;
    let bit = cpu_id % 32;
    CPU_ONLINE[word].load(Ordering::Relaxed) & (1 << bit) != 0
}

/// Set CPU presence
fn set_cpu_present(cpu_id: usize, present: bool) {
    if cpu_id >= MAX_CPUS {
        return;
    }
    let word = cpu_id / 32;
    let bit = cpu_id % 32;
    if present {
        CPU_PRESENT[word].fetch_or(1 << bit, Ordering::SeqCst);
    } else {
        CPU_PRESENT[word].fetch_and(!(1 << bit), Ordering::SeqCst);
    }
}

/// Set CPU online status
pub fn set_cpu_online(cpu_id: usize, online: bool) {
    if cpu_id >= MAX_CPUS {
        return;
    }
    let word = cpu_id / 32;
    let bit = cpu_id % 32;
    if online {
        CPU_ONLINE[word].fetch_or(1 << bit, Ordering::SeqCst);
        ONLINE_CPU_COUNT.fetch_add(1, Ordering::SeqCst);
    } else {
        CPU_ONLINE[word].fetch_and(!(1 << bit), Ordering::SeqCst);
        ONLINE_CPU_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Iterate over all online CPUs
pub fn for_each_online_cpu<F: FnMut(usize)>(mut f: F) {
    for cpu_id in 0..MAX_CPUS {
        if is_cpu_online(cpu_id) {
            f(cpu_id);
        }
    }
}

// =============================================================================
// CPUID Helpers
// =============================================================================

fn cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            out("ebx") ebx,
            out("ecx") ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}

fn cpuid_subleaf(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            out("ebx") ebx,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}

// =============================================================================
// Error Type
// =============================================================================

/// SMP error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpError {
    /// SMP subsystem not initialized
    NotInitialized,
    /// SMP subsystem already initialized
    AlreadyInitialized,
    /// No CPUs found
    NoCpusFound,
    /// AP startup failed
    ApStartupFailed,
    /// CPU enumeration failed
    EnumerationFailed,
    /// Per-CPU initialization failed
    PerCpuInitFailed,
    /// Memory allocation failed
    MemoryAllocationFailed,
    /// Invalid CPU ID
    InvalidCpuId,
    /// CPU already online
    CpuAlreadyOnline,
    /// CPU not present
    CpuNotPresent,
}

impl core::fmt::Display for SmpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SmpError::NotInitialized => write!(f, "SMP not initialized"),
            SmpError::AlreadyInitialized => write!(f, "SMP already initialized"),
            SmpError::NoCpusFound => write!(f, "No CPUs found"),
            SmpError::ApStartupFailed => write!(f, "AP startup failed"),
            SmpError::EnumerationFailed => write!(f, "CPU enumeration failed"),
            SmpError::PerCpuInitFailed => write!(f, "Per-CPU initialization failed"),
            SmpError::MemoryAllocationFailed => write!(f, "Memory allocation failed"),
            SmpError::InvalidCpuId => write!(f, "Invalid CPU ID"),
            SmpError::CpuAlreadyOnline => write!(f, "CPU already online"),
            SmpError::CpuNotPresent => write!(f, "CPU not present"),
        }
    }
}

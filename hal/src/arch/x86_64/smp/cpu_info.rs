//! # CPU Information and Enumeration
//!
//! This module provides CPU enumeration, topology detection, and
//! per-CPU information management.

use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

use super::{SmpError, MAX_CPUS};

// =============================================================================
// CPU State
// =============================================================================

/// CPU state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CpuState {
    /// CPU not present in system
    NotPresent   = 0,
    /// CPU present but not started
    Present      = 1,
    /// CPU is starting up
    Starting     = 2,
    /// CPU is online and running
    Online       = 3,
    /// CPU is going offline
    GoingOffline = 4,
    /// CPU is offline (halted)
    Offline      = 5,
    /// CPU encountered an error
    Error        = 6,
}

impl From<u8> for CpuState {
    fn from(value: u8) -> Self {
        match value {
            0 => CpuState::NotPresent,
            1 => CpuState::Present,
            2 => CpuState::Starting,
            3 => CpuState::Online,
            4 => CpuState::GoingOffline,
            5 => CpuState::Offline,
            _ => CpuState::Error,
        }
    }
}

// =============================================================================
// Per-CPU Information
// =============================================================================

/// Information about a single CPU
#[repr(C)]
pub struct CpuInfo {
    /// APIC ID
    apic_id: AtomicU32,
    /// CPU index (0-based)
    cpu_id: AtomicU32,
    /// Package/socket ID
    package_id: AtomicU8,
    /// Core ID within package
    core_id: AtomicU8,
    /// Thread ID within core (for SMT)
    thread_id: AtomicU8,
    /// CPU state
    state: AtomicU8,
    /// Is this the BSP?
    is_bsp: AtomicU8,
    /// Reserved for alignment
    _reserved: [u8; 3],
}

impl CpuInfo {
    /// Create a new CPU info entry
    pub const fn new() -> Self {
        Self {
            apic_id: AtomicU32::new(0xFFFF_FFFF),
            cpu_id: AtomicU32::new(0xFFFF_FFFF),
            package_id: AtomicU8::new(0),
            core_id: AtomicU8::new(0),
            thread_id: AtomicU8::new(0),
            state: AtomicU8::new(CpuState::NotPresent as u8),
            is_bsp: AtomicU8::new(0),
            _reserved: [0; 3],
        }
    }

    /// Initialize CPU info
    pub fn init(&self, apic_id: u32, cpu_id: u32, is_bsp: bool) {
        self.apic_id.store(apic_id, Ordering::SeqCst);
        self.cpu_id.store(cpu_id, Ordering::SeqCst);
        self.state.store(CpuState::Present as u8, Ordering::SeqCst);
        self.is_bsp
            .store(if is_bsp { 1 } else { 0 }, Ordering::SeqCst);
    }

    /// Get APIC ID
    #[inline]
    pub fn apic_id(&self) -> u32 {
        self.apic_id.load(Ordering::Relaxed)
    }

    /// Get CPU index
    #[inline]
    pub fn cpu_id(&self) -> u32 {
        self.cpu_id.load(Ordering::Relaxed)
    }

    /// Get package ID
    #[inline]
    pub fn package_id(&self) -> u8 {
        self.package_id.load(Ordering::Relaxed)
    }

    /// Get core ID
    #[inline]
    pub fn core_id(&self) -> u8 {
        self.core_id.load(Ordering::Relaxed)
    }

    /// Get thread ID (SMT)
    #[inline]
    pub fn thread_id(&self) -> u8 {
        self.thread_id.load(Ordering::Relaxed)
    }

    /// Get CPU state
    #[inline]
    pub fn state(&self) -> CpuState {
        CpuState::from(self.state.load(Ordering::Acquire))
    }

    /// Set CPU state
    #[inline]
    pub fn set_state(&self, state: CpuState) {
        self.state.store(state as u8, Ordering::Release);
    }

    /// Check if this is the BSP
    #[inline]
    pub fn is_bsp(&self) -> bool {
        self.is_bsp.load(Ordering::Relaxed) != 0
    }

    /// Check if CPU is present
    #[inline]
    pub fn is_present(&self) -> bool {
        self.state() != CpuState::NotPresent
    }

    /// Check if CPU is online
    #[inline]
    pub fn is_online(&self) -> bool {
        self.state() == CpuState::Online
    }

    /// Set topology information
    pub fn set_topology(&self, package: u8, core: u8, thread: u8) {
        self.package_id.store(package, Ordering::SeqCst);
        self.core_id.store(core, Ordering::SeqCst);
        self.thread_id.store(thread, Ordering::SeqCst);
    }
}

impl Default for CpuInfo {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Global CPU Table
// =============================================================================

/// Global CPU information table
static CPU_INFO: [CpuInfo; MAX_CPUS] = [const { CpuInfo::new() }; MAX_CPUS];

/// APIC ID to CPU ID mapping
static APIC_TO_CPU: [AtomicU32; MAX_CPUS] = [const { AtomicU32::new(0xFFFF_FFFF) }; MAX_CPUS];

/// Get CPU info by index
pub fn get_cpu_info(cpu_id: usize) -> Option<&'static CpuInfo> {
    if cpu_id < MAX_CPUS && CPU_INFO[cpu_id].is_present() {
        Some(&CPU_INFO[cpu_id])
    } else {
        None
    }
}

/// Get CPU info by APIC ID
pub fn get_cpu_info_by_apic_id(apic_id: u32) -> Option<&'static CpuInfo> {
    if (apic_id as usize) < MAX_CPUS {
        let cpu_id = APIC_TO_CPU[apic_id as usize].load(Ordering::Relaxed);
        if cpu_id != 0xFFFF_FFFF {
            return get_cpu_info(cpu_id as usize);
        }
    }

    // Linear search fallback
    for cpu in CPU_INFO.iter() {
        if cpu.apic_id() == apic_id && cpu.is_present() {
            return Some(cpu);
        }
    }
    None
}

/// Register a CPU
pub fn register_cpu(cpu_id: usize, apic_id: u32, is_bsp: bool) -> Result<(), SmpError> {
    if cpu_id >= MAX_CPUS {
        return Err(SmpError::InvalidCpuId);
    }

    CPU_INFO[cpu_id].init(apic_id, cpu_id as u32, is_bsp);

    if (apic_id as usize) < MAX_CPUS {
        APIC_TO_CPU[apic_id as usize].store(cpu_id as u32, Ordering::SeqCst);
    }

    Ok(())
}

// =============================================================================
// CPU Enumeration
// =============================================================================

/// Enumerate CPUs in the system
///
/// This function should parse ACPI MADT or MP tables to find all CPUs.
/// For now, it provides a stub implementation.
pub fn enumerate_cpus() -> Result<usize, SmpError> {
    // In a real implementation, this would:
    // 1. Parse ACPI MADT table
    // 2. Extract Local APIC entries
    // 3. Register each CPU

    // For now, use CPUID to detect topology
    let topology = detect_topology();

    // At minimum, we have the BSP
    let bsp_apic_id = super::get_current_apic_id();
    register_cpu(0, bsp_apic_id, true)?;
    CPU_INFO[0].set_state(CpuState::Online);

    // Detect number of logical processors from CPUID
    let logical_cpus = topology.total_logical_cpus as usize;

    if logical_cpus <= 1 {
        return Ok(1);
    }

    log::info!(
        "CPU Topology: {} packages, {} cores/package, {} threads/core",
        topology.packages,
        topology.cores_per_package,
        topology.threads_per_core
    );

    // In a real system, we'd enumerate from ACPI here
    // For now, just report the detected count
    Ok(logical_cpus)
}

// =============================================================================
// CPU Topology
// =============================================================================

/// CPU topology information
#[derive(Debug, Clone, Copy)]
pub struct CpuTopology {
    /// Number of packages/sockets
    pub packages: u32,
    /// Cores per package
    pub cores_per_package: u32,
    /// Threads per core (SMT)
    pub threads_per_core: u32,
    /// Total logical CPUs
    pub total_logical_cpus: u32,
    /// APIC ID bits for SMT
    pub smt_mask_width: u32,
    /// APIC ID bits for core
    pub core_mask_width: u32,
}

impl CpuTopology {
    /// Create an empty topology
    pub const fn new() -> Self {
        Self {
            packages: 1,
            cores_per_package: 1,
            threads_per_core: 1,
            total_logical_cpus: 1,
            smt_mask_width: 0,
            core_mask_width: 0,
        }
    }
}

impl Default for CpuTopology {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect CPU topology using CPUID
pub fn detect_topology() -> CpuTopology {
    let mut topology = CpuTopology::new();

    // Check for extended topology leaf (0x0B)
    let (max_leaf, _, _, _) = cpuid(0);

    if max_leaf >= 0x0B {
        // Use leaf 0x0B for topology enumeration
        detect_topology_0b(&mut topology);
    } else if max_leaf >= 0x04 {
        // Fall back to leaf 0x04
        detect_topology_04(&mut topology);
    } else {
        // Basic detection from leaf 0x01
        detect_topology_01(&mut topology);
    }

    topology
}

/// Detect topology using CPUID leaf 0x0B (x2APIC)
fn detect_topology_0b(topology: &mut CpuTopology) {
    let mut level = 0u32;
    let mut total_threads = 0u32;
    let mut total_cores = 0u32;

    loop {
        let (eax, ebx, ecx, _) = cpuid_subleaf(0x0B, level);

        // Check if this is a valid level
        let level_type = (ecx >> 8) & 0xFF;
        if level_type == 0 && level > 0 {
            break;
        }

        match level_type {
            1 => {
                // SMT level
                topology.threads_per_core = ebx & 0xFFFF;
                topology.smt_mask_width = eax & 0x1F;
            },
            2 => {
                // Core level
                total_cores = ebx & 0xFFFF;
                topology.core_mask_width = eax & 0x1F;
            },
            _ => {},
        }

        total_threads = (ebx & 0xFFFF).max(total_threads);
        level += 1;

        if level > 10 {
            break; // Safety limit
        }
    }

    if total_threads > 0 {
        topology.total_logical_cpus = total_threads;
    }

    if total_cores > 0 && topology.threads_per_core > 0 {
        topology.cores_per_package = total_cores / topology.threads_per_core;
    }

    // Calculate packages (assuming all CPUs are same topology)
    if topology.total_logical_cpus > 0 && total_cores > 0 {
        topology.packages = topology.total_logical_cpus / total_cores;
        if topology.packages == 0 {
            topology.packages = 1;
        }
    }
}

/// Detect topology using CPUID leaf 0x04
fn detect_topology_04(topology: &mut CpuTopology) {
    // Get maximum logical CPUs from leaf 0x01
    let (_, ebx, _, _) = cpuid(1);
    let max_logical = (ebx >> 16) & 0xFF;

    // Get cores from leaf 0x04
    let (eax, _, _, _) = cpuid_subleaf(0x04, 0);
    let max_cores = ((eax >> 26) & 0x3F) + 1;

    topology.total_logical_cpus = max_logical;
    topology.cores_per_package = max_cores;

    if max_cores > 0 {
        topology.threads_per_core = max_logical / max_cores;
        if topology.threads_per_core == 0 {
            topology.threads_per_core = 1;
        }
    }

    topology.packages = 1;
}

/// Detect topology using CPUID leaf 0x01 (basic)
fn detect_topology_01(topology: &mut CpuTopology) {
    let (_, ebx, _, edx) = cpuid(1);

    // Check for HTT (Hyper-Threading)
    if edx & (1 << 28) != 0 {
        let max_logical = (ebx >> 16) & 0xFF;
        topology.total_logical_cpus = max_logical;

        // Assume 2 threads per core if HTT is enabled
        if max_logical > 1 {
            topology.threads_per_core = 2;
            topology.cores_per_package = max_logical / 2;
        }
    }
}

/// Extract topology IDs from APIC ID
pub fn extract_topology_ids(apic_id: u32, topology: &CpuTopology) -> (u8, u8, u8) {
    let smt_mask = (1u32 << topology.smt_mask_width) - 1;
    let core_mask = (1u32 << topology.core_mask_width) - 1;

    let thread_id = (apic_id & smt_mask) as u8;
    let core_id =
        ((apic_id >> topology.smt_mask_width) & (core_mask >> topology.smt_mask_width)) as u8;
    let package_id = (apic_id >> topology.core_mask_width) as u8;

    (package_id, core_id, thread_id)
}

// =============================================================================
// CPUID Helpers
// =============================================================================

fn cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    let (mut eax, ebx, ecx, edx): (u32, u32, u32, u32);
    eax = leaf;
    unsafe {
        core::arch::asm!(
            "mov {tmp:r}, rbx",
            "cpuid",
            "xchg {tmp:r}, rbx",
            tmp = out(reg) ebx,
            inout("eax") eax,
            out("ecx") ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}

fn cpuid_subleaf(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let (mut eax, ebx, mut ecx, edx): (u32, u32, u32, u32);
    eax = leaf;
    ecx = subleaf;
    unsafe {
        core::arch::asm!(
            "mov {tmp:r}, rbx",
            "cpuid",
            "xchg {tmp:r}, rbx",
            tmp = out(reg) ebx,
            inout("eax") eax,
            inout("ecx") ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    (eax, ebx, ecx, edx)
}

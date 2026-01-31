//! # AArch64 Symmetric Multi-Processing (SMP) Framework
//!
//! This module provides comprehensive SMP support for AArch64 systems, including
//! CPU topology detection, secondary CPU startup, per-CPU data management, and
//! inter-processor interrupts.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │                        AArch64 SMP Architecture                          │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │                         CPU Topology                                │ │
//! │  │                                                                     │ │
//! │  │   Node 0 (Aff3=0)                    Node 1 (Aff3=1)               │ │
//! │  │   ┌─────────────────────────┐       ┌─────────────────────────┐   │ │
//! │  │   │  Cluster 0 (Aff2=0)     │       │  Cluster 0 (Aff2=0)     │   │ │
//! │  │   │  ┌──────┐ ┌──────┐     │       │  ┌──────┐ ┌──────┐     │   │ │
//! │  │   │  │CPU 0 │ │CPU 1 │     │       │  │CPU 4 │ │CPU 5 │     │   │ │
//! │  │   │  │Aff1=0│ │Aff1=0│     │       │  │Aff1=0│ │Aff1=0│     │   │ │
//! │  │   │  │Aff0=0│ │Aff0=1│     │       │  │Aff0=0│ │Aff0=1│     │   │ │
//! │  │   │  └──────┘ └──────┘     │       │  └──────┘ └──────┘     │   │ │
//! │  │   │  ┌──────┐ ┌──────┐     │       │  ┌──────┐ ┌──────┐     │   │ │
//! │  │   │  │CPU 2 │ │CPU 3 │     │       │  │CPU 6 │ │CPU 7 │     │   │ │
//! │  │   │  │Aff1=1│ │Aff1=1│     │       │  │Aff1=1│ │Aff1=1│     │   │ │
//! │  │   │  │Aff0=0│ │Aff0=1│     │       │  │Aff0=0│ │Aff0=1│     │   │ │
//! │  │   │  └──────┘ └──────┘     │       │  └──────┘ └──────┘     │   │ │
//! │  │   └─────────────────────────┘       └─────────────────────────┘   │ │
//! │  └────────────────────────────────────────────────────────────────────┘ │
//! │                                                                          │
//! │  Boot Process:                                                           │
//! │  1. BSP (Boot Processor) starts at reset vector                         │
//! │  2. BSP initializes core systems (GIC, MMU, etc.)                       │
//! │  3. BSP uses PSCI to bring up APs (Application Processors)              │
//! │  4. APs start at provided entry point with per-CPU stack               │
//! │  5. APs initialize local systems (GIC redistributor, timers)           │
//! │  6. APs signal ready and enter scheduler                               │
//! │                                                                          │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Key Features
//!
//! - **MPIDR Parsing**: CPU affinity hierarchy extraction
//! - **PSCI Interface**: Standard CPU power management
//! - **Per-CPU Data**: Thread-local storage via TPIDR_EL1
//! - **IPI Support**: Inter-processor interrupts via GIC SGIs
//! - **Spin Tables**: Legacy boot method support (e.g., Raspberry Pi)
//!
//! ## Platform Support
//!
//! | Platform      | Boot Method | Notes                          |
//! |---------------|-------------|--------------------------------|
//! | QEMU virt     | PSCI        | Via `psci-conduit = "smc"`     |
//! | Raspberry Pi  | Spin Table  | Mailbox-based                  |
//! | ARM FVP       | PSCI        | Full PSCI 1.0+                 |
//! | AWS Graviton  | PSCI        | ACPI/PSCI                      |
//! | Ampere Altra  | PSCI        | ACPI/PSCI                      |

pub mod ipi;
pub mod mpidr;
pub mod percpu;
pub mod psci;
pub mod startup;

pub use ipi::*;
pub use mpidr::*;
pub use percpu::*;
pub use psci::*;
pub use startup::*;

// ============================================================================
// SMP Constants
// ============================================================================

/// Maximum number of supported CPUs
pub const MAX_CPUS: usize = 256;

/// Maximum number of clusters
pub const MAX_CLUSTERS: usize = 64;

/// Default stack size for secondary CPUs (16 KB)
pub const DEFAULT_AP_STACK_SIZE: usize = 16 * 1024;

/// IPI vector for reschedule request
pub const IPI_RESCHEDULE: u8 = 0;

/// IPI vector for TLB shootdown
pub const IPI_TLB_SHOOTDOWN: u8 = 1;

/// IPI vector for function call
pub const IPI_CALL_FUNCTION: u8 = 2;

/// IPI vector for CPU stop
pub const IPI_CPU_STOP: u8 = 3;

/// IPI vector for CPU wake
pub const IPI_CPU_WAKE: u8 = 4;

// ============================================================================
// CPU State
// ============================================================================

/// CPU operational state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CpuState {
    /// CPU is offline (not started)
    Offline = 0,
    /// CPU is in the process of coming online
    Starting = 1,
    /// CPU is online and running
    Online = 2,
    /// CPU is going offline
    Stopping = 3,
    /// CPU is in an idle state (low power)
    Idle = 4,
    /// CPU has encountered a fatal error
    Dead = 5,
}

impl CpuState {
    /// Check if the CPU is usable (online or idle)
    #[inline]
    pub const fn is_active(self) -> bool {
        matches!(self, CpuState::Online | CpuState::Idle)
    }

    /// Check if the CPU can accept IPIs
    #[inline]
    pub const fn can_receive_ipi(self) -> bool {
        matches!(self, CpuState::Online | CpuState::Idle | CpuState::Stopping)
    }
}

// ============================================================================
// CPU Information
// ============================================================================

/// Information about a single CPU
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// Linear CPU ID (0, 1, 2, ...)
    pub cpu_id: u32,
    /// MPIDR affinity value
    pub mpidr: Mpidr,
    /// Current state
    pub state: CpuState,
    /// Per-CPU data pointer
    pub percpu_ptr: Option<*mut u8>,
    /// Stack base address
    pub stack_base: Option<usize>,
    /// Stack size
    pub stack_size: usize,
}

impl CpuInfo {
    /// Create info for an offline CPU
    pub const fn offline(cpu_id: u32, mpidr: Mpidr) -> Self {
        Self {
            cpu_id,
            mpidr,
            state: CpuState::Offline,
            percpu_ptr: None,
            stack_base: None,
            stack_size: DEFAULT_AP_STACK_SIZE,
        }
    }
}

// ============================================================================
// Cluster Information
// ============================================================================

/// Information about a CPU cluster
#[derive(Debug, Clone)]
pub struct ClusterInfo {
    /// Cluster ID (from Aff1/Aff2)
    pub cluster_id: u32,
    /// CPUs in this cluster
    pub cpu_mask: u64,
    /// Number of CPUs in this cluster
    pub cpu_count: u32,
    /// Cluster-level features
    pub features: ClusterFeatures,
}

bitflags::bitflags! {
    /// Cluster-level features
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ClusterFeatures: u32 {
        /// Cluster has shared L2 cache
        const SHARED_L2 = 1 << 0;
        /// Cluster has shared L3 cache
        const SHARED_L3 = 1 << 1;
        /// Cluster supports cluster-level power management
        const CLUSTER_PM = 1 << 2;
        /// Cluster has coherent interconnect
        const COHERENT = 1 << 3;
    }
}

// ============================================================================
// Topology
// ============================================================================

/// System CPU topology
#[derive(Debug)]
pub struct CpuTopology {
    /// Total number of CPUs detected
    pub num_cpus: u32,
    /// Number of online CPUs
    pub num_online: u32,
    /// CPU information array
    pub cpus: [Option<CpuInfo>; MAX_CPUS],
    /// BSP (Boot Processor) CPU ID
    pub bsp_id: u32,
    /// Maximum affinity levels used
    pub max_aff_level: u8,
}

impl CpuTopology {
    /// Create an empty topology
    pub const fn new() -> Self {
        const NONE: Option<CpuInfo> = None;
        Self {
            num_cpus: 0,
            num_online: 0,
            cpus: [NONE; MAX_CPUS],
            bsp_id: 0,
            max_aff_level: 0,
        }
    }

    /// Register the BSP (Boot Processor)
    pub fn register_bsp(&mut self) {
        let mpidr = Mpidr::current();
        let cpu_id = 0;

        self.cpus[cpu_id as usize] = Some(CpuInfo {
            cpu_id,
            mpidr,
            state: CpuState::Online,
            percpu_ptr: None,
            stack_base: None,
            stack_size: DEFAULT_AP_STACK_SIZE,
        });

        self.bsp_id = cpu_id;
        self.num_cpus = 1;
        self.num_online = 1;
        self.max_aff_level = 3; // Assume 4 levels by default
    }

    /// Register a CPU from device tree or ACPI
    pub fn register_cpu(&mut self, mpidr: Mpidr) -> Option<u32> {
        if self.num_cpus >= MAX_CPUS as u32 {
            return None;
        }

        // Check if already registered
        for i in 0..self.num_cpus as usize {
            if let Some(ref info) = self.cpus[i] {
                if info.mpidr.value() == mpidr.value() {
                    return Some(info.cpu_id);
                }
            }
        }

        let cpu_id = self.num_cpus;
        self.cpus[cpu_id as usize] = Some(CpuInfo::offline(cpu_id, mpidr));
        self.num_cpus += 1;

        Some(cpu_id)
    }

    /// Get CPU info by ID
    pub fn get_cpu(&self, cpu_id: u32) -> Option<&CpuInfo> {
        self.cpus.get(cpu_id as usize).and_then(|c| c.as_ref())
    }

    /// Get mutable CPU info by ID
    pub fn get_cpu_mut(&mut self, cpu_id: u32) -> Option<&mut CpuInfo> {
        self.cpus.get_mut(cpu_id as usize).and_then(|c| c.as_mut())
    }

    /// Find CPU ID by MPIDR
    pub fn find_by_mpidr(&self, mpidr: Mpidr) -> Option<u32> {
        for i in 0..self.num_cpus as usize {
            if let Some(ref info) = self.cpus[i] {
                if info.mpidr.affinity() == mpidr.affinity() {
                    return Some(info.cpu_id);
                }
            }
        }
        None
    }

    /// Get the current CPU's ID
    pub fn current_cpu_id(&self) -> Option<u32> {
        let mpidr = Mpidr::current();
        self.find_by_mpidr(mpidr)
    }

    /// Iterate over all registered CPUs
    pub fn iter(&self) -> impl Iterator<Item = &CpuInfo> {
        self.cpus[..self.num_cpus as usize]
            .iter()
            .filter_map(|c| c.as_ref())
    }

    /// Iterate over online CPUs
    pub fn iter_online(&self) -> impl Iterator<Item = &CpuInfo> {
        self.iter().filter(|c| c.state.is_active())
    }

    /// Get a mask of all online CPU IDs
    pub fn online_mask(&self) -> u64 {
        let mut mask = 0u64;
        for cpu in self.iter_online() {
            if cpu.cpu_id < 64 {
                mask |= 1 << cpu.cpu_id;
            }
        }
        mask
    }
}

impl Default for CpuTopology {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SMP Operations Trait
// ============================================================================

/// Trait for platform-specific SMP operations
pub trait SmpOperations {
    /// Start a secondary CPU
    fn start_cpu(&self, cpu_id: u32, entry: usize, context: usize) -> Result<(), SmpError>;

    /// Stop a CPU
    fn stop_cpu(&self, cpu_id: u32) -> Result<(), SmpError>;

    /// Send an IPI to a specific CPU
    fn send_ipi(&self, cpu_id: u32, vector: u8) -> Result<(), SmpError>;

    /// Send an IPI to multiple CPUs
    fn send_ipi_mask(&self, mask: u64, vector: u8) -> Result<(), SmpError>;

    /// Send an IPI to all CPUs except self
    fn send_ipi_all_except_self(&self, vector: u8) -> Result<(), SmpError>;

    /// Get the current CPU ID
    fn current_cpu_id(&self) -> u32;
}

/// SMP operation error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpError {
    /// Invalid CPU ID
    InvalidCpu,
    /// CPU is already online
    AlreadyOnline,
    /// CPU is offline
    CpuOffline,
    /// PSCI call failed
    PsciFailed(i32),
    /// Not supported
    NotSupported,
    /// Timeout waiting for CPU
    Timeout,
    /// Out of resources
    OutOfResources,
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize SMP support for the BSP
pub fn init_bsp() {
    // Initialize per-CPU for BSP
    percpu::init_percpu_bsp();

    // Register BSP in topology
    // (topology is typically initialized by the kernel before calling this)
}

/// Entry point for secondary CPUs
///
/// This is called from assembly after the CPU has been brought up.
///
/// # Safety
///
/// Must only be called from the AP startup assembly code.
#[no_mangle]
pub unsafe extern "C" fn ap_entry(cpu_id: u32) -> ! {
    // Initialize per-CPU data
    percpu::init_percpu_ap(cpu_id);

    // Initialize local interrupt controller
    // (This would initialize the GIC redistributor and CPU interface)

    // Initialize local timer
    // (This would set up the timer for this CPU)

    // Memory barrier before signaling ready
    core::arch::asm!("dmb ish", options(nomem, nostack));

    // Signal that we're online
    // (This would update CPU state in topology and signal BSP)

    // Enter idle loop or scheduler
    loop {
        core::arch::asm!("wfe", options(nomem, nostack));
    }
}

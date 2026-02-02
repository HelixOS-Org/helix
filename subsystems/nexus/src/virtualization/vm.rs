//! Virtual Machine Intelligence
//!
//! VM-specific monitoring and optimization.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{VirtId, WorkloadInfo};

/// VM-specific intelligence
pub struct VmIntelligence {
    /// VM info
    vms: BTreeMap<VirtId, VmInfo>,
    /// Performance profiles
    profiles: BTreeMap<VirtId, VmProfile>,
    /// Vmexit statistics
    vmexit_stats: BTreeMap<VirtId, VmExitStats>,
}

/// VM information
#[derive(Debug, Clone)]
pub struct VmInfo {
    /// Base workload info
    pub base: WorkloadInfo,
    /// Guest OS type
    pub guest_os: GuestOs,
    /// Has nested virt?
    pub nested_virt: bool,
    /// Huge pages enabled?
    pub huge_pages: bool,
    /// NUMA aware?
    pub numa_aware: bool,
    /// Disk images
    pub disks: Vec<DiskInfo>,
}

/// Guest OS type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestOs {
    /// Linux
    Linux,
    /// Windows
    Windows,
    /// BSD
    Bsd,
    /// Other Unix
    Unix,
    /// Unknown
    Unknown,
}

/// Disk info
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Path
    pub path: String,
    /// Size bytes
    pub size: u64,
    /// Is readonly?
    pub readonly: bool,
    /// Format
    pub format: DiskFormat,
}

/// Disk format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskFormat {
    /// Raw
    Raw,
    /// QCOW2
    Qcow2,
    /// VHD
    Vhd,
    /// VMDK
    Vmdk,
}

/// VM performance profile
#[derive(Debug, Clone)]
pub struct VmProfile {
    /// Average CPU usage
    pub avg_cpu: f64,
    /// Peak CPU usage
    pub peak_cpu: f64,
    /// Average memory
    pub avg_memory: f64,
    /// IO intensity
    pub io_intensity: IoIntensity,
    /// Network intensity
    pub net_intensity: NetIntensity,
    /// Is bursty?
    pub bursty: bool,
}

/// IO intensity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoIntensity {
    /// Light IO
    Light,
    /// Moderate IO
    Moderate,
    /// Heavy IO
    Heavy,
    /// Extreme IO
    Extreme,
}

/// Network intensity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetIntensity {
    /// Light network
    Light,
    /// Moderate network
    Moderate,
    /// Heavy network
    Heavy,
    /// Extreme network
    Extreme,
}

/// VM exit statistics
#[derive(Debug, Clone, Default)]
pub struct VmExitStats {
    /// Total exits
    pub total_exits: u64,
    /// IO exits
    pub io_exits: u64,
    /// Interrupt exits
    pub interrupt_exits: u64,
    /// CPUID exits
    pub cpuid_exits: u64,
    /// CR access exits
    pub cr_exits: u64,
    /// MSR access exits
    pub msr_exits: u64,
    /// EPT violations
    pub ept_violations: u64,
    /// Average exit latency (ns)
    pub avg_exit_latency: f64,
}

impl VmExitStats {
    /// Record exit
    pub fn record_exit(&mut self, exit_type: VmExitType, latency_ns: u64) {
        self.total_exits += 1;

        match exit_type {
            VmExitType::Io => self.io_exits += 1,
            VmExitType::Interrupt => self.interrupt_exits += 1,
            VmExitType::Cpuid => self.cpuid_exits += 1,
            VmExitType::ControlRegister => self.cr_exits += 1,
            VmExitType::Msr => self.msr_exits += 1,
            VmExitType::EptViolation => self.ept_violations += 1,
            VmExitType::Other => {},
        }

        let alpha = 0.1;
        self.avg_exit_latency = alpha * latency_ns as f64 + (1.0 - alpha) * self.avg_exit_latency;
    }

    /// Get exit rate
    pub fn exit_rate(&self, uptime_ns: u64) -> f64 {
        if uptime_ns == 0 {
            0.0
        } else {
            self.total_exits as f64 * 1_000_000_000.0 / uptime_ns as f64
        }
    }
}

/// VM exit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmExitType {
    /// IO operation
    Io,
    /// Interrupt
    Interrupt,
    /// CPUID instruction
    Cpuid,
    /// Control register access
    ControlRegister,
    /// MSR access
    Msr,
    /// EPT violation
    EptViolation,
    /// Other
    Other,
}

impl VmIntelligence {
    /// Create new VM intelligence
    pub fn new() -> Self {
        Self {
            vms: BTreeMap::new(),
            profiles: BTreeMap::new(),
            vmexit_stats: BTreeMap::new(),
        }
    }

    /// Register VM
    pub fn register(&mut self, info: VmInfo) {
        self.vmexit_stats
            .insert(info.base.id, VmExitStats::default());
        self.vms.insert(info.base.id, info);
    }

    /// Record VM exit
    pub fn record_exit(&mut self, vm_id: VirtId, exit_type: VmExitType, latency_ns: u64) {
        if let Some(stats) = self.vmexit_stats.get_mut(&vm_id) {
            stats.record_exit(exit_type, latency_ns);
        }
    }

    /// Get VM info
    pub fn get(&self, vm_id: VirtId) -> Option<&VmInfo> {
        self.vms.get(&vm_id)
    }

    /// Get exit stats
    pub fn get_exit_stats(&self, vm_id: VirtId) -> Option<&VmExitStats> {
        self.vmexit_stats.get(&vm_id)
    }

    /// Set profile
    pub fn set_profile(&mut self, vm_id: VirtId, profile: VmProfile) {
        self.profiles.insert(vm_id, profile);
    }

    /// Get profile
    pub fn get_profile(&self, vm_id: VirtId) -> Option<&VmProfile> {
        self.profiles.get(&vm_id)
    }

    /// Get high exit rate VMs
    pub fn high_exit_rate_vms(&self, threshold: f64) -> Vec<VirtId> {
        self.vmexit_stats
            .iter()
            .filter_map(|(&id, stats)| {
                let vm = self.vms.get(&id)?;
                let rate = stats.exit_rate(vm.base.uptime() * 1_000_000);
                if rate > threshold { Some(id) } else { None }
            })
            .collect()
    }
}

impl Default for VmIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

//! Container Intelligence
//!
//! Container-specific monitoring and management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{VirtId, WorkloadInfo};

/// Container-specific intelligence
pub struct ContainerIntelligence {
    /// Containers
    containers: BTreeMap<VirtId, ContainerInfo>,
    /// Cgroup stats
    cgroup_stats: BTreeMap<VirtId, CgroupStats>,
    /// Namespace info
    namespaces: BTreeMap<VirtId, NamespaceInfo>,
}

/// Container information
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    /// Base workload info
    pub base: WorkloadInfo,
    /// Image name
    pub image: String,
    /// Entrypoint
    pub entrypoint: Vec<String>,
    /// Environment
    pub environment: BTreeMap<String, String>,
    /// Mounts
    pub mounts: Vec<MountInfo>,
    /// Network mode
    pub network_mode: NetworkMode,
}

/// Mount info
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Source path
    pub source: String,
    /// Destination path
    pub destination: String,
    /// Read only
    pub readonly: bool,
}

/// Network mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    /// Host network
    Host,
    /// Bridge network
    Bridge,
    /// None
    None,
    /// Custom network
    Custom,
}

/// Cgroup statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CgroupStats {
    /// CPU usage (nanoseconds)
    pub cpu_usage_ns: u64,
    /// CPU throttled time
    pub cpu_throttled_ns: u64,
    /// Memory usage
    pub memory_usage: u64,
    /// Memory limit
    pub memory_limit: u64,
    /// Memory swap
    pub memory_swap: u64,
    /// OOM kill count
    pub oom_kills: u32,
    /// Block IO reads
    pub blkio_reads: u64,
    /// Block IO writes
    pub blkio_writes: u64,
    /// Pids current
    pub pids_current: u32,
    /// Pids limit
    pub pids_limit: u32,
}

impl CgroupStats {
    /// Is throttled?
    #[inline(always)]
    pub fn is_throttled(&self) -> bool {
        self.cpu_throttled_ns > 0
    }

    /// Memory pressure
    #[inline]
    pub fn memory_pressure(&self) -> f64 {
        if self.memory_limit == 0 {
            0.0
        } else {
            self.memory_usage as f64 / self.memory_limit as f64
        }
    }

    /// Is OOM risk?
    #[inline(always)]
    pub fn is_oom_risk(&self) -> bool {
        self.memory_pressure() > 0.95
    }
}

/// Namespace information
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    /// PID namespace
    pub pid_ns: u64,
    /// Network namespace
    pub net_ns: u64,
    /// Mount namespace
    pub mnt_ns: u64,
    /// IPC namespace
    pub ipc_ns: u64,
    /// UTS namespace
    pub uts_ns: u64,
    /// User namespace
    pub user_ns: u64,
    /// Cgroup namespace
    pub cgroup_ns: u64,
}

impl ContainerIntelligence {
    /// Create new container intelligence
    pub fn new() -> Self {
        Self {
            containers: BTreeMap::new(),
            cgroup_stats: BTreeMap::new(),
            namespaces: BTreeMap::new(),
        }
    }

    /// Register container
    #[inline]
    pub fn register(&mut self, info: ContainerInfo) {
        self.cgroup_stats
            .insert(info.base.id, CgroupStats::default());
        self.containers.insert(info.base.id, info);
    }

    /// Update cgroup stats
    #[inline(always)]
    pub fn update_cgroup(&mut self, id: VirtId, stats: CgroupStats) {
        self.cgroup_stats.insert(id, stats);
    }

    /// Update namespace info
    #[inline(always)]
    pub fn update_namespaces(&mut self, id: VirtId, info: NamespaceInfo) {
        self.namespaces.insert(id, info);
    }

    /// Get container
    #[inline(always)]
    pub fn get(&self, id: VirtId) -> Option<&ContainerInfo> {
        self.containers.get(&id)
    }

    /// Get cgroup stats
    #[inline(always)]
    pub fn get_cgroup(&self, id: VirtId) -> Option<&CgroupStats> {
        self.cgroup_stats.get(&id)
    }

    /// Get namespace info
    #[inline(always)]
    pub fn get_namespaces(&self, id: VirtId) -> Option<&NamespaceInfo> {
        self.namespaces.get(&id)
    }

    /// Get throttled containers
    #[inline]
    pub fn throttled_containers(&self) -> Vec<VirtId> {
        self.cgroup_stats
            .iter()
            .filter(|(_, stats)| stats.is_throttled())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get OOM risk containers
    #[inline]
    pub fn oom_risk_containers(&self) -> Vec<VirtId> {
        self.cgroup_stats
            .iter()
            .filter(|(_, stats)| stats.is_oom_risk())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Container count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.containers.len()
    }
}

impl Default for ContainerIntelligence {
    fn default() -> Self {
        Self::new()
    }
}

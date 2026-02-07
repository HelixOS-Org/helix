//! # Application Container / Namespace Profiling
//!
//! Container-aware application analysis:
//! - Namespace tracking (PID, net, mount, user, IPC, UTS)
//! - Cgroup resource accounting
//! - Container isolation scoring
//! - Cross-container communication detection
//! - Resource limit enforcement
//! - Container lifecycle management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// NAMESPACE TYPES
// ============================================================================

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespaceType {
    /// PID namespace
    Pid,
    /// Network namespace
    Network,
    /// Mount namespace
    Mount,
    /// User namespace
    User,
    /// IPC namespace
    Ipc,
    /// UTS namespace (hostname)
    Uts,
    /// Cgroup namespace
    Cgroup,
    /// Time namespace
    Time,
}

/// Namespace ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NamespaceId {
    /// Type
    pub ns_type: NamespaceType,
    /// Inode
    pub inode: u64,
}

/// Namespace set for a process
#[derive(Debug, Clone)]
pub struct NamespaceSet {
    /// All namespaces
    pub namespaces: BTreeMap<u8, u64>,
}

impl NamespaceSet {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
        }
    }

    /// Set namespace
    pub fn set(&mut self, ns_type: NamespaceType, inode: u64) {
        self.namespaces.insert(ns_type as u8, inode);
    }

    /// Get namespace inode
    pub fn get(&self, ns_type: NamespaceType) -> Option<u64> {
        self.namespaces.get(&(ns_type as u8)).copied()
    }

    /// Check if in same namespace
    pub fn same_ns(&self, other: &NamespaceSet, ns_type: NamespaceType) -> bool {
        match (self.get(ns_type), other.get(ns_type)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    /// Count distinct namespaces from init
    pub fn isolation_depth(&self, init_set: &NamespaceSet) -> u32 {
        let mut depth = 0;
        for &ns_key in self.namespaces.keys() {
            if let Some(&our_inode) = self.namespaces.get(&ns_key) {
                if let Some(&init_inode) = init_set.namespaces.get(&ns_key) {
                    if our_inode != init_inode {
                        depth += 1;
                    }
                }
            }
        }
        depth
    }
}

// ============================================================================
// CGROUP ACCOUNTING
// ============================================================================

/// Cgroup resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CgroupResource {
    /// CPU time
    Cpu,
    /// Memory
    Memory,
    /// I/O bandwidth
    Io,
    /// PIDs
    Pids,
    /// Huge pages
    HugePages,
    /// CPU set
    Cpuset,
}

/// Cgroup limit
#[derive(Debug, Clone)]
pub struct CgroupLimit {
    /// Resource
    pub resource: CgroupResource,
    /// Limit value
    pub limit: u64,
    /// Current usage
    pub usage: u64,
    /// Peak usage
    pub peak: u64,
}

impl CgroupLimit {
    /// Usage percent
    pub fn usage_pct(&self) -> u32 {
        if self.limit == 0 {
            return 0;
        }
        ((self.usage * 100) / self.limit) as u32
    }

    /// Is near limit
    pub fn is_near_limit(&self, threshold_pct: u32) -> bool {
        self.usage_pct() >= threshold_pct
    }

    /// Remaining capacity
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.usage)
    }
}

/// Cgroup state
#[derive(Debug, Clone)]
pub struct CgroupState {
    /// Cgroup path
    pub path: String,
    /// Resource limits
    pub limits: Vec<CgroupLimit>,
    /// Number of processes
    pub num_processes: u32,
    /// Is frozen
    pub frozen: bool,
}

impl CgroupState {
    pub fn new(path: String) -> Self {
        Self {
            path,
            limits: Vec::new(),
            num_processes: 0,
            frozen: false,
        }
    }

    /// Get limit for resource
    pub fn limit(&self, resource: CgroupResource) -> Option<&CgroupLimit> {
        self.limits.iter().find(|l| l.resource == resource)
    }

    /// Most constrained resource
    pub fn most_constrained(&self) -> Option<&CgroupLimit> {
        self.limits.iter().max_by_key(|l| l.usage_pct())
    }
}

// ============================================================================
// CONTAINER PROFILE
// ============================================================================

/// Container isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationLevel {
    /// No isolation (host process)
    None     = 0,
    /// Partial (some namespaces)
    Partial  = 1,
    /// Standard container
    Standard = 2,
    /// High isolation (all namespaces + seccomp)
    High     = 3,
    /// Maximum (VM-like isolation)
    Maximum  = 4,
}

/// Container state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    /// Creating
    Creating,
    /// Running
    Running,
    /// Paused
    Paused,
    /// Stopping
    Stopping,
    /// Stopped
    Stopped,
}

/// Cross-container communication event
#[derive(Debug, Clone)]
pub struct CrossContainerComm {
    /// Source container
    pub source_container: u64,
    /// Target container
    pub target_container: u64,
    /// Communication type
    pub comm_type: CrossContainerCommType,
    /// Timestamp
    pub timestamp: u64,
    /// Bytes transferred
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossContainerCommType {
    /// Network socket
    Network,
    /// Shared volume
    SharedVolume,
    /// Unix socket
    UnixSocket,
    /// Pipe
    Pipe,
    /// Shared memory
    SharedMemory,
}

/// Container profile
#[derive(Debug, Clone)]
pub struct ContainerProfile {
    /// Container ID
    pub id: u64,
    /// Container name
    pub name: String,
    /// State
    pub state: ContainerState,
    /// Isolation level
    pub isolation: IsolationLevel,
    /// Namespace set
    pub namespaces: NamespaceSet,
    /// Cgroup state
    pub cgroup: CgroupState,
    /// Process IDs in container
    pub pids: Vec<u64>,
    /// CPU usage (ms)
    pub cpu_usage_ms: u64,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
    /// Network bytes in/out
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    /// Disk bytes read/written
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    /// OOM kills
    pub oom_kills: u32,
    /// Created timestamp
    pub created_at: u64,
    /// Cross-container comms detected
    pub cross_comms: u32,
}

impl ContainerProfile {
    pub fn new(id: u64, name: String, cgroup_path: String) -> Self {
        Self {
            id,
            name,
            state: ContainerState::Creating,
            isolation: IsolationLevel::None,
            namespaces: NamespaceSet::new(),
            cgroup: CgroupState::new(cgroup_path),
            pids: Vec::new(),
            cpu_usage_ms: 0,
            memory_bytes: 0,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            oom_kills: 0,
            created_at: 0,
            cross_comms: 0,
        }
    }

    /// Compute isolation score (0-100)
    pub fn isolation_score(&self, init_ns: &NamespaceSet) -> u32 {
        let ns_depth = self.namespaces.isolation_depth(init_ns);
        let base = ns_depth * 12; // 8 namespace types × 12 = 96 max

        let cgroup_bonus = if self.cgroup.limits.is_empty() { 0 } else { 4 };

        (base + cgroup_bonus).min(100)
    }

    /// Is resource constrained
    pub fn is_constrained(&self) -> bool {
        self.cgroup.limits.iter().any(|l| l.usage_pct() > 80)
    }
}

// ============================================================================
// CONTAINER ANALYZER
// ============================================================================

/// Container analyzer stats
#[derive(Debug, Clone, Default)]
pub struct ContainerStats {
    /// Total containers
    pub total: usize,
    /// Running containers
    pub running: usize,
    /// Total cross-container comms
    pub cross_comms: u64,
    /// Total OOM kills
    pub total_oom_kills: u32,
    /// Containers near resource limits
    pub constrained_count: usize,
}

/// Application container analyzer
pub struct AppContainerAnalyzer {
    /// Containers by ID
    containers: BTreeMap<u64, ContainerProfile>,
    /// Process to container mapping
    pid_to_container: BTreeMap<u64, u64>,
    /// Cross-container events
    cross_comms: Vec<CrossContainerComm>,
    /// Init namespace set (host)
    init_namespaces: NamespaceSet,
    /// Stats
    stats: ContainerStats,
    /// Max cross-comm events
    max_cross_comms: usize,
}

impl AppContainerAnalyzer {
    pub fn new(init_namespaces: NamespaceSet) -> Self {
        Self {
            containers: BTreeMap::new(),
            pid_to_container: BTreeMap::new(),
            cross_comms: Vec::new(),
            init_namespaces,
            stats: ContainerStats::default(),
            max_cross_comms: 1024,
        }
    }

    /// Register container
    pub fn register_container(&mut self, profile: ContainerProfile) {
        let id = profile.id;
        for &pid in &profile.pids {
            self.pid_to_container.insert(pid, id);
        }
        self.containers.insert(id, profile);
        self.update_stats();
    }

    /// Add process to container
    pub fn add_process(&mut self, container_id: u64, pid: u64) {
        if let Some(container) = self.containers.get_mut(&container_id) {
            if !container.pids.contains(&pid) {
                container.pids.push(pid);
            }
            self.pid_to_container.insert(pid, container_id);
        }
    }

    /// Remove process from container
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(container_id) = self.pid_to_container.remove(&pid) {
            if let Some(container) = self.containers.get_mut(&container_id) {
                container.pids.retain(|&p| p != pid);
            }
        }
    }

    /// Update container resource usage
    pub fn update_resources(
        &mut self,
        container_id: u64,
        cpu_ms: u64,
        memory_bytes: u64,
        net_rx: u64,
        net_tx: u64,
    ) {
        if let Some(container) = self.containers.get_mut(&container_id) {
            container.cpu_usage_ms += cpu_ms;
            container.memory_bytes = memory_bytes;
            container.net_rx_bytes += net_rx;
            container.net_tx_bytes += net_tx;
        }
    }

    /// Record cross-container communication
    pub fn record_cross_comm(&mut self, comm: CrossContainerComm) {
        if let Some(src) = self.containers.get_mut(&comm.source_container) {
            src.cross_comms += 1;
        }
        if let Some(dst) = self.containers.get_mut(&comm.target_container) {
            dst.cross_comms += 1;
        }

        self.cross_comms.push(comm);
        if self.cross_comms.len() > self.max_cross_comms {
            self.cross_comms.remove(0);
        }

        self.stats.cross_comms += 1;
    }

    /// Detect cross-container communication
    pub fn detect_cross_comm(&self, source_pid: u64, target_pid: u64) -> Option<(u64, u64)> {
        let src_container = self.pid_to_container.get(&source_pid)?;
        let dst_container = self.pid_to_container.get(&target_pid)?;

        if src_container != dst_container {
            Some((*src_container, *dst_container))
        } else {
            None
        }
    }

    /// Get container for process
    pub fn container_for_pid(&self, pid: u64) -> Option<u64> {
        self.pid_to_container.get(&pid).copied()
    }

    /// Set container state
    pub fn set_state(&mut self, container_id: u64, state: ContainerState) {
        if let Some(container) = self.containers.get_mut(&container_id) {
            container.state = state;
        }
        self.update_stats();
    }

    /// Get constrained containers
    pub fn constrained_containers(&self) -> Vec<u64> {
        self.containers
            .iter()
            .filter(|(_, c)| c.is_constrained())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Compute isolation score for container
    pub fn isolation_score(&self, container_id: u64) -> Option<u32> {
        self.containers
            .get(&container_id)
            .map(|c| c.isolation_score(&self.init_namespaces))
    }

    fn update_stats(&mut self) {
        self.stats.total = self.containers.len();
        self.stats.running = self
            .containers
            .values()
            .filter(|c| c.state == ContainerState::Running)
            .count();
        self.stats.constrained_count = self
            .containers
            .values()
            .filter(|c| c.is_constrained())
            .count();
        self.stats.total_oom_kills = self.containers.values().map(|c| c.oom_kills).sum();
    }

    /// Get container
    pub fn container(&self, id: u64) -> Option<&ContainerProfile> {
        self.containers.get(&id)
    }

    /// Get stats
    pub fn stats(&self) -> &ContainerStats {
        &self.stats
    }

    /// Remove container
    pub fn remove_container(&mut self, id: u64) {
        if let Some(container) = self.containers.remove(&id) {
            for pid in &container.pids {
                self.pid_to_container.remove(pid);
            }
        }
        self.update_stats();
    }
}

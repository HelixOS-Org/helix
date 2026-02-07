//! # Application Cgroup Profiling
//!
//! Cgroup interaction and resource control analysis:
//! - Cgroup hierarchy tracking
//! - Resource limit enforcement
//! - CPU, memory, I/O controller profiling
//! - Cgroup migration tracking
//! - Resource pressure analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CGROUP CONTROLLER
// ============================================================================

/// Cgroup controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CgroupController {
    /// CPU controller
    Cpu,
    /// CPU set controller
    Cpuset,
    /// Memory controller
    Memory,
    /// I/O controller
    Io,
    /// PID controller
    Pids,
    /// Hugepage controller
    Hugetlb,
    /// RDMA controller
    Rdma,
}

/// Cgroup version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    /// Cgroup v1
    V1,
    /// Cgroup v2 (unified)
    V2,
}

// ============================================================================
// RESOURCE LIMITS
// ============================================================================

/// CPU limit
#[derive(Debug, Clone)]
pub struct CpuLimit {
    /// CPU quota (μs per period, 0 = unlimited)
    pub quota_us: u64,
    /// CPU period (μs)
    pub period_us: u64,
    /// Weight (1-10000)
    pub weight: u32,
    /// CPU set (allowed CPUs)
    pub cpuset: Vec<u32>,
    /// Throttled count
    pub throttled_count: u64,
    /// Throttled time (ns)
    pub throttled_ns: u64,
}

impl CpuLimit {
    pub fn new(quota_us: u64, period_us: u64) -> Self {
        Self {
            quota_us,
            period_us,
            weight: 100,
            cpuset: Vec::new(),
            throttled_count: 0,
            throttled_ns: 0,
        }
    }

    /// Effective CPU fraction
    pub fn cpu_fraction(&self) -> f64 {
        if self.quota_us == 0 || self.period_us == 0 {
            return 1.0;
        }
        self.quota_us as f64 / self.period_us as f64
    }

    /// Throttle rate
    pub fn throttle_rate(&self, total_periods: u64) -> f64 {
        if total_periods == 0 {
            return 0.0;
        }
        self.throttled_count as f64 / total_periods as f64
    }
}

/// Memory limit
#[derive(Debug, Clone)]
pub struct MemoryLimit {
    /// Max bytes (0 = unlimited)
    pub max_bytes: u64,
    /// High bytes (soft limit)
    pub high_bytes: u64,
    /// Low bytes (best-effort protection)
    pub low_bytes: u64,
    /// Min bytes (guaranteed protection)
    pub min_bytes: u64,
    /// Current usage
    pub current_bytes: u64,
    /// Swap max
    pub swap_max: u64,
    /// Swap current
    pub swap_current: u64,
    /// OOM kills
    pub oom_kills: u64,
    /// OOM group kills
    pub oom_group_kills: u64,
}

impl MemoryLimit {
    pub fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes,
            high_bytes: 0,
            low_bytes: 0,
            min_bytes: 0,
            current_bytes: 0,
            swap_max: 0,
            swap_current: 0,
            oom_kills: 0,
            oom_group_kills: 0,
        }
    }

    /// Memory utilization (fraction of max)
    pub fn utilization(&self) -> f64 {
        if self.max_bytes == 0 {
            return 0.0;
        }
        self.current_bytes as f64 / self.max_bytes as f64
    }

    /// Memory pressure level
    pub fn pressure_level(&self) -> CgroupPressure {
        let util = self.utilization();
        if util > 0.95 {
            CgroupPressure::Critical
        } else if util > 0.8 {
            CgroupPressure::High
        } else if util > 0.6 {
            CgroupPressure::Medium
        } else {
            CgroupPressure::Low
        }
    }
}

/// I/O limit
#[derive(Debug, Clone)]
pub struct IoLimit {
    /// Read BPS limit (0 = unlimited)
    pub rbps_max: u64,
    /// Write BPS limit
    pub wbps_max: u64,
    /// Read IOPS limit
    pub riops_max: u64,
    /// Write IOPS limit
    pub wiops_max: u64,
    /// Weight (1-10000)
    pub weight: u32,
    /// Current read throughput
    pub read_bps: u64,
    /// Current write throughput
    pub write_bps: u64,
}

impl IoLimit {
    pub fn new() -> Self {
        Self {
            rbps_max: 0,
            wbps_max: 0,
            riops_max: 0,
            wiops_max: 0,
            weight: 100,
            read_bps: 0,
            write_bps: 0,
        }
    }

    /// Read utilization
    pub fn read_utilization(&self) -> f64 {
        if self.rbps_max == 0 {
            return 0.0;
        }
        self.read_bps as f64 / self.rbps_max as f64
    }

    /// Write utilization
    pub fn write_utilization(&self) -> f64 {
        if self.wbps_max == 0 {
            return 0.0;
        }
        self.write_bps as f64 / self.wbps_max as f64
    }
}

/// PID limit
#[derive(Debug, Clone)]
pub struct PidLimit {
    /// Max PIDs
    pub max_pids: u32,
    /// Current PIDs
    pub current_pids: u32,
    /// Denials
    pub denials: u64,
}

impl PidLimit {
    pub fn new(max: u32) -> Self {
        Self {
            max_pids: max,
            current_pids: 0,
            denials: 0,
        }
    }

    /// Utilization
    pub fn utilization(&self) -> f64 {
        if self.max_pids == 0 {
            return 0.0;
        }
        self.current_pids as f64 / self.max_pids as f64
    }
}

/// Cgroup pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupPressure {
    /// Low
    Low,
    /// Medium
    Medium,
    /// High
    High,
    /// Critical
    Critical,
}

// ============================================================================
// CGROUP NODE
// ============================================================================

/// Cgroup node in hierarchy
#[derive(Debug, Clone)]
pub struct CgroupNode {
    /// Cgroup ID
    pub id: u64,
    /// Path in hierarchy
    pub path: String,
    /// Parent ID
    pub parent: Option<u64>,
    /// Children IDs
    pub children: Vec<u64>,
    /// Assigned PIDs
    pub pids: Vec<u64>,
    /// CPU limit
    pub cpu: Option<CpuLimit>,
    /// Memory limit
    pub memory: Option<MemoryLimit>,
    /// I/O limit
    pub io: Option<IoLimit>,
    /// PID limit
    pub pid_limit: Option<PidLimit>,
    /// Controllers enabled
    pub controllers: Vec<CgroupController>,
    /// Created at
    pub created_at: u64,
}

impl CgroupNode {
    pub fn new(id: u64, path: String) -> Self {
        Self {
            id,
            path,
            parent: None,
            children: Vec::new(),
            pids: Vec::new(),
            cpu: None,
            memory: None,
            io: None,
            pid_limit: None,
            controllers: Vec::new(),
            created_at: 0,
        }
    }

    /// Process count
    pub fn process_count(&self) -> usize {
        self.pids.len()
    }

    /// Add process
    pub fn add_pid(&mut self, pid: u64) {
        if !self.pids.contains(&pid) {
            self.pids.push(pid);
        }
    }

    /// Remove process
    pub fn remove_pid(&mut self, pid: u64) {
        self.pids.retain(|&p| p != pid);
    }

    /// Overall pressure
    pub fn overall_pressure(&self) -> CgroupPressure {
        let mem_pressure = self
            .memory
            .as_ref()
            .map(|m| m.pressure_level())
            .unwrap_or(CgroupPressure::Low);

        let cpu_pressure = self
            .cpu
            .as_ref()
            .map(|c| {
                if c.throttled_count > 100 {
                    CgroupPressure::High
                } else if c.throttled_count > 10 {
                    CgroupPressure::Medium
                } else {
                    CgroupPressure::Low
                }
            })
            .unwrap_or(CgroupPressure::Low);

        // Return worst
        match (mem_pressure, cpu_pressure) {
            (CgroupPressure::Critical, _) | (_, CgroupPressure::Critical) => {
                CgroupPressure::Critical
            }
            (CgroupPressure::High, _) | (_, CgroupPressure::High) => CgroupPressure::High,
            (CgroupPressure::Medium, _) | (_, CgroupPressure::Medium) => CgroupPressure::Medium,
            _ => CgroupPressure::Low,
        }
    }
}

// ============================================================================
// CGROUP MIGRATION
// ============================================================================

/// Cgroup migration event
#[derive(Debug, Clone)]
pub struct CgroupMigration {
    /// Process ID
    pub pid: u64,
    /// From cgroup
    pub from_cgroup: u64,
    /// To cgroup
    pub to_cgroup: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Voluntary
    pub voluntary: bool,
}

// ============================================================================
// CGROUP ANALYZER
// ============================================================================

/// Cgroup analyzer stats
#[derive(Debug, Clone, Default)]
pub struct AppCgroupStats {
    /// Cgroup count
    pub cgroup_count: usize,
    /// Total processes tracked
    pub total_processes: usize,
    /// OOM kills
    pub oom_kills: u64,
    /// CPU throttle events
    pub throttle_events: u64,
    /// Migrations
    pub migrations: u64,
    /// Cgroups under pressure
    pub pressure_count: usize,
}

/// Application cgroup analyzer
pub struct AppCgroupAnalyzer {
    /// Cgroup nodes
    nodes: BTreeMap<u64, CgroupNode>,
    /// PID to cgroup mapping
    pid_cgroup: BTreeMap<u64, u64>,
    /// Migration log
    migrations: Vec<CgroupMigration>,
    /// Max migration log
    max_migrations: usize,
    /// Next cgroup ID
    next_id: u64,
    /// Stats
    stats: AppCgroupStats,
}

impl AppCgroupAnalyzer {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            pid_cgroup: BTreeMap::new(),
            migrations: Vec::new(),
            max_migrations: 256,
            next_id: 1,
            stats: AppCgroupStats::default(),
        }
    }

    /// Create cgroup
    pub fn create_cgroup(&mut self, path: String, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut node = CgroupNode::new(id, path);
        node.parent = parent;
        if let Some(parent_id) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&parent_id) {
                parent_node.children.push(id);
            }
        }
        self.nodes.insert(id, node);
        self.stats.cgroup_count = self.nodes.len();
        id
    }

    /// Assign process to cgroup
    pub fn assign_process(&mut self, pid: u64, cgroup_id: u64, now: u64) {
        // Remove from old cgroup
        if let Some(&old_cgroup) = self.pid_cgroup.get(&pid) {
            if old_cgroup != cgroup_id {
                if let Some(old_node) = self.nodes.get_mut(&old_cgroup) {
                    old_node.remove_pid(pid);
                }
                self.migrations.push(CgroupMigration {
                    pid,
                    from_cgroup: old_cgroup,
                    to_cgroup: cgroup_id,
                    timestamp: now,
                    voluntary: true,
                });
                if self.migrations.len() > self.max_migrations {
                    self.migrations.remove(0);
                }
                self.stats.migrations += 1;
            }
        }

        // Add to new cgroup
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.add_pid(pid);
        }
        self.pid_cgroup.insert(pid, cgroup_id);
        self.stats.total_processes = self.pid_cgroup.len();
    }

    /// Set CPU limit
    pub fn set_cpu_limit(&mut self, cgroup_id: u64, limit: CpuLimit) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.cpu = Some(limit);
            if !node.controllers.contains(&CgroupController::Cpu) {
                node.controllers.push(CgroupController::Cpu);
            }
        }
    }

    /// Set memory limit
    pub fn set_memory_limit(&mut self, cgroup_id: u64, limit: MemoryLimit) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.memory = Some(limit);
            if !node.controllers.contains(&CgroupController::Memory) {
                node.controllers.push(CgroupController::Memory);
            }
        }
    }

    /// Set I/O limit
    pub fn set_io_limit(&mut self, cgroup_id: u64, limit: IoLimit) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            node.io = Some(limit);
            if !node.controllers.contains(&CgroupController::Io) {
                node.controllers.push(CgroupController::Io);
            }
        }
    }

    /// Update memory usage
    pub fn update_memory_usage(&mut self, cgroup_id: u64, bytes: u64) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            if let Some(mem) = &mut node.memory {
                mem.current_bytes = bytes;
            }
        }
    }

    /// Record OOM kill
    pub fn record_oom_kill(&mut self, cgroup_id: u64) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            if let Some(mem) = &mut node.memory {
                mem.oom_kills += 1;
            }
        }
        self.stats.oom_kills += 1;
    }

    /// Record CPU throttle
    pub fn record_throttle(&mut self, cgroup_id: u64, duration_ns: u64) {
        if let Some(node) = self.nodes.get_mut(&cgroup_id) {
            if let Some(cpu) = &mut node.cpu {
                cpu.throttled_count += 1;
                cpu.throttled_ns += duration_ns;
            }
        }
        self.stats.throttle_events += 1;
    }

    /// Get cgroup for PID
    pub fn cgroup_for_pid(&self, pid: u64) -> Option<&CgroupNode> {
        self.pid_cgroup
            .get(&pid)
            .and_then(|&id| self.nodes.get(&id))
    }

    /// Find pressured cgroups
    pub fn pressured_cgroups(&self) -> Vec<u64> {
        self.nodes
            .iter()
            .filter(|(_, n)| {
                matches!(
                    n.overall_pressure(),
                    CgroupPressure::High | CgroupPressure::Critical
                )
            })
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get node
    pub fn node(&self, id: u64) -> Option<&CgroupNode> {
        self.nodes.get(&id)
    }

    /// Stats
    pub fn stats(&self) -> &AppCgroupStats {
        &self.stats
    }
}

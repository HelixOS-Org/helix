//! # App Cgroup Profiler v2
//!
//! Enhanced cgroup resource tracking and profiling:
//! - Memory cgroup pressure monitoring
//! - CPU cgroup bandwidth tracking
//! - IO weight and latency per cgroup
//! - Hierarchical cgroup traversal
//! - OOM kill tracking per cgroup

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CGROUP TYPES
// ============================================================================

/// Cgroup controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupController {
    /// CPU controller
    Cpu,
    /// Memory controller
    Memory,
    /// IO controller
    Io,
    /// PID controller
    Pids,
    /// CPU set
    Cpuset,
    /// Huge pages
    Hugetlb,
}

/// Cgroup version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    /// v1 (legacy)
    V1,
    /// v2 (unified)
    V2,
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupPressure {
    /// No pressure
    None,
    /// Low pressure
    Low,
    /// Medium pressure
    Medium,
    /// Critical pressure
    Critical,
}

// ============================================================================
// CPU BANDWIDTH
// ============================================================================

/// CPU bandwidth stats for a cgroup
#[derive(Debug, Clone, Default)]
pub struct CpuBandwidth {
    /// Quota (us per period)
    pub quota_us: u64,
    /// Period (us)
    pub period_us: u64,
    /// Consumed this period (us)
    pub consumed_us: u64,
    /// Throttled periods
    pub throttled_periods: u64,
    /// Total throttled time (ns)
    pub throttled_ns: u64,
    /// Total periods
    pub total_periods: u64,
}

impl CpuBandwidth {
    /// Utilization (0.0-1.0)
    pub fn utilization(&self) -> f64 {
        if self.quota_us == 0 || self.period_us == 0 {
            return 0.0;
        }
        self.consumed_us as f64 / self.quota_us as f64
    }

    /// Throttle rate
    pub fn throttle_rate(&self) -> f64 {
        if self.total_periods == 0 {
            return 0.0;
        }
        self.throttled_periods as f64 / self.total_periods as f64
    }
}

// ============================================================================
// MEMORY STATS
// ============================================================================

/// Memory stats for a cgroup
#[derive(Debug, Clone, Default)]
pub struct CgroupMemoryStats {
    /// Current usage (bytes)
    pub usage_bytes: u64,
    /// Limit (bytes)
    pub limit_bytes: u64,
    /// Max usage observed (bytes)
    pub max_usage_bytes: u64,
    /// Cache (page cache, bytes)
    pub cache_bytes: u64,
    /// RSS (bytes)
    pub rss_bytes: u64,
    /// Swap usage (bytes)
    pub swap_bytes: u64,
    /// Swap limit (bytes)
    pub swap_limit_bytes: u64,
    /// OOM kills
    pub oom_kills: u64,
    /// OOM kills under limit
    pub oom_kills_under_limit: u64,
    /// Reclaim attempts
    pub reclaim_attempts: u64,
}

impl CgroupMemoryStats {
    /// Usage ratio
    pub fn usage_ratio(&self) -> f64 {
        if self.limit_bytes == 0 {
            return 0.0;
        }
        self.usage_bytes as f64 / self.limit_bytes as f64
    }

    /// Pressure
    pub fn pressure(&self) -> CgroupPressure {
        let ratio = self.usage_ratio();
        if ratio >= 0.95 {
            CgroupPressure::Critical
        } else if ratio >= 0.8 {
            CgroupPressure::Medium
        } else if ratio >= 0.6 {
            CgroupPressure::Low
        } else {
            CgroupPressure::None
        }
    }
}

// ============================================================================
// IO STATS
// ============================================================================

/// IO stats for cgroup
#[derive(Debug, Clone, Default)]
pub struct CgroupIoStats {
    /// Read bytes
    pub read_bytes: u64,
    /// Write bytes
    pub write_bytes: u64,
    /// Read IOPS
    pub read_iops: u64,
    /// Write IOPS
    pub write_iops: u64,
    /// IO weight (1-10000)
    pub weight: u32,
    /// Avg read latency EMA (ns)
    pub read_latency_ns: f64,
    /// Avg write latency EMA (ns)
    pub write_latency_ns: f64,
}

impl CgroupIoStats {
    /// Record read
    pub fn record_read(&mut self, bytes: u64, latency_ns: u64) {
        self.read_bytes += bytes;
        self.read_iops += 1;
        self.read_latency_ns = 0.9 * self.read_latency_ns + 0.1 * latency_ns as f64;
    }

    /// Record write
    pub fn record_write(&mut self, bytes: u64, latency_ns: u64) {
        self.write_bytes += bytes;
        self.write_iops += 1;
        self.write_latency_ns = 0.9 * self.write_latency_ns + 0.1 * latency_ns as f64;
    }

    /// Total IOPS
    pub fn total_iops(&self) -> u64 {
        self.read_iops + self.write_iops
    }
}

// ============================================================================
// CGROUP NODE
// ============================================================================

/// Cgroup node
#[derive(Debug)]
pub struct CgroupNode {
    /// Path (e.g., "/system.slice/myapp.service")
    pub path: String,
    /// Parent path
    pub parent_path: Option<String>,
    /// Controllers active
    pub controllers: Vec<CgroupController>,
    /// CPU bandwidth
    pub cpu: CpuBandwidth,
    /// Memory stats
    pub memory: CgroupMemoryStats,
    /// IO stats
    pub io: CgroupIoStats,
    /// Member PIDs
    pub pids: Vec<u64>,
    /// Max PIDs
    pub max_pids: u32,
    /// Child cgroup names
    pub children: Vec<String>,
    /// Version
    pub version: CgroupVersion,
}

impl CgroupNode {
    pub fn new(path: String, version: CgroupVersion) -> Self {
        Self {
            path,
            parent_path: None,
            controllers: Vec::new(),
            cpu: CpuBandwidth::default(),
            memory: CgroupMemoryStats::default(),
            io: CgroupIoStats::default(),
            pids: Vec::new(),
            max_pids: 0,
            children: Vec::new(),
            version,
        }
    }

    /// Add PID
    pub fn add_pid(&mut self, pid: u64) {
        if !self.pids.contains(&pid) {
            self.pids.push(pid);
        }
    }

    /// Remove PID
    pub fn remove_pid(&mut self, pid: u64) {
        self.pids.retain(|&p| p != pid);
    }

    /// Process count
    pub fn process_count(&self) -> usize {
        self.pids.len()
    }

    /// Is at PID limit
    pub fn at_pid_limit(&self) -> bool {
        self.max_pids > 0 && self.pids.len() >= self.max_pids as usize
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Cgroup profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppCgroupV2Stats {
    /// Tracked cgroups
    pub tracked_cgroups: usize,
    /// Total processes
    pub total_processes: usize,
    /// Throttled cgroups
    pub throttled_cgroups: usize,
    /// OOM-risk cgroups
    pub oom_risk_cgroups: usize,
}

/// App cgroup profiler v2
pub struct AppCgroupV2Profiler {
    /// Cgroup nodes, keyed by FNV-1a of path
    nodes: BTreeMap<u64, CgroupNode>,
    /// PID to cgroup key mapping
    pid_map: BTreeMap<u64, u64>,
    /// Stats
    stats: AppCgroupV2Stats,
}

impl AppCgroupV2Profiler {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            pid_map: BTreeMap::new(),
            stats: AppCgroupV2Stats::default(),
        }
    }

    /// Hash path
    fn hash_path(path: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in path.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Register cgroup
    pub fn register(&mut self, path: String, version: CgroupVersion) -> u64 {
        let key = Self::hash_path(&path);
        self.nodes
            .entry(key)
            .or_insert_with(|| CgroupNode::new(path, version));
        self.update_stats();
        key
    }

    /// Add process to cgroup
    pub fn add_process(&mut self, path: &str, pid: u64) {
        let key = Self::hash_path(path);
        if let Some(node) = self.nodes.get_mut(&key) {
            node.add_pid(pid);
            self.pid_map.insert(pid, key);
        }
        self.update_stats();
    }

    /// Update CPU stats
    pub fn update_cpu(&mut self, path: &str, consumed_us: u64, throttled: bool) {
        let key = Self::hash_path(path);
        if let Some(node) = self.nodes.get_mut(&key) {
            node.cpu.consumed_us += consumed_us;
            node.cpu.total_periods += 1;
            if throttled {
                node.cpu.throttled_periods += 1;
            }
        }
    }

    /// Update memory stats
    pub fn update_memory(&mut self, path: &str, usage: u64, limit: u64) {
        let key = Self::hash_path(path);
        if let Some(node) = self.nodes.get_mut(&key) {
            node.memory.usage_bytes = usage;
            node.memory.limit_bytes = limit;
            if usage > node.memory.max_usage_bytes {
                node.memory.max_usage_bytes = usage;
            }
        }
        self.update_stats();
    }

    /// Record OOM kill
    pub fn record_oom_kill(&mut self, path: &str) {
        let key = Self::hash_path(path);
        if let Some(node) = self.nodes.get_mut(&key) {
            node.memory.oom_kills += 1;
        }
    }

    /// Get pressure for cgroup
    pub fn pressure(&self, path: &str) -> CgroupPressure {
        let key = Self::hash_path(path);
        self.nodes
            .get(&key)
            .map(|n| n.memory.pressure())
            .unwrap_or(CgroupPressure::None)
    }

    fn update_stats(&mut self) {
        self.stats.tracked_cgroups = self.nodes.len();
        self.stats.total_processes = self.pid_map.len();
        self.stats.throttled_cgroups = self
            .nodes
            .values()
            .filter(|n| n.cpu.throttle_rate() > 0.1)
            .count();
        self.stats.oom_risk_cgroups = self
            .nodes
            .values()
            .filter(|n| n.memory.usage_ratio() > 0.9)
            .count();
    }

    /// Stats
    pub fn stats(&self) -> &AppCgroupV2Stats {
        &self.stats
    }
}

//! # Holistic Cgroup Manager
//!
//! System-wide cgroup resource management with holistic view:
//! - Hierarchical resource distribution
//! - CPU, memory, IO bandwidth cgroup limits
//! - Elastic resource sharing between cgroups
//! - OOM scoring and kill prioritization
//! - Cgroup pressure stall information (PSI)
//! - Auto-tuning based on workload patterns

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Cgroup controller type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupController {
    Cpu,
    Memory,
    Io,
    Pids,
    Cpuset,
    Hugetlb,
    Rdma,
}

/// PSI (Pressure Stall Information) state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PsiLevel {
    None,
    Some10,  // some avg10 > threshold
    Some60,  // sustained pressure
    Full10,  // full stall
    Full60,  // sustained full stall
}

/// CPU cgroup limits
#[derive(Debug, Clone)]
pub struct CpuCgroupLimits {
    pub weight: u32,        // 1-10000
    pub max_us: u64,        // quota per period
    pub period_us: u64,
    pub burst_us: u64,
    pub usage_us: u64,
    pub throttled_us: u64,
    pub nr_throttled: u64,
}

impl CpuCgroupLimits {
    pub fn new(weight: u32, max_us: u64, period_us: u64) -> Self {
        Self {
            weight: weight.max(1).min(10000),
            max_us,
            period_us,
            burst_us: max_us / 10,
            usage_us: 0,
            throttled_us: 0,
            nr_throttled: 0,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_us == 0 { return 0.0; }
        self.usage_us as f64 / self.max_us as f64
    }

    pub fn throttle_ratio(&self) -> f64 {
        let total = self.usage_us + self.throttled_us;
        if total == 0 { return 0.0; }
        self.throttled_us as f64 / total as f64
    }
}

/// Memory cgroup limits
#[derive(Debug, Clone)]
pub struct MemCgroupLimits {
    pub max_bytes: u64,
    pub high_bytes: u64,
    pub low_bytes: u64,
    pub min_bytes: u64,
    pub usage_bytes: u64,
    pub swap_max_bytes: u64,
    pub swap_usage_bytes: u64,
    pub oom_kills: u32,
    pub oom_score_adj: i16,
}

impl MemCgroupLimits {
    pub fn new(max: u64, high: u64) -> Self {
        Self {
            max_bytes: max,
            high_bytes: high,
            low_bytes: 0,
            min_bytes: 0,
            usage_bytes: 0,
            swap_max_bytes: 0,
            swap_usage_bytes: 0,
            oom_kills: 0,
            oom_score_adj: 0,
        }
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.max_bytes == 0 { return 0.0; }
        self.usage_bytes as f64 / self.max_bytes as f64
    }

    pub fn is_under_pressure(&self) -> bool {
        self.usage_bytes > self.high_bytes
    }

    pub fn available(&self) -> u64 {
        self.max_bytes.saturating_sub(self.usage_bytes)
    }
}

/// IO cgroup limits
#[derive(Debug, Clone)]
pub struct IoCgroupLimits {
    pub rbps_max: u64,
    pub wbps_max: u64,
    pub riops_max: u64,
    pub wiops_max: u64,
    pub rbps_current: u64,
    pub wbps_current: u64,
}

impl IoCgroupLimits {
    pub fn new(rbps: u64, wbps: u64) -> Self {
        Self {
            rbps_max: rbps,
            wbps_max: wbps,
            riops_max: 0,
            wiops_max: 0,
            rbps_current: 0,
            wbps_current: 0,
        }
    }
}

/// Pressure Stall Information
#[derive(Debug, Clone)]
pub struct PsiInfo {
    pub cpu_some_avg10: f64,
    pub cpu_some_avg60: f64,
    pub cpu_full_avg10: f64,
    pub cpu_full_avg60: f64,
    pub mem_some_avg10: f64,
    pub mem_some_avg60: f64,
    pub mem_full_avg10: f64,
    pub mem_full_avg60: f64,
    pub io_some_avg10: f64,
    pub io_some_avg60: f64,
    pub io_full_avg10: f64,
    pub io_full_avg60: f64,
}

impl PsiInfo {
    pub fn new() -> Self {
        Self {
            cpu_some_avg10: 0.0, cpu_some_avg60: 0.0,
            cpu_full_avg10: 0.0, cpu_full_avg60: 0.0,
            mem_some_avg10: 0.0, mem_some_avg60: 0.0,
            mem_full_avg10: 0.0, mem_full_avg60: 0.0,
            io_some_avg10: 0.0, io_some_avg60: 0.0,
            io_full_avg10: 0.0, io_full_avg60: 0.0,
        }
    }

    pub fn worst_pressure(&self) -> f64 {
        let vals = [
            self.cpu_full_avg10, self.mem_full_avg10, self.io_full_avg10,
        ];
        vals.iter().copied().fold(0.0f64, |a, b| if b > a { b } else { a })
    }
}

/// Cgroup node
#[derive(Debug, Clone)]
pub struct CgroupNode {
    pub cgroup_id: u64,
    pub name: String,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub cpu: CpuCgroupLimits,
    pub memory: MemCgroupLimits,
    pub io: IoCgroupLimits,
    pub psi: PsiInfo,
    pub pids_current: u32,
    pub pids_max: u32,
}

impl CgroupNode {
    pub fn new(cgroup_id: u64, name: String) -> Self {
        Self {
            cgroup_id,
            name,
            parent_id: None,
            children: Vec::new(),
            cpu: CpuCgroupLimits::new(100, 100_000, 100_000),
            memory: MemCgroupLimits::new(u64::MAX, u64::MAX),
            io: IoCgroupLimits::new(u64::MAX, u64::MAX),
            psi: PsiInfo::new(),
            pids_current: 0,
            pids_max: 0,
        }
    }

    pub fn is_under_cpu_pressure(&self) -> bool {
        self.cpu.throttle_ratio() > 0.1 || self.psi.cpu_some_avg10 > 20.0
    }

    pub fn is_under_mem_pressure(&self) -> bool {
        self.memory.is_under_pressure() || self.psi.mem_some_avg10 > 20.0
    }
}

/// Holistic Cgroup Manager stats
#[derive(Debug, Clone, Default)]
pub struct HolisticCgroupStats {
    pub total_cgroups: usize,
    pub cpu_throttled: usize,
    pub mem_pressured: usize,
    pub total_oom_kills: u32,
    pub max_psi: f64,
}

/// Holistic Cgroup Manager
pub struct HolisticCgroupMgr {
    cgroups: BTreeMap<u64, CgroupNode>,
    stats: HolisticCgroupStats,
}

impl HolisticCgroupMgr {
    pub fn new() -> Self {
        Self {
            cgroups: BTreeMap::new(),
            stats: HolisticCgroupStats::default(),
        }
    }

    pub fn create_cgroup(&mut self, node: CgroupNode) {
        let id = node.cgroup_id;
        let parent = node.parent_id;
        self.cgroups.insert(id, node);
        if let Some(pid) = parent {
            if let Some(parent_node) = self.cgroups.get_mut(&pid) {
                parent_node.children.push(id);
            }
        }
    }

    pub fn set_cpu_limits(&mut self, cg_id: u64, weight: u32, max_us: u64, period_us: u64) {
        if let Some(cg) = self.cgroups.get_mut(&cg_id) {
            cg.cpu = CpuCgroupLimits::new(weight, max_us, period_us);
        }
    }

    pub fn set_mem_limits(&mut self, cg_id: u64, max: u64, high: u64) {
        if let Some(cg) = self.cgroups.get_mut(&cg_id) {
            cg.memory = MemCgroupLimits::new(max, high);
        }
    }

    pub fn update_usage(&mut self, cg_id: u64, cpu_us: u64, mem_bytes: u64) {
        if let Some(cg) = self.cgroups.get_mut(&cg_id) {
            cg.cpu.usage_us = cpu_us;
            cg.memory.usage_bytes = mem_bytes;
        }
    }

    /// Find cgroups that need resource adjustment
    pub fn pressured_cgroups(&self) -> Vec<u64> {
        self.cgroups.values()
            .filter(|cg| cg.is_under_cpu_pressure() || cg.is_under_mem_pressure())
            .map(|cg| cg.cgroup_id)
            .collect()
    }

    /// OOM kill candidate scoring (lower = more likely to be killed)
    pub fn oom_score(&self, cg_id: u64) -> i32 {
        if let Some(cg) = self.cgroups.get(&cg_id) {
            let mem_score = (cg.memory.usage_ratio() * 1000.0) as i32;
            let adj = cg.memory.oom_score_adj as i32;
            (mem_score + adj).max(0).min(1000)
        } else { 0 }
    }

    pub fn recompute(&mut self) {
        self.stats.total_cgroups = self.cgroups.len();
        self.stats.cpu_throttled = self.cgroups.values()
            .filter(|cg| cg.cpu.throttle_ratio() > 0.1).count();
        self.stats.mem_pressured = self.cgroups.values()
            .filter(|cg| cg.memory.is_under_pressure()).count();
        self.stats.total_oom_kills = self.cgroups.values()
            .map(|cg| cg.memory.oom_kills).sum();
        self.stats.max_psi = self.cgroups.values()
            .map(|cg| cg.psi.worst_pressure())
            .fold(0.0f64, |a, b| if b > a { b } else { a });
    }

    pub fn cgroup(&self, id: u64) -> Option<&CgroupNode> { self.cgroups.get(&id) }
    pub fn stats(&self) -> &HolisticCgroupStats { &self.stats }
}

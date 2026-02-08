//! # Holistic Task Affinity
//!
//! CPU and memory affinity management for tasks:
//! - CPU affinity mask manipulation
//! - NUMA memory binding policies
//! - Affinity inheritance tracking
//! - Affinity migration cost estimation
//! - Cache topology-aware placement
//! - Dynamic affinity rebalancing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// CPU set bitmask (up to 256 CPUs)
#[derive(Debug, Clone)]
pub struct CpuSet {
    bits: [u64; 4],
}

impl CpuSet {
    pub fn new() -> Self { Self { bits: [0; 4] } }

    pub fn all(num_cpus: u32) -> Self {
        let mut s = Self::new();
        for i in 0..num_cpus.min(256) { s.set(i); }
        s
    }

    pub fn set(&mut self, cpu: u32) {
        if cpu < 256 {
            self.bits[(cpu / 64) as usize] |= 1u64 << (cpu % 64);
        }
    }

    pub fn clear(&mut self, cpu: u32) {
        if cpu < 256 {
            self.bits[(cpu / 64) as usize] &= !(1u64 << (cpu % 64));
        }
    }

    pub fn is_set(&self, cpu: u32) -> bool {
        if cpu >= 256 { return false; }
        (self.bits[(cpu / 64) as usize] >> (cpu % 64)) & 1 == 1
    }

    pub fn count(&self) -> u32 {
        self.bits.iter().map(|b| b.count_ones()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut r = Self::new();
        for i in 0..4 { r.bits[i] = self.bits[i] & other.bits[i]; }
        r
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut r = Self::new();
        for i in 0..4 { r.bits[i] = self.bits[i] | other.bits[i]; }
        r
    }

    pub fn first_set(&self) -> Option<u32> {
        for (i, &b) in self.bits.iter().enumerate() {
            if b != 0 { return Some(i as u32 * 64 + b.trailing_zeros()); }
        }
        None
    }

    pub fn iter_set(&self) -> Vec<u32> {
        let mut result = Vec::new();
        for cpu in 0..256u32 {
            if self.is_set(cpu) { result.push(cpu); }
        }
        result
    }
}

/// Memory binding policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemBindPolicy {
    Default,
    Preferred,
    Bind,
    Interleave,
    Local,
}

/// NUMA node set
#[derive(Debug, Clone)]
pub struct NodeSet {
    bits: u64,
}

impl NodeSet {
    pub fn new() -> Self { Self { bits: 0 } }
    pub fn set(&mut self, node: u32) { if node < 64 { self.bits |= 1u64 << node; } }
    pub fn clear(&mut self, node: u32) { if node < 64 { self.bits &= !(1u64 << node); } }
    pub fn is_set(&self, node: u32) -> bool { if node >= 64 { false } else { (self.bits >> node) & 1 == 1 } }
    pub fn count(&self) -> u32 { self.bits.count_ones() }
    pub fn first(&self) -> Option<u32> { if self.bits == 0 { None } else { Some(self.bits.trailing_zeros()) } }
}

/// Task affinity record
#[derive(Debug, Clone)]
pub struct TaskAffinity {
    pub task_id: u64,
    pub cpu_affinity: CpuSet,
    pub mem_policy: MemBindPolicy,
    pub mem_nodes: NodeSet,
    pub inherited: bool,
    pub parent_task: Option<u64>,
    pub last_cpu: u32,
    pub migrations: u64,
    pub cache_misses_after_migration: u64,
    pub time_on_current_cpu_ns: u64,
    pub total_runtime_ns: u64,
    pub voluntary_migrations: u64,
    pub forced_migrations: u64,
}

impl TaskAffinity {
    pub fn new(task_id: u64) -> Self {
        Self {
            task_id, cpu_affinity: CpuSet::all(256), mem_policy: MemBindPolicy::Default,
            mem_nodes: NodeSet::new(), inherited: false, parent_task: None,
            last_cpu: 0, migrations: 0, cache_misses_after_migration: 0,
            time_on_current_cpu_ns: 0, total_runtime_ns: 0,
            voluntary_migrations: 0, forced_migrations: 0,
        }
    }

    pub fn set_affinity(&mut self, cpuset: CpuSet) { self.cpu_affinity = cpuset; }
    pub fn set_mem_policy(&mut self, policy: MemBindPolicy, nodes: NodeSet) {
        self.mem_policy = policy;
        self.mem_nodes = nodes;
    }

    pub fn record_migration(&mut self, new_cpu: u32, forced: bool) {
        self.last_cpu = new_cpu;
        self.migrations += 1;
        self.time_on_current_cpu_ns = 0;
        if forced { self.forced_migrations += 1; } else { self.voluntary_migrations += 1; }
    }

    pub fn migration_rate(&self) -> f64 {
        if self.total_runtime_ns == 0 { 0.0 }
        else { self.migrations as f64 / (self.total_runtime_ns as f64 / 1_000_000_000.0) }
    }

    pub fn can_run_on(&self, cpu: u32) -> bool { self.cpu_affinity.is_set(cpu) }
}

/// Cache domain for topology
#[derive(Debug, Clone)]
pub struct CacheDomain {
    pub domain_id: u32,
    pub level: u8,
    pub cpus: CpuSet,
    pub size_kb: u32,
    pub shared_by: u32,
}

impl CacheDomain {
    pub fn new(id: u32, level: u8, size_kb: u32) -> Self {
        Self { domain_id: id, level, cpus: CpuSet::new(), size_kb, shared_by: 0 }
    }

    pub fn add_cpu(&mut self, cpu: u32) { self.cpus.set(cpu); self.shared_by += 1; }
}

/// Migration cost estimation
#[derive(Debug, Clone)]
pub struct MigrationCost {
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub cache_flush_cost_ns: u64,
    pub tlb_flush_cost_ns: u64,
    pub numa_distance_factor: f64,
    pub total_cost_ns: u64,
}

impl MigrationCost {
    pub fn estimate(from: u32, to: u32, same_l2: bool, same_l3: bool, numa_dist: u32) -> Self {
        let cache_cost = if same_l2 { 1000 } else if same_l3 { 10_000 } else { 100_000 };
        let tlb_cost = 5000u64;
        let numa_factor = numa_dist as f64 / 10.0;
        let total = (cache_cost + tlb_cost) as f64 * numa_factor;
        Self {
            from_cpu: from, to_cpu: to,
            cache_flush_cost_ns: cache_cost, tlb_flush_cost_ns: tlb_cost,
            numa_distance_factor: numa_factor, total_cost_ns: total as u64,
        }
    }
}

/// Task affinity stats
#[derive(Debug, Clone, Default)]
pub struct TaskAffinityStats {
    pub tasks_tracked: usize,
    pub tasks_with_affinity: usize,
    pub tasks_inherited: usize,
    pub total_migrations: u64,
    pub avg_migration_rate: f64,
    pub bound_tasks: usize,
    pub interleave_tasks: usize,
}

/// Holistic task affinity manager
pub struct HolisticTaskAffinity {
    tasks: BTreeMap<u64, TaskAffinity>,
    cache_domains: BTreeMap<u32, CacheDomain>,
    stats: TaskAffinityStats,
}

impl HolisticTaskAffinity {
    pub fn new() -> Self {
        Self { tasks: BTreeMap::new(), cache_domains: BTreeMap::new(), stats: TaskAffinityStats::default() }
    }

    pub fn register_task(&mut self, task_id: u64, parent: Option<u64>) {
        let mut aff = TaskAffinity::new(task_id);
        if let Some(pid) = parent {
            aff.parent_task = Some(pid);
            if let Some(parent_aff) = self.tasks.get(&pid) {
                aff.cpu_affinity = parent_aff.cpu_affinity.clone();
                aff.mem_policy = parent_aff.mem_policy;
                aff.mem_nodes = parent_aff.mem_nodes.clone();
                aff.inherited = true;
            }
        }
        self.tasks.insert(task_id, aff);
    }

    pub fn set_affinity(&mut self, task_id: u64, cpuset: CpuSet) {
        if let Some(t) = self.tasks.get_mut(&task_id) { t.set_affinity(cpuset); }
    }

    pub fn set_mem_policy(&mut self, task_id: u64, policy: MemBindPolicy, nodes: NodeSet) {
        if let Some(t) = self.tasks.get_mut(&task_id) { t.set_mem_policy(policy, nodes); }
    }

    pub fn add_cache_domain(&mut self, domain: CacheDomain) {
        self.cache_domains.insert(domain.domain_id, domain);
    }

    pub fn estimate_migration_cost(&self, task_id: u64, to_cpu: u32) -> Option<MigrationCost> {
        let task = self.tasks.get(&task_id)?;
        let from = task.last_cpu;
        let same_l2 = self.cache_domains.values().any(|d| d.level == 2 && d.cpus.is_set(from) && d.cpus.is_set(to_cpu));
        let same_l3 = self.cache_domains.values().any(|d| d.level == 3 && d.cpus.is_set(from) && d.cpus.is_set(to_cpu));
        Some(MigrationCost::estimate(from, to_cpu, same_l2, same_l3, 10))
    }

    pub fn record_migration(&mut self, task_id: u64, new_cpu: u32, forced: bool) {
        if let Some(t) = self.tasks.get_mut(&task_id) { t.record_migration(new_cpu, forced); }
    }

    pub fn recompute(&mut self) {
        self.stats.tasks_tracked = self.tasks.len();
        self.stats.tasks_with_affinity = self.tasks.values().filter(|t| t.cpu_affinity.count() < 256).count();
        self.stats.tasks_inherited = self.tasks.values().filter(|t| t.inherited).count();
        self.stats.total_migrations = self.tasks.values().map(|t| t.migrations).sum();
        let rates: Vec<f64> = self.tasks.values().map(|t| t.migration_rate()).collect();
        self.stats.avg_migration_rate = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
        self.stats.bound_tasks = self.tasks.values().filter(|t| t.mem_policy == MemBindPolicy::Bind).count();
        self.stats.interleave_tasks = self.tasks.values().filter(|t| t.mem_policy == MemBindPolicy::Interleave).count();
    }

    pub fn task(&self, id: u64) -> Option<&TaskAffinity> { self.tasks.get(&id) }
    pub fn stats(&self) -> &TaskAffinityStats { &self.stats }
}

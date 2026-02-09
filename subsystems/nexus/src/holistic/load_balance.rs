//! # Holistic Load Balancer
//!
//! System-wide load balancing engine:
//! - Multi-dimensional load vector (CPU, memory, IO, net)
//! - Pull/push migration decisions
//! - Load balance domains (core → package → NUMA → system)
//! - Imbalance detection with hysteresis
//! - Migration cost estimation
//! - Balance tick scheduling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Load dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadDimension {
    CpuUtil,
    RunqueueDepth,
    MemoryPressure,
    IoBandwidth,
    CachePressure,
}

/// Balance domain level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BalanceDomainLevel {
    /// SMT siblings
    Smt,
    /// Physical core
    Core,
    /// Package / socket
    Package,
    /// NUMA node
    Numa,
    /// System-wide
    System,
}

/// Multi-dimensional load vector
#[derive(Debug, Clone)]
pub struct LoadVector {
    pub cpu_util: f64,
    pub runqueue_depth: u32,
    pub memory_pressure: f64,
    pub io_bandwidth: f64,
    pub cache_pressure: f64,
}

impl LoadVector {
    #[inline]
    pub fn zero() -> Self {
        Self {
            cpu_util: 0.0,
            runqueue_depth: 0,
            memory_pressure: 0.0,
            io_bandwidth: 0.0,
            cache_pressure: 0.0,
        }
    }

    /// Weighted composite load
    #[inline]
    pub fn composite(&self, weights: &LoadWeights) -> f64 {
        self.cpu_util * weights.cpu
            + self.runqueue_depth as f64 * weights.runqueue * 0.1
            + self.memory_pressure * weights.memory
            + self.io_bandwidth * weights.io
            + self.cache_pressure * weights.cache
    }

    /// L2 distance between load vectors
    #[inline]
    pub fn distance(&self, other: &LoadVector) -> f64 {
        let d_cpu = self.cpu_util - other.cpu_util;
        let d_rq = self.runqueue_depth as f64 - other.runqueue_depth as f64;
        let d_mem = self.memory_pressure - other.memory_pressure;
        let d_io = self.io_bandwidth - other.io_bandwidth;
        let d_cache = self.cache_pressure - other.cache_pressure;
        libm::sqrt(d_cpu * d_cpu + d_rq * d_rq * 0.01 + d_mem * d_mem + d_io * d_io + d_cache * d_cache)
    }
}

/// Weights for composite load
#[derive(Debug, Clone)]
pub struct LoadWeights {
    pub cpu: f64,
    pub runqueue: f64,
    pub memory: f64,
    pub io: f64,
    pub cache: f64,
}

impl LoadWeights {
    #[inline]
    pub fn default_weights() -> Self {
        Self {
            cpu: 0.4,
            runqueue: 0.2,
            memory: 0.2,
            io: 0.1,
            cache: 0.1,
        }
    }
}

/// Per-CPU load state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuLoadState {
    pub cpu_id: u32,
    pub domain_level: BalanceDomainLevel,
    pub group_id: u32,
    pub load: LoadVector,
    pub composite_load: f64,
    pub nr_tasks: u32,
    pub idle: bool,
    pub newly_idle: bool,
}

impl CpuLoadState {
    pub fn new(cpu_id: u32, group_id: u32) -> Self {
        Self {
            cpu_id,
            domain_level: BalanceDomainLevel::System,
            group_id,
            load: LoadVector::zero(),
            composite_load: 0.0,
            nr_tasks: 0,
            idle: true,
            newly_idle: false,
        }
    }
}

/// Balance group (collection of CPUs at a domain level)
#[derive(Debug, Clone)]
pub struct BalanceGroup {
    pub group_id: u32,
    pub level: BalanceDomainLevel,
    pub cpus: Vec<u32>,
    pub avg_load: f64,
    pub total_tasks: u32,
    pub imbalanced: bool,
}

impl BalanceGroup {
    pub fn new(group_id: u32, level: BalanceDomainLevel) -> Self {
        Self {
            group_id,
            level,
            cpus: Vec::new(),
            avg_load: 0.0,
            total_tasks: 0,
            imbalanced: false,
        }
    }
}

/// Migration decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationType {
    /// Pull task from busiest to idle
    Pull,
    /// Push task from busy to less busy
    Push,
    /// No migration needed
    None,
}

/// Migration recommendation
#[derive(Debug, Clone)]
pub struct MigrationRecommendation {
    pub migration_type: MigrationType,
    pub from_cpu: u32,
    pub to_cpu: u32,
    pub domain_level: BalanceDomainLevel,
    pub load_delta: f64,
    pub estimated_cost_ns: u64,
    pub nr_tasks_to_move: u32,
}

/// Load balancer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticLoadBalanceStats {
    pub tracked_cpus: usize,
    pub balance_groups: usize,
    pub max_load: f64,
    pub min_load: f64,
    pub avg_load: f64,
    pub imbalance: f64,
    pub idle_cpus: usize,
    pub pending_migrations: usize,
    pub total_migrations: u64,
}

/// Holistic Load Balancer
pub struct HolisticLoadBalance {
    cpus: BTreeMap<u32, CpuLoadState>,
    groups: BTreeMap<u32, BalanceGroup>,
    weights: LoadWeights,
    recommendations: Vec<MigrationRecommendation>,
    hysteresis: f64,
    migration_cost_ns: u64,
    total_migrations: u64,
    stats: HolisticLoadBalanceStats,
}

impl HolisticLoadBalance {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            groups: BTreeMap::new(),
            weights: LoadWeights::default_weights(),
            recommendations: Vec::new(),
            hysteresis: 0.15,
            migration_cost_ns: 500_000,
            total_migrations: 0,
            stats: HolisticLoadBalanceStats::default(),
        }
    }

    #[inline(always)]
    pub fn set_weights(&mut self, weights: LoadWeights) {
        self.weights = weights;
    }

    #[inline]
    pub fn register_cpu(&mut self, cpu_id: u32, group_id: u32) {
        self.cpus.insert(cpu_id, CpuLoadState::new(cpu_id, group_id));
        let group = self.groups.entry(group_id)
            .or_insert_with(|| BalanceGroup::new(group_id, BalanceDomainLevel::System));
        if !group.cpus.contains(&cpu_id) {
            group.cpus.push(cpu_id);
        }
    }

    #[inline]
    pub fn update_load(&mut self, cpu_id: u32, load: LoadVector, nr_tasks: u32) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.composite_load = load.composite(&self.weights);
            cpu.load = load;
            cpu.nr_tasks = nr_tasks;
            cpu.idle = nr_tasks == 0;
        }
    }

    /// Run balance tick
    pub fn balance_tick(&mut self) {
        self.recommendations.clear();

        // Recompute group stats
        for group in self.groups.values_mut() {
            let mut total_load = 0.0;
            let mut total_tasks = 0u32;
            let count = group.cpus.len();

            for &cpu_id in &group.cpus {
                if let Some(cpu) = self.cpus.get(&cpu_id) {
                    total_load += cpu.composite_load;
                    total_tasks += cpu.nr_tasks;
                }
            }

            group.avg_load = if count > 0 { total_load / count as f64 } else { 0.0 };
            group.total_tasks = total_tasks;
        }

        // Find busiest and idlest CPUs per group
        for group in self.groups.values() {
            if group.cpus.len() < 2 { continue; }

            let mut busiest_id = 0u32;
            let mut busiest_load = 0.0f64;
            let mut idlest_id = 0u32;
            let mut idlest_load = f64::MAX;

            for &cpu_id in &group.cpus {
                if let Some(cpu) = self.cpus.get(&cpu_id) {
                    if cpu.composite_load > busiest_load {
                        busiest_load = cpu.composite_load;
                        busiest_id = cpu_id;
                    }
                    if cpu.composite_load < idlest_load {
                        idlest_load = cpu.composite_load;
                        idlest_id = cpu_id;
                    }
                }
            }

            let delta = busiest_load - idlest_load;
            if delta > self.hysteresis && busiest_load > 0.3 {
                let busiest_tasks = self.cpus.get(&busiest_id).map(|c| c.nr_tasks).unwrap_or(0);
                let tasks_to_move = (busiest_tasks / 2).max(1);

                self.recommendations.push(MigrationRecommendation {
                    migration_type: if idlest_load < 0.01 { MigrationType::Pull } else { MigrationType::Push },
                    from_cpu: busiest_id,
                    to_cpu: idlest_id,
                    domain_level: group.level,
                    load_delta: delta,
                    estimated_cost_ns: self.migration_cost_ns * tasks_to_move as u64,
                    nr_tasks_to_move: tasks_to_move,
                });
            }
        }

        // Update stats
        let loads: Vec<f64> = self.cpus.values().map(|c| c.composite_load).collect();
        let max_l = loads.iter().copied().fold(0.0f64, |a, b| if b > a { b } else { a });
        let min_l = loads.iter().copied().fold(f64::MAX, |a, b| if b < a { b } else { a });
        let avg_l = if loads.is_empty() { 0.0 } else { loads.iter().sum::<f64>() / loads.len() as f64 };

        self.stats = HolisticLoadBalanceStats {
            tracked_cpus: self.cpus.len(),
            balance_groups: self.groups.len(),
            max_load: max_l,
            min_load: if min_l == f64::MAX { 0.0 } else { min_l },
            avg_load: avg_l,
            imbalance: if avg_l > 0.01 { (max_l - min_l) / avg_l } else { 0.0 },
            idle_cpus: self.cpus.values().filter(|c| c.idle).count(),
            pending_migrations: self.recommendations.len(),
            total_migrations: self.total_migrations,
        };
    }

    /// Accept a migration (mark completed)
    #[inline]
    pub fn complete_migration(&mut self, idx: usize) {
        if idx < self.recommendations.len() {
            self.total_migrations += 1;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticLoadBalanceStats {
        &self.stats
    }

    #[inline(always)]
    pub fn recommendations(&self) -> &[MigrationRecommendation] {
        &self.recommendations
    }
}

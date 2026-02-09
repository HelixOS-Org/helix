//! # Apps NUMA Profile
//!
//! Per-process NUMA memory placement profiling:
//! - Page distribution across NUMA nodes
//! - Local vs remote memory access ratio
//! - Migration event tracking
//! - Optimal placement suggestion
//! - Memory bandwidth per-node estimation
//! - Interleave policy detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// NUMA access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaAccessType {
    /// Local node access
    Local,
    /// Remote 1-hop access
    Remote1Hop,
    /// Remote 2+ hop access
    RemoteMultiHop,
    /// Interleaved access
    Interleaved,
}

/// NUMA policy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaPolicyType {
    /// Default placement
    Default,
    /// Bind to specific node
    Bind,
    /// Interleave across nodes
    Interleave,
    /// Preferred node
    Preferred,
    /// Preferred many (Linux 5.15+)
    PreferredMany,
}

/// Per-node memory info
#[derive(Debug, Clone)]
pub struct NodeMemInfo {
    pub node_id: u32,
    pub pages_resident: u64,
    pub pages_migrated_in: u64,
    pub pages_migrated_out: u64,
    /// Access count from this node's CPUs
    pub local_accesses: u64,
    /// Access count from other nodes' CPUs
    pub remote_accesses: u64,
    /// Estimated bandwidth usage (bytes/sec)
    pub bandwidth_usage: f64,
    /// Last access timestamp
    pub last_access_ns: u64,
}

impl NodeMemInfo {
    pub fn new(node_id: u32) -> Self {
        Self {
            node_id,
            pages_resident: 0,
            pages_migrated_in: 0,
            pages_migrated_out: 0,
            local_accesses: 0,
            remote_accesses: 0,
            bandwidth_usage: 0.0,
            last_access_ns: 0,
        }
    }

    /// Local access ratio
    #[inline]
    pub fn locality_ratio(&self) -> f64 {
        let total = self.local_accesses + self.remote_accesses;
        if total == 0 {
            return 1.0;
        }
        self.local_accesses as f64 / total as f64
    }

    /// Net migration (positive = gaining pages)
    #[inline(always)]
    pub fn net_migration(&self) -> i64 {
        self.pages_migrated_in as i64 - self.pages_migrated_out as i64
    }
}

/// NUMA migration event
#[derive(Debug, Clone)]
pub struct NumaMigrationEvent {
    pub from_node: u32,
    pub to_node: u32,
    pub page_count: u64,
    pub reason: NumaMigrationReason,
    pub timestamp_ns: u64,
    pub latency_ns: u64,
}

/// Reason for NUMA migration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaMigrationReason {
    /// Automatic balancing
    AutoBalance,
    /// CPU affinity changed
    AffinityChange,
    /// Memory pressure on source node
    MemoryPressure,
    /// Explicit policy
    PolicyDirected,
    /// Optimization hint
    PerfOptimization,
}

/// Per-process NUMA profile
#[derive(Debug)]
pub struct ProcessNumaProfile {
    pub pid: u64,
    pub policy: NumaPolicyType,
    /// Preferred node (if policy is Preferred)
    pub preferred_node: Option<u32>,
    /// Per-node info
    nodes: BTreeMap<u32, NodeMemInfo>,
    /// Migration history (ring buffer of last 64)
    migrations: Vec<NumaMigrationEvent>,
    migration_head: usize,
    pub total_pages: u64,
    pub total_migrations: u64,
}

impl ProcessNumaProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            policy: NumaPolicyType::Default,
            preferred_node: None,
            nodes: BTreeMap::new(),
            migrations: Vec::new(),
            migration_head: 0,
            total_pages: 0,
            total_migrations: 0,
        }
    }

    /// Record page placement on a node
    #[inline]
    pub fn record_placement(&mut self, node_id: u32, pages: u64) {
        let info = self.nodes.entry(node_id)
            .or_insert_with(|| NodeMemInfo::new(node_id));
        info.pages_resident += pages;
        self.total_pages += pages;
    }

    /// Record an access
    #[inline]
    pub fn record_access(&mut self, cpu_node: u32, mem_node: u32, timestamp_ns: u64) {
        let info = self.nodes.entry(mem_node)
            .or_insert_with(|| NodeMemInfo::new(mem_node));
        if cpu_node == mem_node {
            info.local_accesses += 1;
        } else {
            info.remote_accesses += 1;
        }
        info.last_access_ns = timestamp_ns;
    }

    /// Record a migration event
    pub fn record_migration(&mut self, event: NumaMigrationEvent) {
        if let Some(from) = self.nodes.get_mut(&event.from_node) {
            from.pages_migrated_out += event.page_count;
            if from.pages_resident >= event.page_count {
                from.pages_resident -= event.page_count;
            }
        }
        let to = self.nodes.entry(event.to_node)
            .or_insert_with(|| NodeMemInfo::new(event.to_node));
        to.pages_migrated_in += event.page_count;
        to.pages_resident += event.page_count;
        self.total_migrations += 1;

        // Ring buffer
        if self.migrations.len() < 64 {
            self.migrations.push(event);
        } else {
            self.migrations[self.migration_head] = event;
            self.migration_head = (self.migration_head + 1) % 64;
        }
    }

    /// Overall locality ratio
    #[inline]
    pub fn overall_locality(&self) -> f64 {
        let mut local = 0u64;
        let mut total = 0u64;
        for info in self.nodes.values() {
            local += info.local_accesses;
            total += info.local_accesses + info.remote_accesses;
        }
        if total == 0 { 1.0 } else { local as f64 / total as f64 }
    }

    /// Page distribution across nodes (node_id, fraction)
    #[inline]
    pub fn page_distribution(&self) -> Vec<(u32, f64)> {
        if self.total_pages == 0 {
            return Vec::new();
        }
        self.nodes.iter()
            .map(|(&id, info)| (id, info.pages_resident as f64 / self.total_pages as f64))
            .collect()
    }

    /// Is the workload interleaved?
    pub fn is_interleaved(&self) -> bool {
        if self.nodes.len() < 2 || self.total_pages == 0 {
            return false;
        }
        let avg = self.total_pages as f64 / self.nodes.len() as f64;
        let variance: f64 = self.nodes.values()
            .map(|info| {
                let diff = info.pages_resident as f64 - avg;
                diff * diff
            })
            .sum::<f64>() / self.nodes.len() as f64;
        let cv = libm::sqrt(variance) / avg;
        cv < 0.15 // Low coefficient of variation → interleaved
    }

    /// Suggest optimal node
    #[inline]
    pub fn suggest_optimal_node(&self) -> Option<u32> {
        self.nodes.iter()
            .max_by(|a, b| {
                let score_a = a.1.locality_ratio() * a.1.pages_resident as f64;
                let score_b = b.1.locality_ratio() * b.1.pages_resident as f64;
                score_a.partial_cmp(&score_b).unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(&id, _)| id)
    }

    /// Migration rate (migrations per 1000 accesses)
    #[inline]
    pub fn migration_rate(&self) -> f64 {
        let total_accesses: u64 = self.nodes.values()
            .map(|info| info.local_accesses + info.remote_accesses)
            .sum();
        if total_accesses == 0 {
            return 0.0;
        }
        self.total_migrations as f64 * 1000.0 / total_accesses as f64
    }
}

/// NUMA profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppNumaProfileStats {
    pub tracked_processes: usize,
    pub avg_locality: f64,
    pub interleaved_count: usize,
    pub total_migrations: u64,
    pub poor_locality_count: usize,
}

/// App NUMA profiler
pub struct AppNumaProfiler {
    processes: BTreeMap<u64, ProcessNumaProfile>,
    stats: AppNumaProfileStats,
}

impl AppNumaProfiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppNumaProfileStats::default(),
        }
    }

    #[inline(always)]
    pub fn get_or_create(&mut self, pid: u64) -> &mut ProcessNumaProfile {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessNumaProfile::new(pid))
    }

    #[inline(always)]
    pub fn record_access(&mut self, pid: u64, cpu_node: u32, mem_node: u32, ts: u64) {
        self.get_or_create(pid).record_access(cpu_node, mem_node, ts);
        self.update_stats();
    }

    #[inline(always)]
    pub fn record_migration(&mut self, pid: u64, event: NumaMigrationEvent) {
        self.get_or_create(pid).record_migration(event);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        if !self.processes.is_empty() {
            self.stats.avg_locality = self.processes.values()
                .map(|p| p.overall_locality())
                .sum::<f64>() / self.processes.len() as f64;
        }
        self.stats.interleaved_count = self.processes.values()
            .filter(|p| p.is_interleaved())
            .count();
        self.stats.total_migrations = self.processes.values()
            .map(|p| p.total_migrations)
            .sum();
        self.stats.poor_locality_count = self.processes.values()
            .filter(|p| p.overall_locality() < 0.5)
            .count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppNumaProfileStats {
        &self.stats
    }

    /// Processes needing NUMA rebalancing
    #[inline]
    pub fn rebalance_candidates(&self) -> Vec<u64> {
        self.processes.iter()
            .filter(|(_, p)| p.overall_locality() < 0.5 && p.total_pages > 256)
            .map(|(&pid, _)| pid)
            .collect()
    }
}

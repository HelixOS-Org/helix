//! # Holistic NUMA Placement Engine
//!
//! Advanced NUMA-aware memory and task placement:
//! - Multi-socket memory distribution optimization
//! - Remote memory access tracking and migration
//! - Automatic page migration between NUMA nodes
//! - Process-to-node affinity scoring
//! - Memory bandwidth contention detection
//! - Interconnect topology-aware decisions

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// NUMA migration direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaMigrationDir {
    Local,      // Already optimal
    NearRemote, // One hop
    FarRemote,  // Multiple hops
}

/// Memory access locality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLocality {
    NodeLocal,
    CrossSocket,
    CrossDie,
    CrossBoard,
}

/// Per-node memory state
#[derive(Debug, Clone)]
pub struct NumaNodeMemState {
    pub node_id: u32,
    pub total_pages: u64,
    pub free_pages: u64,
    pub file_pages: u64,
    pub anon_pages: u64,
    pub slab_pages: u64,
    pub remote_accesses: u64,
    pub local_accesses: u64,
    pub migration_in: u64,
    pub migration_out: u64,
    pub bandwidth_used_mbps: u64,
    pub bandwidth_capacity_mbps: u64,
}

impl NumaNodeMemState {
    pub fn new(node_id: u32, total_pages: u64) -> Self {
        Self {
            node_id,
            total_pages,
            free_pages: total_pages,
            file_pages: 0,
            anon_pages: 0,
            slab_pages: 0,
            remote_accesses: 0,
            local_accesses: 0,
            migration_in: 0,
            migration_out: 0,
            bandwidth_used_mbps: 0,
            bandwidth_capacity_mbps: 0,
        }
    }

    pub fn locality_ratio(&self) -> f64 {
        let total = self.local_accesses + self.remote_accesses;
        if total == 0 { return 1.0; }
        self.local_accesses as f64 / total as f64
    }

    pub fn free_ratio(&self) -> f64 {
        if self.total_pages == 0 { return 0.0; }
        self.free_pages as f64 / self.total_pages as f64
    }

    pub fn bandwidth_pressure(&self) -> f64 {
        if self.bandwidth_capacity_mbps == 0 { return 0.0; }
        self.bandwidth_used_mbps as f64 / self.bandwidth_capacity_mbps as f64
    }

    pub fn is_bandwidth_constrained(&self) -> bool {
        self.bandwidth_pressure() > 0.85
    }
}

/// Process NUMA affinity profile
#[derive(Debug, Clone)]
pub struct ProcessNumaProfile {
    pub process_id: u64,
    pub preferred_node: u32,
    pub running_node: u32,
    pub pages_per_node: BTreeMap<u32, u64>,
    pub accesses_per_node: BTreeMap<u32, u64>,
    pub total_remote_accesses: u64,
    pub migration_count: u32,
    pub affinity_score: f64, // 0.0 = all remote, 1.0 = all local
}

impl ProcessNumaProfile {
    pub fn new(process_id: u64, preferred: u32) -> Self {
        Self {
            process_id,
            preferred_node: preferred,
            running_node: preferred,
            pages_per_node: BTreeMap::new(),
            accesses_per_node: BTreeMap::new(),
            total_remote_accesses: 0,
            migration_count: 0,
            affinity_score: 1.0,
        }
    }

    pub fn dominant_node(&self) -> u32 {
        self.pages_per_node.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&node, _)| node)
            .unwrap_or(self.preferred_node)
    }

    pub fn is_misplaced(&self) -> bool {
        self.dominant_node() != self.running_node
    }

    pub fn compute_affinity(&mut self) {
        let total: u64 = self.accesses_per_node.values().sum();
        if total == 0 {
            self.affinity_score = 1.0;
            return;
        }
        let local = self.accesses_per_node.get(&self.running_node).copied().unwrap_or(0);
        self.affinity_score = local as f64 / total as f64;
    }

    pub fn pages_on_node(&self, node: u32) -> u64 {
        self.pages_per_node.get(&node).copied().unwrap_or(0)
    }
}

/// NUMA distance entry
#[derive(Debug, Clone)]
pub struct NumaDistanceEntry {
    pub from_node: u32,
    pub to_node: u32,
    pub distance: u32, // lower = closer, 10 = local
}

/// Page migration candidate
#[derive(Debug, Clone)]
pub struct MigrationCandidate {
    pub page_frame: u64,
    pub current_node: u32,
    pub target_node: u32,
    pub access_count: u64,
    pub benefit_score: f64,
}

/// Holistic NUMA Placement stats
#[derive(Debug, Clone, Default)]
pub struct HolisticNumaPlaceStats {
    pub total_nodes: usize,
    pub total_processes: usize,
    pub avg_locality: f64,
    pub misplaced_processes: usize,
    pub total_migrations: u64,
    pub bandwidth_constrained_nodes: usize,
}

/// Holistic NUMA Placement Engine
pub struct HolisticNumaPlace {
    nodes: BTreeMap<u32, NumaNodeMemState>,
    processes: BTreeMap<u64, ProcessNumaProfile>,
    distances: Vec<NumaDistanceEntry>,
    stats: HolisticNumaPlaceStats,
}

impl HolisticNumaPlace {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            processes: BTreeMap::new(),
            distances: Vec::new(),
            stats: HolisticNumaPlaceStats::default(),
        }
    }

    pub fn add_node(&mut self, state: NumaNodeMemState) {
        self.nodes.insert(state.node_id, state);
    }

    pub fn add_distance(&mut self, from: u32, to: u32, distance: u32) {
        self.distances.push(NumaDistanceEntry { from_node: from, to_node: to, distance });
    }

    pub fn register_process(&mut self, profile: ProcessNumaProfile) {
        self.processes.insert(profile.process_id, profile);
    }

    pub fn distance(&self, from: u32, to: u32) -> u32 {
        if from == to { return 10; }
        self.distances.iter()
            .find(|d| d.from_node == from && d.to_node == to)
            .map(|d| d.distance)
            .unwrap_or(255)
    }

    /// Score a process-to-node assignment
    pub fn placement_score(&self, pid: u64, node: u32) -> f64 {
        let proc = match self.processes.get(&pid) {
            Some(p) => p,
            None => return 0.0,
        };

        // Memory locality component
        let pages_here = proc.pages_on_node(node);
        let total_pages: u64 = proc.pages_per_node.values().sum();
        let mem_score = if total_pages > 0 { pages_here as f64 / total_pages as f64 } else { 0.0 };

        // Node capacity component
        let cap_score = self.nodes.get(&node)
            .map(|n| n.free_ratio())
            .unwrap_or(0.0);

        // Bandwidth component
        let bw_score = self.nodes.get(&node)
            .map(|n| 1.0 - n.bandwidth_pressure())
            .unwrap_or(0.0);

        mem_score * 0.6 + cap_score * 0.2 + bw_score * 0.2
    }

    /// Find best node for a process
    pub fn best_node(&self, pid: u64) -> Option<u32> {
        let node_ids: Vec<u32> = self.nodes.keys().copied().collect();
        node_ids.into_iter()
            .max_by(|&a, &b| {
                let sa = self.placement_score(pid, a);
                let sb = self.placement_score(pid, b);
                sa.partial_cmp(&sb).unwrap_or(core::cmp::Ordering::Equal)
            })
    }

    /// Find page migration candidates for a process
    pub fn migration_candidates(&self, pid: u64, max: usize) -> Vec<MigrationCandidate> {
        let proc = match self.processes.get(&pid) {
            Some(p) => p,
            None => return Vec::new(),
        };

        let target = proc.running_node;
        let mut candidates = Vec::new();

        for (&node, &pages) in &proc.pages_per_node {
            if node == target { continue; }
            let dist = self.distance(node, target);
            let benefit = pages as f64 * dist as f64 / 10.0;
            if benefit > 1.0 {
                candidates.push(MigrationCandidate {
                    page_frame: 0, // placeholder — real impl would enumerate pages
                    current_node: node,
                    target_node: target,
                    access_count: pages,
                    benefit_score: benefit,
                });
            }
        }

        candidates.sort_by(|a, b| b.benefit_score.partial_cmp(&a.benefit_score)
            .unwrap_or(core::cmp::Ordering::Equal));
        candidates.truncate(max);
        candidates
    }

    pub fn recompute(&mut self) {
        for proc in self.processes.values_mut() {
            proc.compute_affinity();
        }
        self.stats.total_nodes = self.nodes.len();
        self.stats.total_processes = self.processes.len();
        let sum_locality: f64 = self.processes.values().map(|p| p.affinity_score).sum();
        self.stats.avg_locality = if self.processes.is_empty() { 0.0 }
        else { sum_locality / self.processes.len() as f64 };
        self.stats.misplaced_processes = self.processes.values().filter(|p| p.is_misplaced()).count();
        self.stats.total_migrations = self.nodes.values()
            .map(|n| n.migration_in + n.migration_out).sum::<u64>() / 2;
        self.stats.bandwidth_constrained_nodes = self.nodes.values()
            .filter(|n| n.is_bandwidth_constrained()).count();
    }

    pub fn node(&self, id: u32) -> Option<&NumaNodeMemState> { self.nodes.get(&id) }
    pub fn process(&self, pid: u64) -> Option<&ProcessNumaProfile> { self.processes.get(&pid) }
    pub fn stats(&self) -> &HolisticNumaPlaceStats { &self.stats }
}

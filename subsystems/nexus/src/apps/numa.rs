//! # Application NUMA Profiling
//!
//! NUMA-aware application resource analysis:
//! - NUMA node affinity tracking
//! - Remote memory access detection
//! - Cross-node traffic analysis
//! - NUMA-aware placement recommendations
//! - Memory migration cost analysis
//! - Locality scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// NUMA TOPOLOGY
// ============================================================================

/// NUMA node descriptor
#[derive(Debug, Clone)]
pub struct NumaNode {
    /// Node ID
    pub id: u32,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Free memory (bytes)
    pub free_memory: u64,
    /// CPU cores on this node
    pub cores: Vec<u32>,
    /// Distance to other nodes (node_id -> distance)
    pub distances: BTreeMap<u32, u32>,
}

impl NumaNode {
    pub fn new(id: u32, total_memory: u64) -> Self {
        Self {
            id,
            total_memory,
            free_memory: total_memory,
            cores: Vec::new(),
            distances: BTreeMap::new(),
        }
    }

    /// Memory utilization
    pub fn memory_utilization(&self) -> f64 {
        if self.total_memory == 0 {
            return 0.0;
        }
        1.0 - (self.free_memory as f64 / self.total_memory as f64)
    }

    /// Distance to another node
    pub fn distance_to(&self, other: u32) -> u32 {
        if other == self.id {
            return 10; // local
        }
        *self.distances.get(&other).unwrap_or(&20)
    }

    /// Number of cores
    pub fn core_count(&self) -> usize {
        self.cores.len()
    }
}

/// NUMA topology
#[derive(Debug, Clone)]
pub struct NumaTopology {
    /// Nodes
    pub nodes: BTreeMap<u32, NumaNode>,
    /// Core to node mapping
    core_to_node: BTreeMap<u32, u32>,
}

impl NumaTopology {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            core_to_node: BTreeMap::new(),
        }
    }

    /// Add a node
    pub fn add_node(&mut self, node: NumaNode) {
        for &core in &node.cores {
            self.core_to_node.insert(core, node.id);
        }
        self.nodes.insert(node.id, node);
    }

    /// Get node for a core
    pub fn node_for_core(&self, core: u32) -> Option<u32> {
        self.core_to_node.get(&core).copied()
    }

    /// Total nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Distance between two nodes
    pub fn distance(&self, from: u32, to: u32) -> u32 {
        if from == to {
            return 10;
        }
        self.nodes
            .get(&from)
            .map(|n| n.distance_to(to))
            .unwrap_or(20)
    }
}

// ============================================================================
// NUMA ACCESS PATTERN
// ============================================================================

/// NUMA access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumaAccessType {
    /// Local node access
    Local,
    /// Remote 1-hop access
    Remote1Hop,
    /// Remote 2+ hop access
    RemoteMultiHop,
}

/// NUMA access counters per process
#[derive(Debug, Clone, Default)]
pub struct NumaAccessCounters {
    /// Local accesses
    pub local_accesses: u64,
    /// Remote accesses
    pub remote_accesses: u64,
    /// Remote access breakdown by node
    pub remote_by_node: BTreeMap<u32, u64>,
    /// Page migrations
    pub page_migrations: u64,
    /// Migration cost (ns)
    pub migration_cost_ns: u64,
}

impl NumaAccessCounters {
    /// Locality ratio (local / total)
    pub fn locality_ratio(&self) -> f64 {
        let total = self.local_accesses + self.remote_accesses;
        if total == 0 {
            return 1.0;
        }
        self.local_accesses as f64 / total as f64
    }

    /// Remote ratio
    pub fn remote_ratio(&self) -> f64 {
        1.0 - self.locality_ratio()
    }

    /// Record access
    pub fn record_access(&mut self, access_type: NumaAccessType, remote_node: Option<u32>) {
        match access_type {
            NumaAccessType::Local => {
                self.local_accesses += 1;
            }
            NumaAccessType::Remote1Hop | NumaAccessType::RemoteMultiHop => {
                self.remote_accesses += 1;
                if let Some(node) = remote_node {
                    *self.remote_by_node.entry(node).or_insert(0) += 1;
                }
            }
        }
    }

    /// Most accessed remote node
    pub fn most_accessed_remote_node(&self) -> Option<u32> {
        self.remote_by_node
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&node, _)| node)
    }
}

// ============================================================================
// PROCESS NUMA PROFILE
// ============================================================================

/// Per-process NUMA profile
#[derive(Debug, Clone)]
pub struct ProcessNumaProfile {
    /// Process ID
    pub pid: u64,
    /// Home node
    pub home_node: u32,
    /// Access counters per node
    pub access_counters: BTreeMap<u32, NumaAccessCounters>,
    /// Memory pages per node
    pub memory_per_node: BTreeMap<u32, u64>,
    /// Thread placement (thread_id -> node)
    pub thread_nodes: BTreeMap<u64, u32>,
    /// Locality score (0.0-1.0)
    pub locality_score: f64,
}

impl ProcessNumaProfile {
    pub fn new(pid: u64, home_node: u32) -> Self {
        Self {
            pid,
            home_node,
            access_counters: BTreeMap::new(),
            memory_per_node: BTreeMap::new(),
            thread_nodes: BTreeMap::new(),
            locality_score: 1.0,
        }
    }

    /// Record access from a node
    pub fn record_access(
        &mut self,
        from_node: u32,
        access_type: NumaAccessType,
        remote_node: Option<u32>,
    ) {
        let counters = self
            .access_counters
            .entry(from_node)
            .or_insert_with(NumaAccessCounters::default);
        counters.record_access(access_type, remote_node);
    }

    /// Allocate memory on node
    pub fn allocate_on_node(&mut self, node: u32, pages: u64) {
        *self.memory_per_node.entry(node).or_insert(0) += pages;
    }

    /// Total memory (pages)
    pub fn total_pages(&self) -> u64 {
        self.memory_per_node.values().sum()
    }

    /// Memory fraction on home node
    pub fn home_node_fraction(&self) -> f64 {
        let total = self.total_pages();
        if total == 0 {
            return 1.0;
        }
        let home = self.memory_per_node.get(&self.home_node).copied().unwrap_or(0);
        home as f64 / total as f64
    }

    /// Recalculate locality score
    pub fn recalculate_locality(&mut self) {
        let mut total_local = 0u64;
        let mut total_all = 0u64;

        for counters in self.access_counters.values() {
            total_local += counters.local_accesses;
            total_all += counters.local_accesses + counters.remote_accesses;
        }

        self.locality_score = if total_all == 0 {
            1.0
        } else {
            total_local as f64 / total_all as f64
        };
    }

    /// Place thread on node
    pub fn place_thread(&mut self, thread_id: u64, node: u32) {
        self.thread_nodes.insert(thread_id, node);
    }
}

// ============================================================================
// PLACEMENT RECOMMENDATION
// ============================================================================

/// Placement recommendation
#[derive(Debug, Clone)]
pub struct PlacementRecommendation {
    /// Process ID
    pub pid: u64,
    /// Recommended node
    pub recommended_node: u32,
    /// Expected locality improvement
    pub expected_improvement: f64,
    /// Migration cost (pages)
    pub migration_pages: u64,
    /// Reason
    pub reason: PlacementReason,
}

/// Reason for placement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementReason {
    /// Better locality
    BetterLocality,
    /// Load balancing
    LoadBalance,
    /// Memory pressure relief
    MemoryPressure,
    /// Thread co-location
    ThreadCoLocation,
}

// ============================================================================
// NUMA ANALYZER
// ============================================================================

/// NUMA analyzer stats
#[derive(Debug, Clone, Default)]
pub struct AppNumaStats {
    /// Tracked processes
    pub process_count: usize,
    /// Average locality score
    pub avg_locality: f64,
    /// Total remote accesses
    pub total_remote_accesses: u64,
    /// Pending migrations
    pub pending_migrations: u32,
    /// Recommendations generated
    pub recommendations_generated: u64,
}

/// Application NUMA analyzer
pub struct AppNumaAnalyzer {
    /// NUMA topology
    topology: NumaTopology,
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessNumaProfile>,
    /// Placement recommendations
    recommendations: Vec<PlacementRecommendation>,
    /// Stats
    stats: AppNumaStats,
}

impl AppNumaAnalyzer {
    pub fn new(topology: NumaTopology) -> Self {
        Self {
            topology,
            profiles: BTreeMap::new(),
            recommendations: Vec::new(),
            stats: AppNumaStats::default(),
        }
    }

    /// Register process
    pub fn register_process(&mut self, pid: u64, home_node: u32) {
        self.profiles
            .insert(pid, ProcessNumaProfile::new(pid, home_node));
        self.stats.process_count = self.profiles.len();
    }

    /// Record access
    pub fn record_access(
        &mut self,
        pid: u64,
        from_node: u32,
        target_node: u32,
    ) {
        let access_type = if from_node == target_node {
            NumaAccessType::Local
        } else {
            let dist = self.topology.distance(from_node, target_node);
            if dist <= 20 {
                NumaAccessType::Remote1Hop
            } else {
                NumaAccessType::RemoteMultiHop
            }
        };

        let remote_node = if access_type != NumaAccessType::Local {
            self.stats.total_remote_accesses += 1;
            Some(target_node)
        } else {
            None
        };

        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.record_access(from_node, access_type, remote_node);
        }
    }

    /// Generate placement recommendations
    pub fn generate_recommendations(&mut self) {
        self.recommendations.clear();

        for profile in self.profiles.values() {
            if profile.locality_score > 0.8 {
                continue;
            }

            // Find best node based on access patterns
            let mut best_node = profile.home_node;
            let mut best_local = 0u64;

            for (&node, counters) in &profile.access_counters {
                if counters.local_accesses > best_local {
                    best_local = counters.local_accesses;
                    best_node = node;
                }
            }

            if best_node != profile.home_node {
                let migration_pages = profile
                    .memory_per_node
                    .iter()
                    .filter(|(&n, _)| n != best_node)
                    .map(|(_, &p)| p)
                    .sum();

                self.recommendations.push(PlacementRecommendation {
                    pid: profile.pid,
                    recommended_node: best_node,
                    expected_improvement: 0.8 - profile.locality_score,
                    migration_pages,
                    reason: PlacementReason::BetterLocality,
                });
            }
        }

        self.stats.recommendations_generated += self.recommendations.len() as u64;
    }

    /// Update locality scores
    pub fn update_scores(&mut self) {
        let mut total_locality = 0.0;
        let count = self.profiles.len();
        for profile in self.profiles.values_mut() {
            profile.recalculate_locality();
            total_locality += profile.locality_score;
        }
        self.stats.avg_locality = if count > 0 {
            total_locality / count as f64
        } else {
            0.0
        };
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessNumaProfile> {
        self.profiles.get(&pid)
    }

    /// Get recommendations
    pub fn recommendations(&self) -> &[PlacementRecommendation] {
        &self.recommendations
    }

    /// Topology
    pub fn topology(&self) -> &NumaTopology {
        &self.topology
    }

    /// Stats
    pub fn stats(&self) -> &AppNumaStats {
        &self.stats
    }
}

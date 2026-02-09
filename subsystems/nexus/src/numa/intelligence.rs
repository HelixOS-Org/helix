//! Central NUMA intelligence coordinator.

extern crate alloc;

use alloc::collections::BTreeMap;

use super::affinity::AffinityManager;
use super::bandwidth::BandwidthMonitor;
use super::latency::LatencyPredictor;
use super::migration::MigrationCostAnalyzer;
use super::placement::PlacementOptimizer;
use super::stats::{NodeStats, NumaStats};
use super::topology::NumaTopology;
use super::types::NodeId;

// ============================================================================
// NUMA INTELLIGENCE
// ============================================================================

/// Central NUMA intelligence coordinator
pub struct NumaIntelligence {
    /// Topology
    topology: NumaTopology,
    /// Global stats
    stats: NumaStats,
    /// Per-node stats
    node_stats: BTreeMap<NodeId, NodeStats>,
    /// Placement optimizer
    placement: PlacementOptimizer,
    /// Migration analyzer
    migration: MigrationCostAnalyzer,
    /// Bandwidth monitor
    bandwidth: BandwidthMonitor,
    /// Latency predictor
    latency: LatencyPredictor,
    /// Affinity manager
    affinity: AffinityManager,
}

impl NumaIntelligence {
    /// Create new NUMA intelligence
    pub fn new(node_count: u32) -> Self {
        Self {
            topology: NumaTopology::new(node_count),
            stats: NumaStats::default(),
            node_stats: BTreeMap::new(),
            placement: PlacementOptimizer::default(),
            migration: MigrationCostAnalyzer::default(),
            bandwidth: BandwidthMonitor::default(),
            latency: LatencyPredictor::default(),
            affinity: AffinityManager::default(),
        }
    }

    /// Get topology
    #[inline(always)]
    pub fn topology(&self) -> &NumaTopology {
        &self.topology
    }

    /// Get mutable topology
    #[inline(always)]
    pub fn topology_mut(&mut self) -> &mut NumaTopology {
        &mut self.topology
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &NumaStats {
        &self.stats
    }

    /// Get mutable stats
    #[inline(always)]
    pub fn stats_mut(&mut self) -> &mut NumaStats {
        &mut self.stats
    }

    /// Get node stats
    #[inline(always)]
    pub fn node_stats(&self, node: NodeId) -> Option<&NodeStats> {
        self.node_stats.get(&node)
    }

    /// Get mutable node stats
    #[inline(always)]
    pub fn node_stats_mut(&mut self, node: NodeId) -> &mut NodeStats {
        self.node_stats.entry(node).or_default()
    }

    /// Get placement optimizer
    #[inline(always)]
    pub fn placement(&self) -> &PlacementOptimizer {
        &self.placement
    }

    /// Get mutable placement optimizer
    #[inline(always)]
    pub fn placement_mut(&mut self) -> &mut PlacementOptimizer {
        &mut self.placement
    }

    /// Get migration analyzer
    #[inline(always)]
    pub fn migration(&self) -> &MigrationCostAnalyzer {
        &self.migration
    }

    /// Get mutable migration analyzer
    #[inline(always)]
    pub fn migration_mut(&mut self) -> &mut MigrationCostAnalyzer {
        &mut self.migration
    }

    /// Get bandwidth monitor
    #[inline(always)]
    pub fn bandwidth(&self) -> &BandwidthMonitor {
        &self.bandwidth
    }

    /// Get mutable bandwidth monitor
    #[inline(always)]
    pub fn bandwidth_mut(&mut self) -> &mut BandwidthMonitor {
        &mut self.bandwidth
    }

    /// Get latency predictor
    #[inline(always)]
    pub fn latency(&self) -> &LatencyPredictor {
        &self.latency
    }

    /// Get mutable latency predictor
    #[inline(always)]
    pub fn latency_mut(&mut self) -> &mut LatencyPredictor {
        &mut self.latency
    }

    /// Get affinity manager
    #[inline(always)]
    pub fn affinity(&self) -> &AffinityManager {
        &self.affinity
    }

    /// Get mutable affinity manager
    #[inline(always)]
    pub fn affinity_mut(&mut self) -> &mut AffinityManager {
        &mut self.affinity
    }

    /// Record memory access
    pub fn record_access(&mut self, node: NodeId, is_local: bool, hit: bool, latency_ns: u64) {
        if is_local {
            self.stats.record_local(hit);
        } else {
            self.stats.record_remote(hit);
        }

        if let Some(ns) = self.node_stats.get_mut(&node) {
            ns.record_latency(latency_ns);
        }

        self.latency.record(
            node,
            latency_ns,
            self.topology.nodes[node as usize].memory_usage(),
        );
    }

    /// Record migration
    #[inline]
    pub fn record_migration(&mut self, source: NodeId, dest: NodeId, bytes: u64, duration_ns: u64) {
        self.stats.record_migration(bytes);
        self.migration.record(source, dest, bytes, duration_ns);
        self.bandwidth.record(source, dest, bytes, duration_ns);
    }

    /// Get NUMA efficiency score
    #[inline(always)]
    pub fn efficiency_score(&self) -> f64 {
        self.stats.local_ratio()
    }
}

impl Default for NumaIntelligence {
    fn default() -> Self {
        Self::new(4) // Default 4 nodes
    }
}

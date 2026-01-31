//! NUMA Intelligence Module
//!
//! This module provides AI-driven NUMA topology awareness and memory placement
//! optimization for the NEXUS subsystem.
//!
//! # Submodules
//!
//! - `types` - Core type definitions (NodeId, CpuId, Distance, MemoryBinding)
//! - `topology` - NUMA topology representation and discovery
//! - `stats` - NUMA access statistics tracking
//! - `placement` - Memory placement optimization
//! - `migration` - Page migration cost analysis
//! - `bandwidth` - Inter-node bandwidth monitoring
//! - `latency` - Memory access latency prediction
//! - `affinity` - CPU and memory affinity management
//! - `intelligence` - Central NUMA intelligence coordinator

#![allow(dead_code)]

extern crate alloc;

// ============================================================================
// SUBMODULES
// ============================================================================

mod affinity;
mod bandwidth;
mod intelligence;
mod latency;
mod migration;
mod placement;
mod stats;
mod topology;
mod types;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use affinity::{AffinityInfo, AffinityManager, AffinityViolation, AffinityViolationType};
pub use bandwidth::BandwidthMonitor;
pub use intelligence::NumaIntelligence;
pub use latency::{LatencyModel, LatencyPredictor};
pub use migration::{MigrationCostAnalyzer, MigrationCostModel};
pub use placement::{
    Placement, PlacementEvent, PlacementEventType, PlacementOptimizer, PlacementRecommendation,
};
pub use stats::{NodeStats, NumaStats};
pub use topology::{NumaNode, NumaTopology};
pub use types::{CpuId, Distance, MemoryBinding, NodeId};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numa_topology() {
        let mut topology = NumaTopology::new(4);
        topology.set_distance(0, 1, 20);
        topology.set_distance(0, 2, 30);
        topology.set_distance(0, 3, 30);

        assert_eq!(topology.get_distance(0, 1), 20);
        assert_eq!(topology.get_distance(0, 0), 10);
    }

    #[test]
    fn test_numa_node() {
        let node = NumaNode::new(0)
            .with_cpus(&[0, 1, 2, 3])
            .with_memory(1024 * 1024 * 1024, 512 * 1024 * 1024);

        assert_eq!(node.cpu_count(), 4);
        assert!(node.has_cpu(2));
        assert!(!node.has_cpu(10));
    }

    #[test]
    fn test_numa_stats() {
        let mut stats = NumaStats::default();
        stats.record_local(true);
        stats.record_local(true);
        stats.record_remote(false);

        assert_eq!(stats.local_accesses, 2);
        assert_eq!(stats.remote_accesses, 1);
        assert!(stats.local_ratio() > 0.6);
    }

    #[test]
    fn test_latency_predictor() {
        let mut predictor = LatencyPredictor::default();

        for i in 0..20 {
            predictor.record(0, 100 + i, i as f64 / 20.0);
        }

        let predicted = predictor.predict(0, 0.5);
        assert!(predicted > 0.0);
    }
}

//! Memory and thread placement optimization.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::stats::NumaStats;
use super::topology::NumaTopology;
use super::types::{MemoryBinding, NodeId};
use crate::core::NexusTimestamp;

// ============================================================================
// PLACEMENT
// ============================================================================

/// Placement information
#[derive(Debug, Clone)]
pub struct Placement {
    /// Task ID
    pub task_id: u64,
    /// CPU affinity (nodes)
    pub cpu_nodes: Vec<NodeId>,
    /// Memory binding
    pub memory_binding: MemoryBinding,
    /// Strict binding?
    pub strict: bool,
    /// Weight per node
    pub weights: Vec<(NodeId, u32)>,
}

/// Placement event
#[derive(Debug, Clone)]
pub struct PlacementEvent {
    /// Task ID
    pub task_id: u64,
    /// Event type
    pub event_type: PlacementEventType,
    /// Old placement
    pub old_node: Option<NodeId>,
    /// New placement
    pub new_node: Option<NodeId>,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Placement event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementEventType {
    /// Initial placement
    Initial,
    /// Migration
    Migrated,
    /// Binding changed
    BindingChanged,
    /// Affinity changed
    AffinityChanged,
}

/// Placement recommendation
#[derive(Debug, Clone)]
pub struct PlacementRecommendation {
    /// Task ID
    pub task_id: u64,
    /// Recommended node
    pub recommended_node: NodeId,
    /// Recommended binding
    pub binding: MemoryBinding,
    /// Expected improvement
    pub expected_improvement: f64,
    /// Confidence
    pub confidence: f64,
}

// ============================================================================
// PLACEMENT OPTIMIZER
// ============================================================================

/// Optimizes memory and thread placement
pub struct PlacementOptimizer {
    /// Per-task placement
    placements: BTreeMap<u64, Placement>,
    /// Placement history
    history: Vec<PlacementEvent>,
    /// Optimization scores
    scores: LinearMap<f64, 64>,
    /// Preferred nodes per task
    preferred: BTreeMap<u64, NodeId>,
}

impl PlacementOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            placements: BTreeMap::new(),
            history: Vec::new(),
            scores: LinearMap::new(),
            preferred: BTreeMap::new(),
        }
    }

    /// Set placement
    #[inline(always)]
    pub fn set_placement(&mut self, placement: Placement) {
        self.placements.insert(placement.task_id, placement);
    }

    /// Get placement
    #[inline(always)]
    pub fn get_placement(&self, task_id: u64) -> Option<&Placement> {
        self.placements.get(&task_id)
    }

    /// Record event
    #[inline(always)]
    pub fn record_event(&mut self, event: PlacementEvent) {
        self.history.push(event);
    }

    /// Optimize placement for task
    pub fn optimize(
        &mut self,
        task_id: u64,
        topology: &NumaTopology,
        stats: &NumaStats,
    ) -> Option<PlacementRecommendation> {
        // Analyze current performance
        let local_ratio = stats.local_ratio();

        // If mostly local, no change needed
        if local_ratio >= 0.9 {
            return None;
        }

        // Find best node based on resources
        let best_node = topology.node_with_most_memory()?;

        // Calculate score improvement
        let improvement = 0.9 - local_ratio;

        let score = improvement * 100.0;
        self.scores.insert(task_id, score);

        Some(PlacementRecommendation {
            task_id,
            recommended_node: best_node,
            binding: MemoryBinding::Preferred(best_node),
            expected_improvement: improvement,
            confidence: local_ratio,
        })
    }

    /// Get placement score
    #[inline(always)]
    pub fn get_score(&self, task_id: u64) -> Option<f64> {
        self.scores.get(task_id).copied()
    }

    /// Set preferred node
    #[inline(always)]
    pub fn set_preferred(&mut self, task_id: u64, node: NodeId) {
        self.preferred.insert(task_id, node);
    }

    /// Get preferred node
    #[inline(always)]
    pub fn get_preferred(&self, task_id: u64) -> Option<NodeId> {
        self.preferred.get(&task_id).copied()
    }
}

impl Default for PlacementOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

//! CPU and memory affinity management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use super::stats::NumaStats;
use super::topology::NumaTopology;
use super::types::{CpuId, MemoryBinding, NodeId};
use crate::core::NexusTimestamp;

// ============================================================================
// AFFINITY VIOLATION TYPE
// ============================================================================

/// Affinity violation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffinityViolationType {
    /// Wrong CPU
    WrongCpu,
    /// Wrong node
    WrongNode,
    /// Remote memory access
    RemoteMemory,
    /// Spread too wide
    TooSpread,
}

// ============================================================================
// AFFINITY INFO
// ============================================================================

/// Affinity information
#[derive(Debug, Clone)]
pub struct AffinityInfo {
    /// Task ID
    pub task_id: u64,
    /// CPU affinity mask
    pub cpu_mask: Vec<CpuId>,
    /// NUMA node affinity
    pub node_mask: Vec<NodeId>,
    /// Memory policy
    pub memory_policy: MemoryBinding,
    /// Strict enforcement
    pub strict: bool,
}

// ============================================================================
// AFFINITY VIOLATION
// ============================================================================

/// Affinity violation
#[derive(Debug, Clone)]
pub struct AffinityViolation {
    /// Task ID
    pub task_id: u64,
    /// Violation type
    pub violation_type: AffinityViolationType,
    /// Expected
    pub expected: u32,
    /// Actual
    pub actual: u32,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

// ============================================================================
// AFFINITY MANAGER
// ============================================================================

/// Manages CPU and memory affinity
pub struct AffinityManager {
    /// Task affinities
    affinities: BTreeMap<u64, AffinityInfo>,
    /// Affinity violations
    violations: Vec<AffinityViolation>,
    /// Auto-tuning enabled
    auto_tune: bool,
}

impl AffinityManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            affinities: BTreeMap::new(),
            violations: Vec::new(),
            auto_tune: true,
        }
    }

    /// Set affinity
    pub fn set_affinity(&mut self, affinity: AffinityInfo) {
        self.affinities.insert(affinity.task_id, affinity);
    }

    /// Get affinity
    pub fn get_affinity(&self, task_id: u64) -> Option<&AffinityInfo> {
        self.affinities.get(&task_id)
    }

    /// Record violation
    pub fn record_violation(&mut self, violation: AffinityViolation) {
        self.violations.push(violation);
    }

    /// Get violations for task
    pub fn get_violations(&self, task_id: u64) -> Vec<&AffinityViolation> {
        self.violations
            .iter()
            .filter(|v| v.task_id == task_id)
            .collect()
    }

    /// Enable auto-tuning
    pub fn enable_auto_tune(&mut self) {
        self.auto_tune = true;
    }

    /// Disable auto-tuning
    pub fn disable_auto_tune(&mut self) {
        self.auto_tune = false;
    }

    /// Is auto-tuning enabled?
    pub fn is_auto_tune(&self) -> bool {
        self.auto_tune
    }

    /// Suggest affinity for task
    pub fn suggest(
        &self,
        task_id: u64,
        topology: &NumaTopology,
        stats: &NumaStats,
    ) -> Option<AffinityInfo> {
        if !self.auto_tune {
            return None;
        }

        // If local ratio is good, no change needed
        if stats.local_ratio() >= 0.9 {
            return None;
        }

        // Find node with best resources
        let best_node = topology.node_with_most_memory()?;
        let node = topology.nodes.get(best_node as usize)?;

        Some(AffinityInfo {
            task_id,
            cpu_mask: node.cpus.clone(),
            node_mask: vec![best_node],
            memory_policy: MemoryBinding::Preferred(best_node),
            strict: false,
        })
    }
}

impl Default for AffinityManager {
    fn default() -> Self {
        Self::new()
    }
}

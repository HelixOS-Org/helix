//! Migration Optimizer
//!
//! Smart workload migration optimization.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::{VirtId, WorkloadInfo};
use crate::core::NexusTimestamp;

/// Optimizes workload migration
pub struct MigrationOptimizer {
    /// Migration history
    history: Vec<MigrationRecord>,
    /// Node resources
    node_resources: BTreeMap<u32, NodeResources>,
    /// Migration scores
    scores: BTreeMap<(VirtId, u32), f64>,
}

/// Migration record
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    /// Workload ID
    pub workload_id: VirtId,
    /// Source node
    pub source: u32,
    /// Destination node
    pub destination: u32,
    /// Start time
    pub start_time: NexusTimestamp,
    /// Duration (ms)
    pub duration_ms: u64,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Downtime (ms)
    pub downtime_ms: u64,
    /// Success
    pub success: bool,
}

/// Node resources
#[derive(Debug, Clone)]
pub struct NodeResources {
    /// Node ID
    pub node_id: u32,
    /// Total CPU cores
    pub total_cpus: u32,
    /// Available CPU cores
    pub available_cpus: f64,
    /// Total memory
    pub total_memory: u64,
    /// Available memory
    pub available_memory: u64,
    /// Network bandwidth (Mbps)
    pub network_bandwidth: u64,
    /// Storage capacity
    pub storage_capacity: u64,
}

impl NodeResources {
    /// CPU availability ratio
    pub fn cpu_ratio(&self) -> f64 {
        if self.total_cpus == 0 {
            0.0
        } else {
            self.available_cpus / self.total_cpus as f64
        }
    }

    /// Memory availability ratio
    pub fn memory_ratio(&self) -> f64 {
        if self.total_memory == 0 {
            0.0
        } else {
            self.available_memory as f64 / self.total_memory as f64
        }
    }

    /// Can fit workload?
    pub fn can_fit(&self, vcpus: u32, memory: u64) -> bool {
        self.available_cpus >= vcpus as f64 && self.available_memory >= memory
    }
}

/// Migration recommendation
#[derive(Debug, Clone)]
pub struct MigrationRecommendation {
    /// Workload ID
    pub workload_id: VirtId,
    /// Recommended destination
    pub destination: u32,
    /// Score (higher is better)
    pub score: f64,
    /// Reason
    pub reason: MigrationReason,
    /// Estimated duration
    pub estimated_duration_ms: u64,
    /// Estimated downtime
    pub estimated_downtime_ms: u64,
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    /// Load balancing
    LoadBalance,
    /// Resource shortage
    ResourceShortage,
    /// Power optimization
    PowerOptimization,
    /// Affinity rules
    Affinity,
    /// Maintenance
    Maintenance,
    /// Performance
    Performance,
}

impl MigrationOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            node_resources: BTreeMap::new(),
            scores: BTreeMap::new(),
        }
    }

    /// Update node resources
    pub fn update_node(&mut self, resources: NodeResources) {
        self.node_resources.insert(resources.node_id, resources);
    }

    /// Record migration
    pub fn record_migration(&mut self, record: MigrationRecord) {
        self.history.push(record);
    }

    /// Calculate migration score
    pub fn calculate_score(&mut self, workload: &WorkloadInfo, destination: u32) -> Option<f64> {
        let dest_resources = self.node_resources.get(&destination)?;

        if !dest_resources.can_fit(workload.vcpus, workload.memory) {
            return Some(0.0);
        }

        let cpu_score = dest_resources.cpu_ratio();
        let memory_score = dest_resources.memory_ratio();
        let score = cpu_score * 0.4 + memory_score * 0.4 + 0.2;

        self.scores.insert((workload.id, destination), score);

        Some(score)
    }

    /// Get best destination
    pub fn recommend(&mut self, workload: &WorkloadInfo) -> Option<MigrationRecommendation> {
        let mut best_score = 0.0;
        let mut best_node = None;

        for &node_id in self.node_resources.keys() {
            if Some(node_id) == workload.host_node {
                continue;
            }

            if let Some(score) = self.calculate_score(workload, node_id) {
                if score > best_score {
                    best_score = score;
                    best_node = Some(node_id);
                }
            }
        }

        let destination = best_node?;
        let estimated_duration = (workload.memory / (1024 * 1024 * 100)) as u64 * 1000;
        let estimated_downtime = estimated_duration / 10;

        Some(MigrationRecommendation {
            workload_id: workload.id,
            destination,
            score: best_score,
            reason: MigrationReason::LoadBalance,
            estimated_duration_ms: estimated_duration,
            estimated_downtime_ms: estimated_downtime,
        })
    }

    /// Get node resources
    pub fn get_node(&self, node_id: u32) -> Option<&NodeResources> {
        self.node_resources.get(&node_id)
    }

    /// Get migration history
    pub fn history(&self) -> &[MigrationRecord] {
        &self.history
    }

    /// Average migration duration
    pub fn avg_duration(&self) -> f64 {
        if self.history.is_empty() {
            0.0
        } else {
            let sum: u64 = self.history.iter().map(|r| r.duration_ms).sum();
            sum as f64 / self.history.len() as f64
        }
    }

    /// Node count
    pub fn node_count(&self) -> usize {
        self.node_resources.len()
    }
}

impl Default for MigrationOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

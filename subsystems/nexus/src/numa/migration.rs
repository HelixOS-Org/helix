//! Memory migration cost analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::NodeId;
use crate::core::NexusTimestamp;

// ============================================================================
// MIGRATION RECORD
// ============================================================================

/// Migration record
#[derive(Debug, Clone)]
struct MigrationRecord {
    /// Source node
    source: NodeId,
    /// Destination node
    destination: NodeId,
    /// Bytes migrated
    bytes: u64,
    /// Duration (ns)
    duration_ns: u64,
    /// Timestamp
    timestamp: u64,
}

// ============================================================================
// MIGRATION COST MODEL
// ============================================================================

/// Migration cost model
#[derive(Debug, Clone)]
pub struct MigrationCostModel {
    /// Source node
    pub source: NodeId,
    /// Destination node
    pub destination: NodeId,
    /// Average bandwidth (bytes/sec)
    pub avg_bandwidth: f64,
    /// Fixed overhead (ns)
    pub overhead_ns: u64,
    /// Sample count
    pub samples: u32,
}

impl MigrationCostModel {
    /// Estimate time to migrate bytes
    pub fn estimate_time(&self, bytes: u64) -> u64 {
        if self.avg_bandwidth == 0.0 {
            return u64::MAX;
        }
        self.overhead_ns + (bytes as f64 / self.avg_bandwidth * 1_000_000_000.0) as u64
    }
}

// ============================================================================
// MIGRATION COST ANALYZER
// ============================================================================

/// Analyzes memory migration costs
pub struct MigrationCostAnalyzer {
    /// Migration records
    records: Vec<MigrationRecord>,
    /// Cost models per node pair
    models: BTreeMap<(NodeId, NodeId), MigrationCostModel>,
    /// Total bytes migrated
    total_bytes: AtomicU64,
}

impl MigrationCostAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            models: BTreeMap::new(),
            total_bytes: AtomicU64::new(0),
        }
    }

    /// Record migration
    pub fn record(&mut self, source: NodeId, dest: NodeId, bytes: u64, duration_ns: u64) {
        let record = MigrationRecord {
            source,
            destination: dest,
            bytes,
            duration_ns,
            timestamp: NexusTimestamp::now().raw(),
        };

        self.records.push(record);
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);

        // Update model
        self.update_model(source, dest, bytes, duration_ns);
    }

    /// Update cost model
    fn update_model(&mut self, source: NodeId, dest: NodeId, bytes: u64, duration_ns: u64) {
        let key = (source, dest);
        let bandwidth = if duration_ns > 0 {
            bytes as f64 * 1_000_000_000.0 / duration_ns as f64
        } else {
            0.0
        };

        let model = self.models.entry(key).or_insert(MigrationCostModel {
            source,
            destination: dest,
            avg_bandwidth: bandwidth,
            overhead_ns: 1000, // Default 1µs
            samples: 0,
        });

        // Exponential moving average
        let alpha = 0.1;
        model.avg_bandwidth = alpha * bandwidth + (1.0 - alpha) * model.avg_bandwidth;
        model.samples += 1;
    }

    /// Get cost model
    pub fn get_model(&self, source: NodeId, dest: NodeId) -> Option<&MigrationCostModel> {
        self.models.get(&(source, dest))
    }

    /// Estimate migration cost
    pub fn estimate(&self, source: NodeId, dest: NodeId, bytes: u64) -> u64 {
        if let Some(model) = self.get_model(source, dest) {
            model.estimate_time(bytes)
        } else {
            // Default estimate: 1GB/s
            bytes * 1_000_000_000 / (1024 * 1024 * 1024)
        }
    }

    /// Is migration beneficial?
    pub fn is_beneficial(
        &self,
        source: NodeId,
        dest: NodeId,
        bytes: u64,
        expected_benefit_ns: u64,
    ) -> bool {
        let cost = self.estimate(source, dest, bytes);
        cost < expected_benefit_ns
    }

    /// Get total bytes migrated
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }
}

impl Default for MigrationCostAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

//! Fuzzing statistics

#![allow(dead_code)]

use crate::core::NexusTimestamp;

// ============================================================================
// FUZZ STATS
// ============================================================================

/// Fuzzing statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FuzzStats {
    /// Total executions
    pub executions: u64,
    /// Crashes found
    pub crashes: u64,
    /// Timeouts
    pub timeouts: u64,
    /// Hangs
    pub hangs: u64,
    /// New coverage found
    pub new_coverage: u64,
    /// Corpus size
    pub corpus_size: usize,
    /// Executions per second
    pub exec_per_sec: f64,
    /// Start time
    pub start_time: NexusTimestamp,
    /// Last update time
    pub last_update: NexusTimestamp,
}

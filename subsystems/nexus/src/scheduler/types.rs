//! Task classification types and features.

use alloc::vec;
use alloc::vec::Vec;

use crate::math;

// ============================================================================
// TASK CLASSIFICATION
// ============================================================================

/// Task workload type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkloadType {
    /// CPU-intensive computation
    CpuBound,
    /// I/O-intensive (disk, network)
    IoBound,
    /// Memory-intensive
    MemoryBound,
    /// Interactive (latency-sensitive)
    Interactive,
    /// Background (batch processing)
    Background,
    /// Real-time (strict deadlines)
    RealTime,
    /// Mixed workload
    Mixed,
    /// Unknown (not yet classified)
    Unknown,
}

impl WorkloadType {
    /// Get recommended timeslice multiplier
    pub fn timeslice_multiplier(&self) -> f64 {
        match self {
            Self::CpuBound => 1.5,
            Self::IoBound => 0.5,
            Self::MemoryBound => 1.2,
            Self::Interactive => 0.3,
            Self::Background => 2.0,
            Self::RealTime => 0.2,
            Self::Mixed => 1.0,
            Self::Unknown => 1.0,
        }
    }

    /// Get priority boost
    pub fn priority_boost(&self) -> i32 {
        match self {
            Self::Interactive => 5,
            Self::RealTime => 10,
            Self::IoBound => 2,
            Self::CpuBound => 0,
            Self::MemoryBound => 0,
            Self::Background => -5,
            Self::Mixed => 0,
            Self::Unknown => 0,
        }
    }
}

/// Task behavioral features for classification
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TaskFeatures {
    /// Average CPU usage (0.0-1.0)
    pub avg_cpu_usage: f64,
    /// CPU usage variance
    pub cpu_variance: f64,
    /// Average I/O wait time (microseconds)
    pub avg_io_wait: f64,
    /// I/O operations per second
    pub io_ops_per_sec: f64,
    /// Memory footprint (bytes)
    pub memory_footprint: u64,
    /// Memory access rate
    pub memory_access_rate: f64,
    /// Voluntary context switches per second
    pub voluntary_switches: f64,
    /// Involuntary context switches per second
    pub involuntary_switches: f64,
    /// Average runtime before yield (microseconds)
    pub avg_runtime: f64,
    /// Runtime variance
    pub runtime_variance: f64,
    /// Cache miss rate
    pub cache_miss_rate: f64,
    /// Inter-process communication frequency
    pub ipc_frequency: f64,
    /// Priority changes count
    pub priority_changes: u32,
    /// Sleep/wake frequency
    pub sleep_frequency: f64,
}

impl TaskFeatures {
    /// Create new features
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert to feature vector
    pub fn to_vector(&self) -> Vec<f64> {
        vec![
            self.avg_cpu_usage,
            self.cpu_variance,
            self.avg_io_wait / 1000.0,
            self.io_ops_per_sec / 100.0,
            math::log2(self.memory_footprint as f64) / 32.0,
            self.memory_access_rate,
            self.voluntary_switches / 100.0,
            self.involuntary_switches / 100.0,
            self.avg_runtime / 10000.0,
            self.runtime_variance / 1000.0,
            self.cache_miss_rate,
            self.ipc_frequency / 100.0,
            self.priority_changes as f64 / 10.0,
            self.sleep_frequency / 100.0,
        ]
    }
}

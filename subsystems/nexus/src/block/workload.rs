//! Workload Analysis
//!
//! I/O workload classification and analysis.

/// Workload type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadType {
    /// Sequential read
    SequentialRead,
    /// Sequential write
    SequentialWrite,
    /// Random read
    RandomRead,
    /// Random write
    RandomWrite,
    /// Mixed sequential
    MixedSequential,
    /// Mixed random
    MixedRandom,
    /// Unknown
    Unknown,
}

impl WorkloadType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::SequentialRead => "seq_read",
            Self::SequentialWrite => "seq_write",
            Self::RandomRead => "rand_read",
            Self::RandomWrite => "rand_write",
            Self::MixedSequential => "mixed_seq",
            Self::MixedRandom => "mixed_rand",
            Self::Unknown => "unknown",
        }
    }
}

/// Workload analysis
#[derive(Debug, Clone)]
pub struct WorkloadAnalysis {
    /// Workload type
    pub workload_type: WorkloadType,
    /// Read ratio (0-1)
    pub read_ratio: f32,
    /// Sequential ratio (0-1)
    pub sequential_ratio: f32,
    /// Average I/O size (bytes)
    pub avg_io_size: u64,
    /// IOPS
    pub iops: u64,
    /// Throughput (MB/s)
    pub throughput_mbps: f32,
    /// Queue depth utilization
    pub queue_utilization: f32,
}

impl WorkloadAnalysis {
    /// Create new analysis
    pub fn new() -> Self {
        Self {
            workload_type: WorkloadType::Unknown,
            read_ratio: 0.5,
            sequential_ratio: 0.5,
            avg_io_size: 4096,
            iops: 0,
            throughput_mbps: 0.0,
            queue_utilization: 0.0,
        }
    }

    /// Classify workload
    pub fn classify(&mut self) {
        self.workload_type = match (self.sequential_ratio > 0.7, self.read_ratio) {
            (true, r) if r > 0.8 => WorkloadType::SequentialRead,
            (true, r) if r < 0.2 => WorkloadType::SequentialWrite,
            (true, _) => WorkloadType::MixedSequential,
            (false, r) if r > 0.8 => WorkloadType::RandomRead,
            (false, r) if r < 0.2 => WorkloadType::RandomWrite,
            (false, _) => WorkloadType::MixedRandom,
        };
    }
}

impl Default for WorkloadAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

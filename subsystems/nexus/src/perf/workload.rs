//! Workload Characterization
//!
//! Workload analysis and bottleneck detection.

use alloc::string::String;

use super::PerfMetrics;

// ============================================================================
// WORKLOAD CHARACTERIZATION
// ============================================================================

/// Workload type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadCharacter {
    /// CPU bound (high IPC)
    CpuBound,
    /// Memory bound (high cache misses)
    MemoryBound,
    /// Branch heavy (high branch misses)
    BranchHeavy,
    /// IO bound (lots of context switches)
    IoBound,
    /// Mixed
    Mixed,
    /// Unknown
    Unknown,
}

impl WorkloadCharacter {
    /// Get character name
    pub fn name(&self) -> &'static str {
        match self {
            Self::CpuBound => "cpu-bound",
            Self::MemoryBound => "memory-bound",
            Self::BranchHeavy => "branch-heavy",
            Self::IoBound => "io-bound",
            Self::Mixed => "mixed",
            Self::Unknown => "unknown",
        }
    }
}

/// Workload analysis
#[derive(Debug, Clone)]
pub struct WorkloadAnalysis {
    /// Character
    pub character: WorkloadCharacter,
    /// Confidence (0-100)
    pub confidence: f32,
    /// Details
    pub details: String,
    /// Bottleneck
    pub bottleneck: Option<String>,
}

impl WorkloadAnalysis {
    /// Analyze metrics
    pub fn from_metrics(metrics: &PerfMetrics) -> Self {
        let ipc = metrics.ipc.unwrap_or(0.0);
        let cache_miss = metrics.cache_miss_rate.unwrap_or(0.0);
        let branch_miss = metrics.branch_miss_rate.unwrap_or(0.0);

        // Determine character
        let (character, confidence, bottleneck) = if cache_miss > 20.0 {
            (
                WorkloadCharacter::MemoryBound,
                85.0,
                Some(String::from(
                    "High LLC miss rate - memory latency bottleneck",
                )),
            )
        } else if branch_miss > 10.0 {
            (
                WorkloadCharacter::BranchHeavy,
                80.0,
                Some(String::from(
                    "High branch misprediction - consider restructuring",
                )),
            )
        } else if ipc >= 2.0 {
            (WorkloadCharacter::CpuBound, 90.0, None)
        } else if ipc <= 0.5 {
            (
                WorkloadCharacter::MemoryBound,
                70.0,
                Some(String::from("Low IPC suggests memory stalls")),
            )
        } else {
            (WorkloadCharacter::Mixed, 50.0, None)
        };

        let details = alloc::format!(
            "IPC: {:.2}, Cache miss: {:.1}%, Branch miss: {:.1}%",
            ipc,
            cache_miss,
            branch_miss
        );

        Self {
            character,
            confidence,
            details,
            bottleneck,
        }
    }
}

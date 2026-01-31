//! Performance Monitoring Intelligence Module
//!
//! This module provides AI-powered performance monitoring analysis including
//! PMU hardware counters, perf events, sampling, and workload characterization.

// Submodules
mod events;
mod intelligence;
mod manager;
mod metrics;
mod perf_event;
mod pmu;
mod types;
mod workload;

// Re-export core types
// Re-export event types
pub use events::{
    CacheEvent, CacheLevel, CacheOp, CacheResult, EventType, HardwareEvent, SoftwareEvent,
};
// Re-export intelligence
pub use intelligence::{
    PerfAction, PerfAnalysis, PerfIntelligence, PerfIssue, PerfIssueType, PerfRecommendation,
};
// Re-export manager
pub use manager::PerfManager;
// Re-export metrics
pub use metrics::{BranchMissRate, CacheMissRate, Ipc, PerfMetrics};
// Re-export perf event types
pub use perf_event::{EventConfig, EventState, PerfEvent, Sample, SampleType};
// Re-export PMU types
pub use pmu::{Pmu, PmuCapabilities, PmuType};
pub use types::{CpuId, EventId, PmuId};
// Re-export workload
pub use workload::{WorkloadAnalysis, WorkloadCharacter};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc() {
        let ipc = Ipc::calculate(2_000_000_000, 1_000_000_000);
        assert!((ipc.0 - 2.0).abs() < 0.01);
        assert!(ipc.is_good());
    }

    #[test]
    fn test_cache_miss_rate() {
        let rate = CacheMissRate::calculate(1000, 50);
        assert!((rate.0 - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_perf_metrics() {
        let mut metrics = PerfMetrics::new();
        metrics.cycles = 1_000_000_000;
        metrics.instructions = 1_500_000_000;
        metrics.cache_refs = 10000;
        metrics.cache_misses = 500;
        metrics.calculate_derived();

        assert!(metrics.ipc.unwrap() > 1.0);
        assert!((metrics.cache_miss_rate.unwrap() - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_workload_analysis() {
        let mut metrics = PerfMetrics::new();
        metrics.cycles = 1_000_000_000;
        metrics.instructions = 2_000_000_000;
        metrics.ipc = Some(2.0);

        let analysis = WorkloadAnalysis::from_metrics(&metrics);
        assert!(matches!(analysis.character, WorkloadCharacter::CpuBound));
    }

    #[test]
    fn test_perf_intelligence() {
        let mut intel = PerfIntelligence::new();

        let mut metrics = PerfMetrics::new();
        metrics.cycles = 1_000_000_000;
        metrics.instructions = 400_000_000; // Low IPC
        metrics.ipc = Some(0.4);

        intel.update_metrics(metrics);

        let analysis = intel.analyze();
        // Should detect low IPC
        assert!(
            analysis
                .issues
                .iter()
                .any(|i| matches!(i.issue_type, PerfIssueType::LowIpc))
        );
    }
}

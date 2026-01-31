//! Function Tracing Intelligence Module
//!
//! This module provides AI-powered function tracing analysis including latency
//! measurement, call graph construction, and performance bottleneck detection.

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod callgraph;
mod entry;
mod function;
mod intelligence;
mod latency;
mod manager;
mod tracer;
mod types;

// Re-exports
pub use callgraph::{CallGraph, CallGraphNode};
pub use entry::{TraceEntry, TraceEntryType};
pub use function::FunctionInfo;
pub use intelligence::{
    FtraceAction, FtraceAnalysis, FtraceIntelligence, FtraceIssue, FtraceIssueType,
    FtraceRecommendation, HotFunction, LatencyIssue,
};
pub use latency::{LatencyRecord, LatencyStats, LatencyType};
pub use manager::{FtraceManager, TraceBuffer};
pub use tracer::{TracerOptions, TracerType};
pub use types::{CpuId, FuncAddr, Pid, TraceId};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracer_type() {
        assert_eq!(TracerType::Function.name(), "function");
        assert!(TracerType::IrqsOff.is_latency_tracer());
        assert!(!TracerType::Function.is_latency_tracer());
    }

    #[test]
    fn test_latency_record() {
        let record = LatencyRecord::new(
            LatencyType::IrqOff,
            150_000, // 150us
            0,
            150_000,
            CpuId::new(0),
            Pid::kernel(),
        );

        assert_eq!(record.duration_us(), 150);
    }

    #[test]
    fn test_latency_stats() {
        let mut stats = LatencyStats::new();
        stats.record(100_000);
        stats.record(200_000);
        stats.record(150_000);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.min_ns, 100_000);
        assert_eq!(stats.max_ns, 200_000);
        assert_eq!(stats.avg_ns(), 150_000);
    }

    #[test]
    fn test_call_graph() {
        let mut graph = CallGraph::new();

        let func1 = FuncAddr::new(0x1000);
        let func2 = FuncAddr::new(0x2000);

        graph.add_call(func1, alloc::string::String::from("main"), None, 1000, 500);
        graph.add_call(
            func2,
            alloc::string::String::from("helper"),
            Some(func1),
            400,
            400,
        );

        assert_eq!(graph.function_count(), 2);

        let hottest = graph.hottest(1);
        assert_eq!(hottest[0].name, "main");
    }

    #[test]
    fn test_ftrace_intelligence() {
        let mut intel = FtraceIntelligence::new();

        // Record a high latency
        intel.record_latency(LatencyRecord::new(
            LatencyType::IrqOff,
            500_000, // 500us (high)
            0,
            500_000,
            CpuId::new(0),
            Pid::kernel(),
        ));

        let analysis = intel.analyze();
        // Should detect latency issue
        assert!(analysis.latency_issues.len() > 0);
    }
}

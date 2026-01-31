//! Work Queue Intelligence Module
//!
//! This module provides AI-powered work queue analysis and optimization for the NEXUS subsystem.
//! It includes intelligent work scheduling, queue depth prediction, work stealing optimization,
//! latency analysis, and power-aware work processing.
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//!
//! - [`types`] - Core identifiers and enumerations
//! - [`work`] - Work item and queue information structures
//! - [`predictor`] - Queue depth prediction using time series analysis
//! - [`stealing`] - NUMA-aware work stealing optimization
//! - [`latency`] - Histogram-based latency analysis with SLA tracking
//! - [`power`] - Power-aware scheduling with CPU state management
//! - [`dependency`] - Work dependency tracking with cycle detection
//! - [`intelligence`] - Comprehensive queue analysis and recommendations

// Submodules
pub mod types;
pub mod work;
pub mod predictor;
pub mod stealing;
pub mod latency;
pub mod power;
pub mod dependency;
pub mod intelligence;

// Re-export core types
pub use types::{
    WorkQueueId,
    WorkId,
    CpuId,
    WorkQueueType,
    WorkPriority,
    WorkState,
};

// Re-export work structures
pub use work::{
    WorkInfo,
    WorkQueueInfo,
};

// Re-export predictor
pub use predictor::{
    DepthSample,
    QueueDepthPredictor,
};

// Re-export stealing
pub use stealing::{
    StealingStats,
    StealTarget,
    WorkStealingOptimizer,
};

// Re-export latency
pub use latency::{
    LatencyBucket,
    LatencyStats,
    LatencyTrend,
    WorkLatencyAnalyzer,
};

// Re-export power
pub use power::{
    CpuPowerState,
    PowerSchedulingDecision,
    PowerDecisionReason,
    PowerAwareWorkScheduler,
};

// Re-export dependency
pub use dependency::{
    DependencyType,
    WorkDependency,
    WorkDependencyTracker,
};

// Re-export intelligence
pub use intelligence::{
    WorkQueueAnalysis,
    WorkQueueBottleneck,
    WorkQueueRecommendation,
    WorkQueueAction,
    WorkQueuePrediction,
    WorkQueueIntelligence,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn test_work_queue_id() {
        let id = WorkQueueId::new(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn test_work_priority() {
        assert!(WorkPriority::Critical > WorkPriority::Normal);
        assert!(WorkPriority::Normal > WorkPriority::Low);
        assert_eq!(WorkPriority::from_value(2), WorkPriority::Normal);
    }

    #[test]
    fn test_queue_depth_predictor() {
        let mut predictor = QueueDepthPredictor::new(WorkQueueId::new(1));

        // Record increasing arrival rate
        predictor.record_sample(0, 10, 100.0, 50.0);
        predictor.record_sample(1, 15, 100.0, 50.0);

        // Should predict increasing depth
        let predicted = predictor.predict_depth(15, 1_000_000_000);
        assert!(predicted > 15);
    }

    #[test]
    fn test_work_stealing_optimizer() {
        let mut optimizer = WorkStealingOptimizer::new();

        optimizer.register_cpu(CpuId::new(0));
        optimizer.register_cpu(CpuId::new(1));

        optimizer.update_queue_depth(CpuId::new(0), 10);
        optimizer.update_queue_depth(CpuId::new(1), 2);

        // CPU 1 should want to steal from CPU 0
        let target = optimizer.find_steal_target(CpuId::new(1));
        assert!(target.is_some());
        assert_eq!(target.unwrap().cpu_id, CpuId::new(0));
    }

    #[test]
    fn test_latency_analyzer() {
        let mut analyzer = WorkLatencyAnalyzer::new(WorkQueueId::new(1));

        // Record some latencies
        for i in 0..100 {
            analyzer.record_latency((i * 1000) as u64);
        }

        let stats = analyzer.calculate_stats();
        assert!(stats.min_ns < stats.max_ns);
        assert!(stats.p50_ns <= stats.p99_ns);
    }

    #[test]
    fn test_dependency_tracker() {
        let mut tracker = WorkDependencyTracker::new();

        let work1 = WorkId::new(1);
        let work2 = WorkId::new(2);

        // work2 depends on work1
        assert!(tracker.add_dependency(work1, work2, DependencyType::Hard));

        // work2 should be blocked
        assert!(!tracker.is_ready(work2));

        // work1 should be ready
        assert!(tracker.is_ready(work1));

        // Complete work1
        tracker.mark_completed(work1);

        // work2 should now be ready
        assert!(tracker.is_ready(work2));
    }

    #[test]
    fn test_cycle_detection() {
        let mut tracker = WorkDependencyTracker::new();

        let work1 = WorkId::new(1);
        let work2 = WorkId::new(2);
        let work3 = WorkId::new(3);

        // Create chain: work1 -> work2 -> work3
        assert!(tracker.add_dependency(work1, work2, DependencyType::Hard));
        assert!(tracker.add_dependency(work2, work3, DependencyType::Hard));

        // Try to create cycle: work3 -> work1 (should fail)
        assert!(!tracker.add_dependency(work3, work1, DependencyType::Hard));
        assert_eq!(tracker.cycles_detected(), 1);
    }

    #[test]
    fn test_work_queue_intelligence() {
        let mut intel = WorkQueueIntelligence::new();

        let queue_id = WorkQueueId::new(1);
        intel.register_queue(queue_id, String::from("test"), WorkQueueType::System);

        // Record some work
        let work = WorkInfo::new(
            WorkId::new(1),
            queue_id,
            WorkPriority::Normal,
            String::from("test_fn"),
        );

        intel.record_enqueue(queue_id, &work);
        intel.record_work_started(queue_id, work.id);
        intel.record_work_completed(queue_id, work.id, 10000, true);

        // Analyze
        let analysis = intel.analyze_queue(queue_id);
        assert!(analysis.is_some());
        assert!(analysis.unwrap().health_score > 0.0);
    }

    #[test]
    fn test_power_aware_scheduler() {
        let mut scheduler = PowerAwareWorkScheduler::new();
        
        scheduler.register_cpu(CpuId::new(0), CpuPowerState::Active);
        scheduler.register_cpu(CpuId::new(1), CpuPowerState::Idle);
        
        let work = WorkInfo::new(
            WorkId::new(1),
            WorkQueueId::new(1),
            WorkPriority::Normal,
            String::from("test_fn"),
        );
        
        let decision = scheduler.schedule_work(&work, 0, None);
        // Should prefer active CPU
        assert_eq!(decision.cpu_id, CpuId::new(0));
    }

    #[test]
    fn test_cpu_power_state() {
        assert!(CpuPowerState::Turbo > CpuPowerState::Active);
        assert!(CpuPowerState::Active > CpuPowerState::Idle);
        assert!(CpuPowerState::Idle > CpuPowerState::DeepSleep);
        
        assert!(CpuPowerState::DeepSleep.wakeup_latency_ns() > CpuPowerState::Active.wakeup_latency_ns());
        assert!(CpuPowerState::Turbo.power_units() > CpuPowerState::Idle.power_units());
    }
}

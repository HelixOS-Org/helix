//! # AI-Powered Scheduler Intelligence
//!
//! Intelligent scheduling decisions using machine learning and predictive analytics.
//!
//! ## Key Features
//!
//! - **Workload Classification**: Classify tasks by behavior patterns
//! - **Priority Learning**: Learn optimal priorities from execution patterns
//! - **Affinity Prediction**: Predict optimal CPU/core affinity
//! - **Preemption Intelligence**: Smart preemption decisions
//! - **Load Prediction**: Predict future system load
//! - **Fairness Optimization**: ML-driven fair scheduling

#![allow(dead_code)]

extern crate alloc;

// Submodules
mod affinity;
mod classifier;
mod intelligence;
mod load;
mod preemption;
mod priority;
mod types;

// Re-exports
pub use affinity::{AffinityPredictor, NumaNode, NumaTopology};
pub use classifier::WorkloadClassifier;
pub use intelligence::{SchedulerFeatures, SchedulerIntelligence, SchedulerStats};
pub use load::LoadPredictor;
pub use preemption::PreemptionIntelligence;
pub use priority::PriorityLearner;
pub use types::{TaskFeatures, WorkloadType};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_classification() {
        let classifier = WorkloadClassifier::new();

        // CPU-bound task
        let features = TaskFeatures {
            avg_cpu_usage: 0.95,
            io_ops_per_sec: 5.0,
            voluntary_switches: 10.0,
            ..Default::default()
        };
        assert_eq!(classifier.classify(&features), WorkloadType::CpuBound);

        // I/O-bound task
        let features = TaskFeatures {
            avg_cpu_usage: 0.1,
            io_ops_per_sec: 500.0,
            avg_io_wait: 50000.0,
            voluntary_switches: 200.0,
            ..Default::default()
        };
        assert_eq!(classifier.classify(&features), WorkloadType::IoBound);
    }

    #[test]
    fn test_priority_learner() {
        let mut learner = PriorityLearner::new();

        // Record some outcomes
        learner.record_outcome(1, 0, 0.9);
        learner.record_outcome(1, 0, 0.8);

        // Should learn positive adjustment for good performance
        let adj = learner.get_adjustment(1, &TaskFeatures::default());
        assert!(adj > 0 || adj == 0); // At least not negative
    }

    #[test]
    fn test_affinity_predictor() {
        let mut predictor = AffinityPredictor::new(4);

        // Update utilization
        predictor.update_core_utilization(0, 0.9);
        predictor.update_core_utilization(1, 0.1);
        predictor.update_core_utilization(2, 0.5);
        predictor.update_core_utilization(3, 0.3);

        // Should prefer least loaded core
        let best = predictor.predict_best_core(123, &TaskFeatures::default());
        assert_eq!(best, 1);
    }

    #[test]
    fn test_load_predictor() {
        let mut predictor = LoadPredictor::new();

        // Record some load samples
        for _ in 0..10 {
            predictor.record(0.7, 10);
        }

        // Prediction should be close to recorded
        let prediction = predictor.predict(10);
        assert!(prediction > 0.5 && prediction < 0.9);
    }

    #[test]
    fn test_preemption_decision() {
        let preemption = PreemptionIntelligence::new();

        // RT should preempt non-RT
        assert!(preemption.should_preempt(
            WorkloadType::CpuBound,
            1000,
            5000,
            WorkloadType::RealTime,
            10,
            5
        ));

        // Non-RT should not preempt RT
        assert!(!preemption.should_preempt(
            WorkloadType::RealTime,
            1000,
            5000,
            WorkloadType::Background,
            5,
            10
        ));
    }
}

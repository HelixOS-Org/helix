//! # Crash Prediction Engine
//!
//! Revolutionary crash prediction system that can predict failures 30+ seconds
//! before they occur, using temporal feature analysis and decision trees.
//!
//! ## Key Innovations
//!
//! - **Temporal Features**: Analyze trends over sliding windows
//! - **Decision Trees**: Fast, deterministic, explainable predictions
//! - **Canary Invariants**: Early warning system for corruption
//! - **Multi-Signal Fusion**: Combine memory, CPU, I/O, and timing signals
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core types (PredictionConfidence, PredictionKind, Trend)
//! - `prediction`: Crash prediction structures and actions
//! - `feature`: Feature tracking and statistical analysis
//! - `tree`: Decision tree for prediction
//! - `engine`: Main prediction engine coordinator

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod types;
pub mod prediction;
pub mod feature;
pub mod tree;
pub mod engine;

// Re-export core types
pub use types::{PredictionConfidence, PredictionKind, Trend};

// Re-export prediction types
pub use prediction::{CrashPrediction, PredictionFactor, RecommendedAction};

// Re-export feature types
pub use feature::{Feature, FeatureCategory};

// Re-export tree types
pub use tree::DecisionNode;

// Re-export engine types
pub use engine::PredictionEngine;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_confidence() {
        let conf = PredictionConfidence::new(0.85);
        assert!(conf.is_high());
        assert!(conf.is_above(0.8));
        assert!(!conf.is_above(0.9));
    }

    #[test]
    fn test_feature_statistics() {
        let mut feature = Feature::new(1, "test", FeatureCategory::Memory, 10);

        for i in 0..10 {
            feature.update(i as f64);
        }

        assert!((feature.mean() - 4.5).abs() < 0.001);
        assert!(feature.gradient() > 0.0);
        assert_eq!(feature.trend(), Trend::RapidIncrease);
    }

    #[test]
    fn test_decision_tree() {
        let tree = DecisionNode::split(
            1,
            0.5,
            DecisionNode::leaf(PredictionKind::Crash, 0.3, 10_000),
            DecisionNode::leaf(PredictionKind::Crash, 0.9, 5_000),
        );

        let mut low_feature = Feature::new(1, "test", FeatureCategory::Memory, 10);
        low_feature.update(0.3);

        let result = tree.evaluate(&[low_feature]);
        assert!(result.is_some());
        let (_, conf, _) = result.unwrap();
        assert!((conf - 0.3).abs() < 0.001);

        let mut high_feature = Feature::new(1, "test", FeatureCategory::Memory, 10);
        high_feature.update(0.7);

        let result = tree.evaluate(&[high_feature]);
        let (_, conf, _) = result.unwrap();
        assert!((conf - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_prediction_engine() {
        let mut engine = PredictionEngine::default();

        // Simulate high memory pressure
        for _ in 0..10 {
            engine.update_feature(2, 0.95); // memory_pressure
            engine.update_feature(3, 5000.0); // page_fault_rate
        }

        let predictions = engine.predict();
        assert!(!predictions.is_empty());

        let oom_pred = predictions
            .iter()
            .find(|p| p.kind == PredictionKind::OutOfMemory);
        assert!(oom_pred.is_some());
    }
}

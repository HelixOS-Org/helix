//! # NEXUS Statistics
//!
//! Comprehensive statistics and metrics for the NEXUS system.
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `nexus_stats`: Main NEXUS statistics
//! - `counter`: Atomic counters for thread-safe statistics
//! - `histogram`: Histogram for latency/timing measurements
//! - `rate`: Rate meter for measuring events per time unit
//! - `component`: Component, prediction, and healing statistics

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod component;
pub mod counter;
pub mod histogram;
pub mod nexus_stats;
pub mod rate;

// Re-export nexus stats
pub use nexus_stats::NexusStats;

// Re-export counter
pub use counter::AtomicCounter;

// Re-export histogram
pub use histogram::Histogram;

// Re-export rate meter
pub use rate::RateMeter;

// Re-export component stats
pub use component::{ComponentStats, HealingStats, PredictionStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_counter() {
        let counter = AtomicCounter::new();
        assert_eq!(counter.get(), 0);

        counter.inc();
        assert_eq!(counter.get(), 1);

        counter.add(5);
        assert_eq!(counter.get(), 6);

        counter.reset();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_histogram() {
        let hist = Histogram::for_latency();

        hist.record(50); // < 100ns
        hist.record(200); // 100-500ns
        hist.record(1500); // 1-5µs

        assert_eq!(hist.count(), 3);
        assert_eq!(hist.min(), Some(50));
        assert_eq!(hist.max(), Some(1500));
    }

    #[test]
    fn test_nexus_stats() {
        let mut stats = NexusStats::default();
        stats.predictions_made = 100;
        stats.predictions_correct = 85;

        assert!((stats.prediction_accuracy() - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_prediction_stats() {
        let stats = PredictionStats {
            true_positives: 80,
            false_positives: 10,
            false_negatives: 20,
            true_negatives: 90,
            total_predictions: 200,
            correct_predictions: 170,
            ..Default::default()
        };

        assert!((stats.accuracy() - 0.85).abs() < 0.001);
        assert!(stats.precision() > 0.0);
        assert!(stats.recall() > 0.0);
        assert!(stats.f1_score() > 0.0);
    }
}

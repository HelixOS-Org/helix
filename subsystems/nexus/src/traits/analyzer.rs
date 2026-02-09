//! Analyzer and Comprehension Traits
//!
//! Traits for the UNDERSTAND domain - pattern detection and feature extraction.

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

use super::component::NexusComponent;
use crate::types::{Confidence, NexusResult, PatternId, Timestamp};

// ============================================================================
// ANALYZER TRAIT
// ============================================================================

/// Trait for comprehension analyzers
pub trait Analyzer: NexusComponent {
    /// Input type
    type Input;
    /// Output type
    type Output;

    /// Analyze input and produce output
    fn analyze(&self, input: &Self::Input) -> NexusResult<Self::Output>;

    /// Analyze batch of inputs
    fn analyze_batch(&self, inputs: &[Self::Input]) -> NexusResult<Vec<Self::Output>> {
        inputs.iter().map(|i| self.analyze(i)).collect()
    }

    /// Get analysis confidence
    fn confidence(&self) -> Confidence;

    /// Get analyzer statistics
    fn stats(&self) -> AnalyzerStats;
}

// ============================================================================
// ANALYZER STATS
// ============================================================================

/// Analyzer statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AnalyzerStats {
    /// Total analyses performed
    pub total_analyses: u64,
    /// Successful analyses
    pub successful: u64,
    /// Failed analyses
    pub failed: u64,
    /// Average latency (nanoseconds)
    pub avg_latency_ns: u64,
    /// P99 latency (nanoseconds)
    pub p99_latency_ns: u64,
    /// Last analysis timestamp
    pub last_analysis: Timestamp,
}

impl AnalyzerStats {
    /// Success rate (0.0 to 1.0)
    #[inline]
    pub fn success_rate(&self) -> f32 {
        if self.total_analyses == 0 {
            return 1.0;
        }
        self.successful as f32 / self.total_analyses as f32
    }

    /// Record a successful analysis
    #[inline]
    pub fn record_success(&mut self, latency_ns: u64) {
        self.total_analyses += 1;
        self.successful += 1;
        self.update_latency(latency_ns);
    }

    /// Record a failed analysis
    #[inline]
    pub fn record_failure(&mut self, latency_ns: u64) {
        self.total_analyses += 1;
        self.failed += 1;
        self.update_latency(latency_ns);
    }

    fn update_latency(&mut self, latency_ns: u64) {
        // Simple moving average
        if self.total_analyses == 1 {
            self.avg_latency_ns = latency_ns;
        } else {
            self.avg_latency_ns = (self.avg_latency_ns * 9 + latency_ns) / 10;
        }
        if latency_ns > self.p99_latency_ns {
            self.p99_latency_ns = latency_ns;
        }
        self.last_analysis = Timestamp::now();
    }
}

// ============================================================================
// PATTERN DETECTOR TRAIT
// ============================================================================

/// Pattern detector trait
pub trait PatternDetector: Analyzer {
    /// Pattern type this detector finds
    type Pattern;

    /// Detect patterns in data
    fn detect(&self, data: &Self::Input) -> Vec<Self::Pattern>;

    /// Get known patterns
    fn known_patterns(&self) -> &[Self::Pattern];

    /// Register a new pattern
    fn register_pattern(&mut self, pattern: Self::Pattern) -> PatternId;

    /// Remove a pattern
    fn remove_pattern(&mut self, id: PatternId) -> bool;

    /// Get pattern match count
    fn match_count(&self, id: PatternId) -> u64;
}

// ============================================================================
// FEATURE EXTRACTOR TRAIT
// ============================================================================

/// Feature extractor trait
pub trait FeatureExtractor: NexusComponent {
    /// Input type
    type Input;
    /// Feature vector type
    type Features;

    /// Extract features from input
    fn extract(&self, input: &Self::Input) -> NexusResult<Self::Features>;

    /// Get feature dimension
    fn dimension(&self) -> usize;

    /// Get feature names
    fn feature_names(&self) -> Vec<String>;

    /// Normalize features
    fn normalize(&self, features: &Self::Features) -> Self::Features;
}

// ============================================================================
// CLASSIFIER TRAIT
// ============================================================================

/// Classifier trait for categorization
pub trait Classifier: NexusComponent {
    /// Input type
    type Input;
    /// Class label type
    type Label;

    /// Classify input
    fn classify(&self, input: &Self::Input) -> NexusResult<Self::Label>;

    /// Get classification confidence
    fn classify_with_confidence(
        &self,
        input: &Self::Input,
    ) -> NexusResult<(Self::Label, Confidence)>;

    /// Get all possible labels
    fn labels(&self) -> Vec<Self::Label>;

    /// Get probability distribution over labels
    fn probabilities(&self, input: &Self::Input) -> NexusResult<Vec<(Self::Label, f32)>>;
}

// ============================================================================
// ANOMALY DETECTOR TRAIT
// ============================================================================

/// Anomaly detector trait
pub trait AnomalyDetectorTrait: NexusComponent {
    /// Data point type
    type DataPoint;

    /// Check if data point is anomalous
    fn is_anomaly(&self, data: &Self::DataPoint) -> bool;

    /// Get anomaly score (higher = more anomalous)
    fn anomaly_score(&self, data: &Self::DataPoint) -> f64;

    /// Update baseline with normal data
    fn update_baseline(&mut self, data: &Self::DataPoint);

    /// Get current threshold
    fn threshold(&self) -> f64;

    /// Set threshold
    fn set_threshold(&mut self, threshold: f64);
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_stats() {
        let mut stats = AnalyzerStats::default();
        assert_eq!(stats.success_rate(), 1.0);

        stats.record_success(1000);
        stats.record_success(2000);
        assert_eq!(stats.total_analyses, 2);
        assert_eq!(stats.successful, 2);
        assert_eq!(stats.success_rate(), 1.0);

        stats.record_failure(500);
        assert_eq!(stats.total_analyses, 3);
        assert!((stats.success_rate() - 0.666).abs() < 0.01);
    }
}

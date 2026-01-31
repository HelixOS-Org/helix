//! Prediction configuration.

/// Configuration for crash prediction
#[derive(Debug, Clone)]
pub struct PredictionConfig {
    /// Enable crash prediction
    pub enabled: bool,

    /// Prediction horizon in milliseconds
    pub horizon_ms: u64,

    /// Minimum confidence to trigger action
    pub min_confidence: f32,

    /// Number of features to track
    pub feature_count: usize,

    /// Sample window size
    pub sample_window: usize,

    /// Maximum decision tree depth
    pub max_tree_depth: usize,

    /// Enable degradation detection
    pub detect_degradation: bool,

    /// Enable memory leak detection
    pub detect_memory_leaks: bool,

    /// Enable deadlock prediction
    pub predict_deadlocks: bool,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            horizon_ms: 30_000, // 30 seconds
            min_confidence: 0.7,
            feature_count: 64,
            sample_window: 1000,
            max_tree_depth: 12,
            detect_degradation: true,
            detect_memory_leaks: true,
            predict_deadlocks: true,
        }
    }
}

impl PredictionConfig {
    /// Minimal prediction configuration
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            horizon_ms: 10_000,
            min_confidence: 0.8,
            feature_count: 16,
            sample_window: 100,
            max_tree_depth: 8,
            detect_degradation: false,
            detect_memory_leaks: true,
            predict_deadlocks: false,
        }
    }

    /// Full prediction configuration
    pub fn full() -> Self {
        Self {
            enabled: true,
            horizon_ms: 60_000, // 1 minute
            min_confidence: 0.6,
            feature_count: 128,
            sample_window: 10_000,
            max_tree_depth: 16,
            detect_degradation: true,
            detect_memory_leaks: true,
            predict_deadlocks: true,
        }
    }
}

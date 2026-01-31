//! Detector configuration

#![allow(dead_code)]

// ============================================================================
// DETECTOR CONFIGURATION
// ============================================================================

/// Configuration for anomaly detection
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Z-score threshold for anomaly detection
    pub z_score_threshold: f64,
    /// IQR multiplier for outlier detection
    pub iqr_multiplier: f64,
    /// Window size for moving statistics
    pub window_size: usize,
    /// Minimum data points before detection starts
    pub min_samples: usize,
    /// Enable z-score detection
    pub enable_zscore: bool,
    /// Enable IQR detection
    pub enable_iqr: bool,
    /// Enable trend detection
    pub enable_trend: bool,
    /// Enable pattern detection
    pub enable_pattern: bool,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            z_score_threshold: 2.5,
            iqr_multiplier: 1.5,
            window_size: 100,
            min_samples: 30,
            enable_zscore: true,
            enable_iqr: true,
            enable_trend: true,
            enable_pattern: false,
        }
    }
}

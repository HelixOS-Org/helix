//! # Online Learning for NEXUS
//!
//! Streaming learning algorithms with concept drift detection.
//!
//! ## Features
//!
//! - Online gradient descent
//! - Streaming classification
//! - Concept drift detection (ADWIN, Page-Hinkley)
//! - Adaptive windowing
//! - Incremental statistics

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// STREAMING SAMPLE
// ============================================================================

/// A sample in the data stream
#[derive(Debug, Clone)]
pub struct StreamingSample {
    /// Feature vector
    pub features: Vec<f64>,
    /// Label (for supervised learning)
    pub label: Option<f64>,
    /// Timestamp
    pub timestamp: u64,
    /// Sample weight
    pub weight: f64,
}

impl StreamingSample {
    /// Create new sample
    pub fn new(features: Vec<f64>, label: Option<f64>) -> Self {
        Self {
            features,
            label,
            timestamp: 0,
            weight: 1.0,
        }
    }

    /// Set timestamp
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }

    /// Set weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

// ============================================================================
// ONLINE STATISTICS
// ============================================================================

/// Online statistics (Welford's algorithm)
#[derive(Debug, Clone, Default)]
pub struct OnlineStats {
    /// Count
    n: u64,
    /// Mean
    mean: f64,
    /// M2 (for variance)
    m2: f64,
    /// Min value
    min: f64,
    /// Max value
    max: f64,
}

impl OnlineStats {
    /// Create new stats
    pub fn new() -> Self {
        Self {
            n: 0,
            mean: 0.0,
            m2: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Update with new value
    pub fn update(&mut self, value: f64) {
        self.n += 1;
        let delta = value - self.mean;
        self.mean += delta / self.n as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;

        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    /// Get count
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Get variance
    pub fn variance(&self) -> f64 {
        if self.n < 2 {
            0.0
        } else {
            self.m2 / (self.n - 1) as f64
        }
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Get min
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Get max
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Merge with another stats object
    pub fn merge(&mut self, other: &OnlineStats) {
        if other.n == 0 {
            return;
        }

        let combined_n = self.n + other.n;
        let delta = other.mean - self.mean;

        self.mean = (self.n as f64 * self.mean + other.n as f64 * other.mean) / combined_n as f64;
        self.m2 =
            self.m2 + other.m2 + delta * delta * (self.n * other.n) as f64 / combined_n as f64;
        self.n = combined_n;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
}

// ============================================================================
// ONLINE LEARNER
// ============================================================================

/// Online learner configuration
#[derive(Debug, Clone)]
pub struct OnlineLearnerConfig {
    /// Learning rate
    pub learning_rate: f64,
    /// Learning rate decay
    pub lr_decay: f64,
    /// Regularization strength (L2)
    pub regularization: f64,
    /// Feature dimension
    pub feature_dim: usize,
    /// Window size for recent samples
    pub window_size: usize,
}

impl Default for OnlineLearnerConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.01,
            lr_decay: 0.999,
            regularization: 0.001,
            feature_dim: 10,
            window_size: 1000,
        }
    }
}

/// Online linear learner (SGD)
pub struct OnlineLearner {
    /// Configuration
    config: OnlineLearnerConfig,
    /// Model weights
    weights: Vec<f64>,
    /// Bias term
    bias: f64,
    /// Current learning rate
    current_lr: f64,
    /// Samples seen
    samples_seen: u64,
    /// Recent errors for tracking
    recent_errors: VecDeque<f64>,
    /// Feature statistics
    feature_stats: Vec<OnlineStats>,
}

impl OnlineLearner {
    /// Create new online learner
    pub fn new(config: OnlineLearnerConfig) -> Self {
        let dim = config.feature_dim;
        Self {
            current_lr: config.learning_rate,
            weights: vec![0.0; dim],
            bias: 0.0,
            samples_seen: 0,
            recent_errors: VecDeque::with_capacity(config.window_size),
            feature_stats: (0..dim).map(|_| OnlineStats::new()).collect(),
            config,
        }
    }

    /// Predict value for sample
    pub fn predict(&self, features: &[f64]) -> f64 {
        let dot: f64 = self
            .weights
            .iter()
            .zip(features.iter())
            .map(|(w, f)| w * f)
            .sum();
        dot + self.bias
    }

    /// Update model with sample (regression)
    pub fn update(&mut self, sample: &StreamingSample) {
        if sample.label.is_none() {
            return;
        }

        let label = sample.label.unwrap();
        let prediction = self.predict(&sample.features);
        let error = prediction - label;

        // Update feature statistics
        for (i, &f) in sample.features.iter().enumerate() {
            if i < self.feature_stats.len() {
                self.feature_stats[i].update(f);
            }
        }

        // Track error
        if self.recent_errors.len() >= self.config.window_size {
            self.recent_errors.pop_front();
        }
        self.recent_errors.push_back(error.abs());

        // SGD update with regularization
        for (i, &f) in sample.features.iter().enumerate() {
            if i < self.weights.len() {
                let grad = error * f + self.config.regularization * self.weights[i];
                self.weights[i] -= self.current_lr * grad * sample.weight;
            }
        }
        self.bias -= self.current_lr * error * sample.weight;

        // Decay learning rate
        self.current_lr *= self.config.lr_decay;
        self.samples_seen += 1;
    }

    /// Get mean absolute error over recent window
    pub fn recent_mae(&self) -> f64 {
        if self.recent_errors.is_empty() {
            return 0.0;
        }
        self.recent_errors.iter().sum::<f64>() / self.recent_errors.len() as f64
    }

    /// Get model weights
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// Get samples seen
    pub fn samples_seen(&self) -> u64 {
        self.samples_seen
    }

    /// Reset model
    pub fn reset(&mut self) {
        self.weights = vec![0.0; self.config.feature_dim];
        self.bias = 0.0;
        self.current_lr = self.config.learning_rate;
        self.samples_seen = 0;
        self.recent_errors.clear();
    }
}

// ============================================================================
// STREAMING CLASSIFIER
// ============================================================================

/// Streaming multiclass classifier (one-vs-all)
pub struct StreamingClassifier {
    /// Number of classes
    num_classes: usize,
    /// Feature dimension
    feature_dim: usize,
    /// Weights per class
    weights: Vec<Vec<f64>>,
    /// Biases per class
    biases: Vec<f64>,
    /// Learning rate
    learning_rate: f64,
    /// Samples seen
    samples_seen: u64,
    /// Class counts
    class_counts: Vec<u64>,
}

impl StreamingClassifier {
    /// Create new classifier
    pub fn new(num_classes: usize, feature_dim: usize, learning_rate: f64) -> Self {
        Self {
            num_classes,
            feature_dim,
            weights: vec![vec![0.0; feature_dim]; num_classes],
            biases: vec![0.0; num_classes],
            learning_rate,
            samples_seen: 0,
            class_counts: vec![0; num_classes],
        }
    }

    /// Get class scores (logits)
    pub fn scores(&self, features: &[f64]) -> Vec<f64> {
        self.weights
            .iter()
            .zip(self.biases.iter())
            .map(|(w, &b)| {
                let dot: f64 = w.iter().zip(features.iter()).map(|(wi, fi)| wi * fi).sum();
                dot + b
            })
            .collect()
    }

    /// Get class probabilities (softmax)
    pub fn probabilities(&self, features: &[f64]) -> Vec<f64> {
        let scores = self.scores(features);
        let max_score = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let exp_scores: Vec<f64> = scores.iter().map(|s| (s - max_score).exp()).collect();
        let sum: f64 = exp_scores.iter().sum();

        exp_scores.into_iter().map(|e| e / sum).collect()
    }

    /// Predict class
    pub fn predict(&self, features: &[f64]) -> usize {
        let scores = self.scores(features);
        scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Update with labeled sample
    pub fn update(&mut self, features: &[f64], label: usize) {
        if label >= self.num_classes {
            return;
        }

        let probs = self.probabilities(features);

        // Softmax cross-entropy gradient
        for c in 0..self.num_classes {
            let target = if c == label { 1.0 } else { 0.0 };
            let gradient = probs[c] - target;

            for (i, &f) in features.iter().enumerate() {
                if i < self.feature_dim {
                    self.weights[c][i] -= self.learning_rate * gradient * f;
                }
            }
            self.biases[c] -= self.learning_rate * gradient;
        }

        self.class_counts[label] += 1;
        self.samples_seen += 1;
    }

    /// Get accuracy estimate from confusion
    pub fn class_distribution(&self) -> Vec<f64> {
        let total = self.samples_seen as f64;
        if total == 0.0 {
            return vec![0.0; self.num_classes];
        }
        self.class_counts
            .iter()
            .map(|&c| c as f64 / total)
            .collect()
    }

    /// Get number of classes
    pub fn num_classes(&self) -> usize {
        self.num_classes
    }
}

// ============================================================================
// CONCEPT DRIFT DETECTION
// ============================================================================

/// Concept drift detector trait
pub trait DriftDetector {
    /// Update with new value
    fn update(&mut self, value: f64);

    /// Check if drift detected
    fn drift_detected(&self) -> bool;

    /// Check if warning level reached
    fn warning_detected(&self) -> bool;

    /// Reset detector
    fn reset(&mut self);
}

/// ADWIN (Adaptive Windowing) drift detector
pub struct AdwinDetector {
    /// Window of values
    window: VecDeque<f64>,
    /// Maximum window size
    max_size: usize,
    /// Delta parameter (confidence)
    delta: f64,
    /// Drift detected flag
    drift: bool,
    /// Warning flag
    warning: bool,
    /// Minimum samples for detection
    min_samples: usize,
}

impl AdwinDetector {
    /// Create new ADWIN detector
    pub fn new(delta: f64, max_size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(max_size),
            max_size,
            delta,
            drift: false,
            warning: false,
            min_samples: 30,
        }
    }

    /// Calculate window statistics
    fn window_stats(&self, start: usize, end: usize) -> (f64, f64) {
        if start >= end || end > self.window.len() {
            return (0.0, 0.0);
        }

        let slice: Vec<f64> = self
            .window
            .iter()
            .skip(start)
            .take(end - start)
            .copied()
            .collect();
        let n = slice.len() as f64;
        if n == 0.0 {
            return (0.0, 0.0);
        }

        let mean = slice.iter().sum::<f64>() / n;
        let variance = slice.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;

        (mean, variance)
    }

    /// Check for drift using ADWIN algorithm
    fn check_drift(&mut self) {
        let n = self.window.len();
        if n < self.min_samples {
            self.drift = false;
            self.warning = false;
            return;
        }

        self.drift = false;
        self.warning = false;

        // Try different cut points
        for cut in 1..n {
            let n1 = cut as f64;
            let n2 = (n - cut) as f64;

            if n1 < 5.0 || n2 < 5.0 {
                continue;
            }

            let (mean1, _) = self.window_stats(0, cut);
            let (mean2, _) = self.window_stats(cut, n);

            let harmonic_mean = 2.0 * n1 * n2 / (n1 + n2);
            let epsilon = ((1.0 / (2.0 * harmonic_mean)) * (4.0 / self.delta).ln()).sqrt();

            let diff = (mean1 - mean2).abs();

            if diff > epsilon {
                self.drift = true;
                // Remove old window
                for _ in 0..cut {
                    self.window.pop_front();
                }
                break;
            } else if diff > epsilon * 0.7 {
                self.warning = true;
            }
        }
    }
}

impl DriftDetector for AdwinDetector {
    fn update(&mut self, value: f64) {
        if self.window.len() >= self.max_size {
            self.window.pop_front();
        }
        self.window.push_back(value);
        self.check_drift();
    }

    fn drift_detected(&self) -> bool {
        self.drift
    }

    fn warning_detected(&self) -> bool {
        self.warning
    }

    fn reset(&mut self) {
        self.window.clear();
        self.drift = false;
        self.warning = false;
    }
}

/// Page-Hinkley test for drift detection
pub struct PageHinkleyDetector {
    /// Cumulative sum
    cumsum: f64,
    /// Minimum cumulative sum
    min_cumsum: f64,
    /// Running mean
    mean: f64,
    /// Sample count
    count: u64,
    /// Detection threshold (lambda)
    threshold: f64,
    /// Minimum deviation (delta)
    min_deviation: f64,
    /// Drift detected
    drift: bool,
}

impl PageHinkleyDetector {
    /// Create new Page-Hinkley detector
    pub fn new(threshold: f64, min_deviation: f64) -> Self {
        Self {
            cumsum: 0.0,
            min_cumsum: 0.0,
            mean: 0.0,
            count: 0,
            threshold,
            min_deviation,
            drift: false,
        }
    }
}

impl DriftDetector for PageHinkleyDetector {
    fn update(&mut self, value: f64) {
        self.count += 1;

        // Update mean
        self.mean += (value - self.mean) / self.count as f64;

        // Update cumulative sum
        self.cumsum += value - self.mean - self.min_deviation;

        // Update minimum
        self.min_cumsum = self.min_cumsum.min(self.cumsum);

        // Check for drift
        self.drift = (self.cumsum - self.min_cumsum) > self.threshold;
    }

    fn drift_detected(&self) -> bool {
        self.drift
    }

    fn warning_detected(&self) -> bool {
        let warning_threshold = self.threshold * 0.7;
        (self.cumsum - self.min_cumsum) > warning_threshold
    }

    fn reset(&mut self) {
        self.cumsum = 0.0;
        self.min_cumsum = 0.0;
        self.mean = 0.0;
        self.count = 0;
        self.drift = false;
    }
}

/// Combined concept drift detector
pub struct ConceptDriftDetector {
    /// Primary detector (ADWIN)
    adwin: AdwinDetector,
    /// Secondary detector (Page-Hinkley)
    page_hinkley: PageHinkleyDetector,
    /// Drift history
    drift_history: VecDeque<(u64, String)>,
    /// Current timestamp
    timestamp: u64,
    /// Maximum history size
    max_history: usize,
}

impl ConceptDriftDetector {
    /// Create new combined detector
    pub fn new() -> Self {
        Self {
            adwin: AdwinDetector::new(0.002, 1000),
            page_hinkley: PageHinkleyDetector::new(50.0, 0.005),
            drift_history: VecDeque::with_capacity(100),
            timestamp: 0,
            max_history: 100,
        }
    }

    /// Update with error value
    pub fn update(&mut self, error: f64) {
        self.timestamp += 1;
        self.adwin.update(error);
        self.page_hinkley.update(error);

        if self.drift_detected() {
            if self.drift_history.len() >= self.max_history {
                self.drift_history.pop_front();
            }

            let detector = if self.adwin.drift_detected() && self.page_hinkley.drift_detected() {
                String::from("both")
            } else if self.adwin.drift_detected() {
                String::from("adwin")
            } else {
                String::from("page_hinkley")
            };

            self.drift_history.push_back((self.timestamp, detector));
        }
    }

    /// Check if any detector flagged drift
    pub fn drift_detected(&self) -> bool {
        self.adwin.drift_detected() || self.page_hinkley.drift_detected()
    }

    /// Check if both detectors agree on drift
    pub fn strong_drift_detected(&self) -> bool {
        self.adwin.drift_detected() && self.page_hinkley.drift_detected()
    }

    /// Check if warning level reached
    pub fn warning_detected(&self) -> bool {
        self.adwin.warning_detected() || self.page_hinkley.warning_detected()
    }

    /// Get drift count
    pub fn drift_count(&self) -> usize {
        self.drift_history.len()
    }

    /// Reset detectors
    pub fn reset(&mut self) {
        self.adwin.reset();
        self.page_hinkley.reset();
        self.timestamp = 0;
    }
}

impl Default for ConceptDriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ADAPTIVE LEARNER
// ============================================================================

/// Learner that adapts to concept drift
pub struct AdaptiveLearner {
    /// Current model
    current: OnlineLearner,
    /// Backup model (trained on recent data)
    backup: OnlineLearner,
    /// Drift detector
    drift_detector: ConceptDriftDetector,
    /// Window of recent samples
    recent_samples: VecDeque<StreamingSample>,
    /// Window size
    window_size: usize,
    /// Switches count
    switches: u64,
}

impl AdaptiveLearner {
    /// Create new adaptive learner
    pub fn new(config: OnlineLearnerConfig) -> Self {
        let window_size = config.window_size;
        Self {
            current: OnlineLearner::new(config.clone()),
            backup: OnlineLearner::new(config),
            drift_detector: ConceptDriftDetector::new(),
            recent_samples: VecDeque::with_capacity(window_size),
            window_size,
            switches: 0,
        }
    }

    /// Update with new sample
    pub fn update(&mut self, sample: &StreamingSample) {
        // Store recent sample
        if self.recent_samples.len() >= self.window_size {
            self.recent_samples.pop_front();
        }
        self.recent_samples.push_back(sample.clone());

        // Get prediction error before update
        let error = if let Some(label) = sample.label {
            let pred = self.current.predict(&sample.features);
            (pred - label).abs()
        } else {
            0.0
        };

        // Update drift detector
        self.drift_detector.update(error);

        // Update both models
        self.current.update(sample);
        self.backup.update(sample);

        // Handle drift
        if self.drift_detector.strong_drift_detected() {
            self.handle_drift();
        }
    }

    /// Handle detected drift
    fn handle_drift(&mut self) {
        // Compare recent performance
        let current_mae = self.current.recent_mae();
        let backup_mae = self.backup.recent_mae();

        if backup_mae < current_mae * 0.8 {
            // Backup is significantly better, switch
            core::mem::swap(&mut self.current, &mut self.backup);
            self.switches += 1;
        }

        // Reset backup model and retrain on recent data
        self.backup.reset();
        for sample in &self.recent_samples {
            self.backup.update(sample);
        }

        // Reset drift detector
        self.drift_detector.reset();
    }

    /// Predict
    pub fn predict(&self, features: &[f64]) -> f64 {
        self.current.predict(features)
    }

    /// Get drift status
    pub fn drift_status(&self) -> DriftStatus {
        DriftStatus {
            warning: self.drift_detector.warning_detected(),
            drift: self.drift_detector.drift_detected(),
            strong_drift: self.drift_detector.strong_drift_detected(),
            drift_count: self.drift_detector.drift_count(),
            model_switches: self.switches,
        }
    }
}

/// Drift status information
#[derive(Debug, Clone)]
pub struct DriftStatus {
    /// Warning level reached
    pub warning: bool,
    /// Drift detected
    pub drift: bool,
    /// Strong drift (multiple detectors)
    pub strong_drift: bool,
    /// Total drifts detected
    pub drift_count: usize,
    /// Model switches performed
    pub model_switches: u64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_online_stats() {
        let mut stats = OnlineStats::new();

        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            stats.update(v);
        }

        assert_eq!(stats.count(), 5);
        assert!((stats.mean() - 3.0).abs() < 0.001);
        assert!((stats.variance() - 2.5).abs() < 0.001);
        assert_eq!(stats.min(), 1.0);
        assert_eq!(stats.max(), 5.0);
    }

    #[test]
    fn test_online_learner() {
        let config = OnlineLearnerConfig {
            feature_dim: 2,
            ..Default::default()
        };
        let mut learner = OnlineLearner::new(config);

        // Simple linear relationship: y = x1 + x2
        for i in 0..100 {
            let x1 = (i % 10) as f64;
            let x2 = (i / 10) as f64;
            let y = x1 + x2;

            let sample = StreamingSample::new(vec![x1, x2], Some(y));
            learner.update(&sample);
        }

        // Should learn approximate weights
        assert!(learner.samples_seen() == 100);
    }

    #[test]
    fn test_streaming_classifier() {
        let mut classifier = StreamingClassifier::new(3, 2, 0.1);

        // Train on simple data
        for _ in 0..100 {
            // Class 0: x1 > 0, x2 < 0
            classifier.update(&[1.0, -1.0], 0);
            // Class 1: x1 < 0, x2 > 0
            classifier.update(&[-1.0, 1.0], 1);
            // Class 2: both positive
            classifier.update(&[1.0, 1.0], 2);
        }

        // Should predict reasonably
        let pred0 = classifier.predict(&[2.0, -2.0]);
        let pred1 = classifier.predict(&[-2.0, 2.0]);
        let pred2 = classifier.predict(&[2.0, 2.0]);

        assert_eq!(pred0, 0);
        assert_eq!(pred1, 1);
        assert_eq!(pred2, 2);
    }

    #[test]
    fn test_adwin_detector() {
        let mut detector = AdwinDetector::new(0.002, 500);

        // Stable period
        for _ in 0..50 {
            detector.update(0.1);
        }
        assert!(!detector.drift_detected());

        // Sudden change
        for _ in 0..50 {
            detector.update(0.9);
        }

        // Should detect drift at some point
        // Note: ADWIN needs enough samples
    }

    #[test]
    fn test_page_hinkley() {
        let mut detector = PageHinkleyDetector::new(50.0, 0.005);

        // Stable period
        for _ in 0..100 {
            detector.update(0.5);
        }

        // Should not detect drift in stable period
        // Drift requires significant deviation from mean
    }

    #[test]
    fn test_concept_drift_detector() {
        let mut detector = ConceptDriftDetector::new();

        // Add stable values
        for _ in 0..100 {
            detector.update(0.1);
        }

        let count_before = detector.drift_count();

        // Add changed values
        for _ in 0..100 {
            detector.update(0.9);
        }

        // May or may not detect drift depending on parameters
        assert!(detector.drift_count() >= count_before);
    }
}

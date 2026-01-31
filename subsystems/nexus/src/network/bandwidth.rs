//! Bandwidth Predictor
//!
//! Predicts network bandwidth needs.

use alloc::vec::Vec;

use crate::core::NexusTimestamp;

/// Bandwidth sample
#[derive(Debug, Clone, Copy)]
struct BandwidthSample {
    /// Timestamp
    timestamp: u64,
    /// Bandwidth in bytes/sec
    bandwidth: f64,
    /// Active connections
    connections: u32,
}

/// Predicts network bandwidth needs
pub struct BandwidthPredictor {
    /// Historical bandwidth samples
    history: Vec<BandwidthSample>,
    /// Maximum history size
    max_history: usize,
    /// Prediction model weights
    weights: [f64; 8],
    /// Smoothing factor
    alpha: f64,
    /// Current prediction
    current_prediction: f64,
}

impl BandwidthPredictor {
    /// Create new predictor
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            max_history: 1000,
            weights: [0.4, 0.3, 0.15, 0.08, 0.04, 0.02, 0.01, 0.0],
            alpha: 0.3,
            current_prediction: 0.0,
        }
    }

    /// Record bandwidth sample
    pub fn record(&mut self, bandwidth: f64, connections: u32) {
        let sample = BandwidthSample {
            timestamp: NexusTimestamp::now().raw(),
            bandwidth,
            connections,
        };

        self.history.push(sample);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // Update prediction
        self.update_prediction();
    }

    /// Update prediction using exponential smoothing
    fn update_prediction(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let len = self.history.len();
        if len == 1 {
            self.current_prediction = self.history[0].bandwidth;
            return;
        }

        // Weighted moving average with exponential smoothing
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        for (i, weight) in self.weights.iter().enumerate() {
            if i >= len {
                break;
            }
            let sample = &self.history[len - 1 - i];
            weighted_sum += sample.bandwidth * weight;
            weight_sum += weight;
        }

        if weight_sum > 0.0 {
            let new_value = weighted_sum / weight_sum;
            self.current_prediction =
                self.alpha * new_value + (1.0 - self.alpha) * self.current_prediction;
        }
    }

    /// Get current prediction
    pub fn predict(&self) -> f64 {
        self.current_prediction
    }

    /// Predict bandwidth in N seconds
    pub fn predict_ahead(&self, seconds: u32) -> f64 {
        if self.history.len() < 10 {
            return self.current_prediction;
        }

        // Simple linear trend
        let recent = &self.history[self.history.len() - 10..];
        let first = recent[0].bandwidth;
        let last = recent[9].bandwidth;
        let trend = (last - first) / 10.0;

        (self.current_prediction + trend * seconds as f64).max(0.0)
    }

    /// Get prediction accuracy (based on historical predictions)
    pub fn accuracy(&self) -> f64 {
        if self.history.len() < 20 {
            return 0.5;
        }

        let mut errors = 0.0;
        let len = self.history.len();

        for i in 10..len {
            // Use simple average of last 5 as "prediction"
            let pred: f64 = self.history[i - 5..i]
                .iter()
                .map(|s| s.bandwidth)
                .sum::<f64>()
                / 5.0;
            let actual = self.history[i].bandwidth;

            if actual > 0.0 {
                errors += (pred - actual).abs() / actual;
            }
        }

        1.0 - (errors / (len - 10) as f64).min(1.0)
    }
}

impl Default for BandwidthPredictor {
    fn default() -> Self {
        Self::new()
    }
}

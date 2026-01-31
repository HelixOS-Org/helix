//! Grace Period Predictor
//!
//! This module provides prediction capabilities for RCU grace period durations.

use alloc::vec::Vec;
use super::{RcuDomainId, GracePeriodInfo};

/// Historical grace period sample
#[derive(Debug, Clone, Copy)]
pub struct GpSample {
    /// Timestamp
    pub timestamp: u64,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
    /// CPU count at time
    pub cpu_count: u32,
    /// Callback count
    pub callback_count: u64,
    /// Was expedited
    pub expedited: bool,
}

/// Grace period duration predictor
pub struct GracePeriodPredictor {
    /// Domain ID
    domain_id: RcuDomainId,
    /// Historical samples
    samples: Vec<GpSample>,
    /// Maximum samples
    max_samples: usize,
    /// Exponential smoothing alpha
    alpha: f32,
    /// Smoothed normal GP duration
    smoothed_normal_ns: f64,
    /// Smoothed expedited GP duration
    smoothed_expedited_ns: f64,
    /// Per-CPU quiescent state delay estimate
    per_cpu_delay_ns: f64,
    /// Prediction accuracy
    accuracy: f32,
    /// Prediction count
    prediction_count: u64,
    /// Accurate predictions
    accurate_predictions: u64,
}

impl GracePeriodPredictor {
    /// Create new predictor
    pub fn new(domain_id: RcuDomainId) -> Self {
        Self {
            domain_id,
            samples: Vec::with_capacity(256),
            max_samples: 256,
            alpha: 0.2,
            smoothed_normal_ns: 10_000_000.0,   // 10ms default
            smoothed_expedited_ns: 1_000_000.0, // 1ms default
            per_cpu_delay_ns: 1_000_000.0,      // 1ms per CPU
            accuracy: 0.5,
            prediction_count: 0,
            accurate_predictions: 0,
        }
    }

    /// Record completed grace period
    pub fn record_sample(&mut self, gp: &GracePeriodInfo, cpu_count: u32, callback_count: u64) {
        if let Some(duration) = gp.duration_ns() {
            let sample = GpSample {
                timestamp: gp.start_ns,
                duration_ns: duration,
                cpu_count,
                callback_count,
                expedited: gp.expedited,
            };

            // Update exponential smoothing
            if gp.expedited {
                self.smoothed_expedited_ns = self.alpha as f64 * duration as f64
                    + (1.0 - self.alpha as f64) * self.smoothed_expedited_ns;
            } else {
                self.smoothed_normal_ns = self.alpha as f64 * duration as f64
                    + (1.0 - self.alpha as f64) * self.smoothed_normal_ns;
            }

            // Estimate per-CPU delay
            if cpu_count > 0 && !gp.expedited {
                let per_cpu = duration as f64 / cpu_count as f64;
                self.per_cpu_delay_ns =
                    self.alpha as f64 * per_cpu + (1.0 - self.alpha as f64) * self.per_cpu_delay_ns;
            }

            // Add sample
            if self.samples.len() >= self.max_samples {
                self.samples.remove(0);
            }
            self.samples.push(sample);
        }
    }

    /// Predict grace period duration
    pub fn predict_duration(&self, expedited: bool, cpu_count: u32) -> u64 {
        if expedited {
            self.smoothed_expedited_ns as u64
        } else {
            // Use per-CPU model
            let base = self.smoothed_normal_ns;
            let cpu_factor = self.per_cpu_delay_ns * (cpu_count as f64 - 1.0).max(0.0) * 0.1;
            (base + cpu_factor) as u64
        }
    }

    /// Predict if grace period will stall
    pub fn predict_stall(&self, elapsed_ns: u64, expedited: bool, cpu_count: u32) -> f32 {
        let expected = self.predict_duration(expedited, cpu_count);

        if elapsed_ns < expected {
            return 0.0;
        }

        // Calculate probability based on how much we've exceeded expected duration
        let excess_ratio = elapsed_ns as f32 / expected.max(1) as f32;

        // Sigmoid-like function
        let x = excess_ratio - 2.0;
        1.0 / (1.0 + libm::expf(-x))
    }

    /// Update accuracy
    pub fn update_accuracy(&mut self, predicted: u64, actual: u64, threshold_pct: f32) {
        self.prediction_count += 1;

        let error = if predicted > actual {
            predicted - actual
        } else {
            actual - predicted
        };

        let error_pct = error as f32 / actual.max(1) as f32 * 100.0;

        if error_pct <= threshold_pct {
            self.accurate_predictions += 1;
        }

        self.accuracy = self.accurate_predictions as f32 / self.prediction_count as f32;
    }

    /// Get accuracy
    pub fn accuracy(&self) -> f32 {
        self.accuracy
    }

    /// Get domain ID
    pub fn domain_id(&self) -> RcuDomainId {
        self.domain_id
    }

    /// Get sample count
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

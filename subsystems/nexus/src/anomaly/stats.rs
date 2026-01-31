//! Metric statistics for anomaly detection

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::math;

// ============================================================================
// METRIC STATISTICS
// ============================================================================

/// Running statistics for a metric
#[derive(Debug, Clone)]
pub struct MetricStats {
    /// Metric name
    pub name: String,
    /// Values window
    pub values: Vec<f64>,
    /// Window size
    pub window_size: usize,
    /// Sum
    pub sum: f64,
    /// Sum of squares
    pub sum_sq: f64,
    /// Minimum
    pub min: f64,
    /// Maximum
    pub max: f64,
    /// Count
    pub count: usize,
}

impl MetricStats {
    /// Create new metric stats
    pub fn new(name: impl Into<String>, window_size: usize) -> Self {
        Self {
            name: name.into(),
            values: Vec::with_capacity(window_size),
            window_size,
            sum: 0.0,
            sum_sq: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            count: 0,
        }
    }

    /// Add a value
    pub fn add(&mut self, value: f64) {
        // Update running stats
        self.sum += value;
        self.sum_sq += value * value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.count += 1;

        // Add to window
        if self.values.len() >= self.window_size {
            let removed = self.values.remove(0);
            self.sum -= removed;
            self.sum_sq -= removed * removed;
        }
        self.values.push(value);
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    /// Get variance
    pub fn variance(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }

        let mean = self.mean();
        self.values
            .iter()
            .map(|x| math::powi(x - mean, 2))
            .sum::<f64>()
            / (self.values.len() - 1) as f64
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> f64 {
        math::sqrt(self.variance())
    }

    /// Calculate z-score for a value
    pub fn z_score(&self, value: f64) -> f64 {
        let std = self.std_dev();
        if std == 0.0 {
            return 0.0;
        }
        (value - self.mean()) / std
    }

    /// Get quartiles (Q1, Q2, Q3)
    pub fn quartiles(&self) -> (f64, f64, f64) {
        if self.values.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        let n = sorted.len();
        let q1_idx = n / 4;
        let q2_idx = n / 2;
        let q3_idx = 3 * n / 4;

        (sorted[q1_idx], sorted[q2_idx], sorted[q3_idx])
    }

    /// Get IQR (Interquartile Range)
    pub fn iqr(&self) -> f64 {
        let (q1, _, q3) = self.quartiles();
        q3 - q1
    }

    /// Check if value is outlier using IQR method
    pub fn is_iqr_outlier(&self, value: f64, multiplier: f64) -> bool {
        let (q1, _, q3) = self.quartiles();
        let iqr = q3 - q1;
        let lower = q1 - multiplier * iqr;
        let upper = q3 + multiplier * iqr;
        value < lower || value > upper
    }

    /// Get gradient (trend direction)
    pub fn gradient(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }

        let n = self.values.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = self.mean();

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, y) in self.values.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += math::powi(x - x_mean, 2);
        }

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }
}

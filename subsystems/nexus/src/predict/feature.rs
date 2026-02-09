//! Feature tracking for prediction
//!
//! This module provides feature tracking with sliding window history,
//! statistical analysis, and anomaly detection for crash prediction.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::types::Trend;
use crate::math;

/// Feature category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureCategory {
    /// Memory-related
    Memory,
    /// CPU-related
    Cpu,
    /// I/O-related
    Io,
    /// Timing-related
    Timing,
    /// Lock-related
    Lock,
    /// Interrupt-related
    Interrupt,
    /// Scheduler-related
    Scheduler,
    /// Custom
    Custom,
}

/// A feature for prediction
#[derive(Debug, Clone)]
pub struct Feature {
    /// Feature ID
    pub id: u16,
    /// Feature name
    pub name: &'static str,
    /// Feature category
    pub category: FeatureCategory,
    /// Current value
    pub value: f64,
    /// Historical values (sliding window)
    pub history: VecDeque<f64>,
    /// Window size
    pub window_size: usize,
}

impl Feature {
    /// Create a new feature
    pub fn new(id: u16, name: &'static str, category: FeatureCategory, window_size: usize) -> Self {
        Self {
            id,
            name,
            category,
            value: 0.0,
            history: Vec::with_capacity(window_size),
            window_size,
        }
    }

    /// Update feature value
    #[inline]
    pub fn update(&mut self, value: f64) {
        self.value = value;

        if self.history.len() >= self.window_size {
            self.history.pop_front();
        }
        self.history.push_back(value);
    }

    /// Get mean of history
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }

        let mean = self.mean();
        let variance = self
            .history
            .iter()
            .map(|x| math::powi(x - mean, 2))
            .sum::<f64>()
            / (self.history.len() - 1) as f64;

        math::sqrt(variance)
    }

    /// Get gradient (rate of change)
    pub fn gradient(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }

        // Simple linear regression slope
        let n = self.history.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = self.mean();

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, y) in self.history.iter().enumerate() {
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

    /// Get trend
    #[inline(always)]
    pub fn trend(&self) -> Trend {
        Trend::from_gradient(self.gradient())
    }

    /// Get z-score of current value
    #[inline]
    pub fn z_score(&self) -> f64 {
        let std = self.std_dev();
        if std == 0.0 {
            return 0.0;
        }
        (self.value - self.mean()) / std
    }

    /// Is current value anomalous? (|z| > 2)
    #[inline(always)]
    pub fn is_anomalous(&self) -> bool {
        self.z_score().abs() > 2.0
    }
}

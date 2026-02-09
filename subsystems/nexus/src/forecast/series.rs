//! Time series for forecasting.

use alloc::string::String;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::point::TimePoint;
use crate::math;

/// A time series of values
pub struct TimeSeries {
    /// Name
    pub name: String,
    /// Data points
    points: VecDeque<TimePoint>,
    /// Maximum points to keep
    max_points: usize,
}

impl TimeSeries {
    /// Create a new time series
    pub fn new(name: impl Into<String>, max_points: usize) -> Self {
        Self {
            name: name.into(),
            points: VecDeque::new(),
            max_points,
        }
    }

    /// Add a point
    pub fn add(&mut self, point: TimePoint) {
        // Keep sorted by timestamp
        let pos = self
            .points
            .iter()
            .position(|p| p.timestamp > point.timestamp)
            .unwrap_or(self.points.len());
        self.points.insert(pos, point);

        // Enforce max points
        while self.points.len() > self.max_points {
            self.points.pop_front();
        }
    }

    /// Add value with current timestamp
    #[inline(always)]
    pub fn add_now(&mut self, value: f64) {
        self.add(TimePoint::now(value));
    }

    /// Get points
    #[inline(always)]
    pub fn points(&self) -> &[TimePoint] {
        &self.points
    }

    /// Get latest value
    #[inline(always)]
    pub fn latest(&self) -> Option<f64> {
        self.points.back().map(|p| p.value)
    }

    /// Get mean value
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        self.points.iter().map(|p| p.value).sum::<f64>() / self.points.len() as f64
    }

    /// Get trend (slope of linear regression)
    pub fn trend(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }

        // Simple linear regression
        let n = self.points.len() as f64;
        let sum_x: f64 = self.points.iter().map(|p| p.timestamp as f64).sum();
        let sum_y: f64 = self.points.iter().map(|p| p.value).sum();
        let sum_xy: f64 = self
            .points
            .iter()
            .map(|p| p.timestamp as f64 * p.value)
            .sum();
        let sum_xx: f64 = self
            .points
            .iter()
            .map(|p| math::powi(p.timestamp as f64, 2))
            .sum();

        let denominator = n * sum_xx - math::powi(sum_x, 2);
        if denominator.abs() < 1e-10 {
            return 0.0;
        }

        (n * sum_xy - sum_x * sum_y) / denominator
    }

    /// Get min value
    #[inline]
    pub fn min(&self) -> f64 {
        self.points
            .iter()
            .map(|p| p.value)
            .fold(f64::INFINITY, f64::min)
    }

    /// Get max value
    #[inline]
    pub fn max(&self) -> f64 {
        self.points
            .iter()
            .map(|p| p.value)
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance: f64 = self
            .points
            .iter()
            .map(|p| math::powi(p.value - mean, 2))
            .sum::<f64>()
            / (self.points.len() - 1) as f64;
        math::sqrt(variance)
    }

    /// Get length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Is empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Clear
    #[inline(always)]
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

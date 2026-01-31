//! Forecasting engine.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::math;

use super::point::TimePoint;
use super::result::ForecastResult;
use super::series::TimeSeries;

/// Forecasting engine
pub struct Forecaster {
    /// Time series by name
    series: BTreeMap<String, TimeSeries>,
    /// Maximum series to track
    max_series: usize,
    /// Default forecast horizon (ticks)
    default_horizon: u64,
    /// Confidence level (0-1)
    confidence_level: f64,
    /// Exponential smoothing alpha
    ema_alpha: f64,
}

impl Forecaster {
    /// Create a new forecaster
    pub fn new() -> Self {
        Self {
            series: BTreeMap::new(),
            max_series: 100,
            default_horizon: 60 * 1_000_000_000, // 60 seconds
            confidence_level: 0.95,
            ema_alpha: 0.2,
        }
    }

    /// Set forecast horizon
    pub fn with_horizon(mut self, ticks: u64) -> Self {
        self.default_horizon = ticks;
        self
    }

    /// Set confidence level
    pub fn with_confidence(mut self, level: f64) -> Self {
        self.confidence_level = level.clamp(0.5, 0.99);
        self
    }

    /// Record a value
    pub fn record(&mut self, metric: &str, value: f64) {
        let series = self
            .series
            .entry(metric.into())
            .or_insert_with(|| TimeSeries::new(metric, 1000));
        series.add_now(value);
    }

    /// Record a value with timestamp
    pub fn record_at(&mut self, metric: &str, timestamp: u64, value: f64) {
        let series = self
            .series
            .entry(metric.into())
            .or_insert_with(|| TimeSeries::new(metric, 1000));
        series.add(TimePoint::new(timestamp, value));
    }

    /// Forecast a metric
    pub fn forecast(&self, metric: &str, steps: usize) -> Option<ForecastResult> {
        let series = self.series.get(metric)?;

        if series.len() < 10 {
            return None; // Not enough data
        }

        let mut result = ForecastResult::new(metric);

        // Calculate trend
        let trend = series.trend();
        let mean = series.mean();
        let std_dev = series.std_dev();

        // Calculate trend strength (R-squared approximation)
        let trend_strength = if std_dev > 0.0 {
            (trend.abs() * math::sqrt(series.len() as f64) / std_dev).min(1.0)
        } else {
            1.0
        };

        result = result.with_trend(trend, trend_strength);

        // Get latest timestamp and value
        let latest = series.points().last()?;
        let interval = if series.len() >= 2 {
            let points = series.points();
            (points.last()?.timestamp - points.first()?.timestamp) / (series.len() - 1) as u64
        } else {
            1_000_000_000 // 1 second default
        };

        // Generate forecast using exponential smoothing with trend
        let mut forecast_value = latest.value;
        let z_score = 1.96; // ~95% confidence

        for i in 1..=steps {
            let timestamp = latest.timestamp + (interval * i as u64);

            // Simple trend extrapolation with dampening
            forecast_value += trend * interval as f64 * math::powi(0.9_f64, i as i32);

            // Confidence interval widens with time
            let uncertainty = std_dev * z_score * math::sqrt(i as f64);
            let confidence_low = forecast_value - uncertainty;
            let confidence_high = forecast_value + uncertainty;

            result.add_value(timestamp, forecast_value, confidence_low, confidence_high);
        }

        Some(result)
    }

    /// Forecast time to threshold
    pub fn time_to_threshold(&self, metric: &str, threshold: f64) -> Option<u64> {
        let series = self.series.get(metric)?;

        if series.len() < 10 {
            return None;
        }

        let latest = series.points().last()?;
        let trend = series.trend();

        // Check if trending towards threshold
        let distance = threshold - latest.value;

        if trend.abs() < 1e-10 {
            return None; // No trend
        }

        // Check direction
        if (distance > 0.0 && trend <= 0.0) || (distance < 0.0 && trend >= 0.0) {
            return None; // Moving away from threshold
        }

        // Estimate time
        let time = (distance / trend).abs() as u64;
        Some(time)
    }

    /// Forecast time to exhaustion (when value reaches 100%)
    pub fn time_to_exhaustion(&self, metric: &str, capacity: f64) -> Option<u64> {
        self.time_to_threshold(metric, capacity)
    }

    /// Get trend for a metric
    pub fn get_trend(&self, metric: &str) -> Option<f64> {
        self.series.get(metric).map(|s| s.trend())
    }

    /// Get current value for a metric
    pub fn current(&self, metric: &str) -> Option<f64> {
        self.series.get(metric).and_then(|s| s.latest())
    }

    /// Get series for a metric
    pub fn get_series(&self, metric: &str) -> Option<&TimeSeries> {
        self.series.get(metric)
    }

    /// Get all metrics
    pub fn metrics(&self) -> Vec<&str> {
        self.series.keys().map(|s| s.as_str()).collect()
    }

    /// Clear a metric
    pub fn clear(&mut self, metric: &str) {
        self.series.remove(metric);
    }

    /// Clear all metrics
    pub fn clear_all(&mut self) {
        self.series.clear();
    }
}

impl Default for Forecaster {
    fn default() -> Self {
        Self::new()
    }
}

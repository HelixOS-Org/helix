//! Forecast result types.

use alloc::string::String;
use alloc::vec::Vec;

use super::point::TimePoint;

/// Result of a forecast
#[derive(Debug, Clone)]
pub struct ForecastResult {
    /// Metric name
    pub metric: String,
    /// Forecasted values
    pub values: Vec<TimePoint>,
    /// Confidence interval (lower, upper) for each value
    pub confidence: Vec<(f64, f64)>,
    /// Trend direction (positive = increasing)
    pub trend: f64,
    /// Trend strength (0-1)
    pub trend_strength: f64,
    /// Time to exhaustion (if applicable)
    pub time_to_exhaustion: Option<u64>,
    /// Time to threshold breach (if applicable)
    pub time_to_threshold: Option<u64>,
}

impl ForecastResult {
    /// Create a new forecast result
    pub fn new(metric: impl Into<String>) -> Self {
        Self {
            metric: metric.into(),
            values: Vec::new(),
            confidence: Vec::new(),
            trend: 0.0,
            trend_strength: 0.0,
            time_to_exhaustion: None,
            time_to_threshold: None,
        }
    }

    /// Add a forecasted value
    pub fn add_value(
        &mut self,
        timestamp: u64,
        value: f64,
        confidence_low: f64,
        confidence_high: f64,
    ) {
        self.values.push(TimePoint::new(timestamp, value));
        self.confidence.push((confidence_low, confidence_high));
    }

    /// Set trend
    pub fn with_trend(mut self, trend: f64, strength: f64) -> Self {
        self.trend = trend;
        self.trend_strength = strength.clamp(0.0, 1.0);
        self
    }

    /// Get forecasted value at a specific time
    pub fn value_at(&self, timestamp: u64) -> Option<f64> {
        // Find closest point
        self.values
            .iter()
            .min_by_key(|p| (p.timestamp as i64 - timestamp as i64).abs())
            .map(|p| p.value)
    }
}

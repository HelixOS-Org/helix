//! # Resource Forecasting
//!
//! Predict future resource usage and requirements.
//!
//! ## Key Features
//!
//! - **Time Series Forecasting**: Predict future values
//! - **Trend Analysis**: Detect upward/downward trends
//! - **Seasonality Detection**: Handle periodic patterns
//! - **Capacity Planning**: Predict when resources exhaust

#![allow(dead_code)]

extern crate alloc;

mod forecaster;
mod point;
mod resource;
mod result;
mod series;

// Re-export point
pub use point::TimePoint;

// Re-export series
pub use series::TimeSeries;

// Re-export result
pub use result::ForecastResult;

// Re-export forecaster
pub use forecaster::Forecaster;

// Re-export resource types
pub use resource::{ForecastSeverity, ResourceForecast, ResourceForecaster};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series() {
        let mut series = TimeSeries::new("test", 100);

        for i in 0..10 {
            series.add(TimePoint::new(i * 1000, i as f64));
        }

        assert_eq!(series.len(), 10);
        assert_eq!(series.latest(), Some(9.0));
        assert!(series.trend() > 0.0); // Positive trend
    }

    #[test]
    fn test_forecaster() {
        let mut forecaster = Forecaster::new();

        // Add increasing values
        for i in 0..100 {
            forecaster.record_at("test", i * 1000, i as f64);
        }

        let forecast = forecaster.forecast("test", 10);
        assert!(forecast.is_some());

        let forecast = forecast.unwrap();
        assert!(forecast.trend > 0.0);
        assert!(!forecast.values.is_empty());
    }

    #[test]
    fn test_time_to_threshold() {
        let mut forecaster = Forecaster::new();

        // Add increasing values
        for i in 0..100 {
            forecaster.record_at("test", i * 1000, i as f64);
        }

        // Should be able to predict when we hit 150
        let time = forecaster.time_to_threshold("test", 150.0);
        assert!(time.is_some());
    }

    #[test]
    fn test_resource_forecaster() {
        let mut forecaster = ResourceForecaster::new();

        forecaster.set_capacity("memory", 100.0);
        forecaster.set_threshold("memory", 80.0);

        for i in 0..100 {
            forecaster.record("memory", 50.0 + (i as f64 * 0.3));
        }

        let forecast = forecaster.forecast("memory", 10);
        assert!(forecast.is_some());

        let forecast = forecast.unwrap();
        assert!(forecast.severity <= ForecastSeverity::Critical);
    }
}

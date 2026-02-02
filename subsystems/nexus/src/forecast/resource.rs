//! Resource-specific forecasting.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::forecaster::Forecaster;
use super::result::ForecastResult;

/// Specialized forecaster for resources
pub struct ResourceForecaster {
    /// Base forecaster
    forecaster: Forecaster,
    /// Thresholds by resource
    thresholds: BTreeMap<String, f64>,
    /// Capacities by resource
    capacities: BTreeMap<String, f64>,
}

impl ResourceForecaster {
    /// Create a new resource forecaster
    pub fn new() -> Self {
        Self {
            forecaster: Forecaster::new(),
            thresholds: BTreeMap::new(),
            capacities: BTreeMap::new(),
        }
    }

    /// Set threshold for a resource
    pub fn set_threshold(&mut self, resource: &str, threshold: f64) {
        self.thresholds.insert(resource.into(), threshold);
    }

    /// Set capacity for a resource
    pub fn set_capacity(&mut self, resource: &str, capacity: f64) {
        self.capacities.insert(resource.into(), capacity);
    }

    /// Record resource usage
    pub fn record(&mut self, resource: &str, usage: f64) {
        self.forecaster.record(resource, usage);
    }

    /// Get forecast with threshold analysis
    pub fn forecast(&self, resource: &str, steps: usize) -> Option<ResourceForecast> {
        let forecast = self.forecaster.forecast(resource, steps)?;
        let threshold = self.thresholds.get(resource).copied();
        let capacity = self.capacities.get(resource).copied();

        let time_to_threshold =
            threshold.and_then(|t| self.forecaster.time_to_threshold(resource, t));

        let time_to_exhaustion =
            capacity.and_then(|c| self.forecaster.time_to_exhaustion(resource, c));

        Some(ResourceForecast {
            resource: resource.into(),
            forecast,
            threshold,
            capacity,
            time_to_threshold,
            time_to_exhaustion,
            severity: self.calculate_severity(time_to_threshold, time_to_exhaustion),
        })
    }

    /// Calculate severity based on time remaining
    fn calculate_severity(
        &self,
        time_to_threshold: Option<u64>,
        time_to_exhaustion: Option<u64>,
    ) -> ForecastSeverity {
        let min_time = match (time_to_threshold, time_to_exhaustion) {
            (Some(t), Some(e)) => Some(t.min(e)),
            (Some(t), None) => Some(t),
            (None, Some(e)) => Some(e),
            (None, None) => None,
        };

        match min_time {
            None => ForecastSeverity::Stable,
            Some(t) if t > 3600 * 1_000_000_000 => ForecastSeverity::Stable, // > 1 hour
            Some(t) if t > 600 * 1_000_000_000 => ForecastSeverity::Warning, // > 10 min
            Some(t) if t > 60 * 1_000_000_000 => ForecastSeverity::Concern,  // > 1 min
            Some(t) if t > 10 * 1_000_000_000 => ForecastSeverity::Urgent,   // > 10 sec
            Some(_) => ForecastSeverity::Critical,
        }
    }

    /// Get all resource forecasts
    pub fn forecast_all(&self, steps: usize) -> Vec<ResourceForecast> {
        self.forecaster
            .metrics()
            .iter()
            .filter_map(|m| self.forecast(m, steps))
            .collect()
    }

    /// Get critical forecasts
    pub fn critical_forecasts(&self, steps: usize) -> Vec<ResourceForecast> {
        self.forecast_all(steps)
            .into_iter()
            .filter(|f| f.severity >= ForecastSeverity::Urgent)
            .collect()
    }
}

impl Default for ResourceForecaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource forecast result
#[derive(Debug, Clone)]
pub struct ResourceForecast {
    /// Resource name
    pub resource: String,
    /// Base forecast
    pub forecast: ForecastResult,
    /// Threshold (if set)
    pub threshold: Option<f64>,
    /// Capacity (if set)
    pub capacity: Option<f64>,
    /// Time to threshold breach
    pub time_to_threshold: Option<u64>,
    /// Time to exhaustion
    pub time_to_exhaustion: Option<u64>,
    /// Severity
    pub severity: ForecastSeverity,
}

/// Forecast severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ForecastSeverity {
    /// Stable, no issues expected
    Stable   = 0,
    /// Warning, issues may occur in the future
    Warning  = 1,
    /// Concern, issues expected soon
    Concern  = 2,
    /// Urgent, issues imminent
    Urgent   = 3,
    /// Critical, immediate action required
    Critical = 4,
}

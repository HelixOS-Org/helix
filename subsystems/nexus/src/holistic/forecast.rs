//! # Holistic Forecasting Engine
//!
//! System-wide resource demand forecasting:
//! - Multi-variate time series forecasting
//! - Seasonality detection
//! - Trend decomposition
//! - Capacity planning
//! - What-if scenario analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// FORECAST TYPES
// ============================================================================

/// Forecastable metric
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ForecastMetric {
    /// CPU demand
    CpuDemand,
    /// Memory demand
    MemoryDemand,
    /// I/O demand
    IoDemand,
    /// Network demand
    NetDemand,
    /// Process count
    ProcessCount,
    /// Power consumption
    PowerDemand,
    /// Temperature
    Temperature,
}

/// Forecast horizon
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticForecastHorizon {
    /// Short (seconds)
    Short,
    /// Medium (minutes)
    Medium,
    /// Long (hours)
    Long,
    /// VeryLong (days)
    VeryLong,
}

impl HolisticForecastHorizon {
    /// Steps ahead
    #[inline]
    pub fn steps(&self) -> usize {
        match self {
            Self::Short => 10,
            Self::Medium => 60,
            Self::Long => 360,
            Self::VeryLong => 1440,
        }
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    /// Increasing
    Increasing,
    /// Stable
    Stable,
    /// Decreasing
    Decreasing,
}

// ============================================================================
// FORECAST SERIES
// ============================================================================

/// Time series with decomposition
#[derive(Debug, Clone)]
pub struct ForecastSeries {
    /// Metric
    pub metric: ForecastMetric,
    /// Raw values
    values: VecDeque<f64>,
    /// Max length
    max_len: usize,
    /// Smoothed values (exponential smoothing)
    level: f64,
    /// Trend component
    trend: f64,
    /// Smoothing params
    alpha: f64,
    beta: f64,
}

impl ForecastSeries {
    pub fn new(metric: ForecastMetric) -> Self {
        Self {
            metric,
            values: VecDeque::new(),
            max_len: 1024,
            level: 0.0,
            trend: 0.0,
            alpha: 0.3,
            beta: 0.1,
        }
    }

    /// Add observation
    #[inline]
    pub fn observe(&mut self, value: f64) {
        if self.values.is_empty() {
            self.level = value;
            self.trend = 0.0;
        } else {
            let prev_level = self.level;
            self.level = self.alpha * value + (1.0 - self.alpha) * (self.level + self.trend);
            self.trend = self.beta * (self.level - prev_level) + (1.0 - self.beta) * self.trend;
        }

        self.values.push_back(value);
        if self.values.len() > self.max_len {
            self.values.pop_front();
        }
    }

    /// Forecast k steps ahead
    #[inline]
    pub fn forecast(&self, steps: usize) -> Vec<f64> {
        let mut result = Vec::with_capacity(steps);
        for i in 1..=steps {
            result.push(self.level + self.trend * i as f64);
        }
        result
    }

    /// Current trend direction
    #[inline]
    pub fn trend_direction(&self) -> TrendDirection {
        if self.trend > 0.01 {
            TrendDirection::Increasing
        } else if self.trend < -0.01 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }

    /// Mean
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    /// Detect seasonality period using autocorrelation
    pub fn detect_seasonality(&self) -> Option<usize> {
        if self.values.len() < 20 {
            return None;
        }

        let mean = self.mean();
        let n = self.values.len();

        // Compute variance
        let var: f64 = self.values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / n as f64;
        if var < 1e-10 {
            return None;
        }

        let mut best_lag = 0;
        let mut best_acf = 0.0;

        for lag in 2..n / 3 {
            let mut acf = 0.0;
            for i in 0..n - lag {
                acf += (self.values[i] - mean) * (self.values[i + lag] - mean);
            }
            acf /= (n - lag) as f64 * var;

            if acf > best_acf && acf > 0.3 {
                best_acf = acf;
                best_lag = lag;
            }
        }

        if best_lag > 0 {
            Some(best_lag)
        } else {
            None
        }
    }
}

// ============================================================================
// FORECAST RESULT
// ============================================================================

/// Forecast result
#[derive(Debug, Clone)]
pub struct ForecastResult {
    /// Metric
    pub metric: ForecastMetric,
    /// Horizon
    pub horizon: HolisticForecastHorizon,
    /// Predicted values
    pub predictions: Vec<f64>,
    /// Confidence interval low
    pub ci_low: Vec<f64>,
    /// Confidence interval high
    pub ci_high: Vec<f64>,
    /// Trend
    pub trend: TrendDirection,
    /// Seasonality period (if detected)
    pub seasonality: Option<usize>,
}

/// Scenario for what-if analysis
#[derive(Debug, Clone)]
pub struct ForecastScenario {
    /// Scenario name code
    pub code: u32,
    /// Metric overrides
    pub overrides: BTreeMap<u8, f64>,
    /// Scale factor
    pub scale_factor: f64,
}

impl ForecastScenario {
    pub fn new(code: u32) -> Self {
        Self {
            code,
            overrides: BTreeMap::new(),
            scale_factor: 1.0,
        }
    }

    /// Set override
    #[inline(always)]
    pub fn set_override(&mut self, metric: ForecastMetric, value: f64) {
        self.overrides.insert(metric as u8, value);
    }
}

// ============================================================================
// CAPACITY PLAN
// ============================================================================

/// Capacity planning result
#[derive(Debug, Clone)]
pub struct CapacityPlan {
    /// Metric
    pub metric: ForecastMetric,
    /// Current capacity
    pub current_capacity: f64,
    /// Predicted peak demand
    pub predicted_peak: f64,
    /// Time to capacity (steps)
    pub time_to_capacity: Option<usize>,
    /// Recommended additional capacity
    pub recommended_additional: f64,
    /// Risk level
    pub risk: CapacityRisk,
}

/// Capacity risk
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityRisk {
    /// Low
    Low,
    /// Medium
    Medium,
    /// High
    High,
    /// Critical
    Critical,
}

// ============================================================================
// FORECASTING ENGINE
// ============================================================================

/// Forecast engine stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticForecastStats {
    /// Series tracked
    pub series_count: usize,
    /// Forecasts generated
    pub forecasts_generated: u64,
    /// Seasonalities detected
    pub seasonalities: usize,
    /// Capacity warnings
    pub capacity_warnings: usize,
}

/// Holistic forecasting engine
pub struct HolisticForecastEngine {
    /// Forecast series
    series: BTreeMap<u8, ForecastSeries>,
    /// Capacity limits
    capacity_limits: BTreeMap<u8, f64>,
    /// Stats
    stats: HolisticForecastStats,
}

impl HolisticForecastEngine {
    pub fn new() -> Self {
        Self {
            series: BTreeMap::new(),
            capacity_limits: BTreeMap::new(),
            stats: HolisticForecastStats::default(),
        }
    }

    /// Set capacity limit
    #[inline(always)]
    pub fn set_capacity(&mut self, metric: ForecastMetric, capacity: f64) {
        self.capacity_limits.insert(metric as u8, capacity);
    }

    /// Observe metric
    #[inline]
    pub fn observe(&mut self, metric: ForecastMetric, value: f64) {
        let series = self
            .series
            .entry(metric as u8)
            .or_insert_with(|| ForecastSeries::new(metric));
        series.observe(value);
        self.stats.series_count = self.series.len();
    }

    /// Generate forecast
    pub fn forecast(
        &mut self,
        metric: ForecastMetric,
        horizon: HolisticForecastHorizon,
    ) -> Option<ForecastResult> {
        let series = self.series.get(&(metric as u8))?;
        let steps = horizon.steps();
        let predictions = series.forecast(steps);

        // Confidence interval (± 2 * std_dev, widening)
        let std = series.values.iter().zip(predictions.iter()).count(); // just to get count
        let base_std = if series.values.len() > 10 {
            let last10 = &series.values[series.values.len() - 10..];
            let mean: f64 = last10.iter().sum::<f64>() / 10.0;
            let var: f64 = last10.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / 10.0;
            libm::sqrt(var)
        } else {
            series.mean() * 0.1
        };
        let _ = std;

        let ci_low: Vec<f64> = predictions
            .iter()
            .enumerate()
            .map(|(i, &p)| p - 2.0 * base_std * libm::sqrt((i + 1) as f64))
            .collect();
        let ci_high: Vec<f64> = predictions
            .iter()
            .enumerate()
            .map(|(i, &p)| p + 2.0 * base_std * libm::sqrt((i + 1) as f64))
            .collect();

        let seasonality = series.detect_seasonality();

        self.stats.forecasts_generated += 1;
        if seasonality.is_some() {
            self.stats.seasonalities = self
                .series
                .values()
                .filter(|s| s.detect_seasonality().is_some())
                .count();
        }

        Some(ForecastResult {
            metric,
            horizon,
            predictions,
            ci_low,
            ci_high,
            trend: series.trend_direction(),
            seasonality,
        })
    }

    /// Capacity planning
    pub fn capacity_plan(&self, metric: ForecastMetric) -> Option<CapacityPlan> {
        let series = self.series.get(&(metric as u8))?;
        let capacity = self.capacity_limits.get(&(metric as u8)).copied()?;

        let predictions = series.forecast(100);
        let predicted_peak = predictions
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, |a, b| if a > b { a } else { b });

        let time_to_capacity = predictions.iter().position(|&p| p >= capacity);

        let recommended = if predicted_peak > capacity * 0.8 {
            predicted_peak - capacity * 0.8
        } else {
            0.0
        };

        let risk = if time_to_capacity.map(|t| t < 10).unwrap_or(false) {
            CapacityRisk::Critical
        } else if time_to_capacity.map(|t| t < 30).unwrap_or(false) {
            CapacityRisk::High
        } else if predicted_peak > capacity * 0.8 {
            CapacityRisk::Medium
        } else {
            CapacityRisk::Low
        };

        Some(CapacityPlan {
            metric,
            current_capacity: capacity,
            predicted_peak,
            time_to_capacity,
            recommended_additional: recommended,
            risk,
        })
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticForecastStats {
        &self.stats
    }
}

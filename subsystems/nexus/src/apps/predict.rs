//! # Application Behavior Prediction
//!
//! Predicts future resource needs and behavior phases for applications,
//! enabling proactive resource allocation.

use alloc::collections::VecDeque;

// ============================================================================
// FORECAST TYPES
// ============================================================================

/// How far ahead to forecast
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastHorizon {
    /// 1-10 seconds ahead
    Short,
    /// 10-60 seconds ahead
    Medium,
    /// 1-5 minutes ahead
    Long,
    /// 5-60 minutes ahead
    Extended,
}

impl ForecastHorizon {
    /// Number of seconds this horizon represents
    #[inline]
    pub fn seconds(&self) -> u64 {
        match self {
            Self::Short => 5,
            Self::Medium => 30,
            Self::Long => 180,
            Self::Extended => 1800,
        }
    }
}

/// A predicted resource value
#[derive(Debug, Clone)]
pub struct ResourceForecast {
    /// Predicted value
    pub predicted_value: f64,
    /// Lower bound (95% confidence interval)
    pub lower_bound: f64,
    /// Upper bound (95% confidence interval)
    pub upper_bound: f64,
    /// Confidence in the prediction (0.0 - 1.0)
    pub confidence: f64,
    /// Horizon
    pub horizon: ForecastHorizon,
}

/// Predicted phase transition
#[derive(Debug, Clone)]
pub struct PhasePrediction {
    /// The predicted next phase
    pub phase: &'static str,
    /// Time until transition (ms)
    pub time_until_ms: u64,
    /// Confidence
    pub confidence: f64,
}

/// Complete behavior forecast
#[derive(Debug, Clone)]
pub struct BehaviorForecast {
    /// CPU forecast
    pub cpu: ResourceForecast,
    /// Memory forecast
    pub memory: ResourceForecast,
    /// I/O throughput forecast
    pub io: ResourceForecast,
    /// Network throughput forecast
    pub network: ResourceForecast,
    /// Phase prediction
    pub phase: Option<PhasePrediction>,
    /// Anomaly risk (0.0 - 1.0)
    pub anomaly_risk: f64,
}

// ============================================================================
// WORKLOAD PREDICTOR
// ============================================================================

/// The prediction engine — uses exponential moving averages and trend
/// analysis to forecast application behavior.
pub struct WorkloadPredictor {
    /// CPU usage history
    cpu_history: VecDeque<f64>,
    /// Memory usage history
    memory_history: VecDeque<f64>,
    /// I/O throughput history
    io_history: VecDeque<f64>,
    /// Network throughput history
    network_history: VecDeque<f64>,
    /// Maximum history length
    max_history: usize,
    /// EMA smoothing factor
    alpha: f64,
}

impl WorkloadPredictor {
    pub fn new(max_history: usize) -> Self {
        Self {
            cpu_history: VecDeque::with_capacity(max_history),
            memory_history: VecDeque::with_capacity(max_history),
            io_history: VecDeque::with_capacity(max_history),
            network_history: VecDeque::with_capacity(max_history),
            max_history,
            alpha: 0.3,
        }
    }

    /// Observe CPU usage
    #[inline(always)]
    pub fn observe_cpu(&mut self, usage: f64) {
        Self::push_bounded(&mut self.cpu_history, usage, self.max_history);
    }

    /// Observe memory usage
    #[inline(always)]
    pub fn observe_memory(&mut self, usage: f64) {
        Self::push_bounded(&mut self.memory_history, usage, self.max_history);
    }

    /// Observe I/O throughput
    #[inline(always)]
    pub fn observe_io(&mut self, throughput: f64) {
        Self::push_bounded(&mut self.io_history, throughput, self.max_history);
    }

    /// Observe network throughput
    #[inline(always)]
    pub fn observe_network(&mut self, throughput: f64) {
        Self::push_bounded(&mut self.network_history, throughput, self.max_history);
    }

    /// Predict CPU usage at the given horizon
    #[inline(always)]
    pub fn predict_cpu(&mut self, horizon: ForecastHorizon) -> ResourceForecast {
        self.cpu_history.make_contiguous();
        self.forecast(self.cpu_history.as_slices().0, horizon)
    }

    /// Predict memory usage at the given horizon
    #[inline(always)]
    pub fn predict_memory(&mut self, horizon: ForecastHorizon) -> ResourceForecast {
        self.memory_history.make_contiguous();
        self.forecast(self.memory_history.as_slices().0, horizon)
    }

    /// Predict I/O throughput at the given horizon
    #[inline(always)]
    pub fn predict_io(&mut self, horizon: ForecastHorizon) -> ResourceForecast {
        self.io_history.make_contiguous();
        self.forecast(self.io_history.as_slices().0, horizon)
    }

    /// Predict network throughput at the given horizon
    #[inline(always)]
    pub fn predict_network(&mut self, horizon: ForecastHorizon) -> ResourceForecast {
        self.network_history.make_contiguous();
        self.forecast(self.network_history.as_slices().0, horizon)
    }

    /// Complete behavior forecast
    pub fn forecast_all(&mut self, horizon: ForecastHorizon) -> BehaviorForecast {
        self.cpu_history.make_contiguous();
        self.memory_history.make_contiguous();
        self.io_history.make_contiguous();
        self.network_history.make_contiguous();

        let cpu = self.forecast(self.cpu_history.as_slices().0, horizon);
        let memory = self.forecast(self.memory_history.as_slices().0, horizon);
        let io = self.forecast(self.io_history.as_slices().0, horizon);
        let network = self.forecast(self.network_history.as_slices().0, horizon);

        // Compute anomaly risk based on trend acceleration
        let cpu_accel = self.trend_acceleration(self.cpu_history.as_slices().0);
        let mem_accel = self.trend_acceleration(self.memory_history.as_slices().0);
        let anomaly_risk = ((cpu_accel.abs() + mem_accel.abs()) / 2.0).min(1.0);

        BehaviorForecast {
            cpu,
            memory,
            io,
            network,
            phase: None,
            anomaly_risk,
        }
    }

    /// Forecast for a single resource dimension
    fn forecast(&self, history: &[f64], horizon: ForecastHorizon) -> ResourceForecast {
        if history.len() < 3 {
            return ResourceForecast {
                predicted_value: history.last().copied().unwrap_or(0.0),
                lower_bound: 0.0,
                upper_bound: 1.0,
                confidence: 0.1,
                horizon,
            };
        }

        // Compute EMA
        let ema = self.compute_ema(history);

        // Compute trend (slope of recent values)
        let trend = self.compute_trend(history);

        // Project forward based on horizon
        let steps_ahead = match horizon {
            ForecastHorizon::Short => 1.0,
            ForecastHorizon::Medium => 6.0,
            ForecastHorizon::Long => 36.0,
            ForecastHorizon::Extended => 360.0,
        };

        let predicted = ema + trend * steps_ahead;

        // Confidence interval based on variance
        let variance = self.compute_variance(history);
        let stddev = libm::sqrt(variance);
        let ci_width = 1.96 * stddev * libm::sqrt(steps_ahead);

        // Confidence degrades with horizon
        let base_confidence = if history.len() > 20 { 0.85 } else { 0.5 };
        let confidence = base_confidence
            * match horizon {
                ForecastHorizon::Short => 0.95,
                ForecastHorizon::Medium => 0.80,
                ForecastHorizon::Long => 0.60,
                ForecastHorizon::Extended => 0.40,
            };

        ResourceForecast {
            predicted_value: predicted.max(0.0),
            lower_bound: (predicted - ci_width).max(0.0),
            upper_bound: predicted + ci_width,
            confidence,
            horizon,
        }
    }

    /// Exponential moving average
    fn compute_ema(&self, history: &[f64]) -> f64 {
        let mut ema = history[0];
        for &val in &history[1..] {
            ema = self.alpha * val + (1.0 - self.alpha) * ema;
        }
        ema
    }

    /// Linear trend (slope) using least squares on recent values
    fn compute_trend(&self, history: &[f64]) -> f64 {
        let n = history.len().min(20);
        let recent = &history[history.len() - n..];

        let n_f = n as f64;
        let sum_x: f64 = (0..n).map(|i| i as f64).sum();
        let sum_y: f64 = recent.iter().sum();
        let sum_xy: f64 = recent.iter().enumerate().map(|(i, y)| i as f64 * y).sum();
        let sum_xx: f64 = (0..n).map(|i| (i * i) as f64).sum();

        let denominator = n_f * sum_xx - sum_x * sum_x;
        if denominator.abs() < f64::EPSILON {
            return 0.0;
        }

        (n_f * sum_xy - sum_x * sum_y) / denominator
    }

    /// Variance of the series
    fn compute_variance(&self, history: &[f64]) -> f64 {
        if history.len() < 2 {
            return 0.0;
        }
        let mean = history.iter().sum::<f64>() / history.len() as f64;
        let sum_sq: f64 = history.iter().map(|x| (x - mean) * (x - mean)).sum();
        sum_sq / (history.len() - 1) as f64
    }

    /// Trend acceleration (second derivative)
    fn trend_acceleration(&self, history: &[f64]) -> f64 {
        if history.len() < 6 {
            return 0.0;
        }
        let mid = history.len() / 2;
        let first_half = &history[..mid];
        let second_half = &history[mid..];

        let trend1 = self.compute_trend(first_half);
        let trend2 = self.compute_trend(second_half);

        trend2 - trend1
    }

    fn push_bounded(deque: &mut VecDeque<f64>, value: f64, max: usize) {
        if deque.len() >= max {
            deque.pop_front();
        }
        deque.push_back(value);
    }
}

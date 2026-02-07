//! # Holistic Prediction Engine
//!
//! Uses system-wide trends and cross-subsystem correlations to predict
//! future system states, enabling proactive optimization.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::global::{BottleneckType, SystemSnapshot};

// ============================================================================
// TREND ANALYSIS
// ============================================================================

/// A time-series sample
#[derive(Debug, Clone, Copy)]
struct TrendSample {
    timestamp: u64,
    value: f64,
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
    Volatile,
}

/// Tracks a metric over time
pub struct MetricTrend {
    samples: VecDeque<TrendSample>,
    max_samples: usize,
    ema: f64,
    ema_alpha: f64,
}

impl MetricTrend {
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: VecDeque::new(),
            max_samples,
            ema: 0.0,
            ema_alpha: 0.3,
        }
    }

    /// Record a sample
    pub fn record(&mut self, value: f64, timestamp: u64) {
        if self.samples.is_empty() {
            self.ema = value;
        } else {
            self.ema = self.ema_alpha * value + (1.0 - self.ema_alpha) * self.ema;
        }
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(TrendSample { timestamp, value });
    }

    /// Current EMA value
    pub fn current(&self) -> f64 {
        self.ema
    }

    /// Determine trend direction
    pub fn direction(&self) -> TrendDirection {
        if self.samples.len() < 3 {
            return TrendDirection::Stable;
        }

        let n = self.samples.len();
        let recent = &self.samples.as_slices().0;
        let recent = if recent.len() >= n {
            recent
        } else {
            // Fallback: use individual access
            return self.direction_from_deque();
        };

        let half = n / 2;
        let first_half_avg: f64 =
            recent[..half].iter().map(|s| s.value).sum::<f64>() / half as f64;
        let second_half_avg: f64 =
            recent[half..].iter().map(|s| s.value).sum::<f64>() / (n - half) as f64;

        let diff = second_half_avg - first_half_avg;
        let threshold = 0.05 * first_half_avg.abs().max(0.01);

        // Check volatility
        let variance: f64 = recent
            .iter()
            .map(|s| {
                let d = s.value - self.ema;
                d * d
            })
            .sum::<f64>()
            / n as f64;
        let std_dev = libm::sqrt(variance);
        if std_dev > 0.3 * self.ema.abs().max(0.01) {
            return TrendDirection::Volatile;
        }

        if diff > threshold {
            TrendDirection::Rising
        } else if diff < -threshold {
            TrendDirection::Falling
        } else {
            TrendDirection::Stable
        }
    }

    fn direction_from_deque(&self) -> TrendDirection {
        let n = self.samples.len();
        if n < 3 {
            return TrendDirection::Stable;
        }
        let half = n / 2;
        let mut first_sum = 0.0;
        let mut second_sum = 0.0;
        for (i, s) in self.samples.iter().enumerate() {
            if i < half {
                first_sum += s.value;
            } else {
                second_sum += s.value;
            }
        }
        let first_avg = first_sum / half as f64;
        let second_avg = second_sum / (n - half) as f64;
        let diff = second_avg - first_avg;
        let threshold = 0.05 * first_avg.abs().max(0.01);

        if diff > threshold {
            TrendDirection::Rising
        } else if diff < -threshold {
            TrendDirection::Falling
        } else {
            TrendDirection::Stable
        }
    }

    /// Predict the value at a future timestamp using linear extrapolation
    pub fn predict(&self, future_timestamp: u64) -> Option<f64> {
        if self.samples.len() < 2 {
            return None;
        }

        // Simple linear regression
        let n = self.samples.len() as f64;
        let mut sum_x = 0.0f64;
        let mut sum_y = 0.0f64;
        let mut sum_xy = 0.0f64;
        let mut sum_x2 = 0.0f64;

        for s in &self.samples {
            let x = s.timestamp as f64;
            sum_x += x;
            sum_y += s.value;
            sum_xy += x * s.value;
            sum_x2 += x * x;
        }

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-10 {
            return Some(self.ema);
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / n;

        Some(slope * future_timestamp as f64 + intercept)
    }

    /// Sample count
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

// ============================================================================
// SYSTEM PREDICTOR
// ============================================================================

/// Predicted future system state
#[derive(Debug, Clone)]
pub struct SystemPrediction {
    /// Predicted overall pressure
    pub predicted_pressure: f64,
    /// Predicted bottleneck
    pub predicted_bottleneck: BottleneckType,
    /// Predicted CPU utilization
    pub predicted_cpu: f64,
    /// Predicted memory pressure
    pub predicted_memory_pressure: f64,
    /// Predicted I/O pressure
    pub predicted_io_pressure: f64,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Horizon (ms into the future)
    pub horizon_ms: u64,
}

/// System-wide predictor
pub struct SystemPredictor {
    /// CPU utilization trend
    cpu_trend: MetricTrend,
    /// Memory pressure trend
    mem_trend: MetricTrend,
    /// I/O pressure trend
    io_trend: MetricTrend,
    /// Network pressure trend
    net_trend: MetricTrend,
    /// Snapshots seen
    snapshots_seen: u64,
}

impl SystemPredictor {
    pub fn new() -> Self {
        Self {
            cpu_trend: MetricTrend::new(100),
            mem_trend: MetricTrend::new(100),
            io_trend: MetricTrend::new(100),
            net_trend: MetricTrend::new(100),
            snapshots_seen: 0,
        }
    }

    /// Ingest a system snapshot
    pub fn ingest(&mut self, snapshot: &SystemSnapshot) {
        let ts = snapshot.timestamp;
        self.cpu_trend.record(snapshot.cpu.utilization, ts);
        self.mem_trend.record(snapshot.memory.pressure, ts);
        self.io_trend.record(snapshot.io.pressure, ts);
        self.net_trend.record(snapshot.network.pressure, ts);
        self.snapshots_seen += 1;
    }

    /// Predict system state at a future time
    pub fn predict(&self, horizon_ms: u64) -> Option<SystemPrediction> {
        if self.snapshots_seen < 5 {
            return None;
        }

        let future_ts = self
            .cpu_trend
            .samples
            .back()
            .map(|s| s.timestamp + horizon_ms)?;

        let cpu = self
            .cpu_trend
            .predict(future_ts)
            .unwrap_or(self.cpu_trend.current())
            .clamp(0.0, 1.0);
        let mem = self
            .mem_trend
            .predict(future_ts)
            .unwrap_or(self.mem_trend.current())
            .clamp(0.0, 1.0);
        let io = self
            .io_trend
            .predict(future_ts)
            .unwrap_or(self.io_trend.current())
            .clamp(0.0, 1.0);
        let net = self
            .net_trend
            .predict(future_ts)
            .unwrap_or(self.net_trend.current())
            .clamp(0.0, 1.0);

        let pressure = cpu * 0.35 + mem * 0.35 + io * 0.2 + net * 0.1;

        let bottleneck = if cpu >= mem && cpu >= io && cpu >= net {
            BottleneckType::Cpu
        } else if mem >= cpu && mem >= io && mem >= net {
            BottleneckType::Memory
        } else if io >= cpu && io >= mem && io >= net {
            BottleneckType::Io
        } else {
            BottleneckType::Network
        };

        // Confidence decreases with horizon and volatility
        let base_confidence = (self.snapshots_seen as f64 / 50.0).min(1.0);
        let horizon_penalty = 1.0 / (1.0 + (horizon_ms as f64 / 10000.0));
        let confidence = base_confidence * horizon_penalty;

        Some(SystemPrediction {
            predicted_pressure: pressure,
            predicted_bottleneck: bottleneck,
            predicted_cpu: cpu,
            predicted_memory_pressure: mem,
            predicted_io_pressure: io,
            confidence,
            horizon_ms,
        })
    }

    /// Get current trend directions
    pub fn trends(&self) -> (TrendDirection, TrendDirection, TrendDirection, TrendDirection) {
        (
            self.cpu_trend.direction(),
            self.mem_trend.direction(),
            self.io_trend.direction(),
            self.net_trend.direction(),
        )
    }
}

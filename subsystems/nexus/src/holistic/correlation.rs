//! # Holistic Metric Correlation Engine
//!
//! Cross-metric correlation analysis:
//! - Pearson/Spearman correlation
//! - Cross-correlation with time lag
//! - Causal inference
//! - Correlation clustering
//! - Anomaly correlation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// METRIC TYPES
// ============================================================================

/// Metric source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorrelationMetricSource {
    /// CPU utilization
    CpuUtil,
    /// Memory utilization
    MemUtil,
    /// I/O throughput
    IoThroughput,
    /// Network throughput
    NetThroughput,
    /// Latency
    Latency,
    /// Power consumption
    Power,
    /// Temperature
    Temperature,
    /// Error rate
    ErrorRate,
}

/// Correlation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrelationType {
    /// Positive linear
    PositiveLinear,
    /// Negative linear
    NegativeLinear,
    /// Non-linear
    NonLinear,
    /// No correlation
    None,
}

// ============================================================================
// METRIC SERIES
// ============================================================================

/// Time series for correlation
#[derive(Debug, Clone)]
pub struct CorrelationSeries {
    /// Metric source
    pub source: CorrelationMetricSource,
    /// Values
    values: VecDeque<f64>,
    /// Timestamps
    timestamps: VecDeque<u64>,
    /// Max length
    max_len: usize,
}

impl CorrelationSeries {
    pub fn new(source: CorrelationMetricSource, max_len: usize) -> Self {
        Self {
            source,
            values: VecDeque::new(),
            timestamps: VecDeque::new(),
            max_len,
        }
    }

    /// Add sample
    #[inline]
    pub fn add(&mut self, value: f64, timestamp: u64) {
        self.values.push_back(value);
        self.timestamps.push_back(timestamp);
        if self.values.len() > self.max_len {
            self.values.pop_front();
            self.timestamps.pop_front();
        }
    }

    /// Length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Mean
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        self.values.iter().sum::<f64>() / self.values.len() as f64
    }

    /// Standard deviation
    pub fn std_dev(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance = self
            .values
            .iter()
            .map(|v| (v - mean) * (v - mean))
            .sum::<f64>()
            / (self.values.len() - 1) as f64;
        libm::sqrt(variance)
    }

    /// Get values slice
    #[inline(always)]
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

// ============================================================================
// CORRELATION RESULT
// ============================================================================

/// Correlation result between two series
#[derive(Debug, Clone)]
pub struct CorrelationResult {
    /// Source A
    pub source_a: CorrelationMetricSource,
    /// Source B
    pub source_b: CorrelationMetricSource,
    /// Pearson coefficient (-1.0 to 1.0)
    pub pearson: f64,
    /// Correlation type
    pub correlation_type: CorrelationType,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
    /// Time lag (samples, positive = B lags A)
    pub time_lag: i32,
    /// Sample count
    pub sample_count: usize,
}

impl CorrelationResult {
    /// Is significant
    #[inline(always)]
    pub fn is_significant(&self) -> bool {
        libm::fabs(self.pearson) > 0.5 && self.confidence > 0.7
    }
}

/// Compute Pearson correlation
pub fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n < 3 {
        return 0.0;
    }

    let mean_a: f64 = a[..n].iter().sum::<f64>() / n as f64;
    let mean_b: f64 = b[..n].iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;

    for i in 0..n {
        let da = a[i] - mean_a;
        let db = b[i] - mean_b;
        cov += da * db;
        var_a += da * da;
        var_b += db * db;
    }

    let denom = libm::sqrt(var_a * var_b);
    if denom < 1e-12 {
        return 0.0;
    }
    cov / denom
}

/// Cross-correlation with lag
pub fn cross_correlation(a: &[f64], b: &[f64], max_lag: usize) -> (f64, i32) {
    let mut best_corr = 0.0;
    let mut best_lag: i32 = 0;

    let n = a.len().min(b.len());
    if n < 3 {
        return (0.0, 0);
    }

    for lag in 0..=max_lag.min(n / 4) {
        // Positive lag: B shifted right
        if lag < n {
            let corr = pearson_correlation(&a[lag..], &b[..n - lag]);
            if libm::fabs(corr) > libm::fabs(best_corr) {
                best_corr = corr;
                best_lag = lag as i32;
            }
        }
        // Negative lag: A shifted right
        if lag > 0 && lag < n {
            let corr = pearson_correlation(&a[..n - lag], &b[lag..]);
            if libm::fabs(corr) > libm::fabs(best_corr) {
                best_corr = corr;
                best_lag = -(lag as i32);
            }
        }
    }

    (best_corr, best_lag)
}

// ============================================================================
// CORRELATION CLUSTER
// ============================================================================

/// A cluster of correlated metrics
#[derive(Debug, Clone)]
pub struct CorrelationCluster {
    /// Cluster ID
    pub id: u32,
    /// Member metrics
    pub members: Vec<CorrelationMetricSource>,
    /// Average internal correlation
    pub avg_correlation: f64,
    /// Dominant metric (strongest correlations)
    pub dominant: Option<CorrelationMetricSource>,
}

// ============================================================================
// CORRELATION ENGINE
// ============================================================================

/// Correlation engine stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticCorrelationStats {
    /// Series tracked
    pub series_count: usize,
    /// Correlations computed
    pub correlations_computed: u64,
    /// Significant correlations
    pub significant_count: usize,
    /// Clusters found
    pub cluster_count: usize,
}

/// Holistic correlation engine
pub struct HolisticCorrelationEngine {
    /// Metric series
    series: BTreeMap<u8, CorrelationSeries>,
    /// Cached correlations
    correlations: Vec<CorrelationResult>,
    /// Clusters
    clusters: Vec<CorrelationCluster>,
    /// Max lag for cross-correlation
    max_lag: usize,
    /// Stats
    stats: HolisticCorrelationStats,
}

impl HolisticCorrelationEngine {
    pub fn new(max_lag: usize) -> Self {
        Self {
            series: BTreeMap::new(),
            correlations: Vec::new(),
            clusters: Vec::new(),
            max_lag,
            stats: HolisticCorrelationStats::default(),
        }
    }

    /// Add sample
    #[inline]
    pub fn add_sample(&mut self, source: CorrelationMetricSource, value: f64, timestamp: u64) {
        let series = self
            .series
            .entry(source as u8)
            .or_insert_with(|| CorrelationSeries::new(source, 256));
        series.add(value, timestamp);
        self.stats.series_count = self.series.len();
    }

    /// Compute all pairwise correlations
    pub fn compute_correlations(&mut self) -> &[CorrelationResult] {
        self.correlations.clear();
        let keys: Vec<u8> = self.series.keys().copied().collect();

        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                let sa = &self.series[&keys[i]];
                let sb = &self.series[&keys[j]];

                let n = sa.len().min(sb.len());
                if n < 10 {
                    continue;
                }

                let (corr, lag) = cross_correlation(sa.values(), sb.values(), self.max_lag);

                let corr_type = if libm::fabs(corr) < 0.3 {
                    CorrelationType::None
                } else if corr > 0.0 {
                    CorrelationType::PositiveLinear
                } else {
                    CorrelationType::NegativeLinear
                };

                let confidence = if n > 100 {
                    0.95
                } else if n > 50 {
                    0.8
                } else {
                    0.6
                };

                self.correlations.push(CorrelationResult {
                    source_a: sa.source,
                    source_b: sb.source,
                    pearson: corr,
                    correlation_type: corr_type,
                    confidence,
                    time_lag: lag,
                    sample_count: n,
                });

                self.stats.correlations_computed += 1;
            }
        }

        self.stats.significant_count = self
            .correlations
            .iter()
            .filter(|c| c.is_significant())
            .count();

        &self.correlations
    }

    /// Get significant correlations
    #[inline]
    pub fn significant_correlations(&self) -> Vec<&CorrelationResult> {
        self.correlations
            .iter()
            .filter(|c| c.is_significant())
            .collect()
    }

    /// Get correlation between two sources
    #[inline]
    pub fn correlation(
        &self,
        a: CorrelationMetricSource,
        b: CorrelationMetricSource,
    ) -> Option<&CorrelationResult> {
        self.correlations.iter().find(|c| {
            (c.source_a == a && c.source_b == b) || (c.source_a == b && c.source_b == a)
        })
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticCorrelationStats {
        &self.stats
    }
}

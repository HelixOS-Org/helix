//! # Holistic Telemetry Aggregation Engine
//!
//! System-wide telemetry collection and analysis:
//! - Multi-source telemetry aggregation
//! - Metric rollup and downsampling
//! - Telemetry pipeline management
//! - Data retention policies
//! - Export-ready metric formatting

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// TELEMETRY TYPES
// ============================================================================

/// Telemetry source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TelemetrySource {
    /// Kernel
    Kernel,
    /// Scheduler
    Scheduler,
    /// Memory manager
    MemoryManager,
    /// I/O subsystem
    IoSubsystem,
    /// Network stack
    NetworkStack,
    /// Power manager
    PowerManager,
    /// Driver
    Driver,
    /// Application
    Application,
}

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryMetricType {
    /// Counter (monotonically increasing)
    Counter,
    /// Gauge (instantaneous value)
    Gauge,
    /// Histogram
    Histogram,
    /// Summary
    Summary,
}

/// Aggregation method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationMethod {
    /// Sum
    Sum,
    /// Average
    Average,
    /// Min
    Min,
    /// Max
    Max,
    /// Last
    Last,
    /// Count
    Count,
    /// Percentile 99
    P99,
}

// ============================================================================
// METRIC POINT
// ============================================================================

/// Single metric data point
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricPoint {
    /// Timestamp (ns)
    pub timestamp: u64,
    /// Value
    pub value: f64,
}

/// Metric series
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MetricSeries {
    /// Series name
    pub name: String,
    /// Source
    pub source: TelemetrySource,
    /// Type
    pub metric_type: TelemetryMetricType,
    /// Data points
    points: Vec<MetricPoint>,
    /// Max points before rollup
    max_points: usize,
    /// Running stats
    count: u64,
    sum: f64,
    min: f64,
    max: f64,
    last: f64,
}

impl MetricSeries {
    pub fn new(name: String, source: TelemetrySource, metric_type: TelemetryMetricType) -> Self {
        Self {
            name,
            source,
            metric_type,
            points: Vec::new(),
            max_points: 1024,
            count: 0,
            sum: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            last: 0.0,
        }
    }

    /// Record value
    pub fn record(&mut self, value: f64, timestamp: u64) {
        self.points.push(MetricPoint { timestamp, value });
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        self.last = value;

        if self.points.len() > self.max_points {
            self.rollup();
        }
    }

    /// Aggregate
    pub fn aggregate(&self, method: AggregationMethod) -> f64 {
        match method {
            AggregationMethod::Sum => self.sum,
            AggregationMethod::Average => {
                if self.count == 0 {
                    0.0
                } else {
                    self.sum / self.count as f64
                }
            }
            AggregationMethod::Min => {
                if self.count == 0 {
                    0.0
                } else {
                    self.min
                }
            }
            AggregationMethod::Max => {
                if self.count == 0 {
                    0.0
                } else {
                    self.max
                }
            }
            AggregationMethod::Last => self.last,
            AggregationMethod::Count => self.count as f64,
            AggregationMethod::P99 => self.percentile(99),
        }
    }

    /// Percentile calculation
    #[inline]
    pub fn percentile(&self, pct: u32) -> f64 {
        if self.points.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.points.iter().map(|p| p.value).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let idx = ((pct as f64 / 100.0) * (sorted.len() - 1) as f64) as usize;
        sorted[idx]
    }

    /// Rollup (downsample by 2x averaging pairs)
    fn rollup(&mut self) {
        if self.points.len() < 2 {
            return;
        }
        let mut new_points = Vec::with_capacity(self.points.len() / 2 + 1);
        let mut i = 0;
        while i + 1 < self.points.len() {
            let avg_val = (self.points[i].value + self.points[i + 1].value) / 2.0;
            let avg_ts = (self.points[i].timestamp + self.points[i + 1].timestamp) / 2;
            new_points.push(MetricPoint {
                timestamp: avg_ts,
                value: avg_val,
            });
            i += 2;
        }
        if i < self.points.len() {
            new_points.push(self.points[i].clone());
        }
        self.points = new_points;
    }

    /// Rate (per second)
    pub fn rate(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let first = &self.points[0];
        let last = &self.points[self.points.len() - 1];
        let dt = last.timestamp.saturating_sub(first.timestamp);
        if dt == 0 {
            return 0.0;
        }
        let dv = last.value - first.value;
        dv / (dt as f64 / 1_000_000_000.0)
    }

    /// Recent values
    #[inline]
    pub fn recent(&self, count: usize) -> &[MetricPoint] {
        if self.points.len() > count {
            &self.points[self.points.len() - count..]
        } else {
            &self.points
        }
    }
}

// ============================================================================
// RETENTION POLICY
// ============================================================================

/// Retention policy
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Raw data retention (ns)
    pub raw_retention_ns: u64,
    /// Rollup retention (ns)
    pub rollup_retention_ns: u64,
    /// Rollup interval (ns)
    pub rollup_interval_ns: u64,
}

impl RetentionPolicy {
    /// Default: 1 hour raw, 24 hours rolled up
    #[inline]
    pub fn default_policy() -> Self {
        Self {
            raw_retention_ns: 3_600_000_000_000,
            rollup_retention_ns: 86_400_000_000_000,
            rollup_interval_ns: 60_000_000_000,
        }
    }
}

// ============================================================================
// TELEMETRY ENGINE
// ============================================================================

/// Telemetry stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticTelemetryStats {
    /// Series count
    pub series_count: usize,
    /// Total points
    pub total_points: u64,
    /// Points per second
    pub points_per_second: f64,
    /// Sources active
    pub sources_active: usize,
    /// Rollups performed
    pub rollups: u64,
}

/// Telemetry key
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TelemetryKey {
    source: u8,
    name_hash: u64,
}

/// Holistic telemetry aggregation engine
pub struct HolisticTelemetryEngine {
    /// Metric series by key
    series: BTreeMap<u64, MetricSeries>,
    /// Key counter
    next_key: u64,
    /// Name to key mapping
    name_map: LinearMap<u64, 64>,
    /// Retention policy
    retention: RetentionPolicy,
    /// Stats
    stats: HolisticTelemetryStats,
    /// Start time for rate calculation
    start_time: u64,
}

impl HolisticTelemetryEngine {
    pub fn new() -> Self {
        Self {
            series: BTreeMap::new(),
            next_key: 1,
            name_map: LinearMap::new(),
            retention: RetentionPolicy::default_policy(),
            stats: HolisticTelemetryStats::default(),
            start_time: 0,
        }
    }

    /// Set retention policy
    #[inline(always)]
    pub fn set_retention(&mut self, policy: RetentionPolicy) {
        self.retention = policy;
    }

    fn hash_name(name: &str, source: TelemetrySource) -> u64 {
        let mut h: u64 = 14695981039346656037;
        for b in name.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        h ^= source as u64;
        h = h.wrapping_mul(1099511628211);
        h
    }

    /// Register metric series
    pub fn register(
        &mut self,
        name: String,
        source: TelemetrySource,
        metric_type: TelemetryMetricType,
    ) -> u64 {
        let name_hash = Self::hash_name(&name, source);
        if let Some(&key) = self.name_map.get(name_hash) {
            return key;
        }

        let key = self.next_key;
        self.next_key += 1;

        let series = MetricSeries::new(name, source, metric_type);
        self.series.insert(key, series);
        self.name_map.insert(name_hash, key);
        self.stats.series_count = self.series.len();
        key
    }

    /// Record metric
    pub fn record(&mut self, key: u64, value: f64, timestamp: u64) {
        if self.start_time == 0 {
            self.start_time = timestamp;
        }

        if let Some(series) = self.series.get_mut(&key) {
            series.record(value, timestamp);
            self.stats.total_points += 1;

            let elapsed = timestamp.saturating_sub(self.start_time);
            if elapsed > 0 {
                self.stats.points_per_second =
                    self.stats.total_points as f64 / (elapsed as f64 / 1_000_000_000.0);
            }
        }
    }

    /// Query metric
    #[inline(always)]
    pub fn query(&self, key: u64, method: AggregationMethod) -> Option<f64> {
        self.series.get(&key).map(|s| s.aggregate(method))
    }

    /// Query rate
    #[inline(always)]
    pub fn query_rate(&self, key: u64) -> Option<f64> {
        self.series.get(&key).map(|s| s.rate())
    }

    /// Get series
    #[inline(always)]
    pub fn get_series(&self, key: u64) -> Option<&MetricSeries> {
        self.series.get(&key)
    }

    /// Aggregate across multiple series
    pub fn aggregate_multi(
        &self,
        keys: &[u64],
        method: AggregationMethod,
    ) -> f64 {
        let values: Vec<f64> = keys
            .iter()
            .filter_map(|k| self.series.get(k))
            .map(|s| s.aggregate(method))
            .collect();

        if values.is_empty() {
            return 0.0;
        }

        match method {
            AggregationMethod::Sum => values.iter().sum(),
            AggregationMethod::Average => values.iter().sum::<f64>() / values.len() as f64,
            AggregationMethod::Min => values
                .iter()
                .copied()
                .fold(f64::INFINITY, |a, b| if a < b { a } else { b }),
            AggregationMethod::Max => values
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, |a, b| if a > b { a } else { b }),
            AggregationMethod::Last => *values.last().unwrap_or(&0.0),
            AggregationMethod::Count => values.len() as f64,
            AggregationMethod::P99 => {
                let mut sorted = values;
                sorted.sort_by(|a, b| {
                    a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal)
                });
                let idx = ((99.0 / 100.0) * (sorted.len() - 1) as f64) as usize;
                sorted[idx]
            }
        }
    }

    /// Apply retention (garbage collect old data)
    #[inline]
    pub fn apply_retention(&mut self, now: u64) {
        let cutoff = now.saturating_sub(self.retention.raw_retention_ns);
        for series in self.series.values_mut() {
            let before = series.points.len();
            series.points.retain(|p| p.timestamp >= cutoff);
            if series.points.len() < before {
                self.stats.rollups += 1;
            }
        }
    }

    /// Active sources
    #[inline]
    pub fn active_sources(&self) -> Vec<TelemetrySource> {
        let mut sources: Vec<TelemetrySource> = Vec::new();
        for series in self.series.values() {
            if !sources.contains(&series.source) {
                sources.push(series.source);
            }
        }
        self.stats.sources_active;
        sources
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticTelemetryStats {
        &self.stats
    }
}

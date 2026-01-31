//! # Telemetry and Metrics Collection
//!
//! The Metrics Collector gathers, aggregates, and analyzes system telemetry
//! to provide real-time insights for the AI decision engine.
//!
//! ## Metrics Categories
//!
//! - **System Metrics**: CPU, memory, I/O, network
//! - **Process Metrics**: Per-process resource usage
//! - **Kernel Metrics**: Scheduler, allocator, IPC statistics
//! - **AI Metrics**: Decision counts, accuracy, latency
//!
//! ## Architecture
//!
//! ```text
//!    Telemetry ───────►┌──────────────────────────────────────────┐
//!    Sources           │          Metrics Collector               │
//!                      │                                          │
//!    CPU Stats ───────►│  ┌────────────────────────────────┐     │
//!                      │  │       Ingestion Pipeline       │     │
//!    Memory Stats ────►│  │   - Rate Limiting             │     │
//!                      │  │   - Normalization             │     │
//!    I/O Stats ───────►│  │   - Validation                │     │
//!                      │  └──────────────┬─────────────────┘     │
//!    Network Stats ───►│                 │                       │
//!                      │                 ▼                       │
//!    Process Stats ───►│  ┌────────────────────────────────┐     │
//!                      │  │       Time Series Store        │     │
//!    Custom Metrics ──►│  │   - Rolling Windows           │     │
//!                      │  │   - Aggregations              │     │
//!                      │  │   - Downsampling              │     │
//!                      │  └──────────────┬─────────────────┘     │
//!                      │                 │                       │
//!                      │                 ▼                       │
//!                      │  ┌────────────────────────────────┐     │
//!                      │  │       Analytics Engine         │─────────► Insights
//!                      │  │   - Trend Detection           │     │
//!                      │  │   - Anomaly Detection         │     │
//!                      │  │   - Forecasting               │     │
//!                      │  └────────────────────────────────┘     │
//!                      │                                          │
//!                      └──────────────────────────────────────────┘
//! ```


use alloc::{
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

// =============================================================================
// Metric Types
// =============================================================================

/// Unique identifier for a metric
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MetricId(String);

impl MetricId {
    /// Create from string
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    /// Get the name
    pub fn name(&self) -> &str {
        &self.0
    }
}

impl From<&str> for MetricId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// Type of metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Counter - monotonically increasing value
    Counter,
    /// Gauge - value that can go up and down
    Gauge,
    /// Histogram - distribution of values
    Histogram,
    /// Summary - statistical summary (percentiles)
    Summary,
}

/// A single metric value
#[derive(Debug, Clone)]
pub struct MetricValue {
    /// The value
    pub value: f64,
    /// Timestamp (microseconds)
    pub timestamp: u64,
    /// Optional labels
    pub labels: BTreeMap<String, String>,
}

impl MetricValue {
    /// Create a simple value
    pub fn new(value: f64, timestamp: u64) -> Self {
        Self {
            value,
            timestamp,
            labels: BTreeMap::new(),
        }
    }

    /// Create with labels
    pub fn with_labels(value: f64, timestamp: u64, labels: BTreeMap<String, String>) -> Self {
        Self {
            value,
            timestamp,
            labels,
        }
    }

    /// Add a label
    pub fn label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }
}

/// Metric definition
#[derive(Debug, Clone)]
pub struct MetricDefinition {
    /// Metric ID
    pub id: MetricId,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Unit of measurement
    pub unit: String,
    /// Minimum expected value
    pub min_value: Option<f64>,
    /// Maximum expected value
    pub max_value: Option<f64>,
    /// Warning threshold
    pub warning_threshold: Option<f64>,
    /// Critical threshold
    pub critical_threshold: Option<f64>,
}

impl MetricDefinition {
    /// Create a new counter metric
    pub fn counter(id: &str, name: &str, description: &str) -> Self {
        Self {
            id: MetricId::new(id),
            name: name.to_string(),
            description: description.to_string(),
            metric_type: MetricType::Counter,
            unit: "count".to_string(),
            min_value: Some(0.0),
            max_value: None,
            warning_threshold: None,
            critical_threshold: None,
        }
    }

    /// Create a new gauge metric
    pub fn gauge(id: &str, name: &str, description: &str, unit: &str) -> Self {
        Self {
            id: MetricId::new(id),
            name: name.to_string(),
            description: description.to_string(),
            metric_type: MetricType::Gauge,
            unit: unit.to_string(),
            min_value: None,
            max_value: None,
            warning_threshold: None,
            critical_threshold: None,
        }
    }

    /// Set thresholds
    pub fn with_thresholds(mut self, warning: f64, critical: f64) -> Self {
        self.warning_threshold = Some(warning);
        self.critical_threshold = Some(critical);
        self
    }

    /// Set bounds
    pub fn with_bounds(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }
}

// =============================================================================
// Time Series Storage
// =============================================================================

/// A time series for a single metric
#[derive(Debug)]
pub struct TimeSeries {
    /// Metric definition
    definition: MetricDefinition,
    /// Data points
    data: VecDeque<MetricValue>,
    /// Maximum points to retain
    max_points: usize,
    /// Current aggregated stats
    stats: TimeSeriesStats,
}

/// Statistics for a time series
#[derive(Debug, Clone, Default)]
pub struct TimeSeriesStats {
    pub count: u64,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub variance: f64,
    pub last_value: f64,
    pub last_timestamp: u64,
}

impl TimeSeries {
    /// Create a new time series
    pub fn new(definition: MetricDefinition, max_points: usize) -> Self {
        Self {
            definition,
            data: VecDeque::with_capacity(max_points),
            max_points,
            stats: TimeSeriesStats {
                min: f64::MAX,
                max: f64::MIN,
                ..Default::default()
            },
        }
    }

    /// Add a data point
    pub fn add(&mut self, value: MetricValue) {
        // Update rolling stats
        self.stats.count += 1;
        self.stats.sum += value.value;
        self.stats.min = self.stats.min.min(value.value);
        self.stats.max = self.stats.max.max(value.value);
        self.stats.last_value = value.value;
        self.stats.last_timestamp = value.timestamp;

        // Update mean incrementally
        let n = self.stats.count as f64;
        let delta = value.value - self.stats.mean;
        self.stats.mean += delta / n;

        // Update variance incrementally (Welford's algorithm)
        let delta2 = value.value - self.stats.mean;
        self.stats.variance += delta * delta2;

        // Add to buffer
        if self.data.len() >= self.max_points {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }

    /// Get statistics
    pub fn stats(&self) -> TimeSeriesStats {
        let mut stats = self.stats.clone();
        if stats.count > 1 {
            stats.variance /= (stats.count - 1) as f64;
        }
        stats
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> f64 {
        if self.stats.count > 1 {
            crate::math::sqrt_f64(self.stats.variance / (self.stats.count - 1) as f64)
        } else {
            0.0
        }
    }

    /// Get recent values
    pub fn recent(&self, count: usize) -> impl Iterator<Item = &MetricValue> {
        self.data.iter().rev().take(count)
    }

    /// Get values in time range
    pub fn range(&self, start: u64, end: u64) -> impl Iterator<Item = &MetricValue> {
        self.data
            .iter()
            .filter(move |v| v.timestamp >= start && v.timestamp <= end)
    }

    /// Get definition
    pub fn definition(&self) -> &MetricDefinition {
        &self.definition
    }

    /// Check if value is anomalous (> 3 sigma)
    pub fn is_anomalous(&self, value: f64) -> bool {
        if self.stats.count < 10 {
            return false;
        }
        let std_dev = self.std_dev();
        if std_dev == 0.0 {
            return false;
        }
        ((value - self.stats.mean) / std_dev).abs() > 3.0
    }

    /// Get trend (slope of linear regression)
    pub fn trend(&self) -> f64 {
        if self.data.len() < 2 {
            return 0.0;
        }

        // Simple linear regression
        let n = self.data.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;

        for (i, v) in self.data.iter().enumerate() {
            let x = i as f64;
            let y = v.value;
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_x2 += x * x;
        }

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-10 {
            return 0.0;
        }

        (n * sum_xy - sum_x * sum_y) / denom
    }

    /// Forecast future value
    pub fn forecast(&self, steps: usize) -> f64 {
        let trend = self.trend();
        self.stats.last_value + trend * steps as f64
    }

    /// Get percentile value
    pub fn percentile(&self, p: f64) -> f64 {
        if self.data.is_empty() {
            return 0.0;
        }

        let mut values: Vec<f64> = self.data.iter().map(|v| v.value).collect();
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));

        let idx = ((p / 100.0) * (values.len() - 1) as f64) as usize;
        values[idx.min(values.len() - 1)]
    }

    /// Count of data points
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clear data
    pub fn clear(&mut self) {
        self.data.clear();
        self.stats = TimeSeriesStats {
            min: f64::MAX,
            max: f64::MIN,
            ..Default::default()
        };
    }
}

// =============================================================================
// Histogram
// =============================================================================

/// A histogram for value distributions
#[derive(Debug)]
pub struct Histogram {
    /// Bucket boundaries
    boundaries: Vec<f64>,
    /// Bucket counts
    counts: Vec<u64>,
    /// Total sum
    sum: f64,
    /// Total count
    count: u64,
}

impl Histogram {
    /// Create with default buckets
    pub fn new() -> Self {
        // Default exponential buckets
        Self::with_boundaries(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
    }

    /// Create with custom boundaries
    pub fn with_boundaries(boundaries: Vec<f64>) -> Self {
        let len = boundaries.len();
        Self {
            boundaries,
            counts: vec![0; len + 1], // +1 for infinity bucket
            sum: 0.0,
            count: 0,
        }
    }

    /// Create linear buckets
    pub fn linear(start: f64, width: f64, count: usize) -> Self {
        let boundaries: Vec<f64> = (0..count).map(|i| start + width * i as f64).collect();
        Self::with_boundaries(boundaries)
    }

    /// Create exponential buckets
    pub fn exponential(start: f64, factor: f64, count: usize) -> Self {
        let boundaries: Vec<f64> = (0..count)
            .map(|i| start * crate::math::powi_f64(factor, i as i32))
            .collect();
        Self::with_boundaries(boundaries)
    }

    /// Observe a value
    pub fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;

        // Find bucket
        for (i, &boundary) in self.boundaries.iter().enumerate() {
            if value <= boundary {
                self.counts[i] += 1;
                return;
            }
        }
        // Overflow bucket
        self.counts[self.boundaries.len()] += 1;
    }

    /// Get bucket counts
    pub fn buckets(&self) -> impl Iterator<Item = (f64, u64)> + '_ {
        self.boundaries
            .iter()
            .copied()
            .zip(self.counts.iter().copied())
            .chain(core::iter::once((f64::INFINITY, self.counts[self.boundaries.len()])))
    }

    /// Get count
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get sum
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    /// Estimate percentile
    pub fn percentile(&self, p: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }

        let threshold = (p / 100.0 * self.count as f64) as u64;
        let mut cumulative = 0;

        for (i, &count) in self.counts.iter().enumerate() {
            cumulative += count;
            if cumulative >= threshold {
                if i < self.boundaries.len() {
                    return self.boundaries[i];
                } else {
                    return self.boundaries.last().copied().unwrap_or(0.0);
                }
            }
        }

        self.boundaries.last().copied().unwrap_or(0.0)
    }

    /// Reset histogram
    pub fn reset(&mut self) {
        for count in &mut self.counts {
            *count = 0;
        }
        self.sum = 0.0;
        self.count = 0;
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Metrics Collector
// =============================================================================

/// The Metrics Collector
pub struct MetricsCollector {
    /// Time series by metric ID
    series: RwLock<BTreeMap<MetricId, TimeSeries>>,

    /// Histograms by metric ID
    histograms: RwLock<BTreeMap<MetricId, Histogram>>,

    /// Counters
    counters: RwLock<BTreeMap<MetricId, AtomicU64>>,

    /// Maximum time series length
    max_series_length: usize,

    /// Current time
    current_time: RwLock<u64>,

    /// Statistics
    stats: CollectorStats,
}

struct CollectorStats {
    metrics_recorded: AtomicU64,
    series_count: AtomicU64,
    histogram_count: AtomicU64,
    anomalies_detected: AtomicU64,
}

impl Default for CollectorStats {
    fn default() -> Self {
        Self {
            metrics_recorded: AtomicU64::new(0),
            series_count: AtomicU64::new(0),
            histogram_count: AtomicU64::new(0),
            anomalies_detected: AtomicU64::new(0),
        }
    }
}

impl MetricsCollector {
    /// Default time series length
    const DEFAULT_SERIES_LENGTH: usize = 1000;

    /// Create a new collector
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_SERIES_LENGTH)
    }

    /// Create with custom capacity
    pub fn with_capacity(max_series_length: usize) -> Self {
        let collector = Self {
            series: RwLock::new(BTreeMap::new()),
            histograms: RwLock::new(BTreeMap::new()),
            counters: RwLock::new(BTreeMap::new()),
            max_series_length,
            current_time: RwLock::new(0),
            stats: CollectorStats::default(),
        };

        // Register default metrics
        collector.register_defaults();
        collector
    }

    /// Register default metrics
    fn register_defaults(&self) {
        // CPU metrics
        self.register(
            MetricDefinition::gauge("system.cpu.usage", "CPU Usage", "Overall CPU utilization", "%")
                .with_bounds(0.0, 100.0)
                .with_thresholds(80.0, 95.0),
        );

        // Memory metrics
        self.register(
            MetricDefinition::gauge(
                "system.memory.usage",
                "Memory Usage",
                "System memory utilization",
                "%",
            )
            .with_bounds(0.0, 100.0)
            .with_thresholds(80.0, 95.0),
        );

        // I/O metrics
        self.register(MetricDefinition::gauge(
            "system.io.read_bytes",
            "I/O Read",
            "Bytes read per second",
            "B/s",
        ));

        self.register(MetricDefinition::gauge(
            "system.io.write_bytes",
            "I/O Write",
            "Bytes written per second",
            "B/s",
        ));

        // Process metrics
        self.register(MetricDefinition::gauge(
            "system.processes.count",
            "Process Count",
            "Number of running processes",
            "count",
        ));

        // AI metrics
        self.register(MetricDefinition::counter(
            "ai.decisions.total",
            "AI Decisions",
            "Total AI decisions made",
        ));

        self.register(
            MetricDefinition::gauge(
                "ai.decisions.latency_us",
                "Decision Latency",
                "AI decision latency",
                "μs",
            )
            .with_thresholds(1000.0, 10000.0),
        );

        self.register(MetricDefinition::gauge(
            "ai.decisions.confidence",
            "Decision Confidence",
            "Average decision confidence",
            "ratio",
        ));
    }

    /// Register a metric
    pub fn register(&self, definition: MetricDefinition) {
        let id = definition.id.clone();
        let mut series = self.series.write();

        if !series.contains_key(&id) {
            series.insert(id, TimeSeries::new(definition, self.max_series_length));
            self.stats.series_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Register a histogram metric
    pub fn register_histogram(&self, id: &str, boundaries: Vec<f64>) {
        let metric_id = MetricId::new(id);
        let mut histograms = self.histograms.write();

        if !histograms.contains_key(&metric_id) {
            histograms.insert(metric_id, Histogram::with_boundaries(boundaries));
            self.stats.histogram_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a metric value
    pub fn record(&self, id: &str, value: f64) {
        self.record_at(id, value, *self.current_time.read());
    }

    /// Record with timestamp
    pub fn record_at(&self, id: &str, value: f64, timestamp: u64) {
        let metric_id = MetricId::new(id);
        let metric_value = MetricValue::new(value, timestamp);

        let mut series = self.series.write();
        if let Some(ts) = series.get_mut(&metric_id) {
            // Check for anomaly before recording
            if ts.is_anomalous(value) {
                self.stats.anomalies_detected.fetch_add(1, Ordering::Relaxed);
            }
            ts.add(metric_value);
            self.stats.metrics_recorded.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record with labels
    pub fn record_with_labels(&self, id: &str, value: f64, labels: BTreeMap<String, String>) {
        let timestamp = *self.current_time.read();
        let metric_id = MetricId::new(id);
        let metric_value = MetricValue::with_labels(value, timestamp, labels);

        let mut series = self.series.write();
        if let Some(ts) = series.get_mut(&metric_id) {
            ts.add(metric_value);
            self.stats.metrics_recorded.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Observe histogram value
    pub fn observe(&self, id: &str, value: f64) {
        let metric_id = MetricId::new(id);
        let mut histograms = self.histograms.write();

        if let Some(hist) = histograms.get_mut(&metric_id) {
            hist.observe(value);
            self.stats.metrics_recorded.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Increment counter
    pub fn increment(&self, id: &str) {
        self.increment_by(id, 1);
    }

    /// Increment counter by value
    pub fn increment_by(&self, id: &str, value: u64) {
        let metric_id = MetricId::new(id);
        let counters = self.counters.read();

        if let Some(counter) = counters.get(&metric_id) {
            counter.fetch_add(value, Ordering::Relaxed);
            self.stats.metrics_recorded.fetch_add(1, Ordering::Relaxed);
        } else {
            drop(counters);
            let mut counters = self.counters.write();
            counters
                .entry(metric_id)
                .or_insert_with(|| AtomicU64::new(0))
                .fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Get counter value
    pub fn counter(&self, id: &str) -> u64 {
        let metric_id = MetricId::new(id);
        self.counters
            .read()
            .get(&metric_id)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get current value of a gauge
    pub fn current(&self, id: &str) -> Option<f64> {
        let metric_id = MetricId::new(id);
        self.series
            .read()
            .get(&metric_id)
            .map(|ts| ts.stats().last_value)
    }

    /// Get statistics for a metric
    pub fn stats(&self, id: &str) -> Option<TimeSeriesStats> {
        let metric_id = MetricId::new(id);
        self.series.read().get(&metric_id).map(|ts| ts.stats())
    }

    /// Get trend for a metric
    pub fn trend(&self, id: &str) -> Option<f64> {
        let metric_id = MetricId::new(id);
        self.series.read().get(&metric_id).map(|ts| ts.trend())
    }

    /// Get percentile for a metric
    pub fn percentile(&self, id: &str, p: f64) -> Option<f64> {
        let metric_id = MetricId::new(id);
        self.series
            .read()
            .get(&metric_id)
            .map(|ts| ts.percentile(p))
    }

    /// Forecast future value
    pub fn forecast(&self, id: &str, steps: usize) -> Option<f64> {
        let metric_id = MetricId::new(id);
        self.series
            .read()
            .get(&metric_id)
            .map(|ts| ts.forecast(steps))
    }

    /// Check if value would be anomalous
    pub fn would_be_anomalous(&self, id: &str, value: f64) -> bool {
        let metric_id = MetricId::new(id);
        self.series
            .read()
            .get(&metric_id)
            .map(|ts| ts.is_anomalous(value))
            .unwrap_or(false)
    }

    /// Get histogram percentile
    pub fn histogram_percentile(&self, id: &str, p: f64) -> Option<f64> {
        let metric_id = MetricId::new(id);
        self.histograms
            .read()
            .get(&metric_id)
            .map(|h| h.percentile(p))
    }

    /// Get histogram stats
    pub fn histogram_stats(&self, id: &str) -> Option<(u64, f64, f64)> {
        let metric_id = MetricId::new(id);
        self.histograms
            .read()
            .get(&metric_id)
            .map(|h| (h.count(), h.sum(), h.mean()))
    }

    /// Set current time
    pub fn set_time(&self, time: u64) {
        *self.current_time.write() = time;
    }

    /// Get all metric IDs
    pub fn metric_ids(&self) -> Vec<MetricId> {
        self.series.read().keys().cloned().collect()
    }

    /// Generate summary report
    pub fn summary(&self) -> MetricsSummary {
        let series = self.series.read();
        let histograms = self.histograms.read();
        let counters = self.counters.read();

        let mut metrics = Vec::new();

        for (id, ts) in series.iter() {
            let stats = ts.stats();
            metrics.push(MetricSummaryEntry {
                id: id.clone(),
                current: stats.last_value,
                min: stats.min,
                max: stats.max,
                mean: stats.mean,
                trend: ts.trend(),
                count: stats.count,
            });
        }

        MetricsSummary {
            timestamp: *self.current_time.read(),
            metrics,
            series_count: series.len(),
            histogram_count: histograms.len(),
            counter_count: counters.len(),
            total_recorded: self.stats.metrics_recorded.load(Ordering::Relaxed),
            anomalies_detected: self.stats.anomalies_detected.load(Ordering::Relaxed),
        }
    }

    /// Get collector statistics
    pub fn statistics(&self) -> MetricsCollectorStatistics {
        MetricsCollectorStatistics {
            metrics_recorded: self.stats.metrics_recorded.load(Ordering::Relaxed),
            series_count: self.stats.series_count.load(Ordering::Relaxed),
            histogram_count: self.stats.histogram_count.load(Ordering::Relaxed),
            anomalies_detected: self.stats.anomalies_detected.load(Ordering::Relaxed),
            counter_count: self.counters.read().len() as u64,
        }
    }

    /// Clear all data
    pub fn clear(&self) {
        for ts in self.series.write().values_mut() {
            ts.clear();
        }
        for h in self.histograms.write().values_mut() {
            h.reset();
        }
        for c in self.counters.write().values() {
            c.store(0, Ordering::Relaxed);
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary entry for a metric
#[derive(Debug, Clone)]
pub struct MetricSummaryEntry {
    pub id: MetricId,
    pub current: f64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub trend: f64,
    pub count: u64,
}

/// Complete metrics summary
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub timestamp: u64,
    pub metrics: Vec<MetricSummaryEntry>,
    pub series_count: usize,
    pub histogram_count: usize,
    pub counter_count: usize,
    pub total_recorded: u64,
    pub anomalies_detected: u64,
}

/// Collector statistics
#[derive(Debug, Clone)]
pub struct MetricsCollectorStatistics {
    pub metrics_recorded: u64,
    pub series_count: u64,
    pub histogram_count: u64,
    pub anomalies_detected: u64,
    pub counter_count: u64,
}

// =============================================================================
// Convenience Macros
// =============================================================================

/// Record a metric value
#[macro_export]
macro_rules! metrics_record {
    ($collector:expr, $id:expr, $value:expr) => {
        $collector.record($id, $value)
    };
}

/// Increment a counter
#[macro_export]
macro_rules! metrics_inc {
    ($collector:expr, $id:expr) => {
        $collector.increment($id)
    };
    ($collector:expr, $id:expr, $value:expr) => {
        $collector.increment_by($id, $value)
    };
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_series() {
        let def = MetricDefinition::gauge("test", "Test", "A test metric", "count");
        let mut ts = TimeSeries::new(def, 100);

        for i in 0..10 {
            ts.add(MetricValue::new(i as f64, i as u64));
        }

        let stats = ts.stats();
        assert_eq!(stats.count, 10);
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 9.0);
        assert!((stats.mean - 4.5).abs() < 0.001);
    }

    #[test]
    fn test_time_series_trend() {
        let def = MetricDefinition::gauge("test", "Test", "A test metric", "count");
        let mut ts = TimeSeries::new(def, 100);

        // Linear increasing trend
        for i in 0..10 {
            ts.add(MetricValue::new(i as f64 * 2.0, i as u64));
        }

        let trend = ts.trend();
        assert!((trend - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_histogram() {
        let mut hist = Histogram::linear(0.0, 10.0, 10);

        for i in 0..100 {
            hist.observe(i as f64);
        }

        assert_eq!(hist.count(), 100);
        assert!((hist.mean() - 49.5).abs() < 0.01);
    }

    #[test]
    fn test_histogram_percentile() {
        let mut hist = Histogram::linear(0.0, 10.0, 10);

        for i in 0..100 {
            hist.observe(i as f64);
        }

        let p50 = hist.percentile(50.0);
        assert!(p50 >= 40.0 && p50 <= 60.0);
    }

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        // Record some CPU values
        for i in 0..10 {
            collector.record("system.cpu.usage", 40.0 + i as f64);
        }

        let current = collector.current("system.cpu.usage");
        assert!(current.is_some());
        assert!((current.unwrap() - 49.0).abs() < 0.01);

        let stats = collector.stats("system.cpu.usage");
        assert!(stats.is_some());
    }

    #[test]
    fn test_counter() {
        let collector = MetricsCollector::new();

        collector.increment("ai.decisions.total");
        collector.increment("ai.decisions.total");
        collector.increment_by("ai.decisions.total", 5);

        assert_eq!(collector.counter("ai.decisions.total"), 7);
    }

    #[test]
    fn test_anomaly_detection() {
        let collector = MetricsCollector::new();

        // Record normal values with some variance (required for anomaly detection)
        for i in 0..20 {
            // Values between 48 and 52 to have some variance but still be close
            collector.record("system.cpu.usage", 50.0 + (i as f64 % 5.0) - 2.0);
        }

        // Check if extremely anomalous value would be detected (way outside normal range)
        // With mean ~50 and small std_dev, 500 should be clearly anomalous (>3 sigma)
        let is_anomalous = collector.would_be_anomalous("system.cpu.usage", 500.0);
        assert!(is_anomalous);
    }
}

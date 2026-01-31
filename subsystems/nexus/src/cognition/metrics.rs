//! # Cognitive Metrics System
//!
//! Collects, aggregates, and reports metrics from cognitive domains.
//! Provides dashboards and analytics.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// METRIC TYPES
// ============================================================================

/// A metric value
#[derive(Debug, Clone)]
pub struct Metric {
    /// Metric ID
    pub id: u64,
    /// Metric name
    pub name: String,
    /// Metric type
    pub metric_type: MetricType,
    /// Current value
    pub value: MetricValue,
    /// Source domain
    pub source: DomainId,
    /// Labels
    pub labels: BTreeMap<String, String>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// Counter (monotonically increasing)
    Counter,
    /// Gauge (can go up or down)
    Gauge,
    /// Histogram
    Histogram,
    /// Summary (percentiles)
    Summary,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Histogram buckets
    Histogram(HistogramValue),
    /// Summary percentiles
    Summary(SummaryValue),
}

/// Histogram value
#[derive(Debug, Clone)]
pub struct HistogramValue {
    /// Bucket boundaries
    pub buckets: Vec<f64>,
    /// Bucket counts
    pub counts: Vec<u64>,
    /// Total sum
    pub sum: f64,
    /// Total count
    pub count: u64,
}

impl HistogramValue {
    /// Create a new histogram
    pub fn new(buckets: Vec<f64>) -> Self {
        let bucket_count = buckets.len();
        Self {
            buckets,
            counts: vec![0; bucket_count + 1], // +1 for overflow bucket
            sum: 0.0,
            count: 0,
        }
    }

    /// Observe a value
    pub fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;

        for (i, boundary) in self.buckets.iter().enumerate() {
            if value <= *boundary {
                self.counts[i] += 1;
                return;
            }
        }
        // Overflow bucket
        if let Some(last) = self.counts.last_mut() {
            *last += 1;
        }
    }

    /// Get bucket for a given percentile
    pub fn percentile_bucket(&self, percentile: f64) -> Option<f64> {
        let target = (self.count as f64 * percentile / 100.0) as u64;
        let mut cumulative = 0u64;

        for (i, count) in self.counts.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return self.buckets.get(i).copied();
            }
        }
        None
    }
}

/// Summary value
#[derive(Debug, Clone)]
pub struct SummaryValue {
    /// Percentile quantiles (e.g., 0.5, 0.9, 0.99)
    pub quantiles: Vec<f64>,
    /// Quantile values
    pub values: Vec<f64>,
    /// Total sum
    pub sum: f64,
    /// Total count
    pub count: u64,
}

// ============================================================================
// METRIC COLLECTOR
// ============================================================================

/// Collects and aggregates metrics
pub struct MetricCollector {
    /// Registered metrics
    metrics: BTreeMap<u64, Metric>,
    /// Metric name index
    name_index: BTreeMap<String, u64>,
    /// Metrics by domain
    domain_index: BTreeMap<DomainId, Vec<u64>>,
    /// Next metric ID
    next_id: AtomicU64,
    /// Aggregated stats
    aggregations: BTreeMap<String, AggregatedMetric>,
    /// Configuration
    config: MetricConfig,
    /// Statistics
    stats: CollectorStats,
}

/// Aggregated metric
#[derive(Debug, Clone)]
pub struct AggregatedMetric {
    /// Metric name
    pub name: String,
    /// Count
    pub count: u64,
    /// Sum
    pub sum: f64,
    /// Min
    pub min: f64,
    /// Max
    pub max: f64,
    /// Mean
    pub mean: f64,
    /// Variance
    pub variance: f64,
    /// Last value
    pub last: f64,
    /// Window samples
    pub window: Vec<f64>,
}

impl AggregatedMetric {
    fn new(name: String, window_size: usize) -> Self {
        Self {
            name,
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            mean: 0.0,
            variance: 0.0,
            last: 0.0,
            window: Vec::with_capacity(window_size),
        }
    }

    fn update(&mut self, value: f64, window_size: usize) {
        self.count += 1;
        self.sum += value;
        self.last = value;

        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        // Update rolling mean and variance (Welford's algorithm)
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.variance += delta * delta2;

        // Update window
        if self.window.len() >= window_size {
            self.window.remove(0);
        }
        self.window.push(value);
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> f64 {
        if self.count > 1 {
            (self.variance / (self.count - 1) as f64).sqrt()
        } else {
            0.0
        }
    }

    /// Get windowed mean
    pub fn windowed_mean(&self) -> f64 {
        if self.window.is_empty() {
            self.mean
        } else {
            self.window.iter().sum::<f64>() / self.window.len() as f64
        }
    }
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MetricConfig {
    /// Maximum metrics
    pub max_metrics: usize,
    /// Aggregation window size
    pub window_size: usize,
    /// Enable histograms
    pub enable_histograms: bool,
    /// Default histogram buckets
    pub histogram_buckets: Vec<f64>,
    /// Retention period (cycles)
    pub retention: u64,
}

impl Default for MetricConfig {
    fn default() -> Self {
        Self {
            max_metrics: 10000,
            window_size: 100,
            enable_histograms: true,
            histogram_buckets: vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0],
            retention: 1000,
        }
    }
}

/// Collector statistics
#[derive(Debug, Clone, Default)]
pub struct CollectorStats {
    /// Total metrics recorded
    pub total_recorded: u64,
    /// Active metrics
    pub active_metrics: u64,
    /// Aggregations count
    pub aggregation_count: u64,
    /// Storage bytes (estimated)
    pub storage_bytes: u64,
}

impl MetricCollector {
    /// Create a new collector
    pub fn new(config: MetricConfig) -> Self {
        Self {
            metrics: BTreeMap::new(),
            name_index: BTreeMap::new(),
            domain_index: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            aggregations: BTreeMap::new(),
            config,
            stats: CollectorStats::default(),
        }
    }

    /// Record a counter increment
    pub fn counter(&mut self, name: &str, source: DomainId, increment: i64) -> u64 {
        if let Some(&id) = self.name_index.get(name) {
            if let Some(metric) = self.metrics.get_mut(&id) {
                if let MetricValue::Integer(ref mut v) = metric.value {
                    *v += increment;
                }
                metric.timestamp = Timestamp::now();
                self.update_aggregation(name, increment as f64);
                return id;
            }
        }

        self.create_metric(
            name.into(),
            MetricType::Counter,
            MetricValue::Integer(increment),
            source,
            BTreeMap::new(),
        )
    }

    /// Record a gauge value
    pub fn gauge(&mut self, name: &str, source: DomainId, value: f64) -> u64 {
        if let Some(&id) = self.name_index.get(name) {
            if let Some(metric) = self.metrics.get_mut(&id) {
                metric.value = MetricValue::Float(value);
                metric.timestamp = Timestamp::now();
                self.update_aggregation(name, value);
                return id;
            }
        }

        self.create_metric(
            name.into(),
            MetricType::Gauge,
            MetricValue::Float(value),
            source,
            BTreeMap::new(),
        )
    }

    /// Observe a histogram value
    pub fn histogram(&mut self, name: &str, source: DomainId, value: f64) -> u64 {
        if let Some(&id) = self.name_index.get(name) {
            if let Some(metric) = self.metrics.get_mut(&id) {
                if let MetricValue::Histogram(ref mut h) = metric.value {
                    h.observe(value);
                }
                metric.timestamp = Timestamp::now();
                self.update_aggregation(name, value);
                return id;
            }
        }

        let histogram = HistogramValue::new(self.config.histogram_buckets.clone());
        let mut hist = histogram;
        hist.observe(value);

        self.create_metric(
            name.into(),
            MetricType::Histogram,
            MetricValue::Histogram(hist),
            source,
            BTreeMap::new(),
        )
    }

    /// Create a new metric
    fn create_metric(
        &mut self,
        name: String,
        metric_type: MetricType,
        value: MetricValue,
        source: DomainId,
        labels: BTreeMap<String, String>,
    ) -> u64 {
        // Check capacity
        if self.metrics.len() >= self.config.max_metrics {
            self.evict_oldest();
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let metric = Metric {
            id,
            name: name.clone(),
            metric_type,
            value,
            source,
            labels,
            timestamp: Timestamp::now(),
        };

        self.metrics.insert(id, metric);
        self.name_index.insert(name.clone(), id);
        self.domain_index.entry(source).or_default().push(id);

        // Initialize aggregation
        if !self.aggregations.contains_key(&name) {
            self.aggregations.insert(
                name.clone(),
                AggregatedMetric::new(name, self.config.window_size),
            );
        }

        self.stats.total_recorded += 1;
        self.stats.active_metrics = self.metrics.len() as u64;

        id
    }

    /// Update aggregation
    fn update_aggregation(&mut self, name: &str, value: f64) {
        let window_size = self.config.window_size;
        if let Some(agg) = self.aggregations.get_mut(name) {
            agg.update(value, window_size);
        }
    }

    /// Evict oldest metric
    fn evict_oldest(&mut self) {
        let oldest = self
            .metrics
            .values()
            .min_by_key(|m| m.timestamp.raw())
            .map(|m| m.id);

        if let Some(id) = oldest {
            if let Some(metric) = self.metrics.remove(&id) {
                self.name_index.remove(&metric.name);
                if let Some(ids) = self.domain_index.get_mut(&metric.source) {
                    ids.retain(|i| *i != id);
                }
            }
        }
    }

    /// Get metric by ID
    pub fn get(&self, id: u64) -> Option<&Metric> {
        self.metrics.get(&id)
    }

    /// Get metric by name
    pub fn get_by_name(&self, name: &str) -> Option<&Metric> {
        self.name_index
            .get(name)
            .and_then(|id| self.metrics.get(id))
    }

    /// Get aggregation
    pub fn get_aggregation(&self, name: &str) -> Option<&AggregatedMetric> {
        self.aggregations.get(name)
    }

    /// Get metrics by domain
    pub fn get_by_domain(&self, domain: DomainId) -> Vec<&Metric> {
        self.domain_index
            .get(&domain)
            .map(|ids| ids.iter().filter_map(|id| self.metrics.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all metric names
    pub fn metric_names(&self) -> Vec<&String> {
        self.name_index.keys().collect()
    }

    /// Get all aggregations
    pub fn all_aggregations(&self) -> &BTreeMap<String, AggregatedMetric> {
        &self.aggregations
    }

    /// Get statistics
    pub fn stats(&self) -> &CollectorStats {
        &self.stats
    }

    /// Reset aggregation for a metric
    pub fn reset_aggregation(&mut self, name: &str) {
        if let Some(agg) = self.aggregations.get_mut(name) {
            *agg = AggregatedMetric::new(agg.name.clone(), self.config.window_size);
        }
    }

    /// Generate report
    pub fn generate_report(&self) -> MetricReport {
        let mut domain_summaries = BTreeMap::new();

        for (domain_id, metric_ids) in &self.domain_index {
            let count = metric_ids.len() as u32;
            let metrics: Vec<_> = metric_ids
                .iter()
                .filter_map(|id| self.metrics.get(id))
                .map(|m| m.name.clone())
                .collect();

            domain_summaries.insert(*domain_id, DomainMetricSummary {
                domain_id: *domain_id,
                metric_count: count,
                metric_names: metrics,
            });
        }

        MetricReport {
            timestamp: Timestamp::now(),
            total_metrics: self.metrics.len() as u64,
            total_aggregations: self.aggregations.len() as u64,
            domain_summaries,
            top_metrics: self.get_top_metrics(10),
        }
    }

    /// Get top metrics by count
    fn get_top_metrics(&self, n: usize) -> Vec<(String, u64)> {
        let mut by_count: Vec<_> = self
            .aggregations
            .iter()
            .map(|(name, agg)| (name.clone(), agg.count))
            .collect();
        by_count.sort_by(|a, b| b.1.cmp(&a.1));
        by_count.truncate(n);
        by_count
    }
}

/// Metric report
#[derive(Debug, Clone)]
pub struct MetricReport {
    /// Timestamp
    pub timestamp: Timestamp,
    /// Total metrics
    pub total_metrics: u64,
    /// Total aggregations
    pub total_aggregations: u64,
    /// Per-domain summaries
    pub domain_summaries: BTreeMap<DomainId, DomainMetricSummary>,
    /// Top metrics by count
    pub top_metrics: Vec<(String, u64)>,
}

/// Domain metric summary
#[derive(Debug, Clone)]
pub struct DomainMetricSummary {
    /// Domain ID
    pub domain_id: DomainId,
    /// Metric count
    pub metric_count: u32,
    /// Metric names
    pub metric_names: Vec<String>,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let config = MetricConfig::default();
        let mut collector = MetricCollector::new(config);

        let domain = DomainId::new(1);
        let id = collector.counter("requests_total", domain, 1);
        assert!(id > 0);

        collector.counter("requests_total", domain, 1);
        collector.counter("requests_total", domain, 1);

        let metric = collector.get(id).unwrap();
        assert_eq!(metric.value, MetricValue::Integer(3));
    }

    #[test]
    fn test_gauge() {
        let config = MetricConfig::default();
        let mut collector = MetricCollector::new(config);

        let domain = DomainId::new(1);
        collector.gauge("cpu_usage", domain, 0.5);
        collector.gauge("cpu_usage", domain, 0.7);

        let metric = collector.get_by_name("cpu_usage").unwrap();
        assert!(matches!(metric.value, MetricValue::Float(v) if (v - 0.7).abs() < 0.001));
    }

    #[test]
    fn test_histogram() {
        let config = MetricConfig::default();
        let mut collector = MetricCollector::new(config);

        let domain = DomainId::new(1);

        for i in 0..100 {
            collector.histogram("latency", domain, i as f64 * 0.01);
        }

        let metric = collector.get_by_name("latency").unwrap();
        if let MetricValue::Histogram(h) = &metric.value {
            assert_eq!(h.count, 100);
        } else {
            panic!("Expected histogram");
        }
    }

    #[test]
    fn test_aggregation() {
        let config = MetricConfig::default();
        let mut collector = MetricCollector::new(config);

        let domain = DomainId::new(1);

        for i in 0..10 {
            collector.gauge("temperature", domain, i as f64);
        }

        let agg = collector.get_aggregation("temperature").unwrap();
        assert_eq!(agg.count, 10);
        assert_eq!(agg.min, 0.0);
        assert_eq!(agg.max, 9.0);
        assert!((agg.mean - 4.5).abs() < 0.01);
    }

    impl PartialEq for MetricValue {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Integer(a), Self::Integer(b)) => a == b,
                (Self::Float(a), Self::Float(b)) => (a - b).abs() < 0.0001,
                _ => false,
            }
        }
    }
}
